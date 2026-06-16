---
title: Bug 修复 — Help 模态背景与退出目标
date: 2026-06-17
status: draft
---

## 1. 概述

修复全局 `?` Help 快捷键引入的两个连锁 bug：

- **Bug 1**: Detail 界面按 `?` → 背景渲染为 Main 界面内容而非 Detail → 退出 Help 回到 Main（Detail 编辑丢失）
- **Bug 2**: Execution 界面按 `?` → 背景渲染为 Main → 退出 Help 回到 Main 但 `execution_state` 未清理 → 后续按 Enter 无法进入命令运行界面

## 2. 根因

Round 2（commit `be9dae2`）将 `?` 从 MainScreen 提升为全局快捷键，但 Help 模式的**背景渲染**和**退出目标**均为硬编码：

```rust
// app/render.rs — 始终渲染 Main 背景
AppMode::Help => {
    self.main_screen.render(frame, content_area, ...);
    draw_help(frame, content_area, &self.theme);
}

// app/handler.rs — 始终退回 Main
AppMode::Help => self.mode = AppMode::Main,
```

从 Detail/Execution 进入 Help 时，模式信息丢失。

## 3. 设计

引入 `prev_mode` 字段记录进入 Help 前的模式，用于：
- **渲染**：根据 `prev_mode` 选择对应的 screen 作为背景
- **退出**：恢复到 `prev_mode`
- **状态清理**：从 Execution 进入 Help 时清理 `execution_state`

### 3.1 `App` 字段变更

```rust
pub struct App {
    // ... existing fields ...
    pub prev_mode: Option<AppMode>,  // ← 新增
}
```

### 3.2 `handle_action(Help)` 变更

```rust
AppAction::Help => {
    // 若当前已是 Help，恢复之前模式（关闭 Help）
    if self.mode == AppMode::Help {
        self.mode = self.prev_mode.take().unwrap_or(AppMode::Main);
    } else {
        // 从 Execution 进入 Help 前清理执行状态
        if self.mode == AppMode::Execution {
            self.teardown_execution(false, false);
        }
        self.prev_mode = Some(self.mode);
        self.mode = AppMode::Help;
    }
}
```

### 3.3 Help 渲染变更

```rust
// app/render.rs
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
            self.main_screen.render(frame, content_area, &self.data, &self.theme);
        }
    }
    draw_help(frame, content_area, &self.theme);
}
```

## 4. 行为变更表

| 场景 | 之前（bug） | 之后 |
|------|-----------|------|
| Main → `?` → 背景 | Main ✓ | Main ✓ |
| Main → `?` → 退出 | Main ✓ | Main ✓ |
| Detail → `?` → 背景 | Main 内容 ✗ | Detail 内容 ✓ |
| Detail → `?` → 退出 | Main ✗ | Detail ✓ |
| Execution → `?` → 背景 | Main 内容 ✗ | Execution 内容 ✓ |
| Execution → `?` → 退出 | Main + 状态残留 ✗ | Main（已清理）✓ |
| Help 中再按 `?` | 不变 | 关闭 Help 回到之前模式 ✓ |

## 5. 验证

```bash
cargo test               # 165 pass
cargo clippy
cargo run

# 手动验证:
# 1. Detail 界面按 ? → 背景显示 Detail 内容，退出回到 Detail
# 2. Execution 界面按 ? → 背景显示 Execution 内容，退出回到 Main
# 3. Execution 界面 ? → 退出 → 回到 Main → Enter 执行命令集正常
```

## 6. 变更文件

| 文件 | 操作 |
|------|------|
| `src/app.rs` | `App` struct 添加 `prev_mode`，`new()` 初始化 |
| `src/app/handler.rs` | `handle_action(Help)` 实现保存/恢复逻辑 + 状态清理 |
| `src/app/render.rs` | Help 渲染使用 `prev_mode` 选择背景 screen |
| `src/app/handler.rs` 测试 | 追加帮助模式往返测试 |
