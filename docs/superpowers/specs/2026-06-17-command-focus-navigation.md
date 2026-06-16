---
title: 命令焦点的 ←/→ 导航
date: 2026-06-17
status: draft
---

## 1. 概述

在 Execution 界面中用 ←/→ 键浏览特定命令的完整输出，在执行过程中和执行完成后均可使用。

## 2. 新增字段

`ExecutionScreenState` 新增：

```rust
/// 用户手动聚焦的命令索引。None 表示跟随 auto_scroll。
pub focus_index: Option<usize>,
```

初始化：`focus_index: None`

## 3. 按键行为

`ExecutionScreenState::handle_key()` 新增 ←/→ 处理：

| 键 | 行为 |
|----|------|
| `←` | `focus_index` 移到上一个非 Pending 命令；设 `auto_scroll = false`；返回 `None` |
| `→` | `focus_index` 移到下一个非 Pending 命令（包括 Running）；设 `auto_scroll = false`；返回 `None` |
| `z` | 若 `focus_index == Some`：清除，`auto_scroll = true`（恢复跟随）；否则：切换 `auto_scroll`（现有） |

### 辅助方法

```rust
/// 找到给定索引的最近非 Pending 命令（向前或向后搜索）。
fn nearest_non_pending(&self, from: usize, delta: isize) -> Option<usize>
```

- `delta > 0`：向后搜索（→）
- `delta < 0`：向前搜索（←）
- 跳过 `CmdStatus::Pending`
- 越界返回 None

## 4. 渲染变更

### scroll_offset 计算

```rust
let effective_index = self.focus_index.unwrap_or(self.current_index);
// items_offset_for_command 计算该命令在 flat list 中的位置
let target_offset = self.items_offset_for_command(effective_index);
```

### 状态条

| 状态 | 文本 |
|------|------|
| focus_index == Some | `[←/→] Browse commands  [z] Follow current  [q] Back` |
| focus_index == None | 现有内容不变 |

### 聚焦命令高亮

聚焦命令的 header 行使用 `theme.accent_primary` 前景色 + `Modifier::BOLD`（覆盖状态色）。

## 5. 与事件处理的交互

`process_events()` 中 `Starting` 事件：若 `auto_scroll == true`，`focus_index = None` 确保跟随新命令。

## 6. 验证

```bash
cargo test   # 165 pass
cargo clippy
cargo run    # 手动：多命令执行→←/→浏览→z恢复
```

## 7. 变更文件

| 文件 | 操作 |
|------|------|
| `src/ui/execution_screen/mod.rs` | 新增 `focus_index`、`nearest_non_pending`、←/→/z key handling |
| `src/ui/execution_screen/render.rs` | scroll_offset 基于 focus_index、状态条文本、聚焦高亮 |
| `src/ui/execution_screen/events.rs` | Starting 事件清除 focus_index（当 auto_scroll） |
