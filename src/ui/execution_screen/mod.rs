use crate::action::AppAction;
use crate::ui::widget::TextInput;
use std::collections::VecDeque;

pub(crate) mod events;
pub(crate) mod render;

/// Max output lines per command. When exceeded, old lines are discarded
/// to prevent OOM during long-running or high-output commands.
pub(crate) const MAX_OUTPUT_LINES: usize = 10_000;

/// Number of lines to scroll per PageUp/PageDown.
const PAGE_SIZE: usize = 20;

/// Scroll tracking mode — exactly one variant is active.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ScrollMode {
    /// Tail-follow: auto-scrolls to show latest output at bottom (default).
    Follow,
    /// Browse: locked to a specific command's header line (←/→).
    Browse { index: usize },
    /// Free: user-scrolled to an arbitrary offset (↑/↓/PgUp/PgDn).
    Free { offset: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdStatus {
    Pending,
    Running,
    Success,
    Failure,
    Skipped,
}

pub(crate) struct CmdState {
    pub(crate) status: CmdStatus,
    command: String,
    output_lines: VecDeque<String>,
    duration_ms: Option<u128>,
    pub(crate) truncated: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) defer: bool,
}

/// Output search state — only active while the search bar is open.
#[derive(Debug, Clone)]
enum SearchState {
    Inactive,
    Active {
        input: TextInput,
        matches: Vec<usize>,
        current: usize,
        prev_scroll: ScrollMode,
        prev_offset: usize,
    },
}

pub struct ExecutionScreenState {
    pub set_name: String,
    pub(crate) cmd_states: Vec<CmdState>,
    pub current_index: usize,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub completed: bool,
    pub paused: bool,
    pub deferring: bool,
    pub continue_from: Option<usize>,
    pub total_duration_ms: Option<u128>,
    /// Scroll mode — one of Follow / Browse{index} / Free{offset}.
    scroll: ScrollMode,
    /// Cached offset from the last render frame — used to preserve visual
    /// position when transitioning from Follow mode (which computes its
    /// offset from content_height at render time).
    last_offset: usize,
    pub(crate) output_truncated: bool,
    pub(crate) search: SearchState,
}

impl ExecutionScreenState {
    pub fn new(set_name: String, commands: &[crate::models::Command]) -> Self {
        let cmd_states: Vec<CmdState> = commands
            .iter()
            .map(|c| CmdState {
                status: CmdStatus::Pending,
                command: c.command.clone(),
                output_lines: VecDeque::new(),
                duration_ms: None,
                truncated: false,
                exit_code: None,
                defer: false,
            })
            .collect();

        Self {
            set_name,
            total: cmd_states.len(),
            cmd_states,
            current_index: 0,
            succeeded: 0,
            failed: 0,
            skipped: 0,
            completed: false,
            paused: false,
            deferring: false,
            continue_from: None,
            total_duration_ms: None,
            scroll: ScrollMode::Follow,
            last_offset: 0,
            output_truncated: false,
            search: SearchState::Inactive,
        }
    }

    /// Current on-screen scroll offset, computed from the active mode.
    fn scroll_offset(&self, content_height: u16) -> usize {
        match &self.scroll {
            ScrollMode::Follow => {
                let total = self.items_total();
                let vis = content_height as usize;
                total.saturating_sub(vis)
            }
            ScrollMode::Browse { index } => self.items_offset_for_command(*index),
            ScrollMode::Free { offset } => *offset,
        }
    }

    /// Current scroll position regardless of mode — used as the base offset
    /// when transitioning to Free scrolling. For Follow mode, returns the
    /// tail position (content end); for Browse, the command's header offset.
    fn scroll_base(&self) -> usize {
        match &self.scroll {
            ScrollMode::Follow => self.last_offset,
            ScrollMode::Browse { index } => self.items_offset_for_command(*index),
            ScrollMode::Free { offset } => *offset,
        }
    }

    /// The command index currently being browsed, if any.
    fn browsing_index(&self) -> Option<usize> {
        match self.scroll {
            ScrollMode::Browse { index } => Some(index),
            _ => None,
        }
    }

    /// Transition from Follow/Browse to Free scrolling with delta line offset.
    fn scroll_by(&mut self, delta: isize) {
        let total = self.items_total();
        let base = self.scroll_base();
        let offset = if delta < 0 {
            base.saturating_sub(delta.unsigned_abs())
        } else {
            base.saturating_add(delta as usize)
        }
        .min(total.saturating_sub(1));
        self.scroll = ScrollMode::Free { offset };
    }

    /// Find the nearest non-Pending command from `from` in direction `delta`.
    fn nearest_non_pending(&self, from: usize, delta: isize) -> Option<usize> {
        let len = self.cmd_states.len() as isize;
        if len == 0 {
            return None;
        }
        let mut pos = from as isize + delta;
        while pos >= 0 && pos < len {
            let i = pos as usize;
            if self.cmd_states[i].status != CmdStatus::Pending {
                return Some(i);
            }
            pos += delta;
        }
        None
    }

