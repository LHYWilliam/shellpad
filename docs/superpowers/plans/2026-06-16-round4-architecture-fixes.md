# 第四轮架构优化实施方案

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成 handler 执行路径测试覆盖，并将松散耦合的执行状态字段合并为 `ExecutionState` 枚举

**Architecture:** 2 个顺序任务。T1 追加 8 个 handler 测试（DeleteVariable、DeleteCommand、CancelVariables、ConfirmVariables、ExecuteSet、BackToMain、SkipCurrent），覆盖不需要真实执行线程的路径。T2 引入 `ExecutionState` 枚举替代 `exec_screen`/`pending_set`/`execution` 三个独立字段，所有 match 变换为枚举分派，消除非法状态组合。

**Tech Stack:** Rust 2024 edition, std::sync::mpsc, std::thread, std::sync::atomic

---

## 文件变更总览

| 文件 | 操作 | 涉及任务 |
|------|------|---------|
| `src/app/handler.rs` | 修改（追加测试） | T1 |
| `src/app.rs` | 修改（ExecutionState 枚举 + struct 变更） | T2 |
| `src/app/handler.rs` | 修改（exec_screen/pending_set 访问点） | T2 |
| `src/app/render.rs` | 修改（exec_screen 访问点） | T2 |
| `src/app/execution.rs` | 不变（保持独立，被 ExecutionState::Running 包含） | — |

---

### T1: Handler 执行路径测试

**文件：**
- Modify: `src/app/handler.rs`（追加 8 个测试函数到末尾 tests 模块）

**策略：** 测试不需要真实执行线程的 handler action。包括：detail 屏的 DeleteVariable/DeleteCommand（含 focus 迁移）、变量覆盖层的 CancelVariables/ConfirmVariables、执行相关 action（ExecuteSet 无变量路径、BackToMain、SkipCurrent）。

- [ ] **Step 1: 创建含 variables 和 commands 的测试数据辅助函数**

在现有 `make_data_with_one_group()` 之后追加：

```rust
    fn make_data_with_vars_and_cmds() -> AppData {
        use crate::models::Variable;
        use crate::models::Command;
        let mut g = Group::new("Deploy".to_string());
        let mut set = CommandSet::new("Prod".to_string(), g.id);
        set.variables.push(Variable {
            name: "host".to_string(),
            default_value: "localhost".to_string(),
        });
        set.commands.push(Command { position: 0, command: "echo hi".to_string() });
        set.commands.push(Command { position: 1, command: "echo bye".to_string() });
        g.sets.push(set);
        AppData { groups: vec![g] }
    }
```

- [ ] **Step 2: 追加 DeleteVariable 测试**

```rust
    #[test]
    fn test_handler_delete_variable() {
        let mut app = make_app();
        app.data = make_data_with_vars_and_cmds();
        let set = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::DeleteVariable(0));
        let ds = app.detail_screen.as_ref().unwrap();
        assert!(ds.set.variables.is_empty());
        // After deleting the only variable, focus should move to Name
        assert_eq!(ds.focus, DetailFocus::Name);
    }

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
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(Command { position: 0, command: "only".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::DeleteCommand(0));
        let ds = app.detail_screen.as_ref().unwrap();
        assert!(ds.set.commands.is_empty());
        // After deleting the last command, focus should move to Name
        assert_eq!(ds.focus, DetailFocus::Name);
    }
```

添加 `DetailFocus` import：

```rust
    use crate::ui::detail_screen::DetailScreenState;
// 改为 ↓
    use crate::ui::detail_screen::{DetailFocus, DetailScreenState};
```

- [ ] **Step 3: 追加变量覆盖层测试**

