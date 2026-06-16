# 统一错误类型 — 实施计划

> **For agentic workers:** 按 Task 顺序执行，每项后 `cargo check`。不要跳过任何步骤。

**Goal:** 用 `thiserror` 结构化 `Result<T, String>` 为 `StorageError`/`CliError`，修复非测试 `unwrap()`。

**Architecture:** 新增 `src/error.rs` 定义错误枚举，`storage.rs` 和 `cli.rs` 切换签名，`events.rs` 替换 unwrap。

---

### Task 1: 依赖 + error.rs

- [ ] **Step 1.1: Cargo.toml 添加 thiserror**

```toml
thiserror = "2"
```

- [ ] **Step 1.2: 创建 `src/error.rs`**

```rust
use std::io;
use thiserror::Error;

/// Storage-layer errors (load/corruption/save).
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Failed to create config directory: {0}")]
    CreateDir(String),
    #[error("Corrupted data file `{path}`, backed up to `{backup}`: {detail}")]
    Corrupted { path: String, backup: String, detail: String },
    #[error("Failed to read data file: {0}")]
    ReadFailed(String),
}

/// CLI parsing/resolution errors.
#[derive(Debug, Error)]
pub enum CliError {
    #[error("Invalid UUID: {0}")]
    InvalidUuid(String),
    #[error("No command set with UUID {0}")]
    SetNotFound(String),
    #[error("No command set found for group '{group}' set '{set}'")]
    SetByGroupNotFound { group: String, set: String },
    #[error("Ambiguous: found {count} matches:\n{detail}")]
    Ambiguous { count: usize, detail: String },
    #[error("Invalid --var format '{0}' (expected key=value)")]
    InvalidVar(String),
    #[error("Missing argument: specify --id or --group --set")]
    MissingArgs,
    #[error(transparent)]
    Storage(#[from] StorageError),
}
```

### Task 2: 修改 `storage.rs`

- [ ] **Step 2.1: 替换签名和实现**

将 `load_app_data` 和 `load_app_data_from` 的返回类型从 `Result<AppData, String>` 改为 `Result<AppData, StorageError>`。

在文件顶部 adds import：
```rust
use crate::error::StorageError;
```

完整新实现：

```rust
pub fn load_app_data() -> Result<AppData, StorageError> {
    let path = data_file_path();
    load_app_data_from(&path)
}

fn load_app_data_from(path: &Path) -> Result<AppData, StorageError> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| StorageError::CreateDir(format!(
                "{}: {}", parent.display(), e
            )))?;
        }
        return Ok(AppData::empty());
    }

    let content = fs::read_to_string(path).map_err(|e| {
        StorageError::ReadFailed(format!("{}: {}", path.display(), e))
    })?;

    match serde_json::from_str(&content) {
        Ok(data) => Ok(data),
        Err(e) => {
            let bak = path.with_extension("json.bak");
            let path_str = path.display().to_string();
            let bak_str = bak.display().to_string();
            eprintln!("Corrupted data, backing up to .bak");
            let _ = fs::rename(path, &bak);
            if let Some(parent) = path.parent()
                && let Ok(dir) = fs::File::open(parent)
            {
                let _ = dir.sync_all();
            }
            Err(StorageError::Corrupted {
                path: path_str,
                backup: bak_str,
                detail: e.to_string(),
            })
        }
    }
}
```

- [ ] **Step 2.2: 编译检查**

```bash
cargo check
```

### Task 3: 修改 `cli.rs`

- [ ] **Step 3.1: 添加 import**

```rust
use crate::error::CliError;
```

- [ ] **Step 3.2: 修改 `resolve_set` 签名和实现**

签名：
```rust
fn resolve_set<'a>(
    data: &'a AppData,
    id: Option<String>,
    group: Option<String>,
    set: Option<String>,
) -> Result<(&'a crate::models::CommandSet, usize, usize), CliError> {
```

