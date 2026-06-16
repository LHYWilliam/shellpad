---
title: "统一错误类型设计文档"
date: 2026-06-16
status: draft
---

## 1. 动机

项目当前有三种错误处理策略共存：

| 模块 | 当前方式 | 问题 |
|------|---------|------|
| `executor/blocking.rs` | `ExecuteError` 枚举 ✅ | 好模式，无需改动 |
| `storage.rs` | `Result<T, String>` | 调用者无法模式匹配；错误即兴格式化 |
| `cli.rs` | `Result<_, String>` | 同上 |
| `exec_screen/events.rs` | `tx.send(...).unwrap()` × 9 | 通道断开时 panics |

引入 `thiserror` 统一 `Result<T, String>` 为结构化错误类型，同时修复非测试 `unwrap()`。

## 2. 新增依赖

`Cargo.toml` 添加：

```toml
thiserror = "2"
```

## 3. 错误类型定义

### `src/error.rs` — 新文件

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
    Corrupted {
        path: String,
        backup: String,
        detail: String,
    },
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

## 4. 各模块修改

### 4.1 `storage.rs`

| 函数 | 当前签名 | 改为 |
|------|---------|------|
| `load_app_data()` | `Result<AppData, String>` | `Result<AppData, StorageError>` |
| `load_app_data_from()` | `Result<AppData, String>` | `Result<AppData, StorageError>` |
| `save_app_data()` | `io::Result<()>` | 不变 |
| `save_app_data_to()` | `io::Result<()>` | 不变 |

`load_app_data_from` 实现变更：