```rust
    #[test]
    fn test_handler_cancel_variables() {
        let mut app = make_app();
        app.variable_screen.active = true;
        app.variable_screen.gi = 0;
        app.variable_screen.si = 0;
        app.pending_set = Some((0, 0));

        app.handle_action(AppAction::CancelVariables);
        assert!(!app.variable_screen.active);
        assert!(app.pending_set.is_none());
    }

    #[test]
    fn test_handler_confirm_variables() {
        let mut app = make_app();
        app.data = make_data_with_vars_and_cmds();
        app.variable_screen.activate(&app.data.groups[0].sets[0], 0, 0);
        app.variable_screen.inputs[0].content = "prod.example.com".to_string();

        app.handle_action(AppAction::ConfirmVariables);
        // Variables should be updated
        assert_eq!(
            app.data.groups[0].sets[0].variables[0].default_value,
            "prod.example.com"
        );
        // Should have entered Execution mode
        assert_eq!(app.mode, AppMode::Execution);
        assert!(app.exec_screen.is_some());
        // pending_set should be set for re-execution
        assert_eq!(app.pending_set, Some((0, 0)));
    }
```

- [ ] **Step 4: 追加执行相关 action 测试**

```rust
    #[test]
    fn test_handler_execute_set_no_variables() {
        let mut app = make_app();
        app.data = make_data_with_one_group();

        app.handle_action(AppAction::ExecuteSet(0, 0));
        // Should transition directly to Execution mode (no variable prompt)
        assert_eq!(app.mode, AppMode::Execution);
        assert!(app.exec_screen.is_some());
        assert_eq!(app.pending_set, Some((0, 0)));
    }

    #[test]
    fn test_handler_back_to_main() {
        use crate::ui::execution_screen::ExecutionScreenState;
        let mut app = make_app();
        let cmds = vec![crate::models::Command { position: 0, command: "ok".to_string() }];
        app.exec_screen = Some(ExecutionScreenState::new("test".to_string(), &cmds));
        app.mode = AppMode::Execution;

        app.handle_action(AppAction::BackToMain);
        assert_eq!(app.mode, AppMode::Main);
        assert!(app.exec_screen.is_none());
    }
```

- [ ] **Step 5: 验证**

```bash
cargo check
cargo test              # 预期 154 + 8 = 162 tests pass
cargo clippy 2>&1 | grep -c "warning"
```

- [ ] **Step 6: Commit**

```bash
git add src/app/handler.rs
git commit -m "test: add handler tests for execution, detail deletion, and variable overlay paths (8 tests)"
```

---

### T2: ExecutionState 枚举

**文件：**
- Modify: `src/app.rs`（枚举定义 + 字段替换 + run/do_execute_with/teardown 重构）
- Modify: `src/app/handler.rs`（所有 `self.exec_screen`/`self.pending_set`/`self.execution` 访问点）
- Modify: `src/app/render.rs`（exec_screen 访问点）

**设计决策：** `ExecutionState::Running` 包含 `ExecutionManager`、`ExecutionScreenState`、`pending_set` 三元组。`ExecutionManager` 保持为一个独立 struct 被包含在内——不改变它的内部 API。所有先前分散在三处的访问点改为 `match self.execution_state { ... }` 枚举分派。

- [ ] **Step 1: 在 `app.rs` 中定义 `ExecutionState` 枚举**

在 `TICK_RATE_MS` 常量之后、`pub(crate) mod execution;` 之前追加：

```rust
/// Consolidated execution lifecycle — replaces separate `exec_screen`,
/// `execution`, and `pending_set` fields. Only one variant is active at a time.
pub(crate) enum ExecutionState {
    /// No execution in progress.
    Idle {
        /// Pending set indices after variable resolution but before thread spawn.
        /// Only non-None momentarily between `ConfirmVariables` and
        /// `do_execute()`.
        pending_set: Option<(usize, usize)>,
    },
    /// Background thread is running with active screen.
    Running {
        screen: ExecutionScreenState,
        manager: ExecutionManager,
        /// (group_index, set_index) — saved for restart / continue.
        pending_set: (usize, usize),
    },
}
```

- [ ] **Step 2: 替换 `App` struct 中的三个字段为一个字段**

```rust
// app.rs — App struct 定义变更
pub struct App {
    pub data: AppData,
    pub mode: AppMode,
    pub running: bool,

    pub main_screen: MainScreenState,
    pub detail_screen: Option<DetailScreenState>,

    // 删除：
    // pub exec_screen: Option<ExecutionScreenState>,
    // pub execution: ExecutionManager,
    // pub pending_set: Option<(usize, usize)>,

    // 新增：
    pub execution_state: ExecutionState,

    pub variable_screen: VariableScreenState,

    pub theme: Theme,
    pub toasts: ToastManager,
}
```

