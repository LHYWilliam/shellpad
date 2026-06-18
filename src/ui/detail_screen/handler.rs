use super::editor::{handle_command_edit, handle_variable_edit};
use super::{DetailFocus, DetailScreenState};
use crate::action::{AppAction, DeleteKind, ReorderKind};
use crate::ui::widget::text_input::handle_text_input;
use crate::ui::widget::{InlineEdit, ScrollableList, TextInput};
use crossterm::event::KeyEvent;

enum DetailRegion {
    Properties,
    Variables,
    Commands,
    DeferredCommands,
}

impl DetailScreenState {
    fn region(&self) -> DetailRegion {
        match self.focus {
            DetailFocus::Name
            | DetailFocus::WorkDir
            | DetailFocus::Group
            | DetailFocus::Shell
            | DetailFocus::ExecMode => DetailRegion::Properties,
            DetailFocus::Variables => DetailRegion::Variables,
            DetailFocus::Commands => DetailRegion::Commands,
            DetailFocus::DeferredCommands => DetailRegion::DeferredCommands,
        }
    }

    fn next_region(&mut self) {
        self.commit_name_edit();
        self.commit_workdir_edit();
        match self.region() {
            DetailRegion::Properties => {
                self.focus = DetailFocus::Variables;
                self.var_editor.list.selected = 0;
            }
            DetailRegion::Variables => {
                self.focus = DetailFocus::Commands;
                self.cmd_editor.list.selected = 0;
            }
            DetailRegion::Commands => {
                self.focus = DetailFocus::DeferredCommands;
                self.deferred_editor.list.selected = 0;
            }
            DetailRegion::DeferredCommands => {
                self.focus = DetailFocus::Name;
            }
        }
    }

