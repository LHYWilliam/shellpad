# UI 层重复消除 — 实施计划

> **For agentic workers:** A→C→B→D 顺序执行，每项后 `cargo check`。

**Goal:** 消除 UI 渲染和键盘处理中的 4 项重复模式。

**方法:** 提取辅助函数/宏到 `ui/render.rs`，替换调用点。不改变功能逻辑。

---

### Task A: 提取 `list_item_style()`

**修改文件：** `src/ui/render.rs`（添加），`src/ui/detail_screen/render.rs`（修改 2 处）

- [ ] **Step A1: 在 `ui/render.rs` 末尾添加函数**

在 `render.rs` 末尾添加：
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

- [ ] **Step A2: 修改 `detail_screen/render.rs` — variables 的 item_fn**

找到 `render_variables` 方法中的闭包（第 208-226 行），将样式部分替换为：

```rust
|i, is_editing| {
    let label = if is_editing {
        format!("  ▶ {}", self.var_edit.edit_input.content)
    } else {
        let v = &self.set.variables[i];
        format!("  {} = {}", v.name, v.default_value)
    };
    let is_insert = self.var_edit.insert_at.is_some();
    let is_selected = !is_insert && i == self.variable_list.selected && self.focus == DetailFocus::Variables;
    let style = list_item_style(is_editing, is_selected, theme);
    (label, style)
},
```

需要添加 import: `use crate::ui::render::list_item_style;`（如果还不存在的话）。

- [ ] **Step A3: 修改 `detail_screen/render.rs` — commands 的 item_fn**

同样的修改在 `render_commands` 方法中：

```rust
|i, is_editing| {
    let pos = self.set.commands[i].position;
    let is_insert = self.cmd_edit.insert_at.is_some();
    let display_pos = if is_editing {
        self.cmd_edit.insert_at.unwrap_or(pos)
    } else if is_insert && i >= self.cmd_edit.insert_at.unwrap() {
        pos + 1
    } else {
        pos
    };
    let content = if is_editing {
        self.cmd_edit.edit_input.content.as_str()
    } else {
        self.set.commands[i].command.as_str()
    };
    let label = format!("  #{}  {}", display_pos, content);
    let is_selected = !is_insert && i == self.command_list.selected && self.focus == DetailFocus::Commands;
    let style = list_item_style(is_editing, is_selected, theme);
    (label, style)
},
```

- [ ] **Step A4: 编译检查**

```bash
cargo check
```

### Task C: 提取 `styled_list_item()`

**修改文件：** `src/ui/render.rs`（添加），`src/ui/detail_screen/render.rs`（修改 3 处）

- [ ] **Step C1: 在 `ui/render.rs` 末尾添加函数**

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

- [ ] **Step C2: 修改 `detail_screen/render.rs` — `render_items_list` 中的普通列表项**

第 149-158 行，将：
```rust
let mut items: Vec<ListItem> = (0..count)
    .map(|i| {
        let is_editing = Some(i) == editing_item;
        let (label, style) = item_fn(i, is_editing);
        ListItem::new(fill_row(
            Line::from(Span::styled(label, style)),
            style,
            list_area.width,
        ))
    })
    .collect();
```
改为：
```rust
let mut items: Vec<ListItem> = (0..count)
    .map(|i| {
        let is_editing = Some(i) == editing_item;
        let (label, style) = item_fn(i, is_editing);
        styled_list_item(label, style, list_area.width)
    })
    .collect();
```

- [ ] **Step C3: 修改 `detail_screen/render.rs` — preview 行**

第 166-176 行，将：
```rust
let preview = ListItem::new(fill_row(
    Line::from(Span::styled(label.clone(), style)),
    style,
    list_area.width,
));
```
改为：
```rust
let preview = styled_list_item(label.clone(), style, list_area.width);
```

- [ ] **Step C4: 编译检查**

```bash
cargo check
```

### Task B: 添加 `bordered_block_zone!()` 宏

**修改文件：** `src/ui/render.rs`（添加宏），跨 5 个文件替换 7 处调用

- [ ] **Step B1: 在 `ui/render.rs` 末尾添加两个宏**

```rust
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

注意：使用 `#[macro_export]` 使宏在 crate 内所有模块可用，不需要额外 import。

- [ ] **Step B2: 替换 `main_screen/render.rs` — Group 面板（第 1 处）**

将：
```rust
let block = bordered_block(theme, " Groups ", self.active_panel == Panel::Groups);
let inner = block.inner(area);
frame.render_widget(&block, area);
```
改为：
```rust
let inner = bordered_block_zone!(frame, area, theme, " Groups ", self.active_panel == Panel::Groups);
```

- [ ] **Step B3: 替换 `main_screen/render.rs` — Set 面板（第 2 处）**

