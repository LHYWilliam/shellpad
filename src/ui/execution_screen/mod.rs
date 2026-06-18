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
pub(crate) enum ScrollMode {
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
pub(crate) enum SearchState {
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
    /// Visible content height from last render frame — used by scroll_to_match.
    visible_height: usize,
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
            visible_height: 0,
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

        // ---- Search mode guard ----
        if matches!(self.search, SearchState::Active { .. }) {
            return self.handle_search_key(key);
        }

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
            KeyCode::Char('/') => {
                self.search = SearchState::Active {
                    input: TextInput::new(String::new()),
                    matches: Vec::new(),
                    current: 0,
                    prev_scroll: self.scroll.clone(),
                    prev_offset: self.last_offset,
                };
                AppAction::None
            }
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

    fn handle_search_key(&mut self, key: crossterm::event::KeyEvent) -> AppAction {
        use crossterm::event::KeyCode;

        // Snapshot query before editing
        let before = match &self.search {
            SearchState::Active { input, .. } => input.content.clone(),
            SearchState::Inactive => return AppAction::None,
        };

        // Text editing keys → delegate to handle_text_input
        let is_edit_key = matches!(
            key.code,
            KeyCode::Char(_)
                | KeyCode::Backspace
                | KeyCode::Delete
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Home
                | KeyCode::End
        );
        if is_edit_key && let SearchState::Active { ref mut input, .. } = self.search {
            crate::ui::widget::text_input::handle_text_input(input, key);
        }

        // If query changed, recalculate matches
        let query_changed = match &self.search {
            SearchState::Active { input, .. } => input.content != before,
            SearchState::Inactive => false,
        };
        if query_changed {
            self.refresh_matches();
            return AppAction::None;
        }

        match key.code {
            KeyCode::Enter => {
                self.search = SearchState::Inactive;
                AppAction::None
            }
            KeyCode::Up => {
                let has_prev = match &self.search {
                    SearchState::Active { current, .. } => *current > 0,
                    _ => false,
                };
                if has_prev {
                    if let SearchState::Active { current, .. } = &mut self.search {
                        *current -= 1;
                    }
                    self.scroll_to_match(
                        if let SearchState::Active { current, .. } = &self.search {
                            *current
                        } else {
                            0
                        },
                    );
                }
                AppAction::None
            }
            KeyCode::Down => {
                let has_next = match &self.search {
                    SearchState::Active {
                        matches, current, ..
                    } => *current + 1 < matches.len(),
                    _ => false,
                };
                if has_next {
                    if let SearchState::Active { current, .. } = &mut self.search {
                        *current += 1;
                    }
                    self.scroll_to_match(
                        if let SearchState::Active { current, .. } = &self.search {
                            *current
                        } else {
                            0
                        },
                    );
                }
                AppAction::None
            }
            KeyCode::Esc => {
                let (prev_scroll, prev_offset) = if let SearchState::Active {
                    prev_scroll,
                    prev_offset,
                    ..
                } = &self.search
                {
                    (prev_scroll.clone(), *prev_offset)
                } else {
                    (ScrollMode::Follow, 0)
                };
                self.search = SearchState::Inactive;
                self.scroll = prev_scroll;
                self.last_offset = prev_offset;
                AppAction::None
            }
            _ => AppAction::None,
        }
    }

    fn refresh_matches(&mut self) {
        let query = match &self.search {
            SearchState::Active { input, .. } => input.content.clone(),
            SearchState::Inactive => return,
        };
        if query.is_empty() {
            if let SearchState::Active {
                matches, current, ..
            } = &mut self.search
            {
                matches.clear();
                *current = 0;
            }
            return;
        }

        // Collect match indices first (borrows self immutably via cmd_states)
        let new_matches: Vec<usize> = {
            let mut v = Vec::new();
            for (i, state) in self.cmd_states.iter().enumerate() {
                for (li, line) in state.output_lines.iter().enumerate() {
                    if line.contains(query.as_str()) {
                        v.push(self.flat_output_index(i, li));
                    }
                }
            }
            v
        };

        // Update search state
        let has_matches = !new_matches.is_empty();
        if let SearchState::Active {
            matches, current, ..
        } = &mut self.search
        {
            *matches = new_matches;
            *current = 0;
        }

        if has_matches {
            self.scroll_to_match(0);
        }
    }

