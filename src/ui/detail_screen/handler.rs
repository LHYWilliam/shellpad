use super::{DetailFocus, DetailScreenState};
use crate::action::AppAction;
use super::editor::{handle_command_edit, handle_variable_edit};
use crate::ui::widget::text_input::handle_text_input;
use crate::ui::widget::{InlineEdit, ScrollableList, TextInput};
use crossterm::event::KeyEvent;

impl DetailScreenState {
    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyEvent) -> AppAction {
        use crossterm::event::KeyCode;

        // Handle inline editing
        if let Some(idx) = self.var_edit.editing {
            return handle_variable_edit(
                &mut self.var_edit,
                key,
                idx,
                &mut self.set.variables,
                &mut self.variable_list,
            );
        }
        if let Some(idx) = self.cmd_edit.editing {
            return handle_command_edit(
                &mut self.cmd_edit,
                key,
                idx,
                &mut self.set.commands,
                &mut self.command_list,
            );
        }

        match key.code {
            KeyCode::Tab | KeyCode::Char('\t') => {
                self.commit_name_edit();
                self.focus = match self.focus {
                    DetailFocus::Name => DetailFocus::Group,
                    DetailFocus::Group => DetailFocus::Shell,
                    DetailFocus::Shell => DetailFocus::ExecMode,
                    DetailFocus::ExecMode => DetailFocus::Variables,
                    DetailFocus::Variables => DetailFocus::Commands,
                    DetailFocus::Commands => DetailFocus::Name,
                };
            }
            KeyCode::BackTab => {
                self.commit_name_edit();
                self.focus = match self.focus {
                    DetailFocus::Name => DetailFocus::Commands,
                    DetailFocus::Group => DetailFocus::Name,
                    DetailFocus::Shell => DetailFocus::Group,
                    DetailFocus::ExecMode => DetailFocus::Shell,
                    DetailFocus::Variables => DetailFocus::ExecMode,
                    DetailFocus::Commands => DetailFocus::Variables,
                };
            }
            KeyCode::Up => match self.focus {
                DetailFocus::Variables => {
                    self.variable_list.select_previous();
                }
                DetailFocus::Commands => {
                    self.command_list.select_previous();
                }
                _ => {}
            },
            KeyCode::Down => match self.focus {
                DetailFocus::Variables => {
                    self.variable_list.select_next(self.set.variables.len());
                }
                DetailFocus::Commands => {
                    self.command_list.select_next(self.set.commands.len());
                }
                _ => {}
            },
            KeyCode::Left => match self.focus {
                DetailFocus::Group => {
                    self.cycle_group(-1);
                }
                DetailFocus::Shell => {
                    self.cycle_shell(-1);
                }
                DetailFocus::ExecMode => {
                    self.cycle_exec_mode(-1);
                }
                _ => {}
            },
            KeyCode::Right => match self.focus {
                DetailFocus::Group => {
                    self.cycle_group(1);
                }
                DetailFocus::Shell => {
                    self.cycle_shell(1);
                }
                DetailFocus::ExecMode => {
                    self.cycle_exec_mode(1);
                }
                _ => {}
            },
            KeyCode::Enter => {
                match self.focus {
                    DetailFocus::Name => {
                        if self.editing_name {
                            // Second Enter: confirm edit
                            self.set.name = self.name_input.content.clone();
                            self.editing_name = false;
                        } else {
                            // First Enter: start editing
                            self.name_input = TextInput::new(self.set.name.clone());
                            self.editing_name = true;
                        }
                    }
                    DetailFocus::Variables if !self.set.variables.is_empty() => {
                        let idx = self
                            .variable_list
                            .selected
                            .min(self.set.variables.len() - 1);
                        let text = format!(
                            "{}={}",
                            self.set.variables[idx].name, self.set.variables[idx].default_value
                        );
                        Self::list_edit_begin(
                            &mut self.var_edit,
                            &self.variable_list,
                            text,
                            self.set.variables.len(),
                        );
                    }
                    DetailFocus::Commands if !self.set.commands.is_empty() => {
                        let idx = self.command_list.selected.min(self.set.commands.len() - 1);
                        let text = self.set.commands[idx].command.clone();
                        Self::list_edit_begin(
                            &mut self.cmd_edit,
                            &self.command_list,
                            text,
                            self.set.commands.len(),
                        );
                    }
                    _ => {}
                }
            }
            KeyCode::Char('a' | 'A') => match self.focus {
                DetailFocus::Variables => {
                    Self::list_insert_begin(
                        &mut self.var_edit,
                        &mut self.variable_list,
                        self.set.variables.len(),
                    );
                }
                DetailFocus::Commands => {
                    Self::list_insert_begin(
                        &mut self.cmd_edit,
                        &mut self.command_list,
                        self.set.commands.len(),
                    );
                }
                _ => {}
            },
            KeyCode::Char('d') | KeyCode::Char('D') if self.focus == DetailFocus::Variables
                && !self.set.variables.is_empty() =>
            {
                let idx = self.variable_list.selected.min(self.set.variables.len().saturating_sub(1));
                AppAction::DeleteVariable(idx)
            }
            KeyCode::Char('d') | KeyCode::Char('D') if self.focus == DetailFocus::Commands
                && !self.set.commands.is_empty() =>
            {
                let idx = self.command_list.selected.min(self.set.commands.len().saturating_sub(1));
                AppAction::DeleteCommand(idx)
            }
            KeyCode::Char('d') | KeyCode::Char('D') => AppAction::None,
            KeyCode::Char('s')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                return AppAction::SaveSet(self.set.clone());
            }
            KeyCode::Esc => {
                if self.editing_name {
                    self.editing_name = false;
                } else {
                    return AppAction::CancelEdit;
                }
            }
            _ => {}
        };

        // Handle name editing (Enter to confirm is handled in the outer match)
        if self.editing_name {
            handle_text_input(&mut self.name_input, key);
        }

        AppAction::None
    }

    fn commit_name_edit(&mut self) {
        if self.editing_name {
            self.set.name = self.name_input.content.clone();
            self.editing_name = false;
        }
    }

    /// Begin editing a list item at the current selection.
    fn list_edit_begin(
        edit: &mut InlineEdit,
        list: &ScrollableList,
        initial_text: String,
        total_items: usize,
    ) {
        let idx = list.selected.min(total_items.saturating_sub(1));
        edit.edit_input = TextInput::new(initial_text);
        edit.editing = Some(idx);
    }

    /// Begin inserting a new item after the current selection.
    fn list_insert_begin(edit: &mut InlineEdit, list: &mut ScrollableList, total_items: usize) {
        edit.edit_input = TextInput::new(String::new());
        let pos = (list.selected + 1).min(total_items);
        edit.insert_at = Some(pos);
        edit.editing = Some(total_items);
        list.selected = pos;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::AppAction;
    use crate::models::{CommandSet, Group};
    use crate::ui::detail_screen::DetailFocus;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_state() -> DetailScreenState {
        let group = Group::new("G".to_string());
        let set = CommandSet::new("S".to_string(), group.id);
        DetailScreenState::new(set, vec![group])
    }

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn test_tab_cycles_focus_forward() {
        let mut state = make_state();
        assert_eq!(state.focus, DetailFocus::Name);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Group);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Shell);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::ExecMode);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Variables);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Commands);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Name); // wraps around
    }

    #[test]
    fn test_backtab_cycles_focus_backward() {
        let mut state = make_state();
        // Start at Name (index 0), send backtab -> goes to Commands (last)
        state.handle_key(make_key(KeyCode::BackTab));
        assert_eq!(state.focus, DetailFocus::Commands);
    }

    #[test]
    fn test_enter_on_name_starts_editing() {
        let mut state = make_state();
        assert_eq!(state.focus, DetailFocus::Name);
        assert!(!state.editing_name);
        state.handle_key(make_key(KeyCode::Enter));
        assert!(state.editing_name);
    }

    #[test]
    fn test_enter_on_variables_enters_edit_mode() {
        let mut state = make_state();
        // Add a variable
        state.set.variables.push(crate::models::Variable {
            name: "x".to_string(),
            default_value: "y".to_string(),
        });
        state.focus = DetailFocus::Variables;
        state.handle_key(make_key(KeyCode::Enter));
        assert!(state.var_edit.is_editing());
    }

    #[test]
    fn test_a_on_variables_triggers_insert() {
        let mut state = make_state();
        state.set.variables.push(crate::models::Variable {
            name: "a".to_string(),
            default_value: "b".to_string(),
        });
        state.focus = DetailFocus::Variables;
        let action = state.handle_key(make_key(KeyCode::Char('a')));
        assert!(matches!(action, AppAction::None));
        assert!(state.var_edit.insert_at.is_some());
    }

    #[test]
    fn test_d_on_variables_returns_delete_variable() {
        let mut state = make_state();
        state.set.variables.push(crate::models::Variable {
            name: "x".to_string(),
            default_value: "y".to_string(),
        });
        state.focus = DetailFocus::Variables;
        let action = state.handle_key(make_key(KeyCode::Char('d')));
        assert!(matches!(action, AppAction::DeleteVariable(0)));
    }

    #[test]
    fn test_ctrl_s_returns_save_set() {
        let mut state = make_state();
        let ctrl_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        let action = state.handle_key(ctrl_s);
        assert!(matches!(action, AppAction::SaveSet(_)));
    }

    #[test]
    fn test_esc_returns_cancel_edit() {
        let mut state = make_state();
        let action = state.handle_key(make_key(KeyCode::Esc));
        assert!(matches!(action, AppAction::CancelEdit));
    }
}
