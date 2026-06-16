use super::{DetailFocus, DetailScreenState};
use crate::action::AppAction;
use crate::ui::detail_editor::{handle_command_edit, handle_variable_edit};
use crate::ui::widget::TextInput;
use crate::ui::widget::text_input::handle_text_input;
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
                            .min(self.set.variables.len().saturating_sub(1));
                        let v = &self.set.variables[idx];
                        self.var_edit.edit_input =
                            TextInput::new(format!("{}={}", v.name, v.default_value));
                        self.var_edit.editing = Some(idx);
                    }
                    DetailFocus::Commands if !self.set.commands.is_empty() => {
                        let idx = self
                            .command_list
                            .selected
                            .min(self.set.commands.len().saturating_sub(1));
                        self.cmd_edit.edit_input =
                            TextInput::new(self.set.commands[idx].command.clone());
                        self.cmd_edit.editing = Some(idx);
                    }
                    _ => {}
                }
            }
            KeyCode::Char('a' | 'A') => match self.focus {
                DetailFocus::Variables => {
                    self.var_edit.edit_input = TextInput::new(String::new());
                    let pos = (self.variable_list.selected + 1).min(self.set.variables.len());
                    self.var_edit.insert_at = Some(pos);
                    self.var_edit.editing = Some(self.set.variables.len());
                    self.variable_list.selected = pos;
                }
                DetailFocus::Commands => {
                    self.cmd_edit.edit_input = TextInput::new(String::new());
                    let pos = (self.command_list.selected + 1).min(self.set.commands.len());
                    self.cmd_edit.insert_at = Some(pos);
                    self.cmd_edit.editing = Some(self.set.commands.len());
                    self.command_list.selected = pos;
                }
                _ => {}
            },
            KeyCode::Char('d' | 'D') => match self.focus {
                DetailFocus::Variables if !self.set.variables.is_empty() => {
                    let idx = self
                        .variable_list
                        .selected
                        .min(self.set.variables.len().saturating_sub(1));
                    return AppAction::DeleteVariable(idx);
                }
                DetailFocus::Commands if !self.set.commands.is_empty() => {
                    let idx = self
                        .command_list
                        .selected
                        .min(self.set.commands.len().saturating_sub(1));
                    return AppAction::DeleteCommand(idx);
                }
                _ => {}
            },
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
}
