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

## 5. 集成测试设计

### 5.1 存储全生命周期测试

```rust
#[test]
fn test_storage_full_lifecycle() {
    // 1. 在临时目录中创建空的 AppData
    // 2. save 到临时路径
    // 3. load 回来 → 验证为空
    // 4. 添加 Group + CommandSet，save
    // 5. load 回来 → 验证包含数据
    // 6. 覆盖 save（第二次）
    // 7. load 回来 → 验证数据一致
    // 8. 确认没有遗留 .tmp 文件
}
```

### 5.2 App CRUD 循环测试

```rust
#[test]
fn test_app_crud_cycle() {
    // 1. 创建 App 实例
    // 2. dispatch AppAction::NewGroup
    // 3. 验证 data.groups.len() == 1
    // 4. dispatch AppAction::RenameGroup(0, "new name")
    // 5. 验证 data.groups[0].name == "new name"
    // 6. dispatch AppAction::NewSet(0)
    // 7. 验证 data.groups[0].sets.len() == 1
    // 8. dispatch AppAction::DeleteSet(0, 0)
    // 9. 验证 data.groups[0].sets.is_empty()
    // 10. dispatch AppAction::DeleteGroup(0)
    // 11. 验证 data.groups.is_empty()
}
```

注意：此测试中 `auto_save()` 会尝试写入真实配置路径。当前实现中 `unwrap_or_else` 会静默吞掉写入错误，因此测试行为不受影响。但更好的做法是在测试前备份并恢复。

### 5.3 CLI 参数解析端到端测试

```rust
#[test]
fn test_cli_resolve_full_flow() {
    // 1. 构造测试 AppData
    // 2. 用 Cli::try_parse_from 模拟 --id 参数
    // 3. resolve_set 返回正确 set
    // 4. 用 Cli::try_parse_from 模拟 --group --set 参数
    // 5. resolve_set 返回正确 set
    // 6. resolve_variables 解析 key=value 对
    // 7. 测试缺失参数 → MissingArgs 错误
    // 8. 测试无效 UUID → InvalidUuid 错误
}
```

### 5.4 搜索高亮 + 过滤集成测试

```rust
#[test]
fn test_search_then_edit_then_verify() {
    // 1. 创建 detail_screen + groups
    // 2. 模拟搜索、选中结果
    // 3. 进入 detail_screen
    // 4. 修改变量/命令
    // 5. Save → 验证数据持久化
}
```

### 5.5 合并 match 测试计划

```rust
#[test]
fn test_all_modes_do_not_panic() {
    // 验证 handle_action 对每个 AppAction 变体都能正常返回而不 panic
    // 不需要验证副作用，只需要不 panic
}
```

## 6. 变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/lib.rs` | 新建 | 模块声明，引用 `integration_tests.rs` |
| `src/main.rs` | 修改 | `mod` → `use launcher::...` |
| `src/integration_tests.rs` | 新建 | 5 组集成测试 |
| `src/storage.rs` | 修改 | `load_app_data_from` + `save_app_data_to` 改为 `pub(crate)` |

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
