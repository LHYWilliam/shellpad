#[cfg(test)]
mod tests {
    use crate::action::AppAction;
    use crate::app::App;
    use crate::app::toast::ToastManager;
    use crate::mode::AppMode;
    use crate::models::{AppData, CommandSet, Group, Variable};
    use crate::storage;
    use crate::ui::detail_screen::{DetailFocus, DetailScreenState};
    use crate::ui::main_screen::{MainScreenState, Panel};
    use crate::ui::theme::Theme;
    use uuid::Uuid;

    // ------------------------------------------------------------------
    // Helper: create an App that doesn't touch the real config file
    // ------------------------------------------------------------------
    fn test_app() -> App {
        use crate::app::ExecutionState;
        App {
            data: AppData::empty(),
            mode: AppMode::Main,
            running: true,
            main_screen: MainScreenState::new(),
            detail_screen: None,
            execution_state: ExecutionState::Idle { pending_set: None },
            variable_screen: crate::ui::variable_screen::VariableScreenState::new(),
            theme: Theme::default_dark(),
            toasts: ToastManager::new(),
        }
    }

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
        let mut app = test_app();

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
        let enter = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::empty(),
        );
        assert!(matches!(
            main.handle_key(enter, &data),
            AppAction::ExecuteSet(0, 0)
        ));

        // 'e' -> EditSet
        let e_key = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('e'),
            crossterm::event::KeyModifiers::empty(),
        );
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
        let mut app = test_app();
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
}
