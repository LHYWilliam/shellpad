# 命令焦点 ←/→ 导航

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Execution 界面用 ←/→ 浏览特定命令输出，执行中和执行后均可用

**Architecture:** 3 文件修改。`ExecutionScreenState` 新增 `focus_index` + `nearest_non_pending` 辅助；handle_key 新增 ←/→/z 处理；render 根据 focus_index 计算滚动位置和状态条；events 中 Starting 清除 focus_index。

**Tech Stack:** Rust 2024 edition, ratatui 0.30, crossterm 0.29

---

## 文件变更

| 文件 | 操作 |
|------|------|
| `src/ui/execution_screen/mod.rs` | 新增 `focus_index`、`nearest_non_pending`、←/→/z key |
| `src/ui/execution_screen/render.rs` | scroll_offset 计算 + 状态条 + 聚焦高亮 |
| `src/ui/execution_screen/events.rs` | Starting 事件清除 focus_index |

---

### T1: 字段、辅助方法与按键

**文件：**
- Modify: `src/ui/execution_screen/mod.rs`

- [ ] **Step 1: 新增 `focus_index` 字段**

```rust
// ExecutionScreenState struct 内，auto_scroll 之后追加
    pub focus_index: Option<usize>,

// new() 内追加
            focus_index: None,
```

- [ ] **Step 2: 实现 `nearest_non_pending` 辅助方法**

在 `impl ExecutionScreenState` 内、`handle_key` 之前：

```rust
    /// Find the nearest non-Pending command from `from` in direction `delta`.
    /// Returns None if no such command exists.
    fn nearest_non_pending(&self, from: usize, delta: isize) -> Option<usize> {
        let len = self.cmd_states.len() as isize;
        if len == 0 {
            return None;
        }
        let mut pos = from as isize + delta;
        while pos >= 0 && pos < len {
            let i = pos as usize;
            if self.cmd_states[i].status != CmdStatus::Pending {
                return Some(i);
            }
            pos += delta;
        }
        None
    }
```

- [ ] **Step 3: 新增 ←/→/z 按键处理**

在 `handle_key` 的 `match key.code` 中，`KeyCode::Char('z')` 的 case 之前插入：

```rust
            KeyCode::Left => {
                let target = self.focus_index.unwrap_or(self.current_index);
                if let Some(idx) = self.nearest_non_pending(target, -1) {
                    self.focus_index = Some(idx);
                    self.auto_scroll = false;
                }
                AppAction::None
            }
            KeyCode::Right => {
                let target = self.focus_index.unwrap_or(self.current_index);
                if let Some(idx) = self.nearest_non_pending(target, 1) {
                    self.focus_index = Some(idx);
                    self.auto_scroll = false;
                }
                AppAction::None
            }
```

- [ ] **Step 4: 更新 `z` 按键处理**

```rust
// 将现有：
            KeyCode::Char('z') => {
                self.auto_scroll = !self.auto_scroll;
                AppAction::None
            }

// 改为：
            KeyCode::Char('z') => {
                if self.focus_index.is_some() {
                    self.focus_index = None;
                    self.auto_scroll = true;
                } else {
                    self.auto_scroll = !self.auto_scroll;
                }
                AppAction::None
            }
```

- [ ] **Step 5: 验证编译**

```bash
cargo check
```

---

### T2: 渲染变更

**文件：**
- Modify: `src/ui/execution_screen/render.rs`

- [ ] **Step 1: scroll_offset 基于 focus_index**

在 `items` 构建之后、`footer_text` 之前（约 line 141），修改 scroll_offset 计算：

```rust
        // Compute target command index for scroll positioning
        let target_cmd = self.focus_index.unwrap_or(self.current_index);
        let scroll_offset = if self.auto_scroll || self.focus_index.is_some() {
            self.items_offset_for_command(target_cmd)
        } else {
            self.scroll_offset
        };
```

注意：原代码在 `render` 方法签名中也使用 `self.scroll_offset`（let list_state 那行），需要替换：

```rust
// 将：
        let mut list_state = ratatui::widgets::ListState::default().with_offset(self.scroll_offset);
// 改为：
        let mut list_state = ratatui::widgets::ListState::default().with_offset(scroll_offset);
```

- [ ] **Step 2: 状态条文本**

```rust
// 将 footer_text 构建区改为：
        let footer_text = if self.focus_index.is_some() {
            "[←/→] Browse commands  [z] Follow current  [q] Back"
        } else if self.completed {
            if self.continue_from.is_some() {
                " [q] Back to main  [n] Continue from next  [r] Re-execute all"
            } else {
                " [q] Back to main  [r] Re-execute"
            }
        } else {
            " [q] Back to main  [s] Skip  [z] Auto-scroll  [Ctrl+C] Interrupt"
        };
```

- [ ] **Step 3: 聚焦命令高亮**

在构建每个命令 header 项时（`items.push(ListItem::new(Line::from(Span::styled( ... format!(" {} $ {}{}" ... )))));`），将 header 的 style 判断改为：

```rust
// 原：
            let status_color = match state.status {
                CmdStatus::Success => theme.accent_success,
                ...
            };

            let duration_str = state.duration_ms.map(format_duration).unwrap_or_default();

            items.push(ListItem::new(Line::from(Span::styled(
                format!(" {} $ {}{}", status_symbol, state.command, duration_str),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ))));
```

改为在状态色基础上叠加聚焦样式：

```rust
            let status_color = match state.status {
                CmdStatus::Success => theme.accent_success,
                CmdStatus::Failure => theme.accent_error,
                CmdStatus::Running => theme.accent_warning,
                CmdStatus::Pending => theme.text_disabled,
                CmdStatus::Skipped => theme.text_disabled,
            };

            let duration_str = state.duration_ms.map(format_duration).unwrap_or_default();

            let header_style = if Some(i) == self.focus_index {
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD)
            };

            items.push(ListItem::new(Line::from(Span::styled(
                format!(" {} $ {}{}", status_symbol, state.command, duration_str),
                header_style,
            ))));
```

- [ ] **Step 4: 验证编译**

```bash
cargo check
```

---

### T3: events 清除 focus_index

**文件：**
- Modify: `src/ui/execution_screen/events.rs`

- [ ] **Step 1: Starting 事件中清除 focus_index**

```rust
// events.rs process_events 的 Starting arm 中，在 auto_scroll 处理末尾追加
                ExecutionEvent::Starting { index, command } => {
                    if index < self.cmd_states.len() {
                        self.cmd_states[index].status = CmdStatus::Running;
                        self.cmd_states[index].command = command;
                        self.current_index = index;
                        if self.auto_scroll {
                            self.scroll_offset = self.items_offset_for_command(index);
                            self.focus_index = None;
                        }
                    }
                }
```

- [ ] **Step 2: 全量测试**

```bash
cargo check
cargo test      # 165 pass
cargo clippy
```

- [ ] **Step 3: Commit**

```bash
git add src/ui/execution_screen/
git commit -m "feat: add left/right command focus navigation in execution screen"
```

---

## 验证

- [ ] `cargo test` — 165 pass
- [ ] `cargo clippy` — 1 warning（预存 too_many_arguments）
- [ ] `cargo run` — 多命令执行 → ←/→ 浏览 → z 恢复跟随
