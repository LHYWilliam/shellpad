# 项目架构抽象重构 — 总 TODO + 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 消除 18 类重复模式，将重复代码提取为辅助函数和 trait 方法，重构项目架构。

**Architecture:** 按「先加函数、再迁调用、最后架构调整」的顺序，每步确保编译通过、测试通过。

**Tech Stack:** ratatui 0.30.1, crossterm 0.29.0

---

## 📋 总 TODO List（按优先级排序）

### Phase 1：辅助函数层（纯新增，不改调用，无风险）

- [x] **T1. 基础 UI 辅助函数** — `components.rs` 新增 `render_scrollbar()`, `bordered_block()`, `empty_hint()`, `list_scrollbar_areas()`, `centered_rect()`, `render_status_bar()`, `render_inline_cursor()`
- [x] **T2. Theme 样式构造方法** — `theme.rs` 新增 `selected_style()`, `normal_style()`, `disabled_style()`, `dim_style()`, `border_style()`
- [x] **T3. ScrollableList 增强** — `components.rs` 新增 `clamp_selected()`, `selected_or_none()` 方法

### Phase 2：迁移调用方（逐文件替换，每步验证）

- [x] **T4. 重构 main_screen.rs** — 替换所有重复模式为新辅助函数
- [x] **T5. 重构 detail_screen.rs** — 替换所有重复模式为新辅助函数
- [x] **T6. 重构 execution_screen.rs** — 替换所有重复模式
- [x] **T7. 重构 variable_screen.rs + help_screen.rs** — 替换所有重复模式

### Phase 3：架构改进（需编译检查）

- [x] **T8. 合并 stop/kill_execution** — `app.rs` 中两个重复方法合并
- [x] **T9. cycle_enum 通用化** — `detail_screen.rs` 三个 cycle 方法合并为一个泛型函数
- [x] **T10. 统一删除-选择逻辑** — 确认所有 4 个列表使用同一模式

---

## 详细实施步骤

---

### T1：基础 UI 辅助函数

**文件**: `src/ui/components.rs`

在文件末尾（`handle_text_input` 之后）添加以下函数：

#### T1a. `render_scrollbar()`

```rust
/// Render a default scrollbar at the right side of a list area.
/// `content_len` is the total number of items, `position` is the current selection index.
pub fn render_scrollbar(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    content_len: usize,
    position: usize,
) {
    let pos = position.min(content_len.saturating_sub(1));
    let mut state = ScrollbarState::new(content_len).position(pos);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(theme.surface_border)),
        area,
        &mut state,
    );
}
```

**导入**: 需添加 `Scrollbar`, `ScrollbarOrientation`, `ScrollbarState`（已有）。

#### T1b. `bordered_block()`

```rust
/// Create a bordered Block with a title and optional focus highlighting.
pub fn bordered_block<'a>(theme: &Theme, title: &'a str, focused: bool) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused {
            theme.accent_primary
        } else {
            theme.surface_border
        }))
        .title(title)
}
```

#### T1c. `empty_hint()`

```rust
/// Create a disabled/italic ListItem for empty-state guidance.
pub fn empty_hint<'a>(theme: &Theme, text: &'a str) -> ListItem<'a> {
    ListItem::new(Line::from(Span::styled(
        text,
        Style::default().fg(theme.text_disabled).add_modifier(Modifier::ITALIC),
    )))
}
```

#### T1d. `list_scrollbar_areas()`

```rust
/// Split a Rect into a main list area (left) and a 1-column scrollbar area (right).
pub fn list_scrollbar_areas(area: Rect) -> (Rect, Rect) {
    let layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
    let [list, scrollbar] = layout.areas(area);
    (list, scrollbar)
}
```

#### T1e. `centered_rect()`

```rust
/// Compute a centered Rect of the given width/height within the outer area.
pub fn centered_rect(outer: Rect, width: u16, height: u16) -> Rect {
    let x = outer.x + (outer.width.saturating_sub(width)) / 2;
    let y = outer.y + (outer.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(outer.width), height.min(outer.height))
}
```

#### T1f. `render_status_bar()`

```rust
/// Render a status bar with a top separator line and dim text.
pub fn render_status_bar(frame: &mut Frame, area: Rect, theme: &Theme, text: &str) {
    let sep = "─".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(sep, Style::default().fg(theme.surface_border)))),
        Rect::new(area.x, area.y, area.width, 1),
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(text, Style::default().fg(theme.text_secondary).add_modifier(Modifier::DIM)))),
        Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1)),
    );
}
```

#### T1g. `render_inline_cursor()`