`use` 导入调整：删除 `use crate::app::execution::ExecutionManager;`（类型现在是内部使用，不需要顶层导入），保留或从 crate 级导入 `ExecutionScreenState`。

- [ ] **Step 3: 更新 `App::new()` — 初始化新字段**

```rust
// app.rs — new()
            execution_state: ExecutionState::Idle { pending_set: None },
```

- [ ] **Step 4: 更新 `run()` — event drain 路径**

```rust
// app.rs — run() 方法中的 event drain（替换 exec_screen / execution.rx 访问）
            // Drain execution events on each tick
            if let ExecutionState::Running { ref mut screen, ref manager, .. } = self.execution_state {
                if let Some(ref rx) = manager.rx {
                    screen.process_events(rx);
                }
            }
```

删除原来的分行访问 `self.execution.rx` / `self.exec_screen`，以及 `else` 分支（无 screen 时清空 channel）——在 `ExecutionState::Running` 下 screen 始终存在。

- [ ] **Step 5: 更新 `do_execute_with()`**

```rust
// app.rs — do_execute_with()
    fn do_execute_with(&mut self, gi: usize, si: usize, start_from: usize) {
        if gi >= self.data.groups.len() || si >= self.data.groups[gi].sets.len() {
            return;
        }
        let set = &self.data.groups[gi].sets[si];
        let shell_cmd = set.shell.resolve_command();

        let (commands, index_offset) = if start_from == 0 {
            let cmds = set.commands.clone();
            let screen = ExecutionScreenState::new(set.name.clone(), &cmds);
            let mut manager = ExecutionManager::new();
            manager.start(
                cmds.clone(),
                set.exec_mode,
                set.variables.clone(),
                shell_cmd,
                index_offset,
            );
            self.execution_state = ExecutionState::Running {
                screen,
                manager,
                pending_set: (gi, si),
            };
            return; // mode is set in the next block
        } else {
            let (cmds, offset) = if start_from == 0 {
                (set.commands.clone(), 0usize)
            } else {
                (set.commands[start_from..].to_vec(), start_from)
            };
            if let ExecutionState::Running { ref mut screen, ref mut manager, ref pending_set } = self.execution_state {
                screen.reset_from(start_from);
                manager.start(
                    cmds,
                    set.exec_mode,
                    set.variables.clone(),
                    shell_cmd,
                    offset,
                );
            }
            (cmds, offset)
        };

        // self.execution.start( ... ) — removed, now handled inside the match above
        self.mode = AppMode::Execution;
    }
```

等等——仔细想一下这一段。原 `do_execute_with` 在两个分支后统一调用 `self.execution.start(...)` 和 `self.mode = AppMode::Execution`。

对于 `start_from == 0`（首次执行）：
- 创建 `exec_screen` + `pending_set`
- 调用 `execution.start()`
- 设置 mode = Execution

对于 `start_from != 0`（从跳过处继续）：
- 复用现有 `exec_screen` 调用 `reset_from`
- 获取 commands[start_from..] 子切片
- 调用 `execution.start()` 
- mode 已经是 Execution

让我重新写更清晰的版本：

```rust
    fn do_execute_with(&mut self, gi: usize, si: usize, start_from: usize) {
        if gi >= self.data.groups.len() || si >= self.data.groups[gi].sets.len() {
            return;
        }
        let set = &self.data.groups[gi].sets[si];
        let shell_cmd = set.shell.resolve_command();

        let cmds = if start_from == 0 {
            set.commands.clone()
        } else {
            set.commands[start_from..].to_vec()
        };
        let index_offset = start_from;

        let mut manager = ExecutionManager::new();
        manager.start(
            cmds,
            set.exec_mode,
            set.variables.clone(),
            shell_cmd,
            index_offset,
        );

        let screen = if start_from == 0 {
            ExecutionScreenState::new(set.name.clone(), &set.commands)
        } else if let ExecutionState::Running { ref mut screen, .. } = self.execution_state {
            screen.reset_from(start_from);
            // steal the screen — this is a bit awkward, let's rethink
            // Actually we need to take ownership here...
            unreachable!()
        } else {
            return;
        };
```