完整实现中，所有 `Err(format!(...))` 替换为对应的 `CliError` 变体：
- `format!("Invalid UUID: ...")` → `CliError::InvalidUuid(id_str)`
- `format!("No command set with UUID ...")` → `CliError::SetNotFound(uuid.to_string())`
- `format!("No command set found for group '{}' set '{}'", ...)` → `CliError::SetByGroupNotFound { group: gname, set: sname }`
- 多行 `msg` 构建 → `CliError::Ambiguous { count: n, detail }`（detail 用 join("\n") 构建）
- `"...Specify --id..."` → `CliError::MissingArgs`

- [ ] **Step 3.3: 修改 `resolve_variables` 签名和实现**

签名：
```rust
fn resolve_variables(
    set: &crate::models::CommandSet,
    overrides: &[String],
) -> Result<HashMap<String, String>, CliError> {
```

唯一 `Err` 替换：
```rust
Err(CliError::InvalidVar(format!(
    "Invalid --var format '{}' (expected key=value)", ov
)))
```

- [ ] **Step 3.4: 更新测试断言**

修改 5 个测试，用 `matches!()` 替代 `unwrap_err().contains()`：

```rust
// test_resolve_set_by_id — 不变（Ok 路径）

// test_resolve_set_by_group_and_set_name — 不变（Ok 路径）

#[test]
fn test_resolve_set_not_found() {
    let data = AppData::empty();
    let result = resolve_set(&data, None, Some("Missing".into()), Some("Missing".into()));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), CliError::SetByGroupNotFound { .. }));
}

#[test]
fn test_resolve_set_no_args() {
    let data = AppData::empty();
    let result = resolve_set(&data, None, None, None);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), CliError::MissingArgs));
}

#[test]
fn test_resolve_set_invalid_uuid() {
    let data = AppData::empty();
    let result = resolve_set(&data, Some("not-a-uuid".into()), None, None);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), CliError::InvalidUuid(_)));
}

#[test]
fn test_resolve_set_ambiguous() {
    let mut g = Group::new("G".to_string());
    let set = CommandSet::new("S".to_string(), g.id);
    g.sets.push(set);
    let set2 = CommandSet::new("S".to_string(), g.id);
    g.sets.push(set2);
    let data = AppData { groups: vec![g] };
    let result = resolve_set(&data, None, Some("G".into()), Some("S".into()));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), CliError::Ambiguous { .. }));
}

// 其余测试不变
```

- [ ] **Step 3.5: 编译检查**

```bash
cargo check
```

### Task 4: 修复 events.rs unwrap

- [ ] **Step 4: 替换 9 处 `unwrap()`**

在 `src/ui/execution_screen/events.rs` 中，将所有 `tx.send(...).unwrap()` 替换为 `let _ = tx.send(...);`。

匹配模式：查找 `tx.send(ExecutionEvent::` 后跟 `.unwrap()` 的行。替换 `tx.send(...).unwrap();` → `let _ = tx.send(...);`。

涉及的调用点：
- `ExecutionEvent::Starting { index, command }`
- `ExecutionEvent::StdoutLine { index, line }`
- `ExecutionEvent::StderrLine { index, line }`
- `ExecutionEvent::Finished { index, success, duration_ms }`
- `ExecutionEvent::CompletedAll { total, succeeded, failed, total_duration_ms }`
- `ExecutionEvent::Interrupted { last_index }`

### Task 5: main.rs + app.rs 微调

- [ ] **Step 5.1: `main.rs` 添加 `mod error;`**

在 `mod action;` 附近添加 `mod error;`。

- [ ] **Step 5.2: `app.rs` 格式化更新**

`eprintln!("{}", e)` → `eprintln!("{e}")`（第 41 行）

### Task 6: 最终验证

- [ ] **Step 6.1: 编译 + 测试**

```bash
cargo check
cargo test      # 128 个全通过
cargo clippy    # 无新增 warning
cargo fmt
```

- [ ] **Step 6.2: 提交**

```bash
git add src/error.rs src/main.rs src/storage.rs src/cli.rs src/app.rs src/ui/execution_screen/events.rs Cargo.toml Cargo.lock
git commit -m "refactor: 统一错误类型 — thiserror + StorageError/CliError + 修复 unwrap"
```