统一内联编辑时光标在列表项中的定位守卫逻辑：

```rust
/// Position the cursor on an inline-editing row within a scrollable list.
/// `row_y` is the visual Y position of the item (list_area.y + pos - offset).
/// Does nothing if the item is scrolled out of the visible area.
pub fn render_inline_cursor(
    frame: &mut Frame,
    list_area: Rect,
    list_offset: usize,
    item_index: usize,
    input: &TextInput,
    prefix_display_width: u16,
) {
    let item_y = list_area.y + item_index.saturating_sub(list_offset) as u16;
    if item_y < list_area.y + list_area.height {
        set_cursor_after_prefix(
            frame,
            &input.content,
            input.cursor,
            prefix_display_width,
            Rect::new(list_area.x, item_y, list_area.width, 1),
        );
    }
}
```

#### 验证

```bash
cargo test 2>&1 | tail -3
```

#### 提交

```bash
git add src/ui/components.rs
git commit -m "refactor: add shared UI helper functions (scrollbar, block, hint, layout, centered_rect, status_bar, inline_cursor)"
```

---

### T2：Theme 样式构造方法

**文件**: `src/ui/theme.rs`

在 `impl Theme` 中添加：

```rust
impl Theme {
    // ... existing default_simple, default_dark ...

    /// Style for a selected/highlighted list item.
    pub fn selected_style(&self, bg: Color) -> Style {
        Style::default()
            .fg(self.text_on_selected)
            .bg(bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for a normal (unselected) list item.
    pub fn normal_style(&self) -> Style {
        Style::default().fg(self.text_primary)
    }

    /// Style for a focused (but not editing) label.
    pub fn focused_style(&self) -> Style {
        Style::default().fg(self.accent_primary)
    }

    /// Style for disabled/empty-state text.
    pub fn disabled_style(&self) -> Style {
        Style::default().fg(self.text_disabled).add_modifier(Modifier::ITALIC)
    }

    /// Style for status bar / dim hints.
    pub fn dim_style(&self) -> Style {
        Style::default().fg(self.text_secondary).add_modifier(Modifier::DIM)
    }

    /// Style for a block border that optionally highlights on focus.
    pub fn border_style(&self, focused: bool) -> Style {
        Style::default().fg(if focused { self.accent_primary } else { self.surface_border })
    }
}
```

#### 验证

```bash
cargo test 2>&1 | tail -3
```

#### 提交

```bash
git add src/ui/theme.rs
git commit -m "refactor: add Theme style constructor methods"
```

---

### T3：ScrollableList 增强

**文件**: `src/ui/components.rs`

在 `impl ScrollableList` 中添加：

```rust
    /// Clamp `selected` after a deletion: if the last item was removed,
    /// move selection to the new last item; otherwise keep it.
    pub fn clamp_selected(&mut self, len: usize) {
        if self.selected >= len {
            self.selected = len.saturating_sub(1);
        }
    }

    /// Return `Some(selected)` if the list is non-empty, else `None`,
    /// with the selected index clamped to `len - 1`.
    pub fn selected_or_none(&self, len: usize) -> Option<usize> {
        if len == 0 {
            None
        } else {
            Some(self.selected.min(len.saturating_sub(1)))
        }
    }
```

#### 验证与提交

```bash
cargo test 2>&1 | tail -3
git add src/ui/components.rs
git commit -m "refactor: add ScrollableList::clamp_selected and selected_or_none"
```

---

### T4：重构 main_screen.rs

**文件**: `src/ui/main_screen.rs`

#### T4a. 替换 bordered_block

查找 `render_group_panel` 和 `render_set_panel` 中的 Block 构建，替换为：

```rust
// Before (render_group_panel):
let border_color = if self.active_panel == Panel::Groups {
    theme.accent_primary
} else {
    theme.surface_border
};
let block = Block::default()
    .borders(Borders::ALL)
    .border_style(Style::default().fg(border_color))
    .title(" Groups ");
```

```rust
// After:
let block = bordered_block(theme, " Groups ", self.active_panel == Panel::Groups);
```

对 `render_set_panel` 做同样替换。

#### T4b. 替换 list_scrollbar_areas

```rust
// Before:
let inner_layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
let [list_area, scrollbar_area] = inner_layout.areas(inner);
// After:
let (list_area, scrollbar_area) = list_scrollbar_areas(inner);
```

（两处：groups 和 sets）

#### T4c. 替换 render_scrollbar

