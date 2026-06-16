# 第三轮架构优化实施方案

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 5 项剩余问题，涵盖宏消除、channel drain 健壮性、editor 去重、死泛型清理、handler CRUD 测试覆盖

**Architecture:** 5 个独立任务：macro→函数化（4 调用点）、event polling 常驻 drain（1 文件）、detail_editor 去重（1 文件）、cycle_enum 内联（1 文件）、handler 直接测试（1 文件新增测试）

**Tech Stack:** Rust 2024 edition, Ratatui 0.30, thiserror 2.0

---

## 文件变更总览

| 文件 | 操作 | 涉及任务 |
|------|------|---------|
| `src/ui/render.rs` | 修改（宏→函数 + 宏删除） | T1 |
| `src/ui/execution_screen/render.rs` | 修改（调用点） | T1 |
| `src/ui/main_screen/render.rs` | 修改（调用点） | T1 |
| `src/ui/detail_screen/render.rs` | 修改（调用点） | T1 |
| `src/app.rs` | 修改（event loop） | T2 |
| `src/ui/detail_screen/editor.rs` | 修改（去重） | T3 |
| `src/ui/detail_screen/mod.rs` | 修改（cycle_enum 内联） | T4 |
| `src/app/handler.rs` | 修改（新增 tests 模块） | T5 |

---

### T1: bordered_block_zone! 宏 → 函数

**文件：**
- Modify: `src/ui/render.rs`（追加函数 + 删除宏）
- Modify: `src/ui/execution_screen/render.rs`（调用点）
- Modify: `src/ui/main_screen/render.rs`（调用点）
- Modify: `src/ui/detail_screen/render.rs`（调用点）

**当前调用点：**
```
src/ui/execution_screen/render.rs:156: bordered_block_zone!(frame, list_area, theme, " Output ", false)
src/ui/main_screen/render.rs:24:    bordered_block_zone!(frame, area, theme, " Groups ", ...)
src/ui/main_screen/render.rs:121:   bordered_block_zone!(frame, area, theme, &title, ...)
src/ui/detail_screen/render.rs:21:  bordered_block_zone!(frame, area, theme, " Properties ", props_focused)
src/ui/detail_screen/render.rs:141: bordered_block_zone!(frame, area, theme, title, focused)
```

共 5 处调用。

- [ ] **Step 1: 在 `render.rs` 中添加 `bordered_block_zone` 函数（宏下方）**

宏位置在 `src/ui/render.rs:170-178`（`bordered_block_zone!`）和 `182-188`（`bordered_block_info_zone!`）。在宏后追加等效函数：

```rust
// src/ui/render.rs — 在 bordered_block_zone! 宏和 styled_list_item 函数之间追加

/// Render a bordered block onto the frame, then return the inner Rect.
/// Equivalent to the macro form — used for call-site clarity.
pub fn bordered_block_zone<'a>(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &'a str,
    focused: bool,
) -> Rect {
    let block = bordered_block(theme, title, focused);
    let inner = block.inner(area);
    frame.render_widget(&block, area);
    inner
}

/// Render a bordered info block onto the frame, then return the inner Rect.
pub fn bordered_block_info_zone<'a>(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &'a str,
) -> Rect {
    let block = bordered_block_info(theme, title);
    let inner = block.inner(area);
    frame.render_widget(&block, area);
    inner
}
```

不需要额外 import — 这些函数已在同一个 `ui/render.rs` 文件中，`bordered_block` 和 `bordered_block_info` 在文件顶部已定义为函数。

- [ ] **Step 2: 更新 `execution_screen/render.rs` 调用点（1 处）**

```rust
// execution_screen/render.rs — 替换 import
use crate::bordered_block_zone;
// 改为：
use crate::ui::render::bordered_block_zone;
```

```rust
// execution_screen/render.rs — 替换调用（行 156）
        let list_inner = bordered_block_zone!(frame, list_area, theme, " Output ", false);
// 改为：
        let list_inner = bordered_block_zone(frame, list_area, theme, " Output ", false);
```

- [ ] **Step 3: 更新 `main_screen/render.rs` 调用点（2 处）**

```rust
// main_screen/render.rs — 替换 import
use crate::bordered_block_zone;
// 改为：
use crate::ui::render::bordered_block_zone;
```

```rust
// main_screen/render.rs — 两处调用同时改
        let inner = bordered_block_zone!(
            frame,
            area,
            theme,
            " Groups ",
            self.active_panel == Panel::Groups
        );
// 改为：
        let inner = bordered_block_zone(
            frame,
            area,
            theme,
            " Groups ",
            self.active_panel == Panel::Groups
        );
```