    /// Browse in direction `delta`, falling back to the current command
    /// when there are no adjacent commands (e.g. single-command sets).
    fn browse_command(&mut self, delta: isize) {
        let browsing = self.browsing_index().unwrap_or(self.current_index);
        let idx = self.nearest_non_pending(browsing, delta).or_else(|| {
            if browsing < self.cmd_states.len()
                && self.cmd_states[browsing].status != CmdStatus::Pending
            {
                Some(browsing)
            } else {
                None
            }
        });
        if let Some(idx) = idx {
            self.scroll = ScrollMode::Browse { index: idx };
        }
    }

    /// Handle key events.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> AppAction {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('q') => AppAction::BackToMain,
            KeyCode::Char('s') if !self.completed && !self.paused && !self.deferring => {
                AppAction::Pause
            }
            KeyCode::Char('n') if self.paused && !self.completed && !self.deferring => {
                AppAction::Continue
            }
            KeyCode::Char('c')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
                    && !self.completed
                    && !self.deferring =>
            {
                AppAction::Abort
            }
            KeyCode::Char('r') if self.completed => AppAction::ReExec,
            KeyCode::Left => {
                self.browse_command(-1);
                AppAction::None
            }
            KeyCode::Right => {
                self.browse_command(1);
                AppAction::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_by(-1);
                AppAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_by(1);
                AppAction::None
            }
            KeyCode::PageUp => {
                self.scroll_by(-(PAGE_SIZE as isize));
                AppAction::None
            }
            KeyCode::PageDown => {
                self.scroll_by(PAGE_SIZE as isize);
                AppAction::None
            }
            KeyCode::Char('z') => {
                self.scroll = match &self.scroll {
                    ScrollMode::Browse { .. } => ScrollMode::Follow,
                    ScrollMode::Follow => ScrollMode::Free { offset: 0 },
                    ScrollMode::Free { .. } => ScrollMode::Follow,
                };
                AppAction::None
            }
            _ => AppAction::None,
        }
    }

    /// Total rendered items including summary footer.
    pub(crate) fn items_total(&self) -> usize {
        let mut total = self.items_offset_for_command(self.cmd_states.len());
        if self.completed {
            total += 2; // blank line + summary line
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::make_key;

    fn make_test_state() -> ExecutionScreenState {
        let cmds: Vec<crate::models::Command> = vec![
            crate::models::Command {
                position: 0,
                command: "cmd1".to_string(),
            },
            crate::models::Command {
                position: 1,
                command: "cmd2".to_string(),
            },
        ];
        ExecutionScreenState::new("test".to_string(), &cmds)
    }

    #[test]
    fn test_scroll_up_enters_free_scroll_mode() {
        let mut state = make_test_state();
        // 2 commands, 0 output → items_total=3, max offset=2
        state.scroll = ScrollMode::Free { offset: 1 };

        let action = state.handle_key(make_key(crossterm::event::KeyCode::Up));
        assert!(matches!(action, AppAction::None));
        assert!(matches!(state.scroll, ScrollMode::Free { offset: 0 }));
    }

    #[test]
    fn test_scroll_down_is_clamped_to_content() {
        let mut state = make_test_state();
        // 2 commands, 0 output → items_total = 3 → max scroll_offset = 2
        state.scroll = ScrollMode::Free { offset: 1 };

        let _ = state.handle_key(make_key(crossterm::event::KeyCode::Down));
        assert!(matches!(state.scroll, ScrollMode::Free { offset: 2 }));

        let _ = state.handle_key(make_key(crossterm::event::KeyCode::Down));
        // stays clamped at 2
        assert!(matches!(state.scroll, ScrollMode::Free { offset: 2 }));
    }

    #[test]
    fn test_page_up_down() {
        let mut state = make_test_state();
        // 2 commands, 0 output → items_total=3, max offset=2
        state.scroll = ScrollMode::Free { offset: 2 };

        let _ = state.handle_key(make_key(crossterm::event::KeyCode::PageUp));
        assert!(matches!(state.scroll, ScrollMode::Free { offset: 0 }));

        let _ = state.handle_key(make_key(crossterm::event::KeyCode::PageDown));
        assert!(matches!(state.scroll, ScrollMode::Free { offset: 2 }));
    }

    #[test]
    fn test_jk_vim_keys() {
        let mut state = make_test_state();
        // 2 commands, 0 output → items_total=3, max offset=2
        state.scroll = ScrollMode::Free { offset: 1 };

        let _ = state.handle_key(make_key(crossterm::event::KeyCode::Char('j')));
        assert!(matches!(state.scroll, ScrollMode::Free { offset: 2 }));

        let _ = state.handle_key(make_key(crossterm::event::KeyCode::Char('k')));
        assert!(matches!(state.scroll, ScrollMode::Free { offset: 1 }));
    }
}