    fn prev_region(&mut self) {
        self.commit_name_edit();
        self.commit_workdir_edit();
        match self.region() {
            DetailRegion::Properties => {
                self.focus = DetailFocus::DeferredCommands;
                self.deferred_editor.list.selected = 0;
            }
            DetailRegion::Variables => {
                self.focus = DetailFocus::Name;
            }
            DetailRegion::Commands => {
                self.focus = DetailFocus::Variables;
                self.var_editor.list.selected = 0;
            }
            DetailRegion::DeferredCommands => {
                self.focus = DetailFocus::Commands;
                self.cmd_editor.list.selected = 0;
            }
        }
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyEvent) -> AppAction {
        use crossterm::event::KeyCode;

        // Handle inline editing
        if let Some(idx) = self.var_editor.edit.editing {
            return handle_variable_edit(
                &mut self.var_editor.edit,
                key,
                idx,
                &mut self.set.variables,
                &mut self.var_editor.list,
            );
        }
        if let Some(idx) = self.cmd_editor.edit.editing {
            return handle_command_edit(
                &mut self.cmd_editor.edit,
                key,
                idx,
                &mut self.set.commands,
                &mut self.cmd_editor.list,
            );
        }
        if let Some(idx) = self.deferred_editor.edit.editing {
            return handle_command_edit(
                &mut self.deferred_editor.edit,
                key,
                idx,
                &mut self.set.defer_commands,
                &mut self.deferred_editor.list,
            );
        }

        match key.code {
            KeyCode::Tab | KeyCode::Char('\t') => {
                self.next_region();
            }
            KeyCode::BackTab => {
                self.prev_region();
            }
            KeyCode::Up
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                match self.focus {
                    DetailFocus::Variables if !self.set.variables.is_empty() => {
                        let idx = self
                            .var_editor.list
                            .selected
                            .min(self.set.variables.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Variable(idx), -1);
                    }
                    DetailFocus::Commands if !self.set.commands.is_empty() => {
                        let idx = self
                            .cmd_editor.list
                            .selected
                            .min(self.set.commands.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Command(idx), -1);
                    }
                    DetailFocus::DeferredCommands if !self.set.defer_commands.is_empty() => {
                        let idx = self
                            .deferred_editor.list
                            .selected
                            .min(self.set.defer_commands.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Command(idx), -1);
                    }
                    _ => {}
                }
            }
            KeyCode::Up => match self.region() {
                DetailRegion::Properties => {
                    self.commit_name_edit();
                    self.commit_workdir_edit();
                    self.focus = match self.focus {
                        DetailFocus::Name => DetailFocus::Name,
                        DetailFocus::WorkDir => DetailFocus::Name,
                        DetailFocus::Group => DetailFocus::WorkDir,
                        DetailFocus::Shell => DetailFocus::Group,
                        DetailFocus::ExecMode => DetailFocus::Shell,
                        _ => self.focus,
                    };
                }
                DetailRegion::Variables => {
                    self.var_editor.list.select_previous();
                }
                DetailRegion::Commands => {
                    self.cmd_editor.list.select_previous();
                }
                DetailRegion::DeferredCommands => {
                    self.deferred_editor.list.select_previous();
                }
            },
            KeyCode::Down
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                match self.focus {
                    DetailFocus::Variables if !self.set.variables.is_empty() => {
                        let idx = self
                            .var_editor.list
                            .selected
                            .min(self.set.variables.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Variable(idx), 1);
                    }
                    DetailFocus::Commands if !self.set.commands.is_empty() => {
                        let idx = self
                            .cmd_editor.list
                            .selected
                            .min(self.set.commands.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Command(idx), 1);
                    }
                    DetailFocus::DeferredCommands if !self.set.defer_commands.is_empty() => {
                        let idx = self
                            .deferred_editor.list
                            .selected
                            .min(self.set.defer_commands.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Command(idx), 1);
                    }
                    _ => {}
                }
            }
            KeyCode::Down => match self.region() {
                DetailRegion::Properties => {
                    self.commit_name_edit();
                    self.commit_workdir_edit();
                    self.focus = match self.focus {
                        DetailFocus::Name => DetailFocus::WorkDir,
                        DetailFocus::WorkDir => DetailFocus::Group,
                        DetailFocus::Group => DetailFocus::Shell,
                        DetailFocus::Shell => DetailFocus::ExecMode,
                        DetailFocus::ExecMode => DetailFocus::ExecMode,
                        _ => self.focus,
                    };
                }
                DetailRegion::Variables => {
                    self.var_editor.list.select_next(self.set.variables.len());
                }
                DetailRegion::Commands => {
                    self.cmd_editor.list.select_next(self.set.commands.len());
                }
                DetailRegion::DeferredCommands => {
                    self.deferred_editor.list
                        .select_next(self.set.defer_commands.len());
                }
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
                    DetailFocus::WorkDir => {
                        if self.workdir_editing {
                            let content = self.workdir_input.content.clone();
                            self.set.working_dir = if content.trim().is_empty() {
                                None
                            } else {
                                Some(content)
                            };
                            self.workdir_editing = false;
                        } else {
                            self.workdir_input =
                                TextInput::new(self.set.working_dir.clone().unwrap_or_default());
                            self.workdir_editing = true;
                        }
                    }
                    DetailFocus::Variables if !self.set.variables.is_empty() => {
                        let idx = self
                            .var_editor.list
                            .selected
                            .min(self.set.variables.len() - 1);
                        let text = format!(
                            "{}={}",
                            self.set.variables[idx].name, self.set.variables[idx].default_value
                        );
                        Self::list_edit_begin(
                            &mut self.var_editor.edit,
                            &self.var_editor.list,
                            text,
                            self.set.variables.len(),
                        );
                    }
                    DetailFocus::Commands if !self.set.commands.is_empty() => {
                        let idx = self.cmd_editor.list.selected.min(self.set.commands.len() - 1);
                        let text = self.set.commands[idx].command.clone();
                        Self::list_edit_begin(
                            &mut self.cmd_editor.edit,
                            &self.cmd_editor.list,
                            text,
                            self.set.commands.len(),
                        );
                    }
                    DetailFocus::DeferredCommands if !self.set.defer_commands.is_empty() => {
                        let idx = self
                            .deferred_editor.list
                            .selected
                            .min(self.set.defer_commands.len() - 1);
                        let text = self.set.defer_commands[idx].command.clone();
                        Self::list_edit_begin(
                            &mut self.deferred_editor.edit,
                            &self.deferred_editor.list,
                            text,
                            self.set.defer_commands.len(),
                        );
                    }
                    _ => {}
                }
            }
            KeyCode::Char('a' | 'A') => match self.focus {
                DetailFocus::Variables => {
                    Self::list_insert_begin(
                        &mut self.var_editor.edit,
                        &mut self.var_editor.list,
                        self.set.variables.len(),
                    );
                }
                DetailFocus::Commands => {
                    Self::list_insert_begin(
                        &mut self.cmd_editor.edit,
                        &mut self.cmd_editor.list,
                        self.set.commands.len(),
                    );
                }
                DetailFocus::DeferredCommands => {
                    Self::list_insert_begin(
                        &mut self.deferred_editor.edit,
                        &mut self.deferred_editor.list,
                        self.set.defer_commands.len(),
                    );
                }
                _ => {}
            },
            KeyCode::Char('d' | 'D') => match self.focus {
                DetailFocus::Variables if !self.set.variables.is_empty() => {
                    let idx = self
                        .var_editor.list
                        .selected
                        .min(self.set.variables.len().saturating_sub(1));
                    let var_name = self.set.variables[idx].name.clone();
                    return AppAction::RequestDelete(DeleteKind::Variable {
                        var_index: idx,
                        var_name,
                    });
                }
                DetailFocus::Commands if !self.set.commands.is_empty() => {
                    let idx = self
                        .cmd_editor.list
                        .selected
                        .min(self.set.commands.len().saturating_sub(1));
                    let cmd_preview = self.set.commands[idx].command.clone();
                    return AppAction::RequestDelete(DeleteKind::Command {
                        cmd_index: idx,
                        cmd_preview,
                    });
                }
                DetailFocus::DeferredCommands if !self.set.defer_commands.is_empty() => {
                    let idx = self
                        .deferred_editor.list
                        .selected
                        .min(self.set.defer_commands.len().saturating_sub(1));
                    let cmd_preview = self.set.defer_commands[idx].command.clone();
                    return AppAction::RequestDelete(DeleteKind::Command {
                        cmd_index: idx,
                        cmd_preview,
                    });
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
                } else if self.workdir_editing {
                    self.workdir_editing = false;
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
        if self.workdir_editing {
            handle_text_input(&mut self.workdir_input, key);
        }

        AppAction::None
    }

    fn commit_name_edit(&mut self) {
        if self.editing_name {
            self.set.name = self.name_input.content.clone();
            self.editing_name = false;
        }
    }

    fn commit_workdir_edit(&mut self) {
        if self.workdir_editing {
            let content = self.workdir_input.content.clone();
            self.set.working_dir = if content.trim().is_empty() {
                None
            } else {
                Some(content)
            };
            self.workdir_editing = false;
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
    use crate::action::{AppAction, DeleteKind, ReorderKind};
    use crate::models::{CommandSet, ExecMode, Group};
    use crate::test_utils::make_key;
    use crate::ui::detail_screen::DetailFocus;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_state() -> DetailScreenState {
        let group = Group::new("G".to_string());
        let set = CommandSet::new("S".to_string(), group.id);
        DetailScreenState::new(set, vec![group])
    }

    #[test]
    fn test_tab_cycles_properties_to_variables_to_commands_to_deferred() {
        let mut state = make_state();
        assert_eq!(state.focus, DetailFocus::Name); // Properties first
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Variables);
        assert_eq!(state.var_editor.list.selected, 0);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Commands);
        assert_eq!(state.cmd_editor.list.selected, 0);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::DeferredCommands);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Name); // wraps to Properties
    }

    #[test]
    fn test_backtab_cycles_deferred_to_commands_to_variables_to_properties() {
        let mut state = make_state();
        state.handle_key(make_key(KeyCode::BackTab)); // Name → DeferredCommands
        assert_eq!(state.focus, DetailFocus::DeferredCommands);
        state.handle_key(make_key(KeyCode::BackTab)); // DeferredCommands → Commands
        assert_eq!(state.focus, DetailFocus::Commands);
        assert_eq!(state.cmd_editor.list.selected, 0);
        state.handle_key(make_key(KeyCode::BackTab)); // Commands → Variables
        assert_eq!(state.focus, DetailFocus::Variables);
        assert_eq!(state.var_editor.list.selected, 0);
        state.handle_key(make_key(KeyCode::BackTab)); // Variables → Properties
        assert_eq!(state.focus, DetailFocus::Name);
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
        assert!(state.var_editor.edit.is_editing());
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
        assert!(state.var_editor.edit.insert_at.is_some());
    }

    #[test]
    fn test_d_on_variables_returns_request_delete_variable() {
        let mut state = make_state();
        state.set.variables.push(crate::models::Variable {
            name: "x".to_string(),
            default_value: "y".to_string(),
        });
        state.focus = DetailFocus::Variables;
        let action = state.handle_key(make_key(KeyCode::Char('d')));
        assert!(matches!(
            action,
            AppAction::RequestDelete(DeleteKind::Variable { var_index: 0, .. })
        ));
    }

    #[test]
    fn test_d_on_commands_returns_request_delete_command() {
        let mut state = make_state();
        state.set.commands.push(crate::models::Command {
            position: 0,
            command: "echo hi".to_string(),
        });
        state.focus = DetailFocus::Commands;
        let action = state.handle_key(make_key(KeyCode::Char('d')));
        assert!(matches!(
            action,
            AppAction::RequestDelete(DeleteKind::Command { cmd_index: 0, .. })
        ));
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

    #[test]
    fn test_ctrl_up_returns_reorder_variable() {
        let mut state = make_state();
        state.set.variables.push(crate::models::Variable {
            name: "x".to_string(),
            default_value: "y".to_string(),
        });
        state.set.variables.push(crate::models::Variable {
            name: "z".to_string(),
            default_value: "w".to_string(),
        });
        state.focus = DetailFocus::Variables;
        state.var_editor.list.selected = 1;
        let ctrl_up = KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL);
        let action = state.handle_key(ctrl_up);
        assert!(matches!(
            action,
            AppAction::Reorder(ReorderKind::Variable(1), -1)
        ));
    }

    #[test]
    fn test_ctrl_down_returns_reorder_command() {
        let mut state = make_state();
        state.set.commands.push(crate::models::Command {
            position: 0,
            command: "c1".to_string(),
        });
        state.set.commands.push(crate::models::Command {
            position: 1,
            command: "c2".to_string(),
        });
        state.focus = DetailFocus::Commands;
        state.cmd_editor.list.selected = 0;
        let ctrl_down = KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL);
        let action = state.handle_key(ctrl_down);
        assert!(matches!(
            action,
            AppAction::Reorder(ReorderKind::Command(0), 1)
        ));
    }

    #[test]
    fn test_ctrl_up_ignored_when_not_vars_or_cmds_focus() {
        let mut state = make_state();
        state.focus = DetailFocus::Name;
        let ctrl_up = KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL);
        let action = state.handle_key(ctrl_up);
        assert!(matches!(action, AppAction::None));
    }

    // ---- WorkDir ----
    #[test]
    fn test_enter_on_workdir_starts_editing() {
        let mut state = make_state();
        state.focus = DetailFocus::WorkDir;
        assert!(!state.workdir_editing);
        state.handle_key(make_key(KeyCode::Enter));
        assert!(state.workdir_editing);
    }

    #[test]
    fn test_enter_on_workdir_confirms_editing() {
        let mut state = make_state();
        state.focus = DetailFocus::WorkDir;
        state.handle_key(make_key(KeyCode::Enter)); // start editing
        assert!(state.workdir_editing);
        state.workdir_input = crate::ui::widget::TextInput::new("/tmp/test".to_string());
        state.handle_key(make_key(KeyCode::Enter)); // confirm
        assert!(!state.workdir_editing);
        assert_eq!(state.set.working_dir, Some("/tmp/test".to_string()));
    }

    #[test]
    fn test_enter_on_workdir_empty_string_stores_none() {
        let mut state = make_state();
        state.set.working_dir = Some("/old/path".to_string());
        state.focus = DetailFocus::WorkDir;
        state.handle_key(make_key(KeyCode::Enter)); // start editing
        state.workdir_input = crate::ui::widget::TextInput::new(String::new());
        state.handle_key(make_key(KeyCode::Enter)); // confirm with empty
        assert!(!state.workdir_editing);
        assert_eq!(state.set.working_dir, None);
    }

    #[test]
    fn test_esc_cancels_workdir_editing() {
        let mut state = make_state();
        state.set.working_dir = Some("/existing".to_string());
        state.focus = DetailFocus::WorkDir;
        state.handle_key(make_key(KeyCode::Enter)); // start editing
        assert!(state.workdir_editing);
        state.workdir_input = crate::ui::widget::TextInput::new("/changed".to_string());
        state.handle_key(make_key(KeyCode::Esc));
        assert!(!state.workdir_editing);
        assert_eq!(state.set.working_dir, Some("/existing".to_string()));
    }

    #[test]
    fn test_tab_commits_workdir_editing() {
        let mut state = make_state();
        state.focus = DetailFocus::WorkDir;
        state.handle_key(make_key(KeyCode::Enter)); // start editing
        state.workdir_input = crate::ui::widget::TextInput::new("/committed".to_string());
        state.handle_key(make_key(KeyCode::Tab)); // Tab commits + moves
        assert!(!state.workdir_editing);
        assert_eq!(state.set.working_dir, Some("/committed".to_string()));
        assert_eq!(state.focus, DetailFocus::Variables);
    }

    #[test]
    fn test_properties_down_cycles_through_fields() {
        let mut state = make_state();
        assert_eq!(state.focus, DetailFocus::Name);
        state.handle_key(make_key(KeyCode::Down));
        assert_eq!(state.focus, DetailFocus::WorkDir);
        state.handle_key(make_key(KeyCode::Down));
        assert_eq!(state.focus, DetailFocus::Group);
        state.handle_key(make_key(KeyCode::Down));
        assert_eq!(state.focus, DetailFocus::Shell);
        state.handle_key(make_key(KeyCode::Down));
        assert_eq!(state.focus, DetailFocus::ExecMode);
        state.handle_key(make_key(KeyCode::Down));
        assert_eq!(state.focus, DetailFocus::ExecMode); // stops at bottom
    }

    #[test]
    fn test_properties_up_cycles_reverse() {
        let mut state = make_state();
        state.focus = DetailFocus::ExecMode;
        state.handle_key(make_key(KeyCode::Up));
        assert_eq!(state.focus, DetailFocus::Shell);
        state.handle_key(make_key(KeyCode::Up));
        assert_eq!(state.focus, DetailFocus::Group);
        state.handle_key(make_key(KeyCode::Up));
        assert_eq!(state.focus, DetailFocus::WorkDir);
        state.handle_key(make_key(KeyCode::Up));
        assert_eq!(state.focus, DetailFocus::Name);
        state.handle_key(make_key(KeyCode::Up));
        assert_eq!(state.focus, DetailFocus::Name); // stops at top
    }

    #[test]
    fn test_tab_into_properties_resets_to_name() {
        let mut state = make_state();
        state.focus = DetailFocus::Shell; // somewhere in Properties middle
        state.handle_key(make_key(KeyCode::Tab)); // Properties → Variables
        assert_eq!(state.focus, DetailFocus::Variables);
        state.handle_key(make_key(KeyCode::BackTab)); // Variables → Properties
        assert_eq!(state.focus, DetailFocus::Name); // resets to first
    }

    #[test]
    fn test_tab_into_properties_resets_after_full_cycle() {
        let mut state = make_state();
        state.handle_key(make_key(KeyCode::Tab)); // Properties → Variables
        state.handle_key(make_key(KeyCode::Tab)); // Variables → Commands
        state.handle_key(make_key(KeyCode::Tab)); // Commands → DeferredCommands
        state.handle_key(make_key(KeyCode::Tab)); // DeferredCommands → Properties
        assert_eq!(state.focus, DetailFocus::Name); // resets to first
    }

    #[test]
    fn test_properties_up_at_name_stops() {
        let mut state = make_state();
        assert_eq!(state.focus, DetailFocus::Name);
        state.handle_key(make_key(KeyCode::Up));
        // name only valid for properties, but Up from Name should no-op
        assert_eq!(state.focus, DetailFocus::Name); // stays, does not wrap
    }

    #[test]
    fn test_properties_down_at_exec_mode_stops() {
        let mut state = make_state();
        state.focus = DetailFocus::ExecMode;
        state.handle_key(make_key(KeyCode::Down));
        assert_eq!(state.focus, DetailFocus::ExecMode); // no-op
    }

    #[test]
    fn test_right_past_last_exec_mode_stops() {
        let mut state = make_state();
        state.focus = DetailFocus::ExecMode;
        state.handle_key(make_key(KeyCode::Right)); // Stop → Continue
        state.handle_key(make_key(KeyCode::Right)); // should stop (no wrap)
        assert_eq!(state.set.exec_mode, ExecMode::ContinueOnError);
        assert_eq!(state.focus, DetailFocus::ExecMode);
    }
}