```rust
// main_screen/render.rs 约 121 行 — 第二处调用
            bordered_block_zone!(frame, area, theme, &title, self.active_panel == Panel::Sets);
// 改为：
            bordered_block_zone(frame, area, theme, &title, self.active_panel == Panel::Sets);
```

- [ ] **Step 4: 更新 `detail_screen/render.rs` 调用点（2 处）**

```rust
// detail_screen/render.rs — 替换 import
use crate::bordered_block_zone;
// 改为：
use crate::ui::render::bordered_block_zone;
```

```rust
// detail_screen/render.rs 行 21 — 第一处
        let inner = bordered_block_zone!(frame, area, theme, " Properties ", props_focused);
// 改为：
        let inner = bordered_block_zone(frame, area, theme, " Properties ", props_focused);
```

```rust
// detail_screen/render.rs 行 141 — 第二处（在 render_items_list 泛型函数内）
        let inner = bordered_block_zone!(frame, area, theme, title, focused);
// 改为：
        let inner = bordered_block_zone(frame, area, theme, title, focused);
```

- [ ] **Step 5: 删除 `ui/render.rs` 中的两个宏定义**

删除 `bordered_block_zone!` 宏（lines 169-178）和 `bordered_block_info_zone!` 宏（lines 180-189）：

```rust
// ui/render.rs — 删除这两段：
/// Render a bordered block and return its inner area.
#[macro_export]
macro_rules! bordered_block_zone {
    ($frame:expr, $area:expr, $theme:expr, $title:expr, $focused:expr) => {{
        let block = $crate::ui::render::bordered_block($theme, $title, $focused);
        let inner = block.inner($area);
        $frame.render_widget(&block, $area);
        inner
    }};
}

/// Render a bordered info block and return its inner area.
#[macro_export]
macro_rules! bordered_block_info_zone {
    ($frame:expr, $area:expr, $theme:expr, $title:expr) => {{
        let block = $crate::ui::render::bordered_block_info($theme, $title);
        let inner = block.inner($area);
        $frame.render_widget(&block, $area);
        inner
    }};
}
```

- [ ] **Step 6: 验证**

```bash
cargo check
cargo test
cargo clippy 2>&1 | grep -c "warning"
```

预期：138 通过，clippy 量不变（2 warning 里那个预存的 too_many_arguments）。

- [ ] **Step 7: Commit**

```bash
git add src/ui/render.rs src/ui/execution_screen/render.rs src/ui/main_screen/render.rs src/ui/detail_screen/render.rs
git commit -m "refactor: replace bordered_block_zone! macros with plain functions"
```

---

### T2: event polling 始终 drain 执行 channel

**文件：**
- Modify: `src/app.rs`（run 方法）

**问题：** `process_events` 仅在 `mode == AppMode::Execution` 时调用。BackToMain 切换 mode 后 channel 不再被 drain，线程可能阻塞在 `tx.send()`。

**修复：** 将 drain 移出 mode 条件，有 `rx` 就 drain（即使 exec_screen 已重置为 None）。no-op 丢弃事件数据但防止 channel 堆积。

- [ ] **Step 1: 在 `app.rs:run()` 中重构 event polling 块**

```rust
// app.rs — run() 方法。将原来的条件块：
            // Collect execution events on each tick
            if self.mode == AppMode::Execution
                && let Some(ref rx) = self.execution.rx
                && let Some(ref mut es) = self.exec_screen
            {
                es.process_events(rx);
            }

// 改为两部分：有 screen 时 process，没有 screen 时仍 drain 清理：
            // Process execution events if screen is active
            if let Some(ref rx) = self.execution.rx {
                if let Some(ref mut es) = self.exec_screen {
                    es.process_events(rx);
                } else {
                    // Drain events without processing — screen is gone but
                    // the sender thread may still push into the channel before
                    // kill_signal takes effect.
                    while rx.try_recv().is_ok() {}
                }
            }
```

- [ ] **Step 2: 验证**