```rust
// Before:
let content_len = data.groups.len();
let mut scrollbar_state = ScrollbarState::new(content_len)
    .position(self.group_list.selected);
frame.render_stateful_widget(
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_style(Style::default().fg(theme.surface_border)),
    scrollbar_area,
    &mut scrollbar_state,
);
// After:
render_scrollbar(frame, scrollbar_area, theme, data.groups.len(), self.group_list.selected);
```

对 sets 的滚动条做同样替换。

#### T4d. 替换 empty_hint

```rust
// Before (groups empty):
items.push(ListItem::new(Line::from(Span::styled(
    " (empty — press g to add) ",
    Style::default().fg(theme.text_disabled).add_modifier(Modifier::ITALIC),
))));
// After:
items.push(empty_hint(theme, " (empty — press g to add) "));
```

对 sets empty 做同样替换。

#### T4e. 替换选中/未选中样式

```rust
// Before (group item):
let style = if i == self.group_list.selected {
    Style::default()
        .fg(theme.text_on_selected)
        .bg(theme.selection_bg_primary)
        .add_modifier(Modifier::BOLD)
} else {
    Style::default().fg(theme.text_primary)
};
// After:
let style = if i == self.group_list.selected {
    theme.selected_style(theme.selection_bg_primary)
} else {
    theme.normal_style()
};
```

对 sets items、highlight_style 做同样替换。

#### T4f. 替换 status bar 和 separator 样式

```rust
// Before:
Style::default().fg(theme.text_secondary).add_modifier(Modifier::DIM)
// After:
theme.dim_style()
```

```rust
// Before:
Style::default().fg(theme.surface_border)
// After (for separator line):
theme.border_style(false)  // or keep as is since it's a fixed color
```

#### T4g. 替换 highlight_style

```rust
// Before (groups):
let list = List::new(items).highlight_style(
    Style::default()
        .fg(theme.text_on_selected)
        .bg(theme.selection_bg_primary)
        .add_modifier(Modifier::BOLD),
);
// After:
let list = List::new(items).highlight_style(theme.selected_style(theme.selection_bg_primary));
```

#### 验证

```bash
cargo test 2>&1 | tail -3
```

#### 提交

```bash
git add src/ui/main_screen.rs
git commit -m "refactor(main_screen): use shared helpers for blocks, scrollbars, hints, styles"
```

---

### T5：重构 detail_screen.rs

**文件**: `src/ui/detail_screen.rs`

与 T4 类似，替换以下模式：

- Properties/Variables/Commands 的 Block 构建 → `bordered_block()`
- Variables/Commands 的 list_scrollbar_areas → `list_scrollbar_areas()`
- Variables/Commands 的 render_scrollbar → `render_scrollbar()`
- Variables/Commands/Properties 的空状态 → `empty_hint()`
- 选中/未选中样式 → `theme.selected_style()` / `theme.normal_style()`
- 状态栏样式 → `theme.dim_style()`
- 分隔线样式 → `theme.border_style(false)`
- 列表 highlight_style → 可选的（当前 detail 没有 highlight_style，暂不添加）

特别需要注意：
- Properties 的焦点条件为 `props_focused`（匹配 Name/Group/Shell/ExecMode），调用时为 `bordered_block(theme, " Properties ", props_focused)`
- Variables/Commands 的焦点条件为 `self.focus == DetailFocus::Variables/Commands`
- `render_status_bar` 对 detail_screen 的调用：detail 的状态栏内容是非编辑时显示 `[a] Add [e] Edit ... | [Ctrl+S] Save`，编辑时显示 `[Enter] Confirm [Esc] Cancel | [Ctrl+S] Save`。需要调用 `render_status_bar(frame, status_area, theme, &status_text)` 其中 status_text 已拼接好管道符和 Ctrl+S 提示

#### 验证

```bash
cargo test 2>&1 | tail -3
```

#### 提交

```bash
git add src/ui/detail_screen.rs
git commit -m "refactor(detail_screen): use shared helpers for blocks, scrollbars, hints, styles"
```

---

### T6：重构 execution_screen.rs

**文件**: `src/ui/execution_screen.rs`

- Output block → `bordered_block(theme, " Output ", false)`
- list_scrollbar_areas → `list_scrollbar_areas()`
- render_scrollbar → `render_scrollbar()`
- 状态颜色 `theme.accent_success`/`theme.accent_error`/`theme.accent_warning`（已在使用，无需改）
- 页脚样式 → `theme.dim_style()`
- 分隔线（命令分隔符的 `theme.text_disabled` + DIM）→ 可用 `theme.disabled_style()` 但注意修饰符不同

#### 验证

```bash
cargo test 2>&1 | tail -3
```

#### 提交

