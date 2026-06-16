use crate::action::{AppAction, DeleteKind};
use crate::mode::AppMode;
use crate::models::CommandSet;
use crate::storage;
use crate::ui::detail_screen::DetailScreenState;
use crate::ui::main_screen::Panel;
use crate::ui::toast::ToastSeverity;
use crossterm::event::KeyCode;

use super::{App, ExecutionState};

impl App {
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        if self.variable_screen.active {
            let action = self.variable_screen.handle_key(key);
            self.handle_action(action);
            return;
        }

        // Global Help shortcut — works in all modes
        if key.code == KeyCode::Char('?') {
            self.handle_action(AppAction::Help);
            return;
        }

        match &self.mode {
            AppMode::Main => {
                let action = self.main_screen.handle_key(key, &self.data);
                self.handle_action(action);
            }
            AppMode::Detail => {
                if let Some(ref mut ds) = self.detail_screen {
                    let action = ds.handle_key(key);
                    self.handle_action(action);
                }
            }
            AppMode::Execution => {
                if let ExecutionState::Running { ref mut screen, .. } = self.execution_state {
                    let action = screen.handle_key(key);
                    self.handle_action(action);
                }
            }
            AppMode::ConfirmDelete { kind, prev } => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let action = match kind {
                            DeleteKind::Set {
                                group_index, set_index, ..
                            } => AppAction::DeleteSet(*group_index, *set_index),
                            DeleteKind::Group { group_index, .. } => {
                                AppAction::DeleteGroup(*group_index)
                            }
                            DeleteKind::Variable { var_index, .. } => {
                                AppAction::DeleteVariable(*var_index)
                            }
                            DeleteKind::Command { cmd_index, .. } => {
                                AppAction::DeleteCommand(*cmd_index)
                            }
                        };
                        self.mode = (**prev).clone();
                        self.handle_action(action);
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.mode = (**prev).clone();
                    }
                    _ => {} // Ignore all other keys during confirmation
                }
            }
            AppMode::Help => {
                self.mode = self.prev_mode.take().unwrap_or(AppMode::Main);
            }
        }
    }

    pub fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::None => {}
            AppAction::Quit => self.running = false,
            AppAction::Help => {
                if self.mode == AppMode::Help {
                    self.mode = self.prev_mode.take().unwrap_or(AppMode::Main);
                } else {
                    self.prev_mode = Some(self.mode.clone());
                    self.mode = AppMode::Help;
                }
            }

            // ---- Main screen ----
            AppAction::ExecuteSet(gi, si) => {
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    let set = &self.data.groups[gi].sets[si];
                    if !set.variables.is_empty() {
                        self.variable_screen.activate(set, gi, si);
                    } else {
                        if let ExecutionState::Idle { ref mut pending_set } = self.execution_state {
                            *pending_set = Some((gi, si));
                        }
                        self.do_execute();
                    }
                }
            }
            AppAction::EditSet(gi, si) => {
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    let set = self.data.groups[gi].sets[si].clone();
                    let groups = self.data.groups.clone();
                    self.detail_screen = Some(DetailScreenState::new(set, groups));
                    self.mode = AppMode::Detail;
                }
            }
            AppAction::NewSet(gi) => {
                if gi < self.data.groups.len() {
                    let gid = self.data.groups[gi].id;
                    let set = CommandSet::new("New Command Set".to_string(), gid);
                    let si = (self.main_screen.set_list.selected + 1)
                        .min(self.data.groups[gi].sets.len());
                    self.data.groups[gi].sets.insert(si, set.clone());
                    self.main_screen.set_list.selected = si;
                    self.auto_save();
                    self.toasts.add("Set created", ToastSeverity::Info);
                    let groups = self.data.groups.clone();
                    self.detail_screen = Some(DetailScreenState::new(set, groups));
                    self.mode = AppMode::Detail;
                }
            }
            AppAction::DeleteSet(gi, si) => {
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    self.data.groups[gi].sets.remove(si);
                    self.main_screen
                        .set_list
                        .clamp_selected(self.data.groups[gi].sets.len());
                    if self.data.groups[gi].sets.is_empty() {
                        self.main_screen.active_panel = Panel::Groups;
                    }
                    self.auto_save();
                    self.toasts.add("Set deleted", ToastSeverity::Info);
                }
            }
            AppAction::NewGroup => {
                let gi = (self.main_screen.group_list.selected + 1).min(self.data.groups.len());
                let n = self.data.groups.len() + 1;
                self.data
                    .groups
                    .insert(gi, crate::models::Group::new(format!("Group {}", n)));
                self.main_screen.group_list.selected = gi;
                self.main_screen.set_list.reset();
                self.auto_save();
                self.toasts.add("Group created", ToastSeverity::Info);
            }
            AppAction::RenameGroup(gi, new_name) => {
                if gi < self.data.groups.len() {
                    self.data.groups[gi].name = new_name;
                    self.auto_save();
                    self.toasts.add("Group renamed", ToastSeverity::Info);
                }
            }
            AppAction::DeleteGroup(gi) => {
                if gi < self.data.groups.len() {
                    self.data.groups.remove(gi);
                    self.main_screen
                        .group_list
                        .clamp_selected(self.data.groups.len());
                    self.main_screen.set_list.reset();
                    if self.data.groups.is_empty() {
                        self.main_screen.group_list.reset();
                        self.main_screen.active_panel = Panel::Groups;
                    }
                    self.auto_save();
                    self.toasts.add("Group deleted", ToastSeverity::Info);
                }
            }

            // ---- Detail screen ----
            AppAction::SaveSet(set) => {
                let sid = set.id;
                for group in &mut self.data.groups {
                    if let Some(existing) = group.sets.iter_mut().find(|s| s.id == sid) {
                        *existing = set;
                        existing.updated_at = chrono::Utc::now();
                        break;
                    }
                }
                self.detail_screen = None;
                self.mode = AppMode::Main;
                self.auto_save();
                self.toasts.add("Command set saved", ToastSeverity::Info);
            }
            AppAction::CancelEdit => {
                self.detail_screen = None;
                self.mode = AppMode::Main;
            }
            AppAction::DeleteVariable(idx) => {
                if let Some(ref mut ds) = self.detail_screen
                    && idx < ds.set.variables.len()
                {
                    ds.set.variables.remove(idx);
                    ds.variable_list.clamp_selected(ds.set.variables.len());
                    if ds.set.variables.is_empty() {
                        ds.focus = crate::ui::detail_screen::DetailFocus::Name;
                    }
                    self.auto_save();
                    self.toasts.add("Variable deleted", ToastSeverity::Info);
                }
            }
            AppAction::DeleteCommand(idx) => {
                if let Some(ref mut ds) = self.detail_screen
                    && idx < ds.set.commands.len()
                {
                    ds.set.commands.remove(idx);
                    for (i, c) in ds.set.commands.iter_mut().enumerate() {
                        c.position = i;
                    }
                    ds.command_list.clamp_selected(ds.set.commands.len());
                    if ds.set.commands.is_empty() {
                        ds.focus = crate::ui::detail_screen::DetailFocus::Name;
                    }
                    self.auto_save();
                    self.toasts.add("Command deleted", ToastSeverity::Info);
                }
            }

            // ---- Execution screen ----
            AppAction::BackToMain => {
                if let ExecutionState::Running { ref screen, .. } = self.execution_state
                    && screen.completed
                {
                    let summary = format!(
                        "Done: {}/{}",
                        screen.succeeded + screen.failed + screen.skipped,
                        screen.total,
                    );
                    let severity = if screen.failed > 0 {
                        ToastSeverity::Error
                    } else if screen.skipped > 0 {
                        ToastSeverity::Info
                    } else {
                        ToastSeverity::Success
                    };
                    self.toasts.add(summary, severity);
                }
                self.teardown_execution(false, false);
                self.mode = AppMode::Main;
            }
            AppAction::SkipCurrent => {
                self.teardown_execution(true, true);
                self.mode = AppMode::Execution;
            }
            AppAction::ContinueFrom(start) => {
                if let ExecutionState::Running { pending_set: (gi, si), .. } = self.execution_state {
                    self.do_execute_with(gi, si, start);
                }
            }
            AppAction::ReExec => {
                let pending = if let ExecutionState::Running { pending_set, .. } = self.execution_state {
                    Some(pending_set)
                } else {
                    None
                };
                self.teardown_execution(false, false);
                if let Some((gi, si)) = pending {
                    self.do_execute_with(gi, si, 0);
                }
            }
            // ---- Variable overlay ----
            AppAction::ConfirmVariables => {
                let gi = self.variable_screen.gi;
                let si = self.variable_screen.si;
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    let set = &mut self.data.groups[gi].sets[si];
                    for (i, input) in self.variable_screen.inputs.iter().enumerate() {
                        if i < set.variables.len() {
                            set.variables[i].default_value = input.content.clone();
                        }
                    }
                }
                self.variable_screen = crate::ui::variable_screen::VariableScreenState::new();
                self.auto_save();
                if let ExecutionState::Idle { ref mut pending_set } = self.execution_state {
                    *pending_set = Some((gi, si));
                }
                self.do_execute();
            }
            AppAction::RequestDelete(kind) => {
                self.mode = AppMode::ConfirmDelete {
                    kind,
                    prev: Box::new(self.mode.clone()),
                };
            }

            AppAction::CancelVariables => {
                self.variable_screen = crate::ui::variable_screen::VariableScreenState::new();
                if let ExecutionState::Idle { ref mut pending_set } = self.execution_state {
                    *pending_set = None;
                }
            }
        }
    }

    fn auto_save(&mut self) {
        if let Err(e) = storage::save_app_data(&self.data) {
            self.toasts
                .add(format!("Save failed: {}", e), ToastSeverity::Error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ExecutionState;
    use crate::action::{AppAction, DeleteKind};
    use crate::app::execution::ExecutionManager;
    use crate::mode::AppMode;
    use crate::models::{AppData, CommandSet, Group};
    use crate::test_utils::{make_app, make_key};
    use crate::ui::detail_screen::DetailScreenState;
    use crate::ui::main_screen::Panel;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_data_with_one_group() -> AppData {
        let mut g = Group::new("Deploy".to_string());
        let set = CommandSet::new("Prod".to_string(), g.id);
        g.sets.push(set);
        AppData { groups: vec![g] }
    }

    // ---- NewGroup ----
    #[test]
    fn test_handler_new_group() {
        let mut app = make_app();
        app.handle_action(AppAction::NewGroup);
        assert_eq!(app.data.groups.len(), 1);
        assert_eq!(app.data.groups[0].name, "Group 1");
        assert!(app.toasts.toasts.iter().any(|t| t.message.contains("Group created")));
    }

    // ---- RenameGroup ----
    #[test]
    fn test_handler_rename_group() {
        let mut app = make_app();
        app.handle_action(AppAction::NewGroup);
        app.handle_action(AppAction::RenameGroup(0, "Infra".to_string()));
        assert_eq!(app.data.groups[0].name, "Infra");
    }

    #[test]
    fn test_handler_rename_group_out_of_bounds_noop() {
        let mut app = make_app();
        app.handle_action(AppAction::RenameGroup(0, "X".to_string()));
        assert!(app.data.groups.is_empty());
    }

    // ---- NewSet ----
    #[test]
    fn test_handler_new_set() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::NewSet(0));
        assert_eq!(app.data.groups[0].sets.len(), 2);
        assert_eq!(app.data.groups[0].sets[1].name, "New Command Set");
        assert!(app.detail_screen.is_some());
        assert_eq!(app.mode, AppMode::Detail);
    }

    #[test]
    fn test_handler_new_set_out_of_bounds_noop() {
        let mut app = make_app();
        app.handle_action(AppAction::NewSet(5));
        assert!(app.detail_screen.is_none());
        assert_eq!(app.mode, AppMode::Main);
    }

    // ---- EditSet ----
    #[test]
    fn test_handler_edit_set() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::EditSet(0, 0));
        assert!(app.detail_screen.is_some());
        assert_eq!(app.mode, AppMode::Detail);
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.name, "Prod");
    }

    #[test]
    fn test_handler_edit_set_out_of_bounds_noop() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::EditSet(5, 5));
        assert!(app.detail_screen.is_none());
    }

    // ---- SaveSet ----
    #[test]
    fn test_handler_save_set() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        let set = app.data.groups[0].sets[0].clone();
        let groups = app.data.groups.clone();
        app.detail_screen = Some(DetailScreenState::new(set, groups));
        app.mode = AppMode::Detail;

        let mut saved = app.data.groups[0].sets[0].clone();
        saved.name = "Updated".to_string();
        app.handle_action(AppAction::SaveSet(saved));
        assert!(app.detail_screen.is_none());
        assert_eq!(app.mode, AppMode::Main);
        assert_eq!(app.data.groups[0].sets[0].name, "Updated");
    }

    // ---- CancelEdit ----
    #[test]
    fn test_handler_cancel_edit() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        let set = app.data.groups[0].sets[0].clone();
        let groups = app.data.groups.clone();
        app.detail_screen = Some(DetailScreenState::new(set, groups));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::CancelEdit);
        assert!(app.detail_screen.is_none());
        assert_eq!(app.mode, AppMode::Main);
    }

    // ---- DeleteSet ----
    #[test]
    fn test_handler_delete_set() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::DeleteSet(0, 0));
        assert!(app.data.groups[0].sets.is_empty());
        assert_eq!(app.main_screen.active_panel, Panel::Groups);
    }

    #[test]
    fn test_handler_delete_set_out_of_bounds_noop() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::DeleteSet(5, 5));
        assert_eq!(app.data.groups[0].sets.len(), 1);
    }

    // ---- DeleteGroup ----
    #[test]
    fn test_handler_delete_group() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::DeleteGroup(0));
        assert!(app.data.groups.is_empty());
    }

    #[test]
    fn test_handler_delete_group_out_of_bounds_noop() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::DeleteGroup(5));
        assert_eq!(app.data.groups.len(), 1);
    }

    // ---- Quit ----
    #[test]
    fn test_handler_quit() {
        let mut app = make_app();
        app.handle_action(AppAction::Quit);
        assert!(!app.running);
    }

    // ---- None + Help ----
    #[test]
    fn test_handler_none() {
        let mut app = make_app();
        app.handle_action(AppAction::None);
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_handler_help() {
        let mut app = make_app();
        app.handle_action(AppAction::Help);
        assert_eq!(app.mode, AppMode::Help);
        assert_eq!(app.prev_mode, Some(AppMode::Main));
        // Dismiss Help
        app.handle_action(AppAction::Help);
        assert_eq!(app.mode, AppMode::Main);
        assert!(app.prev_mode.is_none());
    }

    // ---- Data helper with variables and commands ----
    fn make_data_with_vars_and_cmds() -> AppData {
        use crate::models::Variable;
        use crate::models::Command;
        let mut g = Group::new("Deploy".to_string());
        let mut set = CommandSet::new("Prod".to_string(), g.id);
        set.variables.push(Variable { name: "host".to_string(), default_value: "localhost".to_string() });
        set.commands.push(Command { position: 0, command: "echo hi".to_string() });
        set.commands.push(Command { position: 1, command: "echo bye".to_string() });
        g.sets.push(set);
        AppData { groups: vec![g] }
    }

    // ---- DeleteVariable ----
    #[test]
    fn test_handler_delete_variable() {
        use crate::ui::detail_screen::DetailFocus;
        let mut app = make_app();
        app.data = make_data_with_vars_and_cmds();
        let set = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::DeleteVariable(0));
        let ds = app.detail_screen.as_ref().unwrap();
        assert!(ds.set.variables.is_empty());
        assert_eq!(ds.focus, DetailFocus::Name);
    }

    // ---- DeleteCommand ----
    #[test]
    fn test_handler_delete_command() {
        let mut app = make_app();
        app.data = make_data_with_vars_and_cmds();
        let set = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::DeleteCommand(0));
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.commands.len(), 1);
        assert_eq!(ds.set.commands[0].command, "echo bye");
        assert_eq!(ds.set.commands[0].position, 0);
    }

    #[test]
    fn test_handler_delete_command_focus_migration() {
        use crate::ui::detail_screen::DetailFocus;
        use crate::models::Command;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(Command { position: 0, command: "only".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::DeleteCommand(0));
        let ds = app.detail_screen.as_ref().unwrap();
        assert!(ds.set.commands.is_empty());
        assert_eq!(ds.focus, DetailFocus::Name);
    }

    // ---- Variable overlay ----
    #[test]
    fn test_handler_cancel_variables() {
        let mut app = make_app();
        app.variable_screen.active = true;
        app.variable_screen.gi = 0;
        app.variable_screen.si = 0;
        app.execution_state = ExecutionState::Idle { pending_set: Some((0, 0)) };

        app.handle_action(AppAction::CancelVariables);
        assert!(!app.variable_screen.active);
        assert!(matches!(app.execution_state, ExecutionState::Idle { pending_set: None }));
    }

    #[test]
    fn test_handler_confirm_variables() {
        let mut app = make_app();
        app.data = make_data_with_vars_and_cmds();
        app.variable_screen.activate(&app.data.groups[0].sets[0], 0, 0);
        app.variable_screen.inputs[0].content = "prod.example.com".to_string();

        app.handle_action(AppAction::ConfirmVariables);
        assert_eq!(
            app.data.groups[0].sets[0].variables[0].default_value,
            "prod.example.com"
        );
        assert_eq!(app.mode, AppMode::Execution);
        assert!(matches!(app.execution_state, ExecutionState::Running { .. }));
    }

    // ---- Execution actions ----
    #[test]
    fn test_handler_execute_set_no_variables() {
        let mut app = make_app();
        app.data = make_data_with_one_group();

        app.handle_action(AppAction::ExecuteSet(0, 0));
        assert_eq!(app.mode, AppMode::Execution);
        assert!(matches!(app.execution_state, ExecutionState::Running { .. }));
    }

    #[test]
    fn test_handler_back_to_main() {
        use crate::ui::execution_screen::ExecutionScreenState;
        use crate::models::Command;
        let mut app = make_app();
        let cmds = vec![Command { position: 0, command: "ok".to_string() }];
        app.execution_state = ExecutionState::Running {
            screen: Box::new(ExecutionScreenState::new("test".to_string(), &cmds)),
            manager: ExecutionManager::new(),
            pending_set: (0, 0),
        };
        app.mode = AppMode::Execution;

        app.handle_action(AppAction::BackToMain);
        assert_eq!(app.mode, AppMode::Main);
        assert!(matches!(app.execution_state, ExecutionState::Idle { .. }));
    }

    #[test]
    fn test_handler_skip_current() {
        use crate::ui::execution_screen::ExecutionScreenState;
        use crate::models::Command;
        let mut app = make_app();
        let cmds = vec![Command { position: 0, command: "a".to_string() }];
        app.execution_state = ExecutionState::Running {
            screen: Box::new(ExecutionScreenState::new("t".to_string(), &cmds)),
            manager: ExecutionManager::new(),
            pending_set: (0, 0),
        };
        app.mode = AppMode::Execution;

        app.handle_action(AppAction::SkipCurrent);
        // skip_current calls teardown_execution(true, true) → keeps screen + marks skipped
        assert_eq!(app.mode, AppMode::Execution);
        assert!(matches!(app.execution_state, ExecutionState::Running { .. }));
        if let ExecutionState::Running { ref screen, .. } = app.execution_state {
            assert!(screen.completed);
            assert_eq!(screen.skipped, 1);
        }
    }

    #[test]
    fn test_handler_re_exec() {
        use crate::ui::execution_screen::ExecutionScreenState;
        use crate::models::Command;
        let mut app = make_app();
        app.data = make_data_with_one_group();
        let cmds = vec![Command { position: 0, command: "ok".to_string() }];
        app.execution_state = ExecutionState::Running {
            screen: Box::new(ExecutionScreenState::new("t".to_string(), &cmds)),
            manager: ExecutionManager::new(),
            pending_set: (0, 0),
        };
        app.mode = AppMode::Execution;

        app.handle_action(AppAction::ReExec);
        assert_eq!(app.mode, AppMode::Execution);
        assert!(matches!(app.execution_state, ExecutionState::Running { .. }));
    }

    #[test]
    fn test_help_from_detail_mode() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        let set = app.data.groups[0].sets[0].clone();
        let groups = app.data.groups.clone();
        app.detail_screen = Some(DetailScreenState::new(set, groups));
        app.mode = AppMode::Detail;

        let key = make_key(KeyCode::Char('?'));
        app.handle_key(key);
        assert_eq!(app.mode, AppMode::Help);
        assert_eq!(app.prev_mode, Some(AppMode::Detail));
    }

    #[test]
    fn test_help_from_execution_mode() {
        use crate::ui::execution_screen::ExecutionScreenState;
        use crate::models::Command;
        let mut app = make_app();
        let cmds = vec![Command { position: 0, command: "x".to_string() }];
        app.execution_state = ExecutionState::Running {
            screen: Box::new(ExecutionScreenState::new("t".to_string(), &cmds)),
            manager: ExecutionManager::new(),
            pending_set: (0, 0),
        };
        app.mode = AppMode::Execution;

        let key = make_key(KeyCode::Char('?'));
        app.handle_key(key);
        assert_eq!(app.mode, AppMode::Help);
        // execution_state should NOT be cleaned up — Help is an overlay
        assert!(matches!(app.execution_state, ExecutionState::Running { .. }));
        assert_eq!(app.prev_mode, Some(AppMode::Execution));
    }

    // ---- RequestDelete / ConfirmDelete ----
    fn make_data_for_delete_test() -> AppData {
        let mut g = Group::new("Deploy".to_string());
        let mut set = CommandSet::new("Prod".to_string(), g.id);
        set.commands.push(crate::models::Command { position: 0, command: "echo hi".to_string() });
        g.sets.push(set);
        AppData { groups: vec![g] }
    }

    #[test]
    fn test_request_delete_set_enters_confirm_mode() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.handle_action(AppAction::RequestDelete(DeleteKind::Set {
            group_index: 0,
            set_index: 0,
            set_name: "Prod".to_string(),
        }));
        assert!(
            matches!(app.mode, AppMode::ConfirmDelete { .. }),
            "Expected ConfirmDelete mode after RequestDelete"
        );
    }

    #[test]
    fn test_request_delete_group_enters_confirm_mode() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.handle_action(AppAction::RequestDelete(DeleteKind::Group {
            group_index: 0,
            group_name: "Deploy".to_string(),
            set_count: 1,
        }));
        assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
    }

    #[test]
    fn test_request_delete_variable_enters_confirm_mode() {
        use crate::models::Variable;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.variables.push(Variable { name: "host".to_string(), default_value: "".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::RequestDelete(DeleteKind::Variable {
            var_index: 0,
            var_name: "host".to_string(),
        }));
        assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
    }

    #[test]
    fn test_request_delete_command_enters_confirm_mode() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        let set = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::RequestDelete(DeleteKind::Command {
            cmd_index: 0,
            cmd_preview: "echo hi".to_string(),
        }));
        assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
    }

    #[test]
    fn test_confirm_delete_y_executes_delete_set() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set {
                group_index: 0,
                set_index: 0,
                set_name: "Prod".to_string(),
            },
            prev: Box::new(AppMode::Main),
        };
        let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty());
        app.handle_key(y_key);
        assert!(app.data.groups[0].sets.is_empty());
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_delete_y_executes_delete_group() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Group {
                group_index: 0,
                group_name: "Deploy".to_string(),
                set_count: 1,
            },
            prev: Box::new(AppMode::Main),
        };
        let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty());
        app.handle_key(y_key);
        assert!(app.data.groups.is_empty());
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_delete_y_executes_delete_variable() {
        use crate::models::Variable;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.variables.push(Variable { name: "host".to_string(), default_value: "".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));

        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Variable {
                var_index: 0,
                var_name: "host".to_string(),
            },
            prev: Box::new(AppMode::Detail),
        };
        let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty());
        app.handle_key(y_key);
        let ds = app.detail_screen.as_ref().unwrap();
        assert!(ds.set.variables.is_empty());
        assert_eq!(app.mode, AppMode::Detail);
    }

    #[test]
    fn test_confirm_delete_y_executes_delete_command() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        let set = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set, app.data.groups.clone()));

        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Command {
                cmd_index: 0,
                cmd_preview: "echo hi".to_string(),
            },
            prev: Box::new(AppMode::Detail),
        };
        let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty());
        app.handle_key(y_key);
        let ds = app.detail_screen.as_ref().unwrap();
        assert!(ds.set.commands.is_empty());
        assert_eq!(app.mode, AppMode::Detail);
    }

    #[test]
    fn test_confirm_delete_n_cancels() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set {
                group_index: 0,
                set_index: 0,
                set_name: "Prod".to_string(),
            },
            prev: Box::new(AppMode::Main),
        };
        let n_key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty());
        app.handle_key(n_key);
        // Set should NOT be deleted
        assert_eq!(app.data.groups[0].sets.len(), 1);
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_delete_esc_cancels() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set {
                group_index: 0,
                set_index: 0,
                set_name: "Prod".to_string(),
            },
            prev: Box::new(AppMode::Main),
        };
        app.handle_key(make_key(KeyCode::Esc));
        // Set should NOT be deleted
        assert_eq!(app.data.groups[0].sets.len(), 1);
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_delete_other_key_ignored() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set {
                group_index: 0,
                set_index: 0,
                set_name: "Prod".to_string(),
            },
            prev: Box::new(AppMode::Main),
        };
        // Press a key that is not y, n, or Esc
        app.handle_key(make_key(KeyCode::Char('x')));
        // Should still be in ConfirmDelete mode
        assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
        // Set should NOT be deleted
        assert_eq!(app.data.groups[0].sets.len(), 1);
    }

    #[test]
    fn test_help_still_works_during_confirm_delete() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set {
                group_index: 0,
                set_index: 0,
                set_name: "Prod".to_string(),
            },
            prev: Box::new(AppMode::Main),
        };
        // Press '?' for help
        app.handle_key(make_key(KeyCode::Char('?')));
        assert_eq!(app.mode, AppMode::Help);
        // Dismiss Help — should restore to ConfirmDelete
        app.handle_key(make_key(KeyCode::Esc));
        assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
    }
}
