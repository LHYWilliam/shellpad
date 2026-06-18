use crate::action::{AppAction, ConfirmChoice, DeleteKind, ReorderKind};
use crate::app::execution::ExecutionManager;
use crate::mode::AppMode;
use crate::models::CommandSet;
use crate::storage;
use crate::ui::detail_screen::DetailScreenState;
use crate::ui::execution_screen::ExecutionScreenState;
use crate::ui::main_screen::Panel;
use crate::ui::toast::ToastSeverity;
use crossterm::event::KeyCode;

use super::{App, ExecutionState};

impl App {
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        if self.variable_screen.overlay.is_some() {
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
            AppMode::ConfirmDelete {
                kind,
                prev,
                selected,
            } => match key.code {
                KeyCode::Left => {
                    if matches!(selected, ConfirmChoice::Cancel) {
                        self.mode = AppMode::ConfirmDelete {
                            kind: kind.clone(),
                            prev: prev.clone(),
                            selected: ConfirmChoice::Confirm,
                        };
                    }
                }
                KeyCode::Right => {
                    if matches!(selected, ConfirmChoice::Confirm) {
                        self.mode = AppMode::ConfirmDelete {
                            kind: kind.clone(),
                            prev: prev.clone(),
                            selected: ConfirmChoice::Cancel,
                        };
                    }
                }
                KeyCode::Enter => {
                    let action = if matches!(selected, ConfirmChoice::Confirm) {
                        match kind {
                            DeleteKind::Set {
                                group_index,
                                set_index,
                                ..
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
                        }
                    } else {
                        AppAction::None
                    };
                    self.mode = (**prev).clone();
                    if !matches!(action, AppAction::None) {
                        self.handle_action(action);
                    }
                }
                KeyCode::Esc => {
                    self.mode = (**prev).clone();
                }
                _ => {}
            },
            AppMode::Help => {
                self.mode = self.prev_mode.take().unwrap_or(AppMode::Main);
            }
        }
    }