Hmm, this is getting complicated because of the ownership issues. Let me think about this more carefully.

The original code:
```rust
fn do_execute_with(&mut self, gi: usize, si: usize, start_from: usize) {
    let set = &self.data.groups[gi].sets[si];
    let shell_cmd = set.shell.resolve_command();

    let (commands, index_offset) = if start_from == 0 {
        let cmds = set.commands.clone();
        self.exec_screen = Some(ExecutionScreenState::new(set.name.clone(), &cmds));
        self.pending_set = Some((gi, si));
        (cmds, 0usize)
    } else {
        let cmds = set.commands[start_from..].to_vec();
        if let Some(ref mut es) = self.exec_screen {
            es.reset_from(start_from);
        }
        (cmds, start_from)
    };

    self.execution.start(commands, set.exec_mode, set.variables.clone(), shell_cmd, index_offset);
    self.mode = AppMode::Execution;
}
```

The issue with migrating is that in the `start_from != 0` case, we need to mutate `screen` in-place (reset_from), then call `manager.start()` (creating a new manager). With an enum, we need to destructure to access the screen mutably, then reconstruct.

Better approach for `do_execute_with`:

```rust
    fn do_execute_with(&mut self, gi: usize, si: usize, start_from: usize) {
        if gi >= self.data.groups.len() || si >= self.data.groups[gi].sets.len() {
            return;
        }
        let set = &self.data.groups[gi].sets[si];
        let shell_cmd = set.shell.resolve_command();

        let (commands, index_offset) = if start_from == 0 {
            let cmds = set.commands.clone();
            let screen = ExecutionScreenState::new(set.name.clone(), &cmds);
            let mut manager = ExecutionManager::new();
            manager.start(
                cmds.clone(),
                set.exec_mode,
                set.variables.clone(),
                shell_cmd,
                0usize,
            );
            self.execution_state = ExecutionState::Running {
                screen,
                manager,
                pending_set: (gi, si),
            };
            self.mode = AppMode::Execution;
            return;
        } else {
            let cmds = set.commands[start_from..].to_vec();
            if let ExecutionState::Running { ref mut screen, ref mut manager, .. } = self.execution_state {
                screen.reset_from(start_from);
                manager.start(
                    cmds.clone(),
                    set.exec_mode,
                    set.variables.clone(),
                    shell_cmd,
                    start_from,
                );
            }
            (cmds, start_from)
        };

        self.mode = AppMode::Execution;
    }
```

This works cleanly. For `start_from == 0` we create new state and early return. For `start_from != 0` we mutate the existing Running state in-place.

- [ ] **Step 5 (revised): 更新 `do_execute()` + `do_execute_with()`**

```rust
    fn do_execute(&mut self) {
        // Take pending_set from Idle state (set by ExecuteSet / ConfirmVariables)
        let pending = match &mut self.execution_state {
            ExecutionState::Idle { ref mut pending_set } => pending_set.take(),
            ExecutionState::Running { .. } => None,
        };
        if let Some((gi, si)) = pending {
            self.do_execute_with(gi, si, 0);
        }
    }

    fn do_execute_with(&mut self, gi: usize, si: usize, start_from: usize) {
        if gi >= self.data.groups.len() || si >= self.data.groups[gi].sets.len() {
            return;
        }
        let set = &self.data.groups[gi].sets[si];
        let shell_cmd = set.shell.resolve_command();

        if start_from == 0 {
            let cmds = set.commands.clone();
            let screen = ExecutionScreenState::new(set.name.clone(), &cmds);
            let mut manager = ExecutionManager::new();
            manager.start(
                cmds,
                set.exec_mode,
                set.variables.clone(),
                shell_cmd,
                0usize,
            );
            self.execution_state = ExecutionState::Running {
                screen,
                manager,
                pending_set: (gi, si),
            };
            self.mode = AppMode::Execution;
            return;
        }

        // Continuing from a skip point — screen + manager already exist
        let cmds = set.commands[start_from..].to_vec();
        if let ExecutionState::Running { ref mut screen, ref mut manager, .. } = self.execution_state {
            screen.reset_from(start_from);
            manager.start(
                cmds,
                set.exec_mode,
                set.variables.clone(),
                shell_cmd,
                start_from,
            );
        }
        self.mode = AppMode::Execution;
    }
```

