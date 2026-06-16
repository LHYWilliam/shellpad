---
title: Bug 修复 — Help 快捷键 + ReExec 回归
date: 2026-06-17
status: draft
---

## 1. 概述

修复两个运行时 bug：

- **Bug A**: 执行完成后按 `r` 重执行，界面消失、程序无响应
- **Bug B**: Detail / Execution 界面按 `?` 无帮助菜单弹出

两个 bug 共享一个根因模式：Architecture Round 4 引入的 `ExecutionState` 枚举迁移未完全适配所有 action 路径（Bug A）；全局快捷键 `?` 的处理未从 MainScreen 提升到 App 层（Bug B）。

---

## 2. Bug A: ReExec 中 pending_set 在读取前被清除

### 2.1 根因

`app/handler.rs` 中 ReExec handler：

```rust
// 当前代码（有 bug）
AppAction::ReExec => {
    self.teardown_execution(false, false); // ← teardown(false) 清除了 pending_set
    if let ExecutionState::Running { pending_set: (gi, si), .. } = self.execution_state {
        // ← 永远匹配不到：state 已变成 Idle
        self.do_execute_with(gi, si, 0);
    }
}
```

`teardown_execution(false, _)` 将 `execution_state` 设为 `Idle { pending_set: None }`，随后的 `if let Running` 永远无法匹配。`do_execute_with` 不被调用，mode 保持在 Execution，但 state 为 Idle（无 screen），导致渲染空屏、按键无响应。

### 2.2 修复

在 `teardown` 前保存 `pending_set`，teardown 后用保存的值调用 `do_execute_with`：

```rust
AppAction::ReExec => {
    // 在 teardown 之前读取 pending_set
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

### 2.3 影响范围

仅 `ReExec` 一个 action。`ContinueFrom` 调用 `teardown_execution(false, false)` 但只设置 `mode = Execution` 不清除 screen。`BackToMain` 不尝试从 Running 读 pending_set 也不受影响。

### 2.4 需添加的测试

在 `app/handler.rs` 测试模块中追加：

```rust
#[test]
fn test_handler_re_exec() {
    use crate::ui::execution_screen::ExecutionScreenState;
    use crate::models::Command;
    let mut app = make_app();
    let cmds = vec![Command { position: 0, command: "ok".to_string() }];
    app.execution_state = ExecutionState::Running {
        screen: ExecutionScreenState::new("t".to_string(), &cmds),
        manager: ExecutionManager::new(),
        pending_set: (0, 0),
    };
    app.data = make_data_with_one_group();
    app.mode = AppMode::Execution;

    app.handle_action(AppAction::ReExec);
    // 应该进入 Execution 模式并重新启动
    assert_eq!(app.mode, AppMode::Execution);
    assert!(matches!(app.execution_state, ExecutionState::Running { .. }));
}
```

---

## 3. Bug B: Detail / Execution 界面按 `?` 无帮助

### 3.1 根因

`?` 的 `AppAction::Help` 转换仅在 `MainScreenState::handle_key()` 中实现。`App::handle_key()` 直接按当前 mode 分派到对应 screen handler，未做"全局快捷键"第一层过滤。Detail 和 Execution 的 screen handler 不认识 `?`，返回 `AppAction::None`，Help 被吞掉。

### 3.2 设计选择

两个方向：

| 方案 | 操作 | 优点 | 缺点 |
|------|------|------|------|
| A. 全局 `?` 检查 | 在 `handle_key()` 顶部添加 `?` → `Help` 转换，先于屏幕分派 | 集中处理，一行代码 | `Ctrl+H` 也需要同步加 |
| B. 各 screen 分别添加 `?` | Detail + Execution 的 handler 中加 `?` → `Help` | 各模式可定制行为 | 分散，新增模式易漏 |

**推荐方案 A**：`?` 是全局快捷键，应在 `App` 层集中处理。与 `Ctrl+C`（当前特殊处理在 `execution_screen` 内）不同，Help 不需要模式上下文。

### 3.3 修复

在 `app/handler.rs:handle_key()` 顶部、变量覆盖层检查之后、模式分派之前插入：

```rust
    // Global Help shortcut — works in all modes
    if let KeyCode::Char('?') = key.code {
        self.handle_action(AppAction::Help);
        return;
    }
```

同时从 `main_screen/handler.rs` 中**删除** `?` 的独立处理（避免重复）。需要导入 `KeyCode`（`crossterm::event::KeyCode`）。

### 3.4 影响范围

- `app/handler.rs`：新增全局 `?` 检查（+3 行）
- `main_screen/handler.rs`：删除 `?` 的 case（-1 行）
- 行为变化：`?` 现在在 Help 模式下也响应（将其关闭回到 Main）——符合直觉

### 3.5 需添加的测试

在 `app/handler.rs` 测试模块中追加：

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

---

## 4. 验证

```bash
cargo test               # 164 pass（162 + 3 new）
cargo clippy
cargo run

# 手动验证:
# 1. 执行命令集 → 完成后按 r → 界面刷新，命令重新执行
# 2. Detail 界面按 ? → 帮助弹出
# 3. Execution 界面按 ? → 帮助弹出
# 4. Main 界面按 ? → 帮助仍正常
```

---

## 5. 变更文件

| 文件 | 操作 |
|------|------|
| `src/app/handler.rs` | Bug A: ReExec handler 重构。Bug B: 添加全局 `?` 检查。Bug A+B: 追加 3 个测试 |
| `src/ui/main_screen/handler.rs` | 删除 `?` 的独立 case（已提升为全局快捷键） |
