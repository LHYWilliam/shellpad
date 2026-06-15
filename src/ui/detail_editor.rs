use crate::models::{Command, Variable};
use crate::ui::components::{handle_text_input, ScrollableList, TextInput};
use crate::ui::detail_screen::DetailScreenAction;
use crossterm::event::{KeyCode, KeyEvent};

pub struct DetailEditState {
    pub editing_variable: Option<usize>,
    pub editing_command: Option<usize>,
    pub edit_input: TextInput,
    pub insert_at: Option<usize>,
}

impl DetailEditState {
    pub fn new() -> Self {
        Self {
            editing_variable: None,
            editing_command: None,
            edit_input: TextInput::new(String::new()),
            insert_at: None,
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing_variable.is_some() || self.editing_command.is_some()
    }

    pub fn handle_variable_edit(
        &mut self,
        key: KeyEvent,
        idx: usize,
        variables: &mut Vec<Variable>,
        list: &mut ScrollableList,
    ) -> DetailScreenAction {
        match key.code {
            KeyCode::Enter => {
                let input = self.edit_input.content.clone();
                if let Some(eq_pos) = input.find('=') {
                    let name = input[..eq_pos].trim().to_string();
                    let value = input[eq_pos + 1..].trim().to_string();
                    let var = Variable { name, default_value: value };
                    if let Some(insert_pos) = self.insert_at.take() {
                        variables.insert(insert_pos, var);
                        list.selected = insert_pos;
                    } else {
                        variables[idx] = var;
                        list.selected = idx;
                    }
                } else if !input.is_empty() {
                    let var = Variable {
                        name: input.trim().to_string(),
                        default_value: String::new(),
                    };
                    if let Some(insert_pos) = self.insert_at.take() {
                        variables.insert(insert_pos, var);
                        list.selected = insert_pos;
                    } else {
                        variables[idx] = var;
                        list.selected = idx;
                    }
                }
                self.editing_variable = None;
                DetailScreenAction::None
            }
            KeyCode::Esc => {
                self.insert_at = None;
                self.editing_variable = None;
                DetailScreenAction::None
            }
            _ => {
                let n = variables.len();
                if (n > 0 || self.insert_at.is_some()) && self.editing_variable.is_some() {
                    // Protect "key=" prefix from deletion
                    let protect = self.edit_input.content.find('=').map_or(0, |p| p + 1);
                    match key.code {
                        KeyCode::Backspace => {
                            if self.edit_input.cursor > protect {
                                self.edit_input.delete_before();
                            }
                        }
                        KeyCode::Delete => {
                            if self.edit_input.cursor > protect {
                                self.edit_input.delete_at();
                            }
                        }
                        KeyCode::Left => {
                            if self.edit_input.cursor > protect {
                                self.edit_input.move_cursor_left();
                            }
                        }
                        KeyCode::Right => self.edit_input.move_cursor_right(),
                        KeyCode::Home => self.edit_input.move_cursor_to_start(),
                        KeyCode::End => self.edit_input.move_cursor_to_end(),
                        _ => {
                            handle_text_input(&mut self.edit_input, key);
                        }
                    }
                }
                DetailScreenAction::None
            }
        }
    }

    pub fn handle_command_edit(
        &mut self,
        key: KeyEvent,
        idx: usize,
        commands: &mut Vec<Command>,
        list: &mut ScrollableList,
    ) -> DetailScreenAction {
        match key.code {
            KeyCode::Enter => {
                let cmd = self.edit_input.content.clone();
                let command = Command { position: idx, command: cmd };
                if let Some(insert_pos) = self.insert_at.take() {
                    commands.insert(insert_pos, command);
                    list.selected = insert_pos;
                } else {
                    commands[idx] = command;
                    list.selected = idx;
                }
                for (i, c) in commands.iter_mut().enumerate() {
                    c.position = i;
                }
                self.editing_command = None;
                DetailScreenAction::None
            }
            KeyCode::Esc => {
                self.insert_at = None;
                self.editing_command = None;
                DetailScreenAction::None
            }
            _ => {
                handle_text_input(&mut self.edit_input, key);
                DetailScreenAction::None
            }
        }
    }
}
