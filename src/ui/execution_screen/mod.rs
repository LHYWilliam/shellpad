use crate::action::AppAction;

pub(crate) mod events;
pub(crate) mod render;

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
    output_lines: Vec<String>,
    duration_ms: Option<u128>,
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
}

impl ExecutionScreenState {
    pub fn new(set_name: String, commands: &[crate::models::Command]) -> Self {
        let cmd_states: Vec<CmdState> = commands
            .iter()
            .map(|c| CmdState {
                status: CmdStatus::Pending,
                command: c.command.clone(),
                output_lines: Vec::new(),
                duration_ms: None,
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
}
