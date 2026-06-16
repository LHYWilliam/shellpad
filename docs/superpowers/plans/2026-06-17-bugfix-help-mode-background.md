# Bug Fix — Help Modal Background + Exit Target

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Help 模式记录进入前的 mode，还原背景渲染和退出目标

**Architecture:** 单任务，4 文件修改。`App` 新增 `prev_mode: Option<AppMode>`，`handle_action(Help)` 保存/恢复，渲染根据 `prev_mode` 选择背景，测试更新。

**Tech Stack:** Rust 2024 edition, ratatui 0.30

---

## 文件变更

| 文件 | 操作 |
|------|------|
| `src/app.rs` | struct 新增 `prev_mode`，`new()` 初始化 |
| `src/app/handler.rs` | `handle_action(Help)` + `handle_key` Help arm 使用 prev_mode；测试追加/更新 |
| `src/app/render.rs` | Help 背景渲染根据 prev_mode |

---

### Task 1: 引入 `prev_mode` 并修复 Help 行为

- [ ] **Step 1: 新增字段**

`src/app.rs` — struct + new()：

```rust
// struct 内追加（在 pub toasts: ToastManager 之前）
    pub prev_mode: Option<AppMode>,

// new() 内追加
            prev_mode: None,
```

- [ ] **Step 2: 新增 `handle_action(Help)` 双向逻辑**

`src/app/handler.rs` — 将单行 `Help => self.mode = AppMode::Help,` 替换为：

```rust
            AppAction::Help => {
                if self.mode == AppMode::Help {
                    // Dismiss Help — restore previous mode
                    self.mode = self.prev_mode.take().unwrap_or(AppMode::Main);
                } else {
                    // Enter Help — save current mode, clean up if needed
                    if self.mode == AppMode::Execution {
                        self.teardown_execution(false, false);
                    }
                    self.prev_mode = Some(self.mode);
                    self.mode = AppMode::Help;
                }
            }
```

- [ ] **Step 3: 更新 `handle_key` 中 Help 退出 arm**

`src/app/handler.rs` — 将 `AppMode::Help => self.mode = AppMode::Main,` 替换为：

```rust
            AppMode::Help => {
                self.mode = self.prev_mode.take().unwrap_or(AppMode::Main);
            }
```

- [ ] **Step 4: 更新 Help 渲染背景**

`src/app/render.rs` — 将 Help 分支的 `self.main_screen.render(...)` 替换为 prev_mode 分派：

```rust
            AppMode::Help => {
                match self.prev_mode {
                    Some(AppMode::Detail) => {
                        if let Some(ref mut ds) = self.detail_screen {
                            ds.render(frame, content_area, &self.theme);
                        }
                    }
                    Some(AppMode::Execution) => {
                        if let ExecutionState::Running { ref screen, .. } = self.execution_state {
                            screen.render(frame, content_area, &self.theme);
                        }
                    }
                    _ => {
                        self.main_screen
                            .render(frame, content_area, &self.data, &self.theme);
                    }
                }
                draw_help(frame, content_area, &self.theme);
            }
```

`render.rs` header 需要追加 `ExecutionState` import：`use super::{App, ExecutionState};`

- [ ] **Step 5: 更新现有测试**

更新 `make_app()` 辅助函数（新增 `prev_mode` 字段）：

```rust
    fn make_app() -> App {
        App {
            data: AppData::empty(),
            mode: AppMode::Main,
            running: true,
            main_screen: MainScreenState::new(),
            detail_screen: None,
            execution_state: ExecutionState::Idle { pending_set: None },
            prev_mode: None,
            variable_screen: VariableScreenState::new(),
            theme: Theme::default_dark(),
            toasts: ToastManager::new(),
        }
    }
```

更新 `test_handler_help` 使其反映新行为（Help 双向切换）：

```rust
    #[test]
    fn test_handler_help() {
        let mut app = make_app();
        // Enter Help from Main
        app.handle_action(AppAction::Help);
        assert_eq!(app.mode, AppMode::Help);
        assert_eq!(app.prev_mode, Some(AppMode::Main));
        // Dismiss Help — should return to Main
        app.handle_action(AppAction::Help);
        assert_eq!(app.mode, AppMode::Main);
        assert!(app.prev_mode.is_none());
    }
```

更新 `test_help_from_detail_mode` 验证 prev_mode 正确保存：

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
        assert_eq!(app.prev_mode, Some(AppMode::Detail));
    }
```

更新 `test_help_from_execution_mode` 验证清理 + prev_mode：

```rust
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
        // execution_state should have been cleaned up
        assert!(matches!(app.execution_state, ExecutionState::Idle { .. }));
        assert_eq!(app.prev_mode, Some(AppMode::Main));
        // After teardown, prev_mode is set to Main (not Execution)
    }
```

注意：spec 中设计的是从 Execution 进入 Help 前调用 `teardown_execution(false, false)`，这会清除 `execution_state`。然后 `prev_mode` 应设为 Main（因为 Execution 状态已销毁）。这符合直觉——用户退出 Help 后回到 Main，不会再卡在脏的执行状态上。

- [ ] **Step 6: 验证**

```bash
cargo check
cargo test              # 165 pass（测试更新，计数不变）
cargo clippy
```

- [ ] **Step 7: Commit**

```bash
git add src/app.rs src/app/handler.rs src/app/render.rs
git commit -m "fix: implement prev_mode to restore correct background and exit target for Help"
```