    fn flat_output_index(&self, cmd_idx: usize, line_idx: usize) -> usize {
        self.items_offset_for_command(cmd_idx)
            + 1 // command header line
            + if self.cmd_states[cmd_idx].truncated { 1 } else { 0 }
            + line_idx
    }

    fn scroll_to_match(&mut self, match_idx: usize) {
        let target = match &self.search {
            SearchState::Active { matches, .. } if match_idx < matches.len() => {
                Some(matches[match_idx])
            }
            _ => None,
        };
        let Some(target) = target else { return };

        let vis = self.visible_height.max(1);
        let top = self.last_offset;
        let bottom = top.saturating_add(vis).saturating_sub(1);

        if target >= top && target <= bottom {
            // Already visible — don't scroll
            return;
        }

        let new_offset = if target < top {
            target
        } else {
            target.saturating_sub(vis.saturating_sub(1))
        };

        self.scroll = ScrollMode::Free { offset: new_offset };
        self.last_offset = new_offset;
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
    use crossterm::event::KeyCode;

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

    #[test]
    fn test_slash_enters_search_mode() {
        let mut state = make_test_state();
        let action = state.handle_key(make_key(KeyCode::Char('/')));
        assert!(matches!(action, AppAction::None));
        assert!(matches!(state.search, SearchState::Active { .. }));
    }

    #[test]
    fn test_esc_exits_search_and_restores_scroll() {
        let mut state = make_test_state();
        state.scroll = ScrollMode::Free { offset: 5 };
        // Enter search
        let _ = state.handle_key(make_key(KeyCode::Char('/')));
        assert!(matches!(state.search, SearchState::Active { .. }));
        // Exit search
        let action = state.handle_key(make_key(KeyCode::Esc));
        assert!(matches!(action, AppAction::None));
        assert!(matches!(state.search, SearchState::Inactive));
        assert_eq!(state.scroll, ScrollMode::Free { offset: 5 });
    }

    fn enter_search(state: &mut ExecutionScreenState) {
        let _ = state.handle_key(make_key(KeyCode::Char('/')));
    }

    #[test]
    fn test_char_input_updates_query() {
        let mut state = make_test_state();
        enter_search(&mut state);
        let action = state.handle_key(make_key(KeyCode::Char('e')));
        assert!(matches!(action, AppAction::None));
        if let SearchState::Active { ref input, .. } = state.search {
            assert_eq!(input.content, "e");
        } else {
            panic!("expected Active");
        }
    }

    #[test]
    fn test_backspace_removes_last_char() {
        let mut state = make_test_state();
        enter_search(&mut state);
        let _ = state.handle_key(make_key(KeyCode::Char('a')));
        let _ = state.handle_key(make_key(KeyCode::Char('b')));
        let _ = state.handle_key(make_key(KeyCode::Backspace));
        if let SearchState::Active { ref input, .. } = state.search {
            assert_eq!(input.content, "a");
        } else {
            panic!("expected Active");
        }
    }

    #[test]
    fn test_delete_removes_at_cursor() {
        let mut state = make_test_state();
        enter_search(&mut state);
        let _ = state.handle_key(make_key(KeyCode::Char('a')));
        let _ = state.handle_key(make_key(KeyCode::Char('b')));
        // Move cursor left, then delete the character at cursor
        let _ = state.handle_key(make_key(KeyCode::Left));
        let _ = state.handle_key(make_key(KeyCode::Delete));
        if let SearchState::Active { ref input, .. } = state.search {
            assert_eq!(input.content, "a");
        } else {
            panic!("expected Active");
        }
    }

    #[test]
    fn test_cursor_keys_move_without_changing_content() {
        let mut state = make_test_state();
        enter_search(&mut state);
        let _ = state.handle_key(make_key(KeyCode::Char('a')));
        let _ = state.handle_key(make_key(KeyCode::Char('b')));
        // Move left
        let _ = state.handle_key(make_key(KeyCode::Left));
        if let SearchState::Active { input, .. } = state.search {
            assert_eq!(input.content, "ab");
            assert_eq!(input.cursor, 1);
        } else {
            panic!("expected Active");
        }
    }

    fn make_state_with_output() -> ExecutionScreenState {
        let cmds: Vec<crate::models::Command> = vec![crate::models::Command {
            position: 0,
            command: "cmd1".to_string(),
        }];
        let mut state = ExecutionScreenState::new("test".to_string(), &cmds);
        state.cmd_states[0]
            .output_lines
            .push_back("line one".to_string());
        state.cmd_states[0]
            .output_lines
            .push_back("error: something failed".to_string());
        state.cmd_states[0]
            .output_lines
            .push_back("line three".to_string());
        state
    }

    #[test]
    fn test_enter_with_matches_enters_free_mode() {
        let mut state = make_state_with_output();
        enter_search(&mut state);
        let _ = state.handle_key(make_key(KeyCode::Char('e')));
        let _ = state.handle_key(make_key(KeyCode::Char('r')));
        let action = state.handle_key(make_key(KeyCode::Enter));
        assert!(matches!(action, AppAction::None));
        assert!(matches!(state.search, SearchState::Inactive));
        assert_eq!(state.scroll, ScrollMode::Free { offset: 2 });
    }

    #[test]
    fn test_enter_with_no_matches_just_exits() {
        let mut state = make_state_with_output();
        enter_search(&mut state);
        let _ = state.handle_key(make_key(KeyCode::Char('z')));
        let _ = state.handle_key(make_key(KeyCode::Char('z')));
        let action = state.handle_key(make_key(KeyCode::Enter));
        assert!(matches!(action, AppAction::None));
        assert!(matches!(state.search, SearchState::Inactive));
        assert!(!matches!(state.scroll, ScrollMode::Free { .. }));
    }

    #[test]
    fn test_flat_output_index_calculates_correctly() {
        let state = make_state_with_output();
        assert_eq!(state.flat_output_index(0, 0), 1);
        assert_eq!(state.flat_output_index(0, 1), 2);
        assert_eq!(state.flat_output_index(0, 2), 3);
    }

    #[test]
    fn test_empty_query_produces_no_matches() {
        let mut state = make_state_with_output();
        enter_search(&mut state);
        let _ = state.handle_key(make_key(KeyCode::Char('x')));
        let _ = state.handle_key(make_key(KeyCode::Backspace));
        if let SearchState::Active { matches, .. } = state.search {
            assert!(matches.is_empty());
        } else {
            panic!("expected Active");
        }
    }

    #[test]
    fn test_up_down_navigates_matches() {
        let mut state = make_state_with_output();
        enter_search(&mut state);
        let _ = state.handle_key(make_key(KeyCode::Char('l')));
        let _ = state.handle_key(make_key(KeyCode::Char('i')));
        let _ = state.handle_key(make_key(KeyCode::Char('n')));
        let _ = state.handle_key(make_key(KeyCode::Char('e')));
        if let SearchState::Active {
            matches, current, ..
        } = &state.search
        {
            assert_eq!(matches.len(), 2);
            assert_eq!(*current, 0);
        } else {
            panic!("expected Active");
        }

        let _ = state.handle_key(make_key(KeyCode::Down));
        if let SearchState::Active { current, .. } = &state.search {
            assert_eq!(*current, 1);
        }

        let _ = state.handle_key(make_key(KeyCode::Up));
        if let SearchState::Active { current, .. } = &state.search {
            assert_eq!(*current, 0);
        }
    }

    #[test]
    fn test_up_at_first_down_at_last_match_clamps() {
        let mut state = make_state_with_output();
        enter_search(&mut state);
        let _ = state.handle_key(make_key(KeyCode::Char('l')));
        let _ = state.handle_key(make_key(KeyCode::Char('i')));
        let _ = state.handle_key(make_key(KeyCode::Char('n')));
        let _ = state.handle_key(make_key(KeyCode::Char('e')));

        let _ = state.handle_key(make_key(KeyCode::Up));
        if let SearchState::Active { current, .. } = &state.search {
            assert_eq!(*current, 0);
        }

        let _ = state.handle_key(make_key(KeyCode::Down));
        let last = if let SearchState::Active {
            matches, current, ..
        } = &state.search
        {
            let l = matches.len() - 1;
            assert_eq!(*current, l);
            l
        } else {
            return;
        };

        let _ = state.handle_key(make_key(KeyCode::Down));
        if let SearchState::Active { current, .. } = &state.search {
            assert_eq!(*current, last);
        }
    }

    fn make_search_state_with_matches() -> ExecutionScreenState {
        let mut state = make_state_with_output();
        state.search = SearchState::Active {
            input: TextInput::new("line".to_string()),
            matches: vec![1, 3],
            current: 0,
            prev_scroll: ScrollMode::Follow,
            prev_offset: 0,
        };
        state.last_offset = 0;
        state.visible_height = 10;
        state.scroll = ScrollMode::Free { offset: 0 };
        state
    }

    #[test]
    fn test_scroll_to_match_visible_no_scroll() {
        let mut state = make_search_state_with_matches();
        state.scroll_to_match(0);
        assert_eq!(state.scroll, ScrollMode::Free { offset: 0 });
    }

    #[test]
    fn test_scroll_to_match_above_viewport() {
        let mut state = make_search_state_with_matches();
        state.last_offset = 5;
        state.scroll_to_match(0);
        assert_eq!(state.scroll, ScrollMode::Free { offset: 1 });
    }

    #[test]
    fn test_scroll_to_match_below_viewport() {
        let mut state = make_search_state_with_matches();
        state.last_offset = 0;
        state.visible_height = 3;
        state.scroll_to_match(1);
        assert_eq!(state.scroll, ScrollMode::Free { offset: 1 });
    }

    #[test]
    fn test_scroll_to_match_zero_height_no_panic() {
        let mut state = make_search_state_with_matches();
        state.visible_height = 0;
        state.scroll_to_match(1);
    }

    #[test]
    fn test_items_offset_defer_boundary_known_undercount() {
        // Known issue: render pushes an extra empty line before defer-boundary
        // separators (2 items total), but items_offset_for_command counts only 1.
        // This makes flat_output_index off by 1 for commands after defer boundaries.
        let cmds: Vec<crate::models::Command> = vec![
            crate::models::Command {
                position: 0,
                command: "normal".to_string(),
            },
            crate::models::Command {
                position: 1,
                command: "deferred".to_string(),
            },
        ];
        let mut state = ExecutionScreenState::new("test".to_string(), &cmds);
        state.cmd_states[1].defer = true;
        state.cmd_states[0]
            .output_lines
            .push_back("line0".to_string());
        state.cmd_states[1]
            .output_lines
            .push_back("line1".to_string());

        // Real item layout for cmd0: [0]=hdr, [1]=line0, [2]=(empty), [3]=thick sep
        // So items before cmd1 header = 4. But items_offset_for_command(1) returns 3.
        // The correct value should be 4 — this test documents the current buggy value.
        let offset = state.items_offset_for_command(1);
        assert_eq!(
            offset, 3,
            "known bug: undercounts defer-boundary empty line"
        );

        // flat_output_index inherits the error:
        // bug value: 3 + 1 + 0 + 0 = 4, correct value: 4 + 1 + 0 + 0 = 5
        assert_eq!(state.flat_output_index(1, 0), 4);
    }
}
