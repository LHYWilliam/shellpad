---
title: "UI 层重复消除设计文档"
date: 2026-06-16
status: draft
---

## 1. 动机

架构重构（Phase 1-4）将大文件拆分后，一些跨文件的 UI 渲染模式和键盘处理模式暴露了可消除的重复代码。本 spec 覆盖 A/B/C/D 四项提取。

## 2. 四项提取

### A. `list_item_style()` 辅助函数

**目标：** 消除 `detail_screen/render.rs` 中 variables 和 commands 的 `item_fn` 闭包里重复的 is_editing/selected 样式判断。

**当前重复**（2 处，在 `detail_screen/render.rs` 的 `render_variables` 和 `render_commands` 中）：

```rust
let style = if is_editing {
    Style::default()
        .fg(theme.text_on_selected)
        .bg(theme.accent_primary)
        .add_modifier(Modifier::BOLD)
} else if !is_insert && i == self.xxx_list.selected && self.focus == DetailFocus::Xxx {
    theme.selected_style(theme.selection_bg_secondary)
} else {
    theme.normal_style()
};
```

**改动：**

在 `src/ui/render.rs` 添加函数：

```rust
/// Determine the style for a list item based on its editing/selection state.
pub fn list_item_style(is_editing: bool, is_selected: bool, theme: &Theme) -> Style {
    if is_editing {
        Style::default()
            .fg(theme.text_on_selected)
            .bg(theme.accent_primary)
            .add_modifier(Modifier::BOLD)
    } else if is_selected {
        theme.selected_style(theme.selection_bg_secondary)
    } else {
        theme.normal_style()
    }
}
```

在 `detail_screen/render.rs` 中，两个 `item_fn` 闭包内的样式分支替换为：

```rust
let is_selected = !is_insert && i == self.xxx_list.selected && self.focus == DetailFocus::Xxx;
let style = list_item_style(is_editing, is_selected, theme);
```

**变更文件：** `ui/render.rs`（+8 行），`detail_screen/render.rs`（~-8 行）

---

### B. `bordered_block_zone!()` 宏

**目标：** 消除 5 处重复的 `bordered_block` 3 行渲染模板。

**当前重复**（每处 3 行，5 处共 15 行）：
```rust
let block = bordered_block(theme, title, focused);
let inner = block.inner(area);
frame.render_widget(&block, area);
```

**改动：**

在 `src/ui/render.rs` 添加宏：

```rust
/// Render a bordered block and return its inner area.
macro_rules! bordered_block_zone {
    ($frame:expr, $area:expr, $theme:expr, $title:expr, $focused:expr) => {{
        let block = $crate::ui::render::bordered_block($theme, $title, $focused);
        let inner = block.inner($area);
        $frame.render_widget(&block, $area);
        inner
    }};
}
pub(crate) use bordered_block_zone;
```

添加对应的 `bordered_block_info_zone` 宏（针对 `bordered_block_info` 的 2 处调用）：

```rust
macro_rules! bordered_block_info_zone {
    ($frame:expr, $area:expr, $theme:expr, $title:expr) => {{
        let block = $crate::ui::render::bordered_block_info($theme, $title);
        let inner = block.inner($area);
        $frame.render_widget(&block, $area);
        inner
    }};
}
pub(crate) use bordered_block_info_zone;
```

**替换位置（5 + 2 处）：**

| 文件 | 原模式 | 替换为 |
|------|--------|--------|
| `ui/main_screen/render.rs:23-26` | `bordered_block(" Groups ", ...)` | `bordered_block_zone!(...)` |
| `ui/main_screen/render.rs:116-119` | `bordered_block(&title, ...)` | `bordered_block_zone!(...)` |
| `ui/detail_screen/render.rs:20-23` | `bordered_block(" Properties ", ...)` | `bordered_block_zone!(...)` |
| `ui/detail_screen/render.rs:143-145` | `bordered_block(title, ...)` | `bordered_block_zone!(...)` |
| `ui/execution_screen/render.rs:157-159` | `bordered_block(" Output ", false)` | `bordered_block_zone!(...)` |
| `ui/help_screen.rs:14` | `bordered_block_info(" Help ")` | `bordered_block_info_zone!(...)` |
| `ui/variable_screen.rs:90` | `bordered_block_info(" Set Variables ")` | `bordered_block_info_zone!(...)` |

**注意：** `detail_screen/render.rs:20-23` 中的 `props_focused` 变量需要保留（在宏调用前计算）。宏参数接受表达式，所以可以直接传 `props_focused`。

**变更文件：** `ui/render.rs`（+15 行），5 个渲染文件（~-14 行）

---

### C. `styled_list_item()` 辅助函数

**目标：** 消除 4 处重复的 `fill_row` + `ListItem::new` 组合。

**当前重复**（4 处）：
```rust
ListItem::new(fill_row(Line::from(Span::styled(label, style)), style, list_area.width))
```

**改动：**

在 `src/ui/render.rs` 添加函数：

```rust
/// Create a styled ListItem with full-row background fill.
pub fn styled_list_item(label: String, style: Style, width: u16) -> ListItem<'static> {
    ListItem::new(fill_row(
        Line::from(Span::styled(label, style)),
        style,
        width,
    ))
}
```

**替换位置（4 处）：**

| 文件 | 行号 | 原代码 | 替换为 |
|------|------|--------|--------|
| `detail_screen/render.rs:153-154` | `render_items_list` 列表项 | `styled_list_item(label, style, list_area.width)` |
| `detail_screen/render.rs:170` | preview 行 | `styled_list_item(label.clone(), style, list_area.width)` |
| `main_screen/render.rs:57-62` | group 列表项 | `styled_list_item(label, style, list_area.width)` |
| `main_screen/render.rs:227-228` | set 列表项 | `styled_list_item(set_line, ...)`——注意这里 `set_line` 已是 `Line` 类型，不适合。保留此处不变或调整。 |

