# 端到端集成测试 — 实施计划

> **For agentic workers:** 严格按 Task 顺序执行。每项后 `cargo check`。

**Goal:** 创建 `lib.rs` + `integration_tests.rs`，添加 5 组端到端集成测试。

**Architecture:** `lib.rs` 作为 crate 根持有所有 `mod` 声明；`main.rs` 变薄为 `use launcher::...`；`integration_tests.rs` 包含 5 组跨模块测试。

---

### Task 1: 创建 `src/lib.rs`

**文件:** Create: `src/lib.rs`

```rust
pub mod action;
pub mod app;
pub mod cli;
pub mod config;
pub mod error;
pub mod executor;
pub mod mode;
pub mod models;
pub mod storage;
pub mod tui;
pub mod ui;

#[cfg(test)]
mod integration_tests;
```

### Task 2: 精简 `src/main.rs`

**文件:** Modify: `src/main.rs`

将当前内容替换为：

```rust
use launcher::app::App;
use launcher::tui::{init_terminal, restore_terminal};
use std::io;

fn main() -> io::Result<()> {
    // CLI mode
    if let Some(exit_code) = launcher::cli::run_cli() {
        std::process::exit(exit_code);
    }

    let mut terminal = init_terminal()?;
    let mut app = App::new();

    let result = app.run(&mut terminal);

    restore_terminal()?;

    if let Err(e) = &result {
        eprintln!("Error: {}", e);
    }

    result
}
```

**注意：** 检查 `src/cli.rs` 是否仍有 `use crate::storage::load_app_data;` 等——这些在 `lib.rs` 模式下仍通过 `crate::` 路径工作，因为 `crate` 指向库 crate 根。如果 `cli.rs` 中有 `use crate::error::CliError;`，仍有效。

- [ ] **Step 2.1: 编译检查**

```bash
cargo check 2>&1
```

预期输出：编译通过。如果报错，检查 `use crate::xxx` 的路径——在 `lib.rs` 模式下，`crate::` 解析到库根（lib.rs），与之前 `main.rs` 的模块声明相同。

### Task 3: 修改 `storage.rs` — 暴露路径函数

**文件:** Modify: `src/storage.rs`

将两个函数签名从 `fn` 改为 `pub(crate) fn`：

```rust
// 第 23 行：fn load_app_data_from → pub(crate) fn load_app_data_from
pub(crate) fn load_app_data_from(path: &Path) -> Result<AppData, StorageError> {

// 第 65 行：fn save_app_data_to → pub(crate) fn save_app_data_to
pub(crate) fn save_app_data_to(data: &AppData, path: &Path, tmp: &Path) -> io::Result<()> {
```

- [ ] **Step 3.1: 编译检查**

```bash
cargo check 2>&1
```

### Task 4: 修改 `cli.rs` — 暴露解析函数

**文件:** Modify: `src/cli.rs`

将两个函数签名从 `fn` 改为 `pub(crate) fn`：

```rust
// 第 137 行：fn resolve_set → pub(crate) fn resolve_set
pub(crate) fn resolve_set<'a>(

// 第 186 行：fn resolve_variables → pub(crate) fn resolve_variables
pub(crate) fn resolve_variables(
```

- [ ] **Step 4.1: 编译检查**

```bash
cargo check 2>&1
```

### Task 5: 创建 `src/integration_tests.rs`

**文件:** Create: `src/integration_tests.rs`

```rust
#[cfg(test)]
mod tests {
    use crate::action::AppAction;
    use crate::models::{AppData, Group, CommandSet, Variable};
    use crate::storage;
    use crate::app::App;
    use crate::app::execution::ExecutionManager;
    use crate::app::toast::ToastManager;
    use crate::mode::AppMode;
    use crate::ui::theme::Theme;
    use crate::ui::main_screen::{MainScreenState, Panel};
    use crate::ui::detail_screen::{DetailScreenState, DetailFocus};
    use uuid::Uuid;

    // ------------------------------------------------------------------
    // Helper: create an App that doesn't touch the real config file
    // ------------------------------------------------------------------
    fn test_app() -> App {
        App {
            data: AppData::empty(),
            mode: AppMode::Main,
            running: true,
            main_screen: MainScreenState::new(),
            detail_screen: None,
            exec_screen: None,
            execution: ExecutionManager::new(),
            variable_screen: crate::ui::variable_screen::VariableScreenState::new(),
            pending_set: None,
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
        let data = AppData { groups: vec![group] };
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
    // 5.4 Main → Detail flow
    // ------------------------------------------------------------------
    #[test]
    fn test_main_to_detail_flow() {
        let mut g = Group::new("Test".to_string());
        g.sets.push(CommandSet::new("Demo".to_string(), g.id));
        let data = AppData { groups: vec![g.clone()] };

        // Main screen: Enter on set → ExecuteSet
        let mut main = MainScreenState::new();
        main.active_panel = Panel::Sets;
        let enter = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::empty(),
        );
        assert!(matches!(main.handle_key(enter, &data), AppAction::ExecuteSet(0, 0)));

        // 'e' → EditSet
        let e_key = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('e'),
            crossterm::event::KeyModifiers::empty(),
        );
        assert!(matches!(main.handle_key(e_key, &data), AppAction::EditSet(0, 0)));

        // Detail screen: edit → Ctrl+S save
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
            AppAction::ToggleAutoScroll,
            AppAction::CancelVariables,
        ];
        for action in actions {
            app.handle_action(action);
        }
    }
}
```

- [ ] **Step 5.1: 编译检查**

```bash
cargo check 2>&1
```

### Task 6: 最终验证 + 提交

- [ ] **Step 6.1: 运行全部测试**

```bash
cargo test 2>&1 | tail -20
```
预期输出：全部通过（128 个既有测试 + 5 个新集成测试 = 133 个）。

- [ ] **Step 6.2: Clippy**

```bash
cargo clippy 2>&1 | grep -E '^(error|warning:.*\])'
```
预期输出：无新增 warning。

- [ ] **Step 6.3: 格式化**

```bash
cargo fmt
```

- [ ] **Step 6.4: 提交**

```bash
git add src/lib.rs src/main.rs src/integration_tests.rs src/storage.rs src/cli.rs
git commit -m "test: 添加 lib.rs + integration_tests.rs（5 组端到端集成测试）"
```

**注意：** 如果 `cargo test` 的某些测试失败，检查：
- `test_app_crud_cycle` 中的 `handle_action(NewSet(0))` — 会调用 `auto_save()` 尝试写入真实路径。`unwrap_or_else` 吞掉错误，不影响测试结果。
- `test_cli_resolve_with_variables` — `resolve_variables` 函数在 `Cargo.toml` 中没有 `--var` 参数时会尝试从 stdin 读取。测试代码传递了 `&["host=prod"]` 作为 overrides，所以不会阻塞等待输入。
