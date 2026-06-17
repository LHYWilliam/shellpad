//! Shared test helpers for unit and integration tests.
//!
//! This module is `#[cfg(test)]` — only compiled during `cargo test`.

use crate::app::App;
use crate::app::ExecutionState;
use crate::app::toast::ToastManager;
use crate::mode::AppMode;
use crate::models::{AppData, CommandSet, Group};
use crate::ui::main_screen::MainScreenState;
use crate::ui::theme::Theme;
use crate::ui::variable_screen::VariableScreenState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Shorthand for creating a key event with no modifiers.
pub(crate) fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

/// Create a minimal App for testing, with empty data and Main mode.
pub(crate) fn make_app() -> App {
    App {
        data: AppData::empty(),
        mode: AppMode::Main,
        running: true,
        main_screen: MainScreenState::new(),
        detail_screen: None,
        execution_state: ExecutionState::Idle { pending_set: None },
        prev_mode: None,
        variable_screen: VariableScreenState::new(),
        theme: Theme::default_dark(),
        toasts: ToastManager::new(),
    }
}

/// Create AppData with one group containing one set (no commands).
pub(crate) fn make_data_with_one_group() -> AppData {
    let mut g = Group::new("Deploy".to_string());
    let set = CommandSet::new("Prod".to_string(), g.id);
    g.sets.push(set);
    AppData { groups: vec![g] }
}
