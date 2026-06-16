---
title: "端到端集成测试设计文档"
date: 2026-06-17
status: draft
---

## 1. 动机

项目现有 128 个单元测试覆盖了 widget 操作、键盘映射、事件处理、存储读写、CLI 解析等独立模块。但缺少跨模块的集成测试来验证完整的业务流程——如"创建组→重命名→删除"或"保存数据→重新加载→验证一致性"。

## 2. 约束

- 不改动任何生产代码的公共 API 可见性（`pub` 等级）
- 不引入新外部依赖
- 不创建 `tests/` 目录（避免强制 `pub` 污染）
- 所有集成测试在 `src/integration_tests.rs` 中，通过 `main.rs` 或 `lib.rs` 的条件编译引用

## 3. 新增文件

### 3.1 `src/lib.rs`

标准 Rust 模式：二进制 crate 同时拥有 `lib.rs` 和 `main.rs`。`lib.rs` 持有所有模块声明，`main.rs` 引用 `lib.rs`。

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
```

### 3.2 `src/main.rs` 精简

将当前 `main.rs` 中的 `mod` 声明全部移到 `lib.rs`，改为 `use` 导入：

```rust
use launcher::app::App;
use launcher::tui::{init_terminal, restore_terminal};
use std::io;

fn main() -> io::Result<()> {
    // CLI mode: if a subcommand is given, handle it and exit
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

### 3.3 `src/integration_tests.rs`

新建文件，包含所有集成测试。通过 `lib.rs` 的条件编译引用：

在 `lib.rs` 末尾添加：
```rust
#[cfg(test)]
mod integration_tests;
```

测试文件结构：
```rust
#[cfg(test)]
mod tests {
    // 各测试模块
}
```

## 4. 生产代码改动

### 4.1 `storage.rs` — 暴露路径函数为 `pub(crate)`

为了让集成测试使用临时路径而不污染真实配置，需要将两个路径函数暴露为 `pub(crate)`：

```rust
// 当前：fn load_app_data_from(path: &Path) -> Result<AppData, StorageError>
// 改为：
pub(crate) fn load_app_data_from(path: &Path) -> Result<AppData, StorageError>

// 当前：fn save_app_data_to(data: &AppData, path: &Path, tmp: &Path) -> io::Result<()>
// 改为：
pub(crate) fn save_app_data_to(data: &AppData, path: &Path, tmp: &Path) -> io::Result<()>
```

这两处 `pub(crate)` 改动不构成对外 API 泄露（仅同 crate 可见）。

### 4.2 `config.rs` — 不需要改动

测试用临时路径不经过 `config.rs`。`data_file_path()` 等函数保持不变。

### 4.2 `cli.rs` — 暴露解析函数为 `pub(crate)`

为了让 `integration_tests.rs` 调用 CLI 解析函数（`Cli` 结构体私有，无法直接访问），两个解析函数改为 `pub(crate)`：

```rust
// 当前：fn resolve_set(...) -> ...
// 改为：
pub(crate) fn resolve_set(...) -> ...

// 当前：fn resolve_variables(...) -> ...
// 改为：
pub(crate) fn resolve_variables(...) -> ...
```

## 5. 集成测试设计

### 6.1 存储全生命周期测试

```rust
#[test]
fn test_storage_full_lifecycle() {
    let tmp = std::env::temp_dir().join(format!("launcher_test_{}", uuid::Uuid::new_v4()));
    let path = tmp.join("sets.json");
    let tmp_path = tmp.join("sets.json.tmp");
    std::fs::create_dir_all(&tmp).unwrap();

    // 1. 保存空数据
    let empty = crate::models::AppData::empty();
    crate::storage::save_app_data_to(&empty, &path, &tmp_path).unwrap();

    // 2. 加载 → 空
    let loaded = crate::storage::load_app_data_from(&path).unwrap();
    assert!(loaded.groups.is_empty());

    // 3. 添加数据后保存
    let mut group = crate::models::Group::new("test".to_string());
    group.sets.push(crate::models::CommandSet::new("s1".to_string(), group.id));
    let data = crate::models::AppData { groups: vec![group] };
    crate::storage::save_app_data_to(&data, &path, &tmp_path).unwrap();

    // 4. 加载 → 验证数据
    let reloaded = crate::storage::load_app_data_from(&path).unwrap();
    assert_eq!(reloaded.groups.len(), 1);
    assert_eq!(reloaded.groups[0].name, "test");
    assert_eq!(reloaded.groups[0].sets.len(), 1);
    assert_eq!(reloaded.groups[0].sets[0].name, "s1");

    // 5. 无残留 .tmp 文件
    assert!(!tmp_path.exists());

    let _ = std::fs::remove_dir_all(&tmp);
}
```

### 6.2 App CRUD 循环测试

直接构造 `App` 结构体（所有字段均为 `pub`），避免 `App::new()` 加载/写入真实配置。

```rust
fn create_test_app() -> crate::app::App {
    crate::app::App {
        data: crate::models::AppData::empty(),
        mode: crate::mode::AppMode::Main,
        running: true,
        main_screen: crate::ui::main_screen::MainScreenState::new(),
        detail_screen: None,
        exec_screen: None,
        execution: crate::app::execution::ExecutionManager::new(),
        variable_screen: crate::ui::variable_screen::VariableScreenState::new(),
        pending_set: None,
        theme: crate::ui::theme::Theme::default_dark(),
        toasts: crate::app::toast::ToastManager::new(),
    }
}

#[test]
fn test_app_crud_cycle() {
    let mut app = create_test_app();
    use crate::action::AppAction;

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
```

### 6.3 CLI 解析端到端测试

`resolve_set` 和 `resolve_variables` 已改为 `pub(crate)`，可在 `integration_tests.rs` 中直接调用：

```rust
#[test]
fn test_cli_resolve_with_variables() {
    use crate::cli::{resolve_set, resolve_variables};
    use crate::models::{AppData, Group, CommandSet};

    let mut g = Group::new("G".to_string());
    let mut set = CommandSet::new("S".to_string(), g.id);
    set.variables.push(crate::models::Variable {
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
```

### 6.4 跨组件操作测试（main→detail 流程）

```rust
#[test]
fn test_main_to_detail_flow() {
    use crate::action::AppAction;
    use crate::models::{AppData, Group, CommandSet};

    let mut g = Group::new("Test".to_string());
    g.sets.push(CommandSet::new("Demo".to_string(), g.id));
    let data = AppData { groups: vec![g] };

    let mut main_screen = crate::ui::main_screen::MainScreenState::new();
    main_screen.active_panel = crate::ui::main_screen::Panel::Sets;

    let enter = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::empty(),
    );
    assert!(matches!(main_screen.handle_key(enter, &data), AppAction::ExecuteSet(0, 0)));

    let e_key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('e'),
        crossterm::event::KeyModifiers::empty(),
    );
    assert!(matches!(main_screen.handle_key(e_key, &data), AppAction::EditSet(0, 0)));

    // Detail screen: edit name → Ctrl+S save
    let group = Group::new("Test".to_string());
    let set = CommandSet::new("Demo".to_string(), group.id);
    let groups = data.groups.clone();
    let mut detail = crate::ui::detail_screen::DetailScreenState::new(set, groups);
    detail.focus = crate::ui::detail_screen::DetailFocus::Name;
    detail.handle_key(enter);
    assert!(detail.editing_name);

    let ctrl_s = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('s'),
        crossterm::event::KeyModifiers::CONTROL,
    );
    assert!(matches!(detail.handle_key(ctrl_s), AppAction::SaveSet(_)));
}
```

### 6.5 安全变体不 panic

```rust
#[test]
fn test_safe_actions_do_not_panic() {
    let mut app = create_test_app();
    use crate::action::AppAction;

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
```

## 6. 变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/lib.rs` | 新建 | 模块声明，引用 `integration_tests.rs` |
| `src/main.rs` | 修改 | `mod` → `use launcher::...` |
| `src/integration_tests.rs` | 新建 | 5 组集成测试 |
| `src/storage.rs` | 修改 | `load_app_data_from` + `save_app_data_to` 改为 `pub(crate)` |
| `src/cli.rs` | 修改 | `resolve_set` + `resolve_variables` 改为 `pub(crate)` |

## 7. 非目标

- 不创建 `tests/` 目录
- 不引入 `tempfile` 等 dev-dependency（使用 `std::env::temp_dir()` + `Uuid` 生成临时路径，复用现有模式）
- 不改动 `executor/`（不测真实线程执行）
- 不测 `render()` 方法（需要终端）
- 不测 `run()` 事件循环（阻塞）

## 8. 验证

```bash
cargo test      # 128 + 新集成测试全部通过
cargo run       # 确保 main.rs 改动不影响正常启动
```
