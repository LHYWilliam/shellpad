use crate::action::AppAction;
use crate::models::AppData;
use crate::ui::main_screen::MainScreenState;
use crate::ui::widget::TextInput;
use crate::ui::widget::text_input::handle_text_input;
use crossterm::event::KeyEvent;
use super::Panel;

impl MainScreenState {
    /// Handle a key event, returning an action.
    pub fn handle_key(&mut self, key: KeyEvent, data: &AppData) -> AppAction {
        use crossterm::event::KeyCode;

        // Rename mode (takes priority over search)
        if self.rename_mode {
            return match key.code {
                KeyCode::Enter => {
                    let name = self.rename_input.content.clone();
                    let gi = self.group_list.selected;
                    self.rename_mode = false;
                    AppAction::RenameGroup(gi, name)
                }
                KeyCode::Esc => {
                    self.rename_mode = false;
                    AppAction::None
                }
                _ => {
                    handle_text_input(&mut self.rename_input, key);
                    AppAction::None
                }
            };
        }

        // Search mode
        if self.search_mode {
            return match key.code {
                KeyCode::Esc => {
                    self.search_mode = false;
                    self.search_input = TextInput::new(String::new());
                    self.set_list.reset();
                    self.active_panel = Panel::Groups;
                    AppAction::None
                }
                KeyCode::Enter => {
                    let results = data.filter_sets(&self.search_input.content);
                    if let Some((gi, si, _)) = results.get(self.set_list.selected) {
                        self.group_list.selected = *gi;
                        self.set_list.selected = *si;
                        self.active_panel = Panel::Sets;
                    }
                    self.search_mode = false;
                    self.search_input = TextInput::new(String::new());
                    AppAction::None
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.set_list.select_previous();
                    AppAction::None
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    let n = data.filter_sets(&self.search_input.content).len();
                    self.set_list.select_next(n);
                    AppAction::None
                }
                _ => {
                    handle_text_input(&mut self.search_input, key);
                    self.active_panel = Panel::Sets;
                    self.set_list.reset();
                    AppAction::None
                }
            };
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                match self.active_panel {
                    Panel::Groups => self.group_list.select_previous(),
                    Panel::Sets => {
                        if self.visible_sets(data).is_empty() {
                            self.active_panel = Panel::Groups;
                        } else {
                            self.set_list.select_previous();
                        }
                    }
                }
                AppAction::None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                match self.active_panel {
                    Panel::Groups => {
                        self.group_list.select_next(data.groups.len())
                    }
                    Panel::Sets => {
                        let n = self.visible_sets(data).len();
                        if n == 0 {
                            self.active_panel = Panel::Groups;
                        } else {
                            self.set_list.select_next(n);
                        }
                    }
                }
                AppAction::None
            }
            KeyCode::Left => {
                match self.active_panel {
                    Panel::Sets => {
                        self.active_panel = Panel::Groups
                    }
                    Panel::Groups => { /* already on the leftmost panel */ }
                }
                AppAction::None
            }
            KeyCode::Right => {
                match self.active_panel {
                    Panel::Groups => {
                        let has_sets = self
                            .selected_group_idx(data)
                            .map(|gi| !data.groups[gi].sets.is_empty())
                            .unwrap_or(false);
                        if has_sets {
                            self.active_panel = Panel::Sets;
                        }
                    }
                    Panel::Sets => { /* already on the rightmost panel */ }
                }
                AppAction::None
            }
            KeyCode::Enter => {
                if self.active_panel == Panel::Sets
                    && let Some((gi, si)) = self.selected_set_idx(data)
                {
                    return AppAction::ExecuteSet(gi, si);
                }
                AppAction::None
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if self.active_panel == Panel::Sets
                    && let Some((gi, si)) = self.selected_set_idx(data)
                {
                    return AppAction::EditSet(gi, si);
                }
                AppAction::None
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if let Some(gi) = self.selected_group_idx(data) {
                    AppAction::NewSet(gi)
                } else {
                    AppAction::None
                }
            }
            KeyCode::Char('d') => {
                if self.active_panel == Panel::Sets
                    && let Some((gi, si)) = self.selected_set_idx(data)
                {
                    return AppAction::DeleteSet(gi, si);
                }
                AppAction::None
            }
            KeyCode::Char('D') => {
                if self.active_panel == Panel::Groups
                    && let Some(gi) = self.selected_group_idx(data)
                {
                    return AppAction::DeleteGroup(gi);
                }
                AppAction::None
            }
            KeyCode::Char('g') => AppAction::NewGroup,
            KeyCode::Char('R') => {
                if self.active_panel == Panel::Groups
                    && let Some(gi) = self.selected_group_idx(data)
                {
                    let current = data.groups[gi].name.clone();
                    self.rename_mode = true;
                    self.rename_input = TextInput::new(current);
                }
                AppAction::None
            }
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.search_input.content.clear();
                self.set_list.reset();
                self.active_panel = Panel::Sets;
                AppAction::None
            }
            KeyCode::Char('h') | KeyCode::Char('H')
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                AppAction::Help
            }
            KeyCode::Char('h') | KeyCode::Char('H') => AppAction::None,
            KeyCode::Esc | KeyCode::Char('q') => {
                if key.code == KeyCode::Esc {
                    AppAction::None
                } else {
                    AppAction::Quit
                }
            }
            _ => AppAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::AppAction;
    use crate::models::{AppData, CommandSet, Group};
    use crate::test_utils::make_key;
    use crossterm::event::KeyCode;

    fn make_data() -> AppData {
        let mut g = Group::new("Test Group".to_string());
        let set = CommandSet::new("Test Set".to_string(), g.id);
        g.sets.push(set);
        AppData { groups: vec![g] }
    }

    #[test]
    fn test_nav_down_returns_none() {
        let mut state = MainScreenState::new();
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Down), &data);
        assert!(matches!(action, AppAction::None));
        assert_eq!(state.group_list.selected, 0); // stays at first
    }

    #[test]
    fn test_enter_on_set_returns_execute_set() {
        let mut state = MainScreenState::new();
        state.active_panel = Panel::Sets;
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Enter), &data);
        assert!(matches!(action, AppAction::ExecuteSet(0, 0)));
    }

    #[test]
    fn test_e_returns_edit_set() {
        let mut state = MainScreenState::new();
        state.active_panel = Panel::Sets;
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('e')), &data);
        assert!(matches!(action, AppAction::EditSet(0, 0)));
    }

    #[test]
    fn test_n_returns_new_set() {
        let mut state = MainScreenState::new();
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('n')), &data);
        assert!(matches!(action, AppAction::NewSet(0)));
    }

    #[test]
    fn test_d_returns_delete_set() {
        let mut state = MainScreenState::new();
        state.active_panel = Panel::Sets;
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('d')), &data);
        assert!(matches!(action, AppAction::DeleteSet(0, 0)));
    }

    #[test]
    fn test_big_d_returns_delete_group() {
        let mut state = MainScreenState::new();
        state.active_panel = Panel::Groups;
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('D')), &data);
        assert!(matches!(action, AppAction::DeleteGroup(0)));
    }

    #[test]
    fn test_g_returns_new_group() {
        let mut state = MainScreenState::new();
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('g')), &data);
        assert!(matches!(action, AppAction::NewGroup));
    }

    #[test]
    fn test_q_returns_quit() {
        let mut state = MainScreenState::new();
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('q')), &data);
        assert!(matches!(action, AppAction::Quit));
    }

    #[test]
    fn test_question_mark_is_delegated_to_app() {
        let mut state = MainScreenState::new();
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('?')), &data);
        // '?' is now a global shortcut in App::handle_key, not in MainScreenState
        assert!(matches!(action, AppAction::None));
    }

    #[test]
    fn test_slash_enters_search_mode() {
        let mut state = MainScreenState::new();
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('/')), &data);
        assert!(matches!(action, AppAction::None));
        assert!(state.search_mode);
    }
}
