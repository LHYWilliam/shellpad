# Bug Fix — Help Global Shortcut + ReExec Regression

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 ReExec 卡死（Round 4 ExecutionState 枚举迁移回归）和 `?` 帮助菜单在 Detail/Execution 模式失效两个 bug

**Architecture:** 两个独立 bug 共享 `app/handler.rs` 修改点。Bug A 在 teardown 前保存 pending_set；Bug B 在 handle_key 顶部添加全局 `?` 检查。3 个新测试。

**Tech Stack:** Rust 2024 edition, crossterm 0.29

---

## 文件变更总览

| 文件 | 操作 | 涉及 |
|------|------|------|
| `src/app/handler.rs` | 修改 | ReExec handler + 全局 `?` 检查 + 3 tests |
| `src/ui/main_screen/handler.rs` | 修改 | 删除 `?` case |

---

### T1: Bug A — ReExec 卡死修复

**文件：**
- Modify: `src/app/handler.rs`（ReExec handler + test）

- [ ] **Step 1: 先追加失败测试**

在 `test_handler_skip_current` 之后、`}` 之前插入：

```rust
    #[test]
    fn test_handler_re_exec() {
        use crate::ui::execution_screen::ExecutionScreenState;
        use crate::models::Command;
        let mut app = make_app();
        app.data = make_data_with_one_group();
        let cmds = vec![Command { position: 0, command: "ok".to_string() }];
        app.execution_state = ExecutionState::Running {
            screen: ExecutionScreenState::new("t".to_string(), &cmds),
            manager: ExecutionManager::new(),
            pending_set: (0, 0),
        };
        app.mode = AppMode::Execution;

        app.handle_action(AppAction::ReExec);
        assert_eq!(app.mode, AppMode::Execution);
        assert!(matches!(app.execution_state, ExecutionState::Running { .. }));
    }
```

- [ ] **Step 2: 运行确认测试失败**

```bash
cargo test test_handler_re_exec 2>&1 | tail -5
```

预期：`FAILED` — ReExec 后 `execution_state` 是 `Idle` 而非 `Running`。

- [ ] **Step 3: 修复 ReExec handler**

原文（`src/app/handler.rs:207-212`）：

```rust
            AppAction::ReExec => {
                self.teardown_execution(false, false);
                if let ExecutionState::Running { pending_set: (gi, si), .. } = self.execution_state {
                    self.do_execute_with(gi, si, 0);
                }
            }
```

改为：

```rust
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
```

- [ ] **Step 4: 验证**

```bash
cargo test test_handler_re_exec   # PASS
cargo test                        # 163 pass
cargo clippy
```

- [ ] **Step 5: Commit**

```bash
git add src/app/handler.rs
git commit -m "fix: save pending_set before teardown in ReExec handler"
```

---

### T2: Bug B — 全局 `?` Help 快捷键

**文件：**
- Modify: `src/app/handler.rs`（全局检查 + import + 2 tests）
- Modify: `src/ui/main_screen/handler.rs`（删除 `?` case）

- [ ] **Step 1: 追加两个失败测试**

在 `test_handler_re_exec` 之后追加：

```rust
    #[test]
    fn test_help_from_detail_mode() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        let set = app.data.groups[0].sets[0].clone();
        let groups = app.data.groups.clone();
        app.detail_screen = Some(DetailScreenState::new(set, groups));
        app.mode = AppMode::Detail;

        let key = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('?'),
            crossterm::event::KeyModifiers::empty(),
        );
        app.handle_key(key);
        assert_eq!(app.mode, AppMode::Help);
    }

    #[test]
    fn test_help_from_execution_mode() {
        use crate::ui::execution_screen::ExecutionScreenState;
        use crate::models::Command;
        let mut app = make_app();
        let cmds = vec![Command { position: 0, command: "x".to_string() }];
        app.execution_state = ExecutionState::Running {
            screen: ExecutionScreenState::new("t".to_string(), &cmds),
            manager: ExecutionManager::new(),
            pending_set: (0, 0),
        };
        app.mode = AppMode::Execution;

        let key = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('?'),
            crossterm::event::KeyModifiers::empty(),
        );
        app.handle_key(key);
        assert_eq!(app.mode, AppMode::Help);
    }
```

- [ ] **Step 2: 运行确认测试失败**

```bash
cargo test test_help_from_detail_mode test_help_from_execution_mode 2>&1 | tail -5
```

预期：两个都 `FAILED` — `?` 键在 Detail/Execution 模式不触发 Help。

- [ ] **Step 3: 添加 `KeyCode` import**

```rust
// app/handler.rs — 在现有的 use block 中追加 KeyCode 导入
use crossterm::event::KeyCode;
```

- [ ] **Step 4: 在 `handle_key()` 顶部添加全局 `?` 检查**

在 `if self.variable_screen.active { ... return; }` 之后、`match self.mode {` 之前插入：

```rust
        // Global Help shortcut — works in all modes
        if key.code == KeyCode::Char('?') {
            self.handle_action(AppAction::Help);
            return;
        }
```

- [ ] **Step 5: 从 `main_screen/handler.rs` 删除 `?` case**

```rust
// main_screen/handler.rs — 删除此行（line 183）
            KeyCode::Char('?') => AppAction::Help,
```

- [ ] **Step 6: 验证**

```bash
cargo test test_help_from_detail_mode   # PASS
cargo test test_help_from_execution_mode # PASS
cargo test                              # 165 pass
cargo clippy
```

- [ ] **Step 7: Commit**

```bash
git add src/app/handler.rs src/ui/main_screen/handler.rs
git commit -m "fix: add global ? key handler for Help in all modes"
```

---

## 验证清单

所有任务完成后：

- [ ] `cargo test` — 165 pass（162 + 3 new）
- [ ] `cargo clippy` — 无新增 warning
- [ ] `cargo run` — 手动验证：执行完成后按 `r` 重执行；Detail/Execution 界面按 `?` 弹出帮助