```rust
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

测试无需大改——`unwrap()` / `is_err()` 在 `Result<T, StorageError>` 上同样工作。但 `test_corrupted_file_backs_up` 只需 `assert!(is_err())`，不变。

`app.rs:40-42` 调用处调整：

```rust
// 当前：
storage::load_app_data().unwrap_or_else(|e| { eprintln!("{}", e); AppData::empty() })
// 改为：
storage::load_app_data().unwrap_or_else(|e| { eprintln!("{e}"); AppData::empty() })
```

### 4.2 `cli.rs`

| 函数 | 当前签名 | 改为 |
|------|---------|------|
| `resolve_set()` | `Result<..., String>` | `Result<..., CliError>` |
| `resolve_variables()` | `Result<..., String>` | `Result<..., CliError>` |

`resolve_set` 完整实现：

```rust
fn resolve_set<'a>(
    data: &'a AppData,
    id: Option<String>,
    group: Option<String>,
    set: Option<String>,
) -> Result<(&'a crate::models::CommandSet, usize, usize), CliError> {
    if let Some(id_str) = id {
        let uuid = Uuid::parse_str(&id_str).map_err(|_| CliError::InvalidUuid(id_str))?;
        let (gi, si) = data
            .find_set_by_id(uuid)
            .ok_or(CliError::SetNotFound(uuid.to_string()))?;
        Ok((&data.groups[gi].sets[si], gi, si))
    } else if let (Some(gname), Some(sname)) = (group, set) {
        let gl = gname.to_lowercase();
        let sl = sname.to_lowercase();
        let mut matches = Vec::new();
        for (gi, g) in data.groups.iter().enumerate() {
            if g.name.to_lowercase() == gl {
                for (si, s) in g.sets.iter().enumerate() {
                    if s.name.to_lowercase() == sl {
                        matches.push((gi, si));
                    }
                }
            }
        }
        match matches.len() {
            0 => Err(CliError::SetByGroupNotFound {
                group: gname,
                set: sname,
            }),
            1 => {
                let (gi, si) = matches[0];
                Ok((&data.groups[gi].sets[si], gi, si))
            }
            n => {
                let detail = matches
                    .iter()
                    .map(|&(gi, si)| {
                        let g = &data.groups[gi];
                        let s = &g.sets[si];
                        format!("  {} | {} | {}", s.id, g.name, s.name)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Err(CliError::Ambiguous { count: n, detail })
            }
        }
    } else {
        Err(CliError::MissingArgs)
    }
}
```

`resolve_variables` 实现变更：仅 `Err(...)` 改为 `Err(CliError::InvalidVar(...))`，其余逻辑不变。

测试调整：当前测试用 `unwrap_err().contains("text")` 验证错误消息。改为：

```rust
// 当前：assert!(result.unwrap_err().contains("Invalid UUID"));
// 改为：
let err = result.unwrap_err();
assert!(matches!(err, CliError::InvalidUuid(_)));
assert!(err.to_string().contains("Invalid UUID"));
```

这种改变涉及 `cli.rs` 中的 5 个测试：
- `test_resolve_set_not_found` — `contains("No command set found")` → `matches!(CliError::SetByGroupNotFound)`
- `test_resolve_set_no_args` — `contains("Specify")` → `matches!(CliError::MissingArgs)`
- `test_resolve_set_invalid_uuid` — `contains("Invalid UUID")` → `matches!(CliError::InvalidUuid)`
- `test_resolve_set_ambiguous` — `contains("Ambiguous")` → `matches!(CliError::Ambiguous)`

### 4.3 `execution_screen/events.rs`

9 处 `tx.send(...).unwrap()` 在非测试代码中。改为安全忽略：

```rust
// 当前：
tx.send(ExecutionEvent::Starting { index, command }).unwrap();

// 改为：
let _ = tx.send(ExecutionEvent::Starting { index, command });
```

涉及的位置：
- `process_events()` 中的每条 `send` 调用（Starting、StdoutLine、StderrLine、Finished、CompletedAll、Interrupted）

### 4.4 `executor/blocking.rs` — 不变

`ExecuteError` 已有 `#[derive(Debug)]` + `Display` + `impl std::error::Error`。已良好实现，不改为 `thiserror` 以保持改动最小。

### 4.5 `main.rs` — 新增模块声明

在 `main.rs` 中添加 `mod error;`：

```rust
mod action;
mod app;
mod cli;
mod config;
mod error;    // ← 新增
mod executor;
mod mode;
mod models;
mod storage;
mod tui;
mod ui;
```

### 4.6 调用处微调

`app.rs:40-42`：

```rust
// 当前：
storage::load_app_data().unwrap_or_else(|e| { eprintln!("{}", e); AppData::empty() })
// 改为：
storage::load_app_data().unwrap_or_else(|e| { eprintln!("{e}"); AppData::empty() })
```

`cli.rs:55-61`（`run_cli` 中 `load_app_data()` 的调用）：

`run_cli()` 返回 `Option<i32>` 而非 `Result`，无法使用 `?`。match 结构不变，仅错误类型从 `String` 变为 `StorageError`；`eprintln!("{}", e)` 表现一致（调用 `Display`）。

## 5. 变更清单总结

| 文件 | 操作 | 说明 |
|------|------|------|
| `Cargo.toml` | +1 dep | `thiserror = "2"` |
| `src/error.rs` | 新建 | `StorageError` + `CliError` 枚举 |
| `src/main.rs` | 修改 | +`mod error;` |
| `src/storage.rs` | 修改 | 2 个函数签名 + 实现 |
| `src/cli.rs` | 修改 | 2 个函数签名 + 5 个测试断言 |
| `src/app.rs` | 微调 | `eprintln!("{}")` → `eprintln!("{e}")` |
| `src/ui/execution_screen/events.rs` | 修改 | 9 处 `unwrap()` → `let _ =` |

## 6. 测试影响

| 测试文件 | 涉及改动 | 验证方式 |
|---------|---------|---------|
| `storage.rs` 5 个测试 | 无断言改动 | `cargo test storage` 全部通过 |
| `cli.rs` 4 个测试 | `unwrap_err().contains()` → `matches!()` | `cargo test cli` 全部通过 |
| 其余 119 个测试 | 无影响 | `cargo test` 全部通过 |

## 7. 非目标

- 不改动 `executor/` 的 `ExecuteError`（已良好）
- 不改动 `io::Result` 的 I/O 路径（`save_app_data`）
- 不改动 `anyhow` 处理（不引入）
- 不改动 test-only `unwrap()`（仅在测试中使用的安全 unwrap 保持原样）
