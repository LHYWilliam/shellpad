use crate::action::AppAction;
use crate::mode::AppMode;
use crate::models::CommandSet;
use crate::storage;
use crate::ui::detail_screen::DetailScreenState;
use crate::ui::main_screen::Panel;
use crate::ui::toast::ToastSeverity;

use super::App;

impl App {
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        if self.variable_screen.active {
            let action = self.variable_screen.handle_key(key);
            self.handle_action(action);
            return;
        }
        match self.mode {
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
                if let Some(ref mut es) = self.exec_screen {
                    let action = es.handle_key(key);
                    self.handle_action(action);
                }
            }
            AppMode::Help => self.mode = AppMode::Main,
        }
    }

    pub fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::None => {}
            AppAction::Quit => self.running = false,
            AppAction::Help => self.mode = AppMode::Help,

            // ---- Main screen ----
            AppAction::ExecuteSet(gi, si) => {
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    let set = &self.data.groups[gi].sets[si];
                    if !set.variables.is_empty() {
                        self.variable_screen.activate(set, gi, si);
                    } else {
                        self.pending_set = Some((gi, si));
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
                self.toasts.add("Command set saved", ToastSeverity::Success);
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
                    self.toasts.add("Command deleted", ToastSeverity::Info);
                }
            }

            // ---- Execution screen ----
            AppAction::BackToMain => {
                if let Some(ref es) = self.exec_screen
                    && es.completed
                {
                    let summary = format!(
                        "Done: {}/{}",
                        es.succeeded + es.failed + es.skipped,
                        es.total,
                    );
                    let severity = if es.failed > 0 {
                        ToastSeverity::Error
                    } else if es.skipped > 0 {
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
                if let Some((gi, si)) = self.pending_set {
                    self.do_execute_with(gi, si, start);
                }
            }
            AppAction::ReExec => {
                self.teardown_execution(false, false);
                if let Some((gi, si)) = self.pending_set {
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
                self.pending_set = Some((gi, si));
                self.do_execute();
            }
            AppAction::CancelVariables => {
                self.variable_screen = crate::ui::variable_screen::VariableScreenState::new();
                self.pending_set = None;
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
    use super::App;
    use crate::action::AppAction;
    use crate::app::execution::ExecutionManager;
    use crate::app::toast::ToastManager;
    use crate::mode::AppMode;
    use crate::models::{AppData, CommandSet, Group};
    use crate::ui::detail_screen::DetailScreenState;
    use crate::ui::main_screen::{MainScreenState, Panel};
    use crate::ui::theme::Theme;
    use crate::ui::variable_screen::VariableScreenState;

    fn make_app() -> App {
        App {
            data: AppData::empty(),
            mode: AppMode::Main,
            running: true,
            main_screen: MainScreenState::new(),
            detail_screen: None,
            exec_screen: None,
            execution: ExecutionManager::new(),
            variable_screen: VariableScreenState::new(),
            pending_set: None,
            theme: Theme::default_dark(),
            toasts: ToastManager::new(),
        }
    }

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
    }
}