    fn with_execution_mut(&mut self, f: impl FnOnce(&mut ExecutionScreenState, &ExecutionManager)) {
        if let ExecutionState::Running {
            ref mut screen,
            ref manager,
            ..
        } = self.execution_state
        {
            f(screen, manager);
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
                        if let ExecutionState::Idle {
                            ref mut pending_set,
                        } = self.execution_state
                        {
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
                    let removed = self.data.groups[gi].sets[si].clone();
                    self.trash.push(crate::app::TrashEntry {
                        timestamp: chrono::Local::now(),
                        item: crate::app::TrashedItem::Set {
                            set: removed,
                            group_index: gi,
                            set_index: si,
                        },
                    });
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
                    let removed = self.data.groups[gi].clone();
                    self.trash.push(crate::app::TrashEntry {
                        timestamp: chrono::Local::now(),
                        item: crate::app::TrashedItem::Group {
                            group: removed,
                            index: gi,
                        },
                    });
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
                let mut old_gi = None;
                let mut old_pos = None;
                for (gi, group) in self.data.groups.iter().enumerate() {
                    if let Some(pos) = group.sets.iter().position(|s| s.id == sid) {
                        old_gi = Some(gi);
                        old_pos = Some(pos);
                        break;
                    }
                }
                let mut target_gi = None;
                let mut new_si = None;
                if let (Some(gi), Some(pos)) = (old_gi, old_pos) {
                    if set.group_id != self.data.groups[gi].id {
                        // Group changed — remove from old group
                        self.data.groups[gi].sets.remove(pos);
                        self.main_screen
                            .set_list
                            .clamp_selected(self.data.groups[gi].sets.len());
                        // Insert at end of target group
                        if let Some(ti) = self.data.groups.iter().position(|g| g.id == set.group_id)
                        {
                            let mut moved = set;
                            moved.updated_at = chrono::Utc::now();
                            self.data.groups[ti].sets.push(moved);
                            target_gi = Some(ti);
                            new_si = Some(self.data.groups[ti].sets.len().saturating_sub(1));
                        }
                    } else {
                        // Same group — replace in place
                        self.data.groups[gi].sets[pos] = set;
                        self.data.groups[gi].sets[pos].updated_at = chrono::Utc::now();
                    }
                }
                if let (Some(tgi), Some(si)) = (target_gi, new_si) {
                    self.main_screen.group_list.selected = tgi;
                    self.main_screen.set_list.selected = si;
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
                    ds.var_editor.list.clamp_selected(ds.set.variables.len());
                    if ds.set.variables.is_empty() {
                        ds.focus = crate::ui::detail_screen::DetailFocus::Name;
                    }
                    self.auto_save();
                    self.toasts.add("Variable deleted", ToastSeverity::Info);
                }
            }
            AppAction::DeleteCommand(idx) => {
                if let Some(ref mut ds) = self.detail_screen {
                    let is_deferred =
                        ds.focus == crate::ui::detail_screen::DetailFocus::DeferredCommands;
                    if is_deferred && idx < ds.set.defer_commands.len() {
                        ds.set.defer_commands.remove(idx);
                        ds.deferred_editor
                            .list
                            .clamp_selected(ds.set.defer_commands.len());
                        if ds.set.defer_commands.is_empty() {
                            ds.focus = crate::ui::detail_screen::DetailFocus::Commands;
                        }
                        self.auto_save();
                        self.toasts
                            .add("Deferred command deleted", ToastSeverity::Info);
                    } else if idx < ds.set.commands.len() {
                        ds.set.commands.remove(idx);
                        for (i, c) in ds.set.commands.iter_mut().enumerate() {
                            c.position = i;
                        }
                        ds.cmd_editor.list.clamp_selected(ds.set.commands.len());
                        if ds.set.commands.is_empty() {
                            ds.focus = crate::ui::detail_screen::DetailFocus::Name;
                        }
                        self.auto_save();
                        self.toasts.add("Command deleted", ToastSeverity::Info);
                    }
                }
            }
            AppAction::VariableSaved => {
                self.toasts.add("Variable saved", ToastSeverity::Info);
            }
            AppAction::CommandSaved => {
                self.toasts.add("Command saved", ToastSeverity::Info);
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
                    self.teardown_execution(false, false);
                    self.mode = AppMode::Main;
                }
            }
            AppAction::Pause => self.with_execution_mut(|s, m| {
                m.skip_current();
                s.paused = true;
            }),
            AppAction::Continue => self.with_execution_mut(|s, m| {
                m.continue_next();
                s.paused = false;
            }),
            AppAction::Abort => self.with_execution_mut(|s, m| {
                m.abort_all();
                s.paused = false;
                s.mark_remaining_as_skipped();
            }),
            AppAction::ReExec => {
                let pending =
                    if let ExecutionState::Running { pending_set, .. } = self.execution_state {
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
                let overlay = self.variable_screen.overlay.as_ref().unwrap();
                let gi = overlay.gi;
                let si = overlay.si;
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    let set = &mut self.data.groups[gi].sets[si];
                    for (i, input) in overlay.inputs.iter().enumerate() {
                        if i < set.variables.len() {
                            set.variables[i].default_value = input.content.clone();
                        }
                    }
                }
                self.variable_screen = crate::ui::variable_screen::VariableScreenState::new();
                self.auto_save();
                self.toasts.add("Variables saved", ToastSeverity::Info);
                if let ExecutionState::Idle {
                    ref mut pending_set,
                } = self.execution_state
                {
                    *pending_set = Some((gi, si));
                }
                self.do_execute();
            }
            AppAction::Reorder(kind, dir) => {
                let new_idx = |i: usize, len: usize| -> Option<usize> {
                    let c = i as isize + dir;
                    if c >= 0 && (c as usize) < len {
                        Some(c as usize)
                    } else {
                        None
                    }
                };
                match kind {
                    ReorderKind::Group(gi) => {
                        if let Some(ni) = new_idx(gi, self.data.groups.len()) {
                            self.data.groups.swap(gi, ni);
                            self.main_screen.group_list.selected = ni;
                            self.auto_save();
                            self.toasts.add("Group moved", ToastSeverity::Info);
                        }
                    }
                    ReorderKind::Set(gi, si) => {
                        if gi < self.data.groups.len()
                            && let Some(ni) = new_idx(si, self.data.groups[gi].sets.len())
                        {
                            self.data.groups[gi].sets.swap(si, ni);
                            self.main_screen.set_list.selected = ni;
                            self.auto_save();
                            self.toasts.add("Set moved", ToastSeverity::Info);
                        }
                    }
                    ReorderKind::Variable(idx) => {
                        if let Some(ref mut ds) = self.detail_screen
                            && let Some(ni) = new_idx(idx, ds.set.variables.len())
                        {
                            ds.set.variables.swap(idx, ni);
                            ds.var_editor.list.selected = ni;
                            self.auto_save();
                            self.toasts.add("Variable moved", ToastSeverity::Info);
                        }
                    }
                    ReorderKind::Command(idx) => {
                        if let Some(ref mut ds) = self.detail_screen {
                            let is_deferred =
                                ds.focus == crate::ui::detail_screen::DetailFocus::DeferredCommands;
                            if is_deferred {
                                if let Some(ni) = new_idx(idx, ds.set.defer_commands.len()) {
                                    ds.set.defer_commands.swap(idx, ni);
                                    ds.deferred_editor.list.selected = ni;
                                    self.auto_save();
                                    self.toasts
                                        .add("Deferred command moved", ToastSeverity::Info);
                                }
                            } else if let Some(ni) = new_idx(idx, ds.set.commands.len()) {
                                ds.set.commands.swap(idx, ni);
                                for (i, c) in ds.set.commands.iter_mut().enumerate() {
                                    c.position = i;
                                }
                                ds.cmd_editor.list.selected = ni;
                                self.auto_save();
                                self.toasts.add("Command moved", ToastSeverity::Info);
                            }
                        }
                    }
                }
            }
            AppAction::RequestDelete(kind) => {
                self.mode = AppMode::ConfirmDelete {
                    kind,
                    prev: Box::new(self.mode.clone()),
                    selected: ConfirmChoice::Cancel,
                };
            }

            AppAction::CancelVariables => {
                self.variable_screen = crate::ui::variable_screen::VariableScreenState::new();
                if let ExecutionState::Idle {
                    ref mut pending_set,
                } = self.execution_state
                {
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
    use crate::action::{AppAction, ConfirmChoice, DeleteKind, ReorderKind};
    use crate::app::TrashedItem;
    use crate::app::execution::ExecutionManager;
    use crate::mode::AppMode;
    use crate::models::{AppData, CommandSet, Group};
    use crate::test_utils::{make_app, make_data_with_one_group, make_key};
    use crate::ui::detail_screen::DetailScreenState;
    use crate::ui::main_screen::Panel;
    use crossterm::event::KeyCode;

    // ---- NewGroup ----
    #[test]
    fn test_handler_new_group() {
        let mut app = make_app();
        app.handle_action(AppAction::NewGroup);
        assert_eq!(app.data.groups.len(), 1);
        assert_eq!(app.data.groups[0].name, "Group 1");
        assert!(
            app.toasts
                .toasts
                .iter()
                .any(|t| t.message.contains("Group created"))
        );
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

    #[test]
    fn test_handler_save_set_moves_to_new_group() {
        let mut app = make_app();
        let mut g1 = Group::new("Deploy".to_string());
        let set = CommandSet::new("Prod".to_string(), g1.id);
        g1.sets.push(set);
        let g2 = Group::new("Infra".to_string());
        app.data = AppData {
            groups: vec![g1, g2],
        };
        let set = app.data.groups[0].sets[0].clone();
        let groups = app.data.groups.clone();
        app.detail_screen = Some(DetailScreenState::new(set, groups));
        app.mode = AppMode::Detail;

        let mut saved = app.data.groups[0].sets[0].clone();
        saved.group_id = app.data.groups[1].id;
        saved.name = "Prod (moved)".to_string();
        app.handle_action(AppAction::SaveSet(saved));

        assert!(app.detail_screen.is_none());
        assert_eq!(app.mode, AppMode::Main);
        assert!(app.data.groups[0].sets.is_empty());
        assert_eq!(app.data.groups[1].sets.len(), 1);
        assert_eq!(app.data.groups[1].sets[0].name, "Prod (moved)");
        assert_eq!(app.data.groups[1].sets[0].group_id, app.data.groups[1].id);
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
        use crate::models::Command;
        use crate::models::Variable;
        let mut g = Group::new("Deploy".to_string());
        let mut set = CommandSet::new("Prod".to_string(), g.id);
        set.variables.push(Variable {
            name: "host".to_string(),
            default_value: "localhost".to_string(),
        });
        set.commands.push(Command {
            position: 0,
            command: "echo hi".to_string(),
        });
        set.commands.push(Command {
            position: 1,
            command: "echo bye".to_string(),
        });
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
        use crate::models::Command;
        use crate::ui::detail_screen::DetailFocus;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(Command {
            position: 0,
            command: "only".to_string(),
        });
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
        app.variable_screen.overlay = Some(crate::ui::variable_screen::VariableOverlay {
            inputs: vec![],
            names: vec![],
            focus: 0,
            gi: 0,
            si: 0,
        });
        app.execution_state = ExecutionState::Idle {
            pending_set: Some((0, 0)),
        };

        app.handle_action(AppAction::CancelVariables);
        assert!(app.variable_screen.overlay.is_none());
        assert!(matches!(
            app.execution_state,
            ExecutionState::Idle { pending_set: None }
        ));
    }

    #[test]
    fn test_handler_confirm_variables() {
        let mut app = make_app();
        app.data = make_data_with_vars_and_cmds();
        app.variable_screen
            .activate(&app.data.groups[0].sets[0], 0, 0);
        app.variable_screen.overlay.as_mut().unwrap().inputs[0].content =
            "prod.example.com".to_string();

        app.handle_action(AppAction::ConfirmVariables);
        assert_eq!(
            app.data.groups[0].sets[0].variables[0].default_value,
            "prod.example.com"
        );
        assert_eq!(app.mode, AppMode::Execution);
        assert!(matches!(
            app.execution_state,
            ExecutionState::Running { .. }
        ));
    }

    // ---- Execution actions ----
    #[test]
    fn test_handler_execute_set_no_variables() {
        let mut app = make_app();
        app.data = make_data_with_one_group();

        app.handle_action(AppAction::ExecuteSet(0, 0));
        assert_eq!(app.mode, AppMode::Execution);
        assert!(matches!(
            app.execution_state,
            ExecutionState::Running { .. }
        ));
    }

    #[test]
    fn test_handler_back_to_main() {
        use crate::models::Command;
        use crate::ui::execution_screen::ExecutionScreenState;
        let mut app = make_app();
        let cmds = vec![Command {
            position: 0,
            command: "ok".to_string(),
        }];
        let mut screen = ExecutionScreenState::new("test".to_string(), &cmds);
        screen.completed = true; // BackToMain requires completion
        app.execution_state = ExecutionState::Running {
            screen: Box::new(screen),
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
        use crate::models::Command;
        use crate::ui::execution_screen::ExecutionScreenState;
        let mut app = make_app();
        let cmds = vec![Command {
            position: 0,
            command: "a".to_string(),
        }];
        app.execution_state = ExecutionState::Running {
            screen: Box::new(ExecutionScreenState::new("t".to_string(), &cmds)),
            manager: ExecutionManager::new(),
            pending_set: (0, 0),
        };
        app.mode = AppMode::Execution;

        app.handle_action(AppAction::Abort);
        // Abort marks remaining as Skipped, keeps screen
        assert_eq!(app.mode, AppMode::Execution);
        assert!(matches!(
            app.execution_state,
            ExecutionState::Running { .. }
        ));
        if let ExecutionState::Running { ref screen, .. } = app.execution_state {
            assert!(!screen.completed); // completed only set by CompletedAll event
            assert_eq!(screen.skipped, 1);
            assert!(!screen.paused);
        }
    }

    #[test]
    fn test_handler_re_exec() {
        use crate::models::Command;
        use crate::ui::execution_screen::ExecutionScreenState;
        let mut app = make_app();
        app.data = make_data_with_one_group();
        let cmds = vec![Command {
            position: 0,
            command: "ok".to_string(),
        }];
        app.execution_state = ExecutionState::Running {
            screen: Box::new(ExecutionScreenState::new("t".to_string(), &cmds)),
            manager: ExecutionManager::new(),
            pending_set: (0, 0),
        };
        app.mode = AppMode::Execution;

        app.handle_action(AppAction::ReExec);
        assert_eq!(app.mode, AppMode::Execution);
        assert!(matches!(
            app.execution_state,
            ExecutionState::Running { .. }
        ));
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
        use crate::models::Command;
        use crate::ui::execution_screen::ExecutionScreenState;
        let mut app = make_app();
        let cmds = vec![Command {
            position: 0,
            command: "x".to_string(),
        }];
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
        assert!(matches!(
            app.execution_state,
            ExecutionState::Running { .. }
        ));
        assert_eq!(app.prev_mode, Some(AppMode::Execution));
    }

    // ---- RequestDelete / ConfirmDelete ----
    fn make_data_for_delete_test() -> AppData {
        let mut g = Group::new("Deploy".to_string());
        let mut set = CommandSet::new("Prod".to_string(), g.id);
        set.commands.push(crate::models::Command {
            position: 0,
            command: "echo hi".to_string(),
        });
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
        set.variables.push(Variable {
            name: "host".to_string(),
            default_value: "".to_string(),
        });
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
    fn test_confirm_dialog_default_focus_is_cancel() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.handle_action(AppAction::RequestDelete(DeleteKind::Set {
            group_index: 0,
            set_index: 0,
            set_name: "P".to_string(),
        }));
        if let AppMode::ConfirmDelete { selected, .. } = &app.mode {
            assert!(matches!(selected, ConfirmChoice::Cancel));
        } else {
            panic!("expected ConfirmDelete");
        }
    }

    #[test]
    fn test_confirm_dialog_left_to_confirm_enter_deletes() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set {
                group_index: 0,
                set_index: 0,
                set_name: "P".to_string(),
            },
            prev: Box::new(AppMode::Main),
            selected: ConfirmChoice::Cancel,
        };
        app.handle_key(make_key(KeyCode::Left)); // Cancel → Confirm
        app.handle_key(make_key(KeyCode::Enter)); // Confirm executes delete
        assert!(app.data.groups[0].sets.is_empty());
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_dialog_right_to_cancel_enter_noop() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set {
                group_index: 0,
                set_index: 0,
                set_name: "P".to_string(),
            },
            prev: Box::new(AppMode::Main),
            selected: ConfirmChoice::Confirm,
        };
        app.handle_key(make_key(KeyCode::Right)); // Confirm → Cancel
        app.handle_key(make_key(KeyCode::Enter)); // Cancel = no-op
        assert_eq!(app.data.groups[0].sets.len(), 1);
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_dialog_enter_on_confirm_deletes() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set {
                group_index: 0,
                set_index: 0,
                set_name: "P".to_string(),
            },
            prev: Box::new(AppMode::Main),
            selected: ConfirmChoice::Confirm,
        };
        app.handle_key(make_key(KeyCode::Enter));
        assert!(app.data.groups[0].sets.is_empty());
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_dialog_enter_on_cancel_noop() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set {
                group_index: 0,
                set_index: 0,
                set_name: "P".to_string(),
            },
            prev: Box::new(AppMode::Main),
            selected: ConfirmChoice::Cancel,
        };
        app.handle_key(make_key(KeyCode::Enter));
        assert_eq!(app.data.groups[0].sets.len(), 1);
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_dialog_esc_cancels() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set {
                group_index: 0,
                set_index: 0,
                set_name: "P".to_string(),
            },
            prev: Box::new(AppMode::Main),
            selected: ConfirmChoice::Cancel,
        };
        app.handle_key(make_key(KeyCode::Esc));
        assert_eq!(app.data.groups[0].sets.len(), 1);
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_dialog_arrow_boundary_noop() {
        let mut app = make_app();
        app.data = make_data_for_delete_test();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set {
                group_index: 0,
                set_index: 0,
                set_name: "P".to_string(),
            },
            prev: Box::new(AppMode::Main),
            selected: ConfirmChoice::Confirm,
        };
        app.handle_key(make_key(KeyCode::Left)); // already at Confirm
        assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
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
            selected: ConfirmChoice::Cancel,
        };
        // Press '?' for help
        app.handle_key(make_key(KeyCode::Char('?')));
        assert_eq!(app.mode, AppMode::Help);
        // Dismiss Help — should restore to ConfirmDelete
        app.handle_key(make_key(KeyCode::Esc));
        assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
    }

    // ---- Reorder ----
    #[test]
    fn test_reorder_group_up() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.data.groups.push(Group::new("Second".to_string()));
        app.handle_action(AppAction::Reorder(ReorderKind::Group(1), -1));
        assert_eq!(app.data.groups[0].name, "Second");
        assert_eq!(app.data.groups[1].name, "Deploy");
        assert_eq!(app.main_screen.group_list.selected, 0);
    }

    #[test]
    fn test_reorder_group_down() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.data.groups.push(Group::new("Second".to_string()));
        app.handle_action(AppAction::Reorder(ReorderKind::Group(0), 1));
        assert_eq!(app.data.groups[0].name, "Second");
        assert_eq!(app.data.groups[1].name, "Deploy");
        assert_eq!(app.main_screen.group_list.selected, 1);
    }

    #[test]
    fn test_reorder_group_up_boundary_noop() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::Reorder(ReorderKind::Group(0), -1));
        assert_eq!(app.data.groups[0].name, "Deploy");
    }

    #[test]
    fn test_reorder_set_up() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        let mut set2 = CommandSet::new("set2".to_string(), app.data.groups[0].id);
        set2.commands.push(crate::models::Command {
            position: 0,
            command: "cmd".to_string(),
        });
        app.data.groups[0].sets.push(set2);
        app.handle_action(AppAction::Reorder(ReorderKind::Set(0, 1), -1));
        assert_eq!(app.data.groups[0].sets[0].name, "set2");
        assert_eq!(app.data.groups[0].sets[1].name, "Prod");
        assert_eq!(app.main_screen.set_list.selected, 0);
    }

    #[test]
    fn test_reorder_set_down_boundary_noop() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::Reorder(ReorderKind::Set(0, 0), 1));
        assert_eq!(app.data.groups[0].sets[0].name, "Prod");
    }

    #[test]
    fn test_reorder_variable_up() {
        use crate::models::Variable;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.variables.push(Variable {
            name: "a".to_string(),
            default_value: "".to_string(),
        });
        set.variables.push(Variable {
            name: "b".to_string(),
            default_value: "".to_string(),
        });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::Reorder(ReorderKind::Variable(1), -1));
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.variables[0].name, "b");
        assert_eq!(ds.set.variables[1].name, "a");
        assert_eq!(ds.var_editor.list.selected, 0);
    }

    #[test]
    fn test_reorder_variable_up_boundary_noop() {
        use crate::models::Variable;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.variables.push(Variable {
            name: "a".to_string(),
            default_value: "".to_string(),
        });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::Reorder(ReorderKind::Variable(0), -1));
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.variables.len(), 1);
        assert_eq!(ds.set.variables[0].name, "a");
    }

    #[test]
    fn test_reorder_command_up_renumbers_positions() {
        use crate::models::Command;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(Command {
            position: 0,
            command: "echo first".to_string(),
        });
        set.commands.push(Command {
            position: 1,
            command: "echo second".to_string(),
        });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::Reorder(ReorderKind::Command(1), -1));
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.commands[0].command, "echo second");
        assert_eq!(ds.set.commands[0].position, 0);
        assert_eq!(ds.set.commands[1].command, "echo first");
        assert_eq!(ds.set.commands[1].position, 1);
        assert_eq!(ds.cmd_editor.list.selected, 0);
    }

    #[test]
    fn test_reorder_command_down() {
        use crate::models::Command;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(Command {
            position: 0,
            command: "a".to_string(),
        });
        set.commands.push(Command {
            position: 1,
            command: "b".to_string(),
        });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::Reorder(ReorderKind::Command(0), 1));
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.commands[0].command, "b");
        assert_eq!(ds.set.commands[0].position, 0);
        assert_eq!(ds.set.commands[1].command, "a");
        assert_eq!(ds.set.commands[1].position, 1);
        assert_eq!(ds.cmd_editor.list.selected, 1);
    }

    #[test]
    fn test_reorder_command_down_boundary_noop() {
        use crate::models::Command;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(Command {
            position: 0,
            command: "only".to_string(),
        });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::Reorder(ReorderKind::Command(0), 1));
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.commands.len(), 1);
        assert_eq!(ds.set.commands[0].command, "only");
    }

    // ---- Trash push ----
    #[test]
    fn test_delete_set_pushes_to_trash() {
        let mut app = make_app();
        app.data = make_data_with_one_group();

        app.handle_action(AppAction::DeleteSet(0, 0));

        assert_eq!(app.trash.len(), 1);
        assert!(app.data.groups[0].sets.is_empty());
    }

    #[test]
    fn test_delete_group_pushes_to_trash() {
        let mut app = make_app();
        app.data = make_data_with_one_group();

        app.handle_action(AppAction::DeleteGroup(0));

        assert_eq!(app.trash.len(), 1);
        assert!(app.data.groups.is_empty());
    }

    #[test]
    fn test_delete_set_preserves_full_content_in_trash() {
        use crate::models::Command;
        let mut app = make_app();
        let mut g = Group::new("Deploy".to_string());
        let mut set = CommandSet::new("Prod".to_string(), g.id);
        set.commands.push(Command {
            position: 0,
            command: "echo hi".to_string(),
        });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };

        app.handle_action(AppAction::DeleteSet(0, 0));

        assert_eq!(app.trash.len(), 1);
        if let TrashedItem::Set {
            ref set,
            group_index,
            set_index,
        } = app.trash[0].item
        {
            assert_eq!(set.name, "Prod");
            assert_eq!(set.commands.len(), 1);
            assert_eq!(set.commands[0].command, "echo hi");
            assert_eq!(group_index, 0);
            assert_eq!(set_index, 0);
        } else {
            panic!("expected TrashedItem::Set");
        }
    }
}