```bash
git add src/ui/execution_screen.rs
git commit -m "refactor(execution_screen): use shared helpers for block and scrollbar"
```

---

### T7：重构 variable_screen.rs + help_screen.rs

**文件**: `src/ui/variable_screen.rs`, `src/ui/help_screen.rs`

- variable_screen: 对话框边框 → `bordered_block()`; 聚焦行样式 → `theme.selected_style()`; 提示行样式 → `theme.dim_style()`
- help_screen: 对话框边框 → `bordered_block()`; 对话框背景 → `theme.surface`
- 两个对话框的居中计算 → `centered_rect()`

#### 验证

```bash
cargo test 2>&1 | tail -3
```

#### 提交

```bash
git add src/ui/variable_screen.rs src/ui/help_screen.rs
git commit -m "refactor: use shared helpers for dialogs and styles"
```

---

### T8：合并 stop/kill_execution

**文件**: `src/app.rs`

```rust
/// Tear down the execution thread. `keep_screen=true` preserves the exec screen
/// (for Skip/Interrupt), `mark_skipped=true` marks remaining commands as skipped.
fn teardown_execution(&mut self, keep_screen: bool, mark_skipped: bool) {
    kill_execution(&mut self.kill_signal, &mut self.execution_rx, &mut self.execution_handle);
    if mark_skipped {
        if let Some(ref mut es) = self.exec_screen {
            es.mark_remaining_as_skipped();
        }
    }
    if !keep_screen {
        self.exec_screen = None;
    }
}
```

更新调用点：
- `BackToMain`: `self.teardown_execution(false, false)`（kill + destroy screen）
- `Interrupt | Skip`: `self.teardown_execution(true, true)`（kill + keep + mark skipped）
- `Reexecute`: `self.teardown_execution(false, false)`（kill + destroy, then restart）

删除旧的 `stop_execution()` 和 `kill_execution()` 方法。

#### 验证

```bash
cargo test 2>&1 | tail -3
```

#### 提交

```bash
git add src/app.rs
git commit -m "refactor: merge stop_execution and kill_execution into teardown_execution"
```

---

### T9：cycle_enum 通用化

**文件**: `src/ui/detail_screen.rs`

```rust
/// Generic cycle helper for enum variants.
/// `variants` is a slice of all variants, `current` is the current value,
/// `delta` is +1 for next, -1 for previous.
fn cycle_enum<T: Clone + PartialEq>(variants: &[T], current: &T, delta: isize) -> T {
    let pos = variants.iter().position(|v| v == current).unwrap_or(0);
    let next = (pos as isize + delta).rem_euclid(variants.len() as isize) as usize;
    variants[next].clone()
}
```

然后替换三个 cycle 方法：

- `cycle_group`: 不需要，`groups` 是动态列表不是静态切片
- **`cycle_shell`**: ⚠️ 保持手动处理。当前实现有 `Custom` 分支逻辑（保存现有 Custom 路径），`cycle_enum` 的 `Clone` 约束无法处理 `Custom` 的动态路径。只将 `Builtin` 部分改用 `cycle_enum`，或保持原样不动
- `cycle_exec_mode`: 用 `cycle_enum(&[ExecMode::StopOnError, ExecMode::ContinueOnError], &self.set.exec_mode, delta)` 替换整个方法体

#### 验证

```bash
cargo test 2>&1 | tail -3
```

#### 提交

```bash
git add src/ui/detail_screen.rs
git commit -m "refactor: replace cycle_group/shell/mode with generic cycle_enum helper"
```

---

### T10：统一删除-选择逻辑确认

**文件**: `src/app.rs`

确认所有 4 个列表的删除后选择调整已统一（上一步已做），使用 `clamp_selected()`：

```rust
// DeleteSet:
self.data.groups[gi].sets.remove(si);
self.main_screen.set_list.clamp_selected(self.data.groups[gi].sets.len());

// DeleteGroup:
self.data.groups.remove(gi);
self.main_screen.group_list.clamp_selected(self.data.groups.len());

// DeleteVariable:
ds.set.variables.remove(idx);
ds.variable_list.clamp_selected(ds.set.variables.len());

// DeleteCommand:
ds.set.commands.remove(idx);
ds.command_list.clamp_selected(ds.set.commands.len());
```

替换所有旧的 `if selected >= len` 逻辑。

#### 验证

```bash
cargo test 2>&1 | tail -3
```

#### 提交

```bash
git add src/app.rs
git commit -m "refactor: unify delete selection clamping using ScrollableList::clamp_selected"
```

---

## 总验证

```bash
cargo test
cargo clippy 2>&1 | grep '^error'
cargo build
```