将：
```rust
let block = bordered_block(theme, &title, self.active_panel == Panel::Sets);
let inner = block.inner(area);
frame.render_widget(&block, area);
```
改为：
```rust
let inner = bordered_block_zone!(frame, area, theme, &title, self.active_panel == Panel::Sets);
```

- [ ] **Step B4: 替换 `detail_screen/render.rs` — Properties 面板（第 3 处）**

将：
```rust
let block = bordered_block(theme, " Properties ", props_focused);
let inner = block.inner(area);
frame.render_widget(&block, area);
```
改为：
```rust
let inner = bordered_block_zone!(frame, area, theme, " Properties ", props_focused);
```

- [ ] **Step B5: 替换 `detail_screen/render.rs` — `render_items_list` 中的 block（第 4 处）**

将：
```rust
let block = bordered_block(theme, title, focused);
let inner = block.inner(area);
frame.render_widget(&block, area);
```
改为：
```rust
let inner = bordered_block_zone!(frame, area, theme, title, focused);
```

- [ ] **Step B6: 替换 `execution_screen/render.rs` — Output 面板（第 5 处）**

将：
```rust
let list_block = bordered_block(theme, " Output ", false);
let list_inner = list_block.inner(list_area);
frame.render_widget(&list_block, list_area);
```
改为：
```rust
let list_inner = bordered_block_zone!(frame, list_area, theme, " Output ", false);
```
（注意：变量名从 `block` 改为宏内建，但 `list_inner` 变量名保持不变）

- [ ] **Step B7: 替换 `help_screen.rs`（第 6 处 — `bordered_block_info`）**

将：
```rust
let block = bordered_block_info(theme, " Help ");
let inner = block.inner(area);
frame.render_widget(&block, area);
```
改为：
```rust
let inner = bordered_block_info_zone!(frame, area, theme, " Help ");
```

- [ ] **Step B8: 替换 `variable_screen.rs`（第 7 处 — `bordered_block_info`）**

将：
```rust
let block = bordered_block_info(theme, " Set Variables ");
let inner = block.inner(area);
frame.render_widget(&block, area);
```
改为：
```rust
let inner = bordered_block_info_zone!(frame, area, theme, " Set Variables ");
```

- [ ] **Step B9: 编译检查**

```bash
cargo check
```

### Task D: 简化 detail_screen 键盘处理

**修改文件：** `src/ui/detail_screen/handler.rs`

- [ ] **Step D1: 添加两个辅助方法**

在 `impl DetailScreenState` 块中（`handle_key` 方法之前或 `commit_name_edit` 之后），添加：

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

- [ ] **Step D2: 简化 Enter 手臂**

将（第 111-129 行）：
```rust
DetailFocus::Variables if !self.set.variables.is_empty() => {
    let idx = self
        .variable_list
        .selected
        .min(self.set.variables.len().saturating_sub(1));
    let v = &self.set.variables[idx];
    self.var_edit.edit_input =
        TextInput::new(format!("{}={}", v.name, v.default_value));
    self.var_edit.editing = Some(idx);
}
DetailFocus::Commands if !self.set.commands.is_empty() => {
    let idx = self
        .command_list
        .selected
        .min(self.set.commands.len().saturating_sub(1));
    self.cmd_edit.edit_input =
        TextInput::new(self.set.commands[idx].command.clone());
    self.cmd_edit.editing = Some(idx);
}
```
改为：
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

- [ ] **Step D3: 简化 'a' 手臂**

将（第 133-147 行）：
```rust
DetailFocus::Variables => {
    self.var_edit.edit_input = TextInput::new(String::new());
    let pos = (self.variable_list.selected + 1).min(self.set.variables.len());
    self.var_edit.insert_at = Some(pos);
    self.var_edit.editing = Some(self.set.variables.len());
    self.variable_list.selected = pos;
}
DetailFocus::Commands => {
    self.cmd_edit.edit_input = TextInput::new(String::new());
    let pos = (self.command_list.selected + 1).min(self.set.commands.len());
    self.cmd_edit.insert_at = Some(pos);
    self.cmd_edit.editing = Some(self.set.commands.len());
    self.command_list.selected = pos;
}
```
改为：
```rust
DetailFocus::Variables => {
    Self::list_insert_begin(&mut self.var_edit, &mut self.variable_list, self.set.variables.len());
}
DetailFocus::Commands => {
    Self::list_insert_begin(&mut self.cmd_edit, &mut self.command_list, self.set.commands.len());
}
```

- [ ] **Step D4: 编译检查 + 全测试**

```bash
cargo check
```

### 最终验证

```bash
cargo test      # 128/128 全部通过
cargo clippy    # 无新增 warning
cargo fmt
git add src/ui/
git commit -m "refactor: 消除 UI 层重复 — list_item_style / bordered_block_zone / styled_list_item / handler helpers"
```
