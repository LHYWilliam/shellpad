---
title: 状态栏文本分派模式统一
date: 2026-06-17
status: draft
---

## 1. 问题

三处状态栏文本分派使用了三种不同模式：

| 文件 | 当前模式 | 区分条件数 |
|------|---------|-----------|
| `main_screen/render.rs` | `if-else if-else` | 3 (rename/search/normal) |
| `detail_screen/render.rs` | `if-else { match }` | 7 (editing + 6 foci) |
| `execution_screen/render.rs` | 嵌套 `if-else` | 5 (focus+completed/continue/normal) |

## 2. 统一模式

统一为两步结构：**变量赋值 → 调用 render**。顶层分派用 `match` + 元组模式匹配静态条件，返回 `&'static str`。

```rust
pub(crate) fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
    // Step 1: determine text
    let text = match (self.state_field_a, self.state_field_b) {
        (VariantX, _) => "status text",
        (VariantY, true) => "status text",
        (VariantY, false) => "status text",
    };
    // Step 2: render
    render_status_bar(frame, area, theme, text);
}
```

### 要求

- 所有状态文本为 `&'static str`，不使用 `String`/`into()`
- 无嵌套分支——扁平化为单一 `match` 表达式
- 可读性优先：条件分支 ≥3 用 `match`，≤2 用 `if-else`

## 3. 各文件明细

### 3.1 `main_screen/render.rs`

```rust
pub(crate) fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
    let text = if self.rename_mode {
        "[Enter] Confirm  [Esc] Cancel — renaming group"
    } else if self.search_mode {
        "[Enter] Confirm  [Esc] Cancel  [↑/↓] Nav — searching"
    } else {
        "[↑/↓] Nav  [←/→] Panel  [Enter] Run  [e] Edit  [n] New  [R] Rename  [d] Del set  [D] Del group  [g] New group  [/] Search  [q] Quit"
    };
    render_status_bar(frame, area, theme, text);
}
```

只有 3 个条件，`if-else if-else` 已经清晰，不需改动。保留。

### 3.2 `detail_screen/render.rs`

当前：`if-else { match { 6 arm } }`，内层 match 需要 `String` 因为外层用 `format!`。

改为治表内联到单一 match：

```rust
pub(crate) fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
    let is_editing = self.var_edit.is_editing() || self.cmd_edit.is_editing();
    let text = match (is_editing, self.focus) {
        (true, _) => "[Enter] Confirm  [Esc] Cancel",
        (false, DetailFocus::Name) => "[Enter] Edit name  [Tab] Next  |  [Ctrl+S] Save",
        (false, DetailFocus::Group) => "[←/→] Change group  [Tab] Next  |  [Ctrl+S] Save",
        (false, DetailFocus::Shell) => "[←/→] Change shell  [Tab] Next  |  [Ctrl+S] Save",
        (false, DetailFocus::ExecMode) => "[←/→] Change mode  [Tab] Next  |  [Ctrl+S] Save",
        (false, DetailFocus::Variables) => {
            "[a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Tab] Next  |  [Ctrl+S] Save"
        }
        (false, DetailFocus::Commands) => {
            "[a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Tab] Next  |  [Ctrl+S] Save"
        }
    };
    render_status_bar(frame, area, theme, text);
}
```

注意：`|  [Ctrl+S] Save` 之前在第二段 `format!(" ...  |  [Ctrl+S] Save  [Esc] Cancel", status)` 中。统一后每个 arm 自身包含完整状态栏文本。`[Esc] Cancel` 在非编辑模式下与 Save 无关，移除。需要确认这个行为变更是否符合预期。

### 3.3 `execution_screen/render.rs`

当前：嵌套 `if-else`，5 个分支。

改为用元组 `(focus_index, completed, continue_from)` 三字段匹配：

```rust
    let footer_text = match (self.focus_index, self.completed, self.continue_from) {
        (Some(_), _, _) => "[←/→] Browse  [z] Follow  [q] Back",
        (None, true, None) => " [←/→] Browse  [r] Re-execute  [q] Back",
        (None, true, Some(_)) => " [←/→] Browse  [n] Continue from next  [r] Re-execute  [q] Back",
        (None, false, _) => " [←/→] Browse  [s] Skip  [z] Auto-scroll  [Ctrl+C] Interrupt  [q] Back",
    };
```

## 4. 变更文件

| 文件 | 操作 |
|------|------|
| `src/ui/detail_screen/render.rs` | `if-else { match }` → 单一 `match` 元组 |
| `src/ui/execution_screen/render.rs` | 嵌套 `if-else` → `match` 元组 |
| `src/ui/main_screen/render.rs` | 无需改动 |