- [ ] **Step 6: 更新 `teardown_execution()`**

```rust
    fn teardown_execution(&mut self, keep_screen: bool, mark_skipped: bool) {
        if let ExecutionState::Running { ref mut screen, ref mut manager, .. } = self.execution_state {
            manager.kill();
            if mark_skipped {
                screen.mark_remaining_as_skipped();
            }
        }
        if !keep_screen {
            self.execution_state = ExecutionState::Idle { pending_set: None };
        }
    }
```

- [ ] **Step 7: 更新 `Drop` impl**

```rust
impl Drop for App {
    fn drop(&mut self) {
        if let ExecutionState::Running { ref mut manager, .. } = self.execution_state {
            manager.kill();
        }
        let _ = storage::save_app_data(&self.data);
    }
}
```

- [ ] **Step 8: 更新 `app/handler.rs` 所有访问点**

`execution_state` 为 `Idle` 时的访问（ExecuteSet handler, ConfirmVariables, CancelVariables）：

```rust
// handler.rs — ExecuteSet 无变量路径（行 52）
                        self.pending_set = Some((gi, si));
// 改为：
                        if let ExecutionState::Idle { ref mut pending_set } = self.execution_state {
                            *pending_set = Some((gi, si));
                        }
```

`execution_state` 为 `Running` 时的访问（BackToMain, SkipCurrent, ContinueFrom, ReExec）：

```rust
// handler.rs — BackToMain handler（行 176-192）
            AppAction::BackToMain => {
                if let ExecutionState::Running { ref screen, .. } = self.execution_state {
                    if screen.completed {
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
                    }
                }
                self.teardown_execution(false, false);
                self.mode = AppMode::Main;
            }
```

```rust
// handler.rs — SkipCurrent handler（行 196）
            AppAction::SkipCurrent => {
                self.teardown_execution(true, true);
                self.mode = AppMode::Execution;
            }
```

```rust
// handler.rs — ContinueFrom handler（行 200）
            AppAction::ContinueFrom(start) => {
                if let ExecutionState::Running { pending_set: (gi, si), .. } = self.execution_state {
                    self.do_execute_with(gi, si, start);
                }
            }
```

```rust
// handler.rs — ReExec handler（行 205）
            AppAction::ReExec => {
                self.teardown_execution(false, false);
                if let ExecutionState::Running { pending_set: (gi, si), .. } = self.execution_state {
                    self.do_execute_with(gi, si, 0);
                }
            }
```

```rust
// handler.rs — ConfirmVariables handler（行 225-226）
                self.pending_set = Some((gi, si));
// 改为：
                if let ExecutionState::Idle { ref mut pending_set } = self.execution_state {
                    *pending_set = Some((gi, si));
                }
```

```rust
// handler.rs — CancelVariables handler（行 230）
                self.pending_set = None;
// 改为：
                if let ExecutionState::Idle { ref mut pending_set } = self.execution_state {
                    *pending_set = None;
                }
```

```rust
// handler.rs — handle_key for Execution mode（行 30）
                if let Some(ref mut es) = self.exec_screen {
// 改为：
                if let ExecutionState::Running { ref mut screen, .. } = self.execution_state {
```

`exec_screen` 引用（行 30, 31）：`es` → `screen`，`es.handle_key(key)` → `screen.handle_key(key)`：

```rust
            AppMode::Execution => {
                if let ExecutionState::Running { ref mut screen, .. } = self.execution_state {
                    let action = screen.handle_key(key);
                    self.handle_action(action);
                }
            }
```

- [ ] **Step 9: 更新 `app/render.rs` 的 exec_screen 访问点**

```rust
// render.rs — Execution mode 渲染（行 64-67）
            AppMode::Execution => {
                if let Some(ref es) = self.exec_screen {
                    es.render(frame, content_area, &self.theme);
                }
            }
// 改为：
            AppMode::Execution => {
                if let ExecutionState::Running { ref screen, .. } = self.execution_state {
                    screen.render(frame, content_area, &self.theme);
                }
            }
```