```bash
cargo check
cargo test
cargo clippy 2>&1 | grep -c "warning"
```

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "fix: always drain execution channel, even after mode switch"
```

---

### T3: detail_editor 内联编辑处理去重

**文件：**
- Modify: `src/ui/detail_screen/editor.rs`

**结构分析：** `handle_variable_edit` 和 `handle_command_edit` 的公共结构：
1. Enter → commit + `AppAction::None`
2. Esc → cancel + `AppAction::None`
3. 默认 → handle text input → `AppAction::None`

差异：
- Enter 分支：variable 调用 `edit.commit(variables, parsed_var, list)`，command 调用 `edit.commit(commands, new_cmd, list)` + 重排 position
- 默认分支：variable 用 `handle_key_protected`（带 `=` 保护），command 用 `handle_key`

**方法：** 提取公共的 `enter`/`esc`/`default` 分派，用闭包传 commit 逻辑和 key handler 逻辑。

- [ ] **Step 1: 在 `editor.rs` 中添加私有辅助函数**

```rust
// editor.rs — 追加在 handle_variable_edit 之前

/// Generic inline-edit enter/esc/default dispatcher.
/// - `on_commit`: called when Enter is pressed with valid input
/// - `on_other`: called for non-Enter/non-Esc keys (text input delegation)
fn dispatch_inline_edit(
    edit: &mut InlineEdit,
    key: KeyEvent,
    on_commit: impl FnOnce(&mut InlineEdit),
    on_other: impl FnOnce(&mut InlineEdit),
) -> AppAction {
    match key.code {
        KeyCode::Enter => {
            on_commit(edit);
            AppAction::None
        }
        KeyCode::Esc => {
            edit.cancel();
            AppAction::None
        }
        _ => {
            if edit.editing.is_some() {
                on_other(edit);
            }
            AppAction::None
        }
    }
}
```

- [ ] **Step 2: 将 `handle_variable_edit` 委托给辅助函数**

```rust
// editor.rs — 替换 handle_variable_edit 的实现
pub fn handle_variable_edit(
    edit: &mut InlineEdit,
    key: KeyEvent,
    idx: usize,
    variables: &mut Vec<Variable>,
    list: &mut ScrollableList,
) -> AppAction {
    dispatch_inline_edit(edit, key,
        // on_commit: parse "name=value" or name-only, then commit
        |e| {
            let input = e.edit_input.content.clone();
            if let Some(eq_pos) = input.find('=') {
                let name = input[..eq_pos].trim().to_string();
                let value = input[eq_pos + 1..].trim().to_string();
                e.commit(idx, variables, Variable { name, default_value: value }, list);
            } else if !input.is_empty() {
                e.commit(idx, variables, Variable { name: input.trim().to_string(), default_value: String::new() }, list);
            }
        },
        // on_other: protect name part from deletion
        |e| {
            let protect = e.edit_input.content.find('=').map(|p| p + 1);
            e.handle_key_protected(key, protect);
        },
    )
}
```

- [ ] **Step 3: 将 `handle_command_edit` 委托给辅助函数**

```rust
// editor.rs — 替换 handle_command_edit 的实现
pub fn handle_command_edit(
    edit: &mut InlineEdit,
    key: KeyEvent,
    idx: usize,
    commands: &mut Vec<Command>,
    list: &mut ScrollableList,
) -> AppAction {
    dispatch_inline_edit(edit, key,
        // on_commit: build Command from text, commit, renumber positions
        |e| {
            let cmd = e.edit_input.content.clone();
            e.commit(idx, commands, Command { position: idx, command: cmd }, list);
            for (i, c) in commands.iter_mut().enumerate() {
                c.position = i;
            }
        },
        // on_other: plain text input
        |e| e.handle_key(key),
    )
}
```

- [ ] **Step 4: 验证 — 现有测试须无需修改通过**

所有 4 个测试保持原有 API 和语义不变：

```bash
cargo check
cargo test              # 138 tests pass
cargo clippy 2>&1 | grep -c "warning"
```

- [ ] **Step 5: Commit**

```bash
git add src/ui/detail_screen/editor.rs
git commit -m "refactor: extract dispatch_inline_edit to deduplicate handle_variable_edit and handle_command_edit"
```

---

### T4: cycle_enum 内联到 cycle_exec_mode

**文件：**
- Modify: `src/ui/detail_screen/mod.rs`

**问题：** `cycle_enum` 泛型函数仅被 `cycle_exec_mode` 调用一次，`cycle_group` 和 `cycle_shell` 有其特殊的循环逻辑（非恒定枚举），不使用它。独立泛型函数造成误导——"为什么有些用泛型有些不用"。

- [ ] **Step 1: 内联 cycle_enum 到 cycle_exec_mode**

```rust
// detail_screen/mod.rs — 替换 cycle_exec_mode 实现（行 124-130）
    fn cycle_exec_mode(&mut self, delta: isize) {
        let variants = &[ExecMode::StopOnError, ExecMode::ContinueOnError];
        let pos = variants
            .iter()
            .position(|v| *v == self.set.exec_mode)
            .unwrap_or(0);
        let next = (pos as isize + delta).rem_euclid(variants.len() as isize) as usize;
        self.set.exec_mode = variants[next];
    }
