# S3 — render 模式统一

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 main_screen 的 `render_group_panel` 列表项构建改为使用共享的 `styled_list_item` 辅助函数（T10）。T11（空状态）和 T12（可见性）在前期已完成。

**Architecture:** 单文件修改——main_screen/render.rs 的 `render_group_panel` 中 1 处列表项构建从手动 `fill_row` → `ListItem::new` 替换为 `styled_list_item`。`render_set_panel` 因搜索高亮需要多 Span 保留不动。

**Tech Stack:** Rust 2024 edition, Ratatui

---

## 前置说明

### T11 — S1 期间已完成

`execution_screen/render.rs:128-129` 已追加 empty_hint：

```rust
if self.cmd_states.is_empty() {
    items.push(empty_hint(theme, " (no commands — press q to go back) "));
}
```

无需处理。

### T12 — 已完成

所有 render 方法的可见性已是 `pub(crate)`：

| 文件 | 方法 | 可见性 |
|------|------|--------|
| `main_screen/render.rs` | `render_group_panel`, `render_set_panel`, `render_status_bar` | `pub(crate)` ✅ |
| `detail_screen/render.rs` | `render_metadata`, `render_items_list`, `render_variables`, `render_commands`, `render_status_bar` | `pub(crate)` ✅ |
| `execution_screen/render.rs` | `render`, `format_duration` | `pub(crate)` ✅ |

无需处理。

---

## 文件变更

| 文件 | 涉及任务 |
|------|---------|
| `src/ui/main_screen/render.rs` | T10 |

---

### T10: render_group_panel 改用 styled_list_item

**文件：**
- Modify: `src/ui/main_screen/render.rs`

**当前代码**（L40-66 of `render_group_panel`）：

```rust
let mut items: Vec<ListItem> = data
    .groups
    .iter()
    .enumerate()
    .map(|(i, g)| {
        let marker = if i == self.group_list.selected {
            "▶ "
        } else {
            "  "
        };
        let display_name = if self.rename_mode && i == self.group_list.selected {
            &self.rename_input.content
        } else {
            &g.name
        };
        let name = format!("{}{}", marker, display_name);
        let count = format!("({})", g.sets.len());
        let name_width = unicode_width::UnicodeWidthStr::width(name.as_str());
        let pad = avail.saturating_sub(name_width + count.len());
        let label = format!("{}{:>pad$}{}", name, "", count, pad = pad);
        let style = if i == self.group_list.selected {
            theme.selected_style(theme.selection_bg_primary)
        } else {
            theme.normal_style()
        };
        let line = fill_row(
            Line::from(Span::styled(label, style)),
            style,
            list_area.width,
        );
        ListItem::new(line)
    })
    .collect();
```

注意最后三行——这正是 `styled_list_item` 的完整实现：

```rust
// src/ui/render.rs:197-203
pub fn styled_list_item(label: String, style: Style, width: u16) -> ListItem<'static> {
    ListItem::new(fill_row(
        Line::from(Span::styled(label, style)),
        style,
        width,
    ))
}
```

- [ ] **Step 1: 添加 `styled_list_item` 到 import**

在 `src/ui/main_screen/render.rs` 的 import 块（L5-8）中追加 `styled_list_item`：

```rust
use crate::ui::render::{
    empty_hint, fill_row, list_scrollbar_areas, render_inline_cursor, render_scrollbar,
    render_status_bar, set_cursor_after_prefix, styled_list_item,
};
```

- [ ] **Step 2: 替换 `render_group_panel` 中的手动构建**

将 map 闭包末尾的：

```rust
        let line = fill_row(
            Line::from(Span::styled(label, style)),
            style,
            list_area.width,
        );
        ListItem::new(line)
```

替换为：

```rust
        styled_list_item(label, style, list_area.width)
```

完整的变更后闭包：

```rust
            .map(|(i, g)| {
                let marker = if i == self.group_list.selected {
                    "▶ "
                } else {
                    "  "
                };
                let display_name = if self.rename_mode && i == self.group_list.selected {
                    &self.rename_input.content
                } else {
                    &g.name
                };
                let name = format!("{}{}", marker, display_name);
                let count = format!("({})", g.sets.len());
                let name_width = unicode_width::UnicodeWidthStr::width(name.as_str());
                let pad = avail.saturating_sub(name_width + count.len());
                let label = format!("{}{:>pad$}{}", name, "", count, pad = pad);
                let style = if i == self.group_list.selected {
                    theme.selected_style(theme.selection_bg_primary)
                } else {
                    theme.normal_style()
                };
                styled_list_item(label, style, list_area.width)
            })
```

- [ ] **Step 3: 验证**

```bash
cargo check
cargo test      # 165 pass
cargo clippy    # 无新增 warning
```

- [ ] **Step 4: Commit**

```bash
git add src/ui/main_screen/render.rs
git commit -m "refactor: use styled_list_item in render_group_panel"
```

---

## 验证清单

- [ ] `cargo check` — 编译通过
- [ ] `cargo test` — 165 pass
- [ ] `cargo clippy` — 无新增 warning（2 pre-existing）
