use crate::models::{Command, Variable};
use crate::ui::detail_screen::DetailScreenAction;
use crate::ui::widget::{InlineEdit, ScrollableList};
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_variable_edit(
    edit: &mut InlineEdit,
    key: KeyEvent,
    idx: usize,
    variables: &mut Vec<Variable>,
    list: &mut ScrollableList,
) -> DetailScreenAction {
    match key.code {
        KeyCode::Enter => {
            let input = edit.edit_input.content.clone();
            if let Some(eq_pos) = input.find('=') {
                let name = input[..eq_pos].trim().to_string();
                let value = input[eq_pos + 1..].trim().to_string();
                edit.commit(
                    idx,
                    variables,
                    Variable {
                        name,
                        default_value: value,
                    },
                    list,
                );
            } else if !input.is_empty() {
                edit.commit(
                    idx,
                    variables,
                    Variable {
                        name: input.trim().to_string(),
                        default_value: String::new(),
                    },
                    list,
                );
            }
            edit.editing = None;
            DetailScreenAction::None
        }
        KeyCode::Esc => {
            edit.cancel();
            DetailScreenAction::None
        }
        _ => {
            let n = variables.len();
            if (n > 0 || edit.insert_at.is_some()) && edit.editing.is_some() {
                let protect = edit.edit_input.content.find('=').map(|p| p + 1);
                edit.handle_key_protected(key, protect);
            }
            DetailScreenAction::None
        }
    }
}

pub fn handle_command_edit(
    edit: &mut InlineEdit,
    key: KeyEvent,
    idx: usize,
    commands: &mut Vec<Command>,
    list: &mut ScrollableList,
) -> DetailScreenAction {
    match key.code {
        KeyCode::Enter => {
            let cmd = edit.edit_input.content.clone();
            edit.commit(
                idx,
                commands,
                Command {
                    position: idx,
                    command: cmd,
                },
                list,
            );
            for (i, c) in commands.iter_mut().enumerate() {
                c.position = i;
            }
            edit.editing = None;
            DetailScreenAction::None
        }
        KeyCode::Esc => {
            edit.cancel();
            DetailScreenAction::None
        }
        _ => {
            edit.handle_key(key);
            DetailScreenAction::None
        }
    }
}