```

- [ ] **Step 2: 删除 cycle_enum 函数定义**

```rust
// detail_screen/mod.rs — 删除以下两行和函数体（约行 134-140）
/// Generic cycle helper for enum variants.
fn cycle_enum<T: Clone + PartialEq>(variants: &[T], current: &T, delta: isize) -> T {
    let pos = variants.iter().position(|v| *v == *current).unwrap_or(0);
    let next = (pos as isize + delta).rem_euclid(variants.len() as isize) as usize;
    variants[next].clone()
}
```

- [ ] **Step 3: 验证**

```bash
cargo check
cargo test
cargo clippy 2>&1 | grep -c "warning"
```

- [ ] **Step 4: Commit**

```bash
git add src/ui/detail_screen/mod.rs
git commit -m "refactor: inline cycle_enum into cycle_exec_mode (only caller)"
```

---

### T5: handler CRUD action 直接测试

**文件：**
- Modify: `src/app/handler.rs`（追加 `#[cfg(test)] mod tests`）

**测试范围：** CRUD 操作的 handler 行为（NewGroup/RenameGroup/EditSet/SaveSet/CancelEdit/DeleteSet/DeleteGroup）。不包括执行流程（KillExec/SkipCurrent/ContinueFrom/ReExec/BackToMain 需要 mock 执行线程）。

- [ ] **Step 1: 在 `handler.rs` 末尾 `#[cfg(test)] mod tests` 内追加测试**

在预存的 `fn auto_save()` 之后、`}` 之前追加测试模块。

