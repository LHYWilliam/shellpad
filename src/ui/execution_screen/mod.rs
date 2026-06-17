use crate::action::AppAction;
use std::collections::VecDeque;

pub(crate) mod events;
pub(crate) mod render;

/// Max output lines per command. When exceeded, old lines are discarded
/// to prevent OOM during long-running or high-output commands.
pub(crate) const MAX_OUTPUT_LINES: usize = 10_000;

/// Number of lines to scroll per PageUp/PageDown.
const PAGE_SIZE: usize = 20;

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
    pub continue_from: Option<usize>,
    pub total_duration_ms: Option<u128>,
    pub auto_scroll: bool,
    pub scroll_offset: usize,
    pub focus_index: Option<usize>,
    pub(crate) output_truncated: bool,
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
            continue_from: None,
            total_duration_ms: None,
            auto_scroll: true,
            scroll_offset: 0,
            focus_index: None,
            output_truncated: false,
        }
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

    /// Handle key events.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> AppAction {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('q') => AppAction::BackToMain,
            KeyCode::Char('c')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                AppAction::SkipCurrent
            }
            KeyCode::Char('s') if !self.completed => AppAction::SkipCurrent,
            KeyCode::Char('n') if self.completed && self.continue_from.is_some() => {
                let start = self.continue_from.unwrap_or(0);
                AppAction::ContinueFrom(start)
            }
            KeyCode::Char('r') if self.completed => AppAction::ReExec,
            KeyCode::Left => {
                let target = self.focus_index.unwrap_or(self.current_index);
                if let Some(idx) = self.nearest_non_pending(target, -1) {
                    self.focus_index = Some(idx);
                    self.auto_scroll = false;
                }
                AppAction::None
            }
            KeyCode::Right => {
                let target = self.focus_index.unwrap_or(self.current_index);
                if let Some(idx) = self.nearest_non_pending(target, 1) {
                    self.focus_index = Some(idx);
                    self.auto_scroll = false;
                }
                AppAction::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.focus_index = None;
                self.auto_scroll = false;
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                AppAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.focus_index = None;
                self.auto_scroll = false;
                self.scroll_offset = self.scroll_offset.saturating_add(1);
                self.clamp_scroll_offset();
                AppAction::None
            }
            KeyCode::PageUp => {
                self.focus_index = None;
                self.auto_scroll = false;
                self.scroll_offset = self.scroll_offset.saturating_sub(PAGE_SIZE);
                AppAction::None
            }
            KeyCode::PageDown => {
                self.focus_index = None;
                self.auto_scroll = false;
                self.scroll_offset = self.scroll_offset.saturating_add(PAGE_SIZE);
                self.clamp_scroll_offset();
                AppAction::None
            }
            KeyCode::Char('z') => {
                if self.focus_index.is_some() {
                    self.focus_index = None;
                    self.auto_scroll = true;
                } else {
                    self.auto_scroll = !self.auto_scroll;
                }
                AppAction::None
            }
            _ => AppAction::None,
        }
    }

    /// Clamp scroll_offset to valid range [0, items_total-1].
    fn clamp_scroll_offset(&mut self) {
        let max = self.items_total().saturating_sub(1);
        if self.scroll_offset > max {
            self.scroll_offset = max;
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
        state.scroll_offset = 10;
        state.focus_index = Some(0);
        state.auto_scroll = true;

        let action = state.handle_key(make_key(crossterm::event::KeyCode::Up));
        assert!(matches!(action, AppAction::None));
        assert_eq!(state.focus_index, None);
        assert!(!state.auto_scroll);
        assert_eq!(state.scroll_offset, 9);
    }

    #[test]
    fn test_scroll_down_is_clamped_to_content() {
        let mut state = make_test_state();
        // 2 commands, 0 output → items_total = 3 → max scroll_offset = 2
        state.scroll_offset = 5;

        let _ = state.handle_key(make_key(crossterm::event::KeyCode::Down));
        assert_eq!(state.scroll_offset, 2);
        assert!(!state.auto_scroll);
    }

    #[test]
    fn test_page_up_down() {
        let mut state = make_test_state();
        state.scroll_offset = 50;

        let _ = state.handle_key(make_key(crossterm::event::KeyCode::PageUp));
        assert_eq!(state.scroll_offset, 30);

        let _ = state.handle_key(make_key(crossterm::event::KeyCode::PageDown));
        // items_total = 3, max = 2
        assert_eq!(state.scroll_offset, 2);
    }

    #[test]
    fn test_jk_vim_keys() {
        let mut state = make_test_state();
        state.scroll_offset = 10;

        let _ = state.handle_key(make_key(crossterm::event::KeyCode::Char('j')));
        // items_total = 3, max = 2
        assert_eq!(state.scroll_offset, 2);

        let _ = state.handle_key(make_key(crossterm::event::KeyCode::Char('k')));
        assert_eq!(state.scroll_offset, 1);
    }
}