**注意：** `main_screen/render.rs:227-228` 处 `set_line` 已经是 `Line` 类型（从 `parts: Vec<Span>` 构造），不是 `String`。不适合用 `styled_list_item`。所以该替换适用于 3/4 的位置（detail_screen 的 3 处）。

**变更文件：** `ui/render.rs`（+12 行），`detail_screen/render.rs`（~-4 行），`main_screen/render.rs`（~-2 行）

---

### D. detail_screen 键盘处理简化

**目标：** 减少 `handler.rs` 中 Enter/'a'/Up/Down 手臂的重复逻辑。

**当前重复模式**（Enter 和 'a' 中各 2 对重复手臂）：

**改动：**

在 `src/ui/detail_screen/handler.rs` 的 `impl DetailScreenState` 中添加 2 个辅助方法：

```rust
/// Begin editing a list item at the current selection.
fn list_edit_begin(
    edit: &mut InlineEdit,
    list: &ScrollableList,
    initial_text: String,
    total_items: usize,
) {
    let idx = list.selected.min(total_items.saturating_sub(1));
    edit.edit_input = TextInput::new(initial_text);
    edit.editing = Some(idx);
}

/// Begin inserting a new item after the current selection.
fn list_insert_begin(edit: &mut InlineEdit, list: &mut ScrollableList, total_items: usize) {
    edit.edit_input = TextInput::new(String::new());
    let pos = (list.selected + 1).min(total_items);
    edit.insert_at = Some(pos);
    edit.editing = Some(total_items);
    list.selected = pos;
}
```

**替换后的 Enter 手臂：**

```rust
DetailFocus::Variables if !self.set.variables.is_empty() => {
    let text = format!("{}={}", self.set.variables[idx].name, self.set.variables[idx].default_value);
    // 需要先计算 idx
    let idx = self.variable_list.selected.min(self.set.variables.len() - 1);
    // 但 text 需要借用 self.set.variables[idx]
    // Rust 借用规则不允许同时可变借用 var_edit 和不可变借用 set.variables
    Self::list_edit_begin(&mut self.var_edit, &self.variable_list, text, self.set.variables.len());
}
```

**注意：** Rust 的借用规则限制——`list_edit_begin` 需要 `&mut self.var_edit`，但计算 `text` 需要 `&self.set.variables[idx]`。这要求先完成所有借用再调用辅助方法。可行方案：先提取 `idx` 和 `text` 到局部变量，再调用：

```rust
DetailFocus::Variables if !self.set.variables.is_empty() => {
    let idx = self.variable_list.selected.min(self.set.variables.len() - 1);
    let text = format!("{}={}", self.set.variables[idx].name, self.set.variables[idx].default_value);
    Self::list_edit_begin(&mut self.var_edit, &self.variable_list, text, self.set.variables.len());
}
DetailFocus::Commands if !self.set.commands.is_empty() => {
    let idx = self.command_list.selected.min(self.set.commands.len() - 1);
    let text = self.set.commands[idx].command.clone();
    Self::list_edit_begin(&mut self.cmd_edit, &self.command_list, text, self.set.commands.len());
}
```

**替换后的 'a' 手臂：**

```rust
DetailFocus::Variables => {
    Self::list_insert_begin(&mut self.var_edit, &mut self.variable_list, self.set.variables.len());
}
DetailFocus::Commands => {
    Self::list_insert_begin(&mut self.cmd_edit, &mut self.command_list, self.set.commands.len());
}
```

**'d' 手臂保持不变**（返回不同 `AppAction` 变体，逻辑已足够简洁）。

**变更文件：** `detail_screen/handler.rs`（约 +12 / -14 行，净减 ~2 行）

---

## 3. 不变项

以下模式经评估无需修改：
- **Up/Down/Left/Right** — 每对仅 4-6 行，提取反而增加间接层
- **render_status_bar** — 底层已经是统一辅助函数，各屏生成不同文本属于逻辑需要
- **scrollbar 模式** — update_offset / selected_or_none / render_scrollbar 分布在 mod.rs 和 render.rs 中，统一后收益小且跨文件耦合

## 4. 风险

1. **宏的可见性：** `bordered_block_zone!` 是宏，在 Rust 2018+ 中需要 `pub(crate) use` 才能跨模块使用。确保在 `render.rs` 中用 `pub(crate) use bordered_block_zone;` 导出。
2. **`styled_list_item` 返回 `ListItem<'static>`：** 字符串是 `String` 类型（owned），而 `fill_row` 的 `Line` 使用 `Span<'a>`。需要确认生命周期能满足——由于 `label` 被 move 进 `Span::styled`，生命周期正确的是 `ListItem<'static>`。
3. **`list_edit_begin` 的 `initial_text` 借用：** 调用方需要先完成对 `self.set` 的所有读取再调用辅助方法（Rust 借用规则）。已在设计中考虑。
4. **测试不受影响：** 所有提取不改变功能行为，现有 128 个测试应全部通过。

## 5. 实施顺序

按独立程度排列，建议顺序：

1. **A** — `list_item_style()`：最独立，只改 `render.rs` 和 `detail_screen/render.rs`
2. **C** — `styled_list_item()`：独立，改 `render.rs`、`detail_screen/render.rs`、`main_screen/render.rs`
3. **B** — `bordered_block_zone!()`：涉及 7 处替换，跨 5 个文件
4. **D** — handler 简化：仅改 `detail_screen/handler.rs`，最独立

每项后运行 `cargo check` 确保通过，全部完成后再 `cargo test`。