需要在此文件的 import 中添加：`use super::ExecutionState;`。

- [ ] **Step 10: 更新 handler 测试中的 `make_app()` 和测试**

```rust
// handler.rs tests — make_app() 函数
    fn make_app() -> App {
        App {
            data: AppData::empty(),
            mode: AppMode::Main,
            running: true,
            main_screen: MainScreenState::new(),
            detail_screen: None,
            execution_state: ExecutionState::Idle { pending_set: None },
            variable_screen: VariableScreenState::new(),
            theme: Theme::default_dark(),
            toasts: ToastManager::new(),
        }
    }
```

现有测试中引用了 `app.exec_screen.is_some()` / `app.pending_set` / `app.exec_screen` 的断言需要更新：

搜索需要修改的断言：

`test_handler_new_set` — 不涉及（detail_screen 在另一个字段）
`test_handler_edit_set` — 不涉及
`test_handler_save_set` — 不涉及
`test_handler_cancel_edit` — 不涉及

T1 新增的 `test_handler_confirm_variables` 使用了 `app.exec_screen.is_some()` 和 `app.pending_set`：
```rust
        assert!(app.exec_screen.is_some());
        assert_eq!(app.pending_set, Some((0, 0)));
// 改为：
        assert!(matches!(app.execution_state, ExecutionState::Running { .. }));
```

`test_handler_execute_set_no_variables` 同样：
```rust
        assert!(app.exec_screen.is_some());
        assert_eq!(app.pending_set, Some((0, 0)));
// 改为：
        assert!(matches!(app.execution_state, ExecutionState::Running { .. }));
```

`test_handler_back_to_main`：
```rust
        app.exec_screen = Some(ExecutionScreenState::new("test".to_string(), &cmds));
        app.mode = AppMode::Execution;
// 改为直接用 ExecutionState::Running 构造：
        let mut mgr = ExecutionManager::new();
        // Don't actually start — we just need screen state
        let screen = ExecutionScreenState::new("test".to_string(), &cmds);
        app.execution_state = ExecutionState::Running {
            screen,
            manager: mgr,
            pending_set: (0, 0),
        };
        app.mode = AppMode::Execution;
```

`test_handler_back_to_main` 的断言：
```rust
        assert!(app.exec_screen.is_none());
// 改为：
        assert!(matches!(app.execution_state, ExecutionState::Idle { .. }));
```

`test_handler_cancel_variables`：
```rust
        app.pending_set = Some((0, 0));
// 改为：
        if let ExecutionState::Idle { ref mut pending_set } = app.execution_state {
            *pending_set = Some((0, 0));
        }
```
断言：
```rust
        assert!(app.pending_set.is_none());
// 改为：
        assert!(matches!(app.execution_state, ExecutionState::Idle { pending_set: None }));
```

- [ ] **Step 11: 更新 `integration_tests.rs` 中的 `test_app()` 辅助函数**

```rust
// integration_tests.rs — test_app()
            execution: ExecutionManager::new(),
            exec_screen: None,
            pending_set: None,
// 改为：
            execution_state: ExecutionState::Idle { pending_set: None },
```

search-and-destroy：运行 `grep -rn "exec_screen\|pending_set\|\.execution[\.\s]" src/` 确认所有调用已更新，无残留。

- [ ] **Step 12: 验证**

```bash
cargo check
cargo test              # 预期全部通过
cargo clippy 2>&1 | grep -c "warning"
```

- [ ] **Step 13: Commit**

```bash
git add src/app.rs src/app/handler.rs src/app/render.rs src/integration_tests.rs
git commit -m "refactor: replace exec_screen/execution/pending_set with ExecutionState enum"
```

---

## 验证清单

所有任务完成后：

- [ ] `cargo check` — 无编译错误
- [ ] `cargo test` — 全部 162+ 测试通过
- [ ] `cargo clippy` — 无新增 warning
- [ ] `cargo run` — TUI 正常启动、执行功能正常（肉眼验证）

---

## 回滚策略

两个任务独立 commit。T2 是较大的重构，可用 `git revert <hash>` 单独回滚。