```rust
// app/handler.rs — auto_save() 的 } 和 impl App 的 } 之间，追加：

#[cfg(test)]
mod tests {
    use super::App;
    use crate::action::AppAction;
    use crate::app::execution::ExecutionManager;
    use crate::app::toast::ToastManager;
    use crate::mode::AppMode;
    use crate::models::{AppData, CommandSet, Group};
    use crate::ui::detail_screen::{DetailFocus, DetailScreenState};
    use crate::ui::execution_screen::ExecutionScreenState;
    use crate::ui::main_screen::{MainScreenState, Panel};
    use crate::ui::theme::Theme;
    use crate::ui::variable_screen::VariableScreenState;

    fn make_app() -> App {
        App {
            data: AppData::empty(),
            mode: AppMode::Main,
            running: true,
            main_screen: MainScreenState::new(),
            detail_screen: None,
            exec_screen: None,
            execution: ExecutionManager::new(),
            variable_screen: VariableScreenState::new(),
            pending_set: None,
            theme: Theme::default_dark(),
            toasts: ToastManager::new(),
        }
    }

    fn make_data_with_one_group() -> AppData {
        let mut g = Group::new("Deploy".to_string());
        let set = CommandSet::new("Prod".to_string(), g.id);
        g.sets.push(set);
        AppData { groups: vec![g] }
    }

    // ---- NewGroup ----
    #[test]
    fn test_handler_new_group() {
        let mut app = make_app();
        app.handle_action(AppAction::NewGroup);
        assert_eq!(app.data.groups.len(), 1);
        assert_eq!(app.data.groups[0].name, "Group 1");
        assert!(app.toasts.toasts.len() > 0);
        assert!(app.toasts.toasts[0].message.contains("Group created"));
    }

    // ---- RenameGroup ----
    #[test]
    fn test_handler_rename_group() {
        let mut app = make_app();
        app.handle_action(AppAction::NewGroup);
        app.handle_action(AppAction::RenameGroup(0, "Infra".to_string()));
        assert_eq!(app.data.groups[0].name, "Infra");
    }

    // ---- RenameGroup out-of-bounds ----
    #[test]
    fn test_handler_rename_group_out_of_bounds_noop() {
        let mut app = make_app();
        app.handle_action(AppAction::RenameGroup(0, "X".to_string()));
        assert!(app.data.groups.is_empty());
    }

    // ---- NewSet ----
    #[test]
    fn test_handler_new_set() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::NewSet(0));
        assert_eq!(app.data.groups[0].sets.len(), 2);
        assert_eq!(app.data.groups[0].sets[1].name, "New Command Set");
        assert!(app.detail_screen.is_some());
        assert_eq!(app.mode, AppMode::Detail);
    }

    // ---- NewSet out-of-bounds ----
    #[test]
    fn test_handler_new_set_out_of_bounds_noop() {
        let mut app = make_app();
        app.handle_action(AppAction::NewSet(5));
        assert!(app.detail_screen.is_none());
        assert_eq!(app.mode, AppMode::Main);
    }

    // ---- EditSet ----
    #[test]
    fn test_handler_edit_set() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::EditSet(0, 0));
        assert!(app.detail_screen.is_some());
        assert_eq!(app.mode, AppMode::Detail);
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.name, "Prod");
    }

    // ---- EditSet out-of-bounds ----
    #[test]
    fn test_handler_edit_set_out_of_bounds_noop() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::EditSet(5, 5));
        assert!(app.detail_screen.is_none());
    }

    // ---- SaveSet ----
    #[test]
    fn test_handler_save_set() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        // setup detail screen
        let set = app.data.groups[0].sets[0].clone();
        let groups = app.data.groups.clone();
        app.detail_screen = Some(DetailScreenState::new(set, groups));
        app.mode = AppMode::Detail;

        let mut saved = app.data.groups[0].sets[0].clone();
        saved.name = "Updated".to_string();
        app.handle_action(AppAction::SaveSet(saved));
        assert!(app.detail_screen.is_none());
        assert_eq!(app.mode, AppMode::Main);
        assert_eq!(app.data.groups[0].sets[0].name, "Updated");
    }

    // ---- CancelEdit ----
    #[test]
    fn test_handler_cancel_edit() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        let set = app.data.groups[0].sets[0].clone();
        let groups = app.data.groups.clone();
        app.detail_screen = Some(DetailScreenState::new(set, groups));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::CancelEdit);
        assert!(app.detail_screen.is_none());
        assert_eq!(app.mode, AppMode::Main);
    }

    // ---- DeleteSet ----
    #[test]
    fn test_handler_delete_set() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::DeleteSet(0, 0));
        assert!(app.data.groups[0].sets.is_empty());
        assert_eq!(app.main_screen.active_panel, Panel::Groups);
    }

    // ---- DeleteSet out-of-bounds ----
    #[test]
    fn test_handler_delete_set_out_of_bounds_noop() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::DeleteSet(5, 5));
        assert_eq!(app.data.groups[0].sets.len(), 1);
    }

    // ---- DeleteGroup ----
    #[test]
    fn test_handler_delete_group() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::DeleteGroup(0));
        assert!(app.data.groups.is_empty());
    }

    // ---- DeleteGroup out-of-bounds ----
    #[test]
    fn test_handler_delete_group_out_of_bounds_noop() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::DeleteGroup(5));
        assert_eq!(app.data.groups.len(), 1);
    }

    // ---- Quit ----
    #[test]
    fn test_handler_quit() {
        let mut app = make_app();
        app.handle_action(AppAction::Quit);
        assert!(!app.running);
    }

    // ---- None + Help ----
    #[test]
    fn test_handler_none() {
        let mut app = make_app();
        app.handle_action(AppAction::None);
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_handler_help() {
        let mut app = make_app();
        app.handle_action(AppAction::Help);
        assert_eq!(app.mode, AppMode::Help);
    }
}
```

- [ ] **Step 2: 确认测试数组**

18 个测试函数：
- `new_group` / `rename_group` / `rename_group_out_of_bounds`
- `new_set` / `new_set_out_of_bounds`
- `edit_set` / `edit_set_out_of_bounds`
- `save_set` / `cancel_edit`
- `delete_set` / `delete_set_out_of_bounds`
- `delete_group` / `delete_group_out_of_bounds`
- `quit` / `none` / `help`

- [ ] **Step 3: 验证**

```bash
cargo check
cargo test              # 预期 138 + 18 = 156 tests pass
cargo clippy 2>&1 | grep -c "warning"
```

- [ ] **Step 4: Commit**

```bash
git add src/app/handler.rs
git commit -m "test: add direct unit tests for handler CRUD actions"
```

---

## 验证清单

所有任务完成后：

- [ ] `cargo check` — 无编译错误
- [ ] `cargo test` — 全部 156+ 测试通过
- [ ] `cargo clippy` — 无新增 warning

---

## 回滚策略

每个任务独立 commit，可用 `git revert <commit-hash>` 单独回滚。
