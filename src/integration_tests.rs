#[cfg(test)]
mod tests {
    use crate::action::{AppAction, DeleteKind, ReorderKind};
    use crate::mode::AppMode;
    use crate::models::{AppData, CommandSet, Group, Variable};
    use crate::storage;
    use crate::ui::detail_screen::{DetailFocus, DetailScreenState};
    use crate::ui::main_screen::{MainScreenState, Panel};
    use uuid::Uuid;

    use crate::test_utils::make_app;
    use crate::test_utils::make_key;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // ------------------------------------------------------------------
    // 5.1 Storage full lifecycle
    // ------------------------------------------------------------------
    #[test]
    fn test_storage_full_lifecycle() {
        let tmp = std::env::temp_dir().join(format!("launcher_test_{}", Uuid::new_v4()));
        let path = tmp.join("sets.json");
        let tmp_path = tmp.join("sets.json.tmp");
        std::fs::create_dir_all(&tmp).unwrap();

        // Save empty
        let empty = AppData::empty();
        storage::save_app_data_to(&empty, &path, &tmp_path).unwrap();
        let loaded = storage::load_app_data_from(&path).unwrap();
        assert!(loaded.groups.is_empty());

        // Save with data
        let mut group = Group::new("test".to_string());
        group.sets.push(CommandSet::new("s1".to_string(), group.id));
        let data = AppData {
            groups: vec![group],
        };
        storage::save_app_data_to(&data, &path, &tmp_path).unwrap();

        let reloaded = storage::load_app_data_from(&path).unwrap();
        assert_eq!(reloaded.groups.len(), 1);
        assert_eq!(reloaded.groups[0].name, "test");
        assert_eq!(reloaded.groups[0].sets.len(), 1);
        assert_eq!(reloaded.groups[0].sets[0].name, "s1");

        // No leftover .tmp
        assert!(!tmp_path.exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ------------------------------------------------------------------
    // 5.2 App CRUD cycle
    // ------------------------------------------------------------------
    #[test]
    fn test_app_crud_cycle() {
        let mut app = make_app();

        app.handle_action(AppAction::NewGroup);
        assert_eq!(app.data.groups.len(), 1);

        app.handle_action(AppAction::RenameGroup(0, "renamed".to_string()));
        assert_eq!(app.data.groups[0].name, "renamed");

        app.handle_action(AppAction::NewSet(0));
        assert_eq!(app.data.groups[0].sets.len(), 1);
        assert_eq!(app.data.groups[0].sets[0].name, "New Command Set");

        app.handle_action(AppAction::DeleteSet(0, 0));
        assert!(app.data.groups[0].sets.is_empty());

        app.handle_action(AppAction::DeleteGroup(0));
        assert!(app.data.groups.is_empty());
    }

    // ------------------------------------------------------------------
    // 5.3 CLI resolve + variables
    // ------------------------------------------------------------------
    #[test]
    fn test_cli_resolve_with_variables() {
        use crate::cli::{resolve_set, resolve_variables};

        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.variables.push(Variable {
            name: "host".to_string(),
            default_value: "localhost".to_string(),
        });
        g.sets.push(set);
        let data = AppData { groups: vec![g] };

        let (s, _, _) = resolve_set(&data, None, Some("G".into()), Some("S".into())).unwrap();
        assert_eq!(s.name, "S");

        let vars = resolve_variables(s, &["host=prod".to_string()]).unwrap();
        assert_eq!(vars.get("host").unwrap(), "prod");
    }

    // ------------------------------------------------------------------
    // 5.4 Main to Detail flow
    // ------------------------------------------------------------------
    #[test]
    fn test_main_to_detail_flow() {
        let mut g = Group::new("Test".to_string());
        g.sets.push(CommandSet::new("Demo".to_string(), g.id));
        let data = AppData {
            groups: vec![g.clone()],
        };

        // Main screen: Enter on set -> ExecuteSet
        let mut main = MainScreenState::new();
        main.active_panel = Panel::Sets;
        let enter = make_key(KeyCode::Enter);
        assert!(matches!(
            main.handle_key(enter, &data),
            AppAction::ExecuteSet(0, 0)
        ));

        // 'e' -> EditSet
        let e_key = make_key(KeyCode::Char('e'));
        assert!(matches!(
            main.handle_key(e_key, &data),
            AppAction::EditSet(0, 0)
        ));

        // Detail screen: edit -> Ctrl+S save
        let set = CommandSet::new("Demo".to_string(), g.id);
        let groups = vec![g];
        let mut detail = DetailScreenState::new(set, groups);
        detail.focus = DetailFocus::Name;
        detail.handle_key(enter);
        assert!(detail.editing_name);

        let ctrl_s = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('s'),
            crossterm::event::KeyModifiers::CONTROL,
        );
        assert!(matches!(detail.handle_key(ctrl_s), AppAction::SaveSet(_)));
    }

    // ------------------------------------------------------------------
    // 5.5 Safe actions don't panic
    // ------------------------------------------------------------------
    #[test]
    fn test_safe_actions_do_not_panic() {
        let mut app = make_app();
        let actions = vec![
            AppAction::None,
            AppAction::Help,
            AppAction::NewGroup,
            AppAction::CancelEdit,
            AppAction::BackToMain,
            AppAction::CancelVariables,
        ];
        for action in actions {
            app.handle_action(action);
        }
    }

    // ------------------------------------------------------------------
    // 5.6 Delete confirmation flow
    // ------------------------------------------------------------------
    #[test]
    fn test_delete_set_with_confirmation_flow() {
        let mut app = make_app();
        // Set up data with one group and one set
        let mut group = Group::new("Test".to_string());
        let set = CommandSet::new("target-set".to_string(), group.id);
        group.sets.push(set);
        app.data = AppData { groups: vec![group] };

        // Step 1: Request delete via action
        app.handle_action(AppAction::RequestDelete(DeleteKind::Set {
            group_index: 0,
            set_index: 0,
            set_name: "target-set".to_string(),
        }));
        assert!(
            matches!(app.mode, AppMode::ConfirmDelete { .. }),
            "Should enter ConfirmDelete mode"
        );

        // Step 2: Press 'n' to cancel — set should remain
        let n_key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty());
        app.handle_key(n_key);
        assert_eq!(app.mode, AppMode::Main);
        assert_eq!(app.data.groups[0].sets.len(), 1);

        // Step 3: Request delete again, this time confirm with 'y'
        app.handle_action(AppAction::RequestDelete(DeleteKind::Set {
            group_index: 0,
            set_index: 0,
            set_name: "target-set".to_string(),
        }));
        assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));

        let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty());
        app.handle_key(y_key);
        assert_eq!(app.mode, AppMode::Main);
        assert!(app.data.groups[0].sets.is_empty());
    }

    // ------------------------------------------------------------------
    // 5.7 Command reorder flow
    // ------------------------------------------------------------------
    #[test]
    fn test_reorder_command_flow() {
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(crate::models::Command { position: 0, command: "first".to_string() });
        set.commands.push(crate::models::Command { position: 1, command: "second".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        // Move second command up
        app.handle_action(AppAction::Reorder(ReorderKind::Command(1), -1));
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.commands[0].command, "second");
        assert_eq!(ds.set.commands[0].position, 0);
        assert_eq!(ds.set.commands[1].command, "first");
        assert_eq!(ds.set.commands[1].position, 1);

        // Move first command down (back to original order)
        app.handle_action(AppAction::Reorder(ReorderKind::Command(0), 1));
        let ds2 = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds2.set.commands[0].command, "first");
        assert_eq!(ds2.set.commands[1].command, "second");
    }
}
