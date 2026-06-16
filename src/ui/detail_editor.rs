use crate::action::AppAction;
use crate::models::{Command, Variable};
use crate::ui::widget::{InlineEdit, ScrollableList};
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_variable_edit(
    edit: &mut InlineEdit,
    key: KeyEvent,
    idx: usize,
    variables: &mut Vec<Variable>,
    list: &mut ScrollableList,
) -> AppAction {
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
            AppAction::None
        }
        KeyCode::Esc => {
            edit.cancel();
            AppAction::None
        }
        _ => {
            let n = variables.len();
            if (n > 0 || edit.insert_at.is_some()) && edit.editing.is_some() {
                let protect = edit.edit_input.content.find('=').map(|p| p + 1);
                edit.handle_key_protected(key, protect);
            }
            AppAction::None
        }
    }
}

pub fn handle_command_edit(
    edit: &mut InlineEdit,
    key: KeyEvent,
    idx: usize,
    commands: &mut Vec<Command>,
    list: &mut ScrollableList,
) -> AppAction {
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
            AppAction::None
        }
        KeyCode::Esc => {
            edit.cancel();
            AppAction::None
        }
        _ => {
            edit.handle_key(key);
            AppAction::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::AppAction;
    use crate::models::Variable;
    use crate::ui::widget::{InlineEdit, ScrollableList};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn test_handle_variable_edit_enter_commits() {
        let mut edit = InlineEdit::new();
        edit.editing = Some(0);
        edit.edit_input = crate::ui::widget::TextInput::new("x=y".to_string());
        let mut vars = vec![Variable {
            name: "old".into(),
            default_value: "old".into(),
        }];
        let mut list = ScrollableList::new();
        let action =
            handle_variable_edit(&mut edit, make_key(KeyCode::Enter), 0, &mut vars, &mut list);
        assert!(matches!(action, AppAction::None));
        assert_eq!(vars[0].name, "x");
        assert_eq!(vars[0].default_value, "y");
        assert!(edit.editing.is_none());
    }

    #[test]
    fn test_handle_variable_edit_esc_cancels() {
        let mut edit = InlineEdit::new();
        edit.editing = Some(0);
        edit.edit_input = crate::ui::widget::TextInput::new("a=b".to_string());
        let mut vars = vec![Variable {
            name: "orig".into(),
            default_value: "orig".into(),
        }];
        let mut list = ScrollableList::new();
        let action =
            handle_variable_edit(&mut edit, make_key(KeyCode::Esc), 0, &mut vars, &mut list);
        assert!(matches!(action, AppAction::None));
        assert_eq!(vars[0].name, "orig"); // unchanged
        assert!(edit.editing.is_none());
    }

    #[test]
    fn test_handle_variable_edit_text_input() {
        let mut edit = InlineEdit::new();
        edit.editing = Some(0);
        edit.edit_input = crate::ui::widget::TextInput::new(String::new());
        let mut vars = vec![Variable {
            name: "a".into(),
            default_value: "b".into(),
        }];
        let mut list = ScrollableList::new();
        let action = handle_variable_edit(
            &mut edit,
            make_key(KeyCode::Char('x')),
            0,
            &mut vars,
            &mut list,
        );
        assert!(matches!(action, AppAction::None));
        assert_eq!(edit.edit_input.content, "x");
    }

    #[test]
    fn test_handle_command_edit_enter_commits() {
        let mut edit = InlineEdit::new();
        edit.editing = Some(0);
        edit.edit_input = crate::ui::widget::TextInput::new("echo new".to_string());
        let mut cmds = vec![crate::models::Command {
            position: 0,
            command: "echo old".to_string(),
        }];
        let mut list = ScrollableList::new();
        let action =
            handle_command_edit(&mut edit, make_key(KeyCode::Enter), 0, &mut cmds, &mut list);
        assert!(matches!(action, AppAction::None));
        assert_eq!(cmds[0].command, "echo new");
    }
}
