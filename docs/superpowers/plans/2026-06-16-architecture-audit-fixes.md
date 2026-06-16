# Architecture Audit Fixes (V2) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all 11 consistency issues found during the full architecture audit.

**Architecture:** 9 tasks in dependency order: toast fixes, then behavior fixes, then code dedup, then abstraction cleanup, then call-site normalization.

**Tech Stack:** ratatui 0.30.1

---

## TODO

- [ ] T1. Detail delete toasts
- [ ] T2. Create/update toasts (NewSet/NewGroup/RenameGroup/Execute)
- [ ] T3. Empty-list focus migration after detail deletion
- [ ] T4. commit_edit() extraction
- [ ] T5. Execution footer → render_status_bar()
- [ ] T6. bordered_block_info() + dialog migration
- [ ] T7. highlight_style normalization
- [ ] T8. selected_or_none() normalization
- [ ] T9. render_items_list() — extract common renderer for variables/commands

---

### T1. Detail deletion toasts

**Files:** `src/app.rs`

- [ ] Add `push_toast` to DeleteVariable (lines 345 and 355):

```rust
// DeleteVariable (after clamp_selected):
self.push_toast("Variable deleted", ToastSeverity::Info);

// DeleteCommand (after clamp_selected):
self.push_toast("Command deleted", ToastSeverity::Info);
```

- [ ] Commit: `fix: add toast notifications for variable and command deletions`

---

### T2. Creation/update toasts

**Files:** `src/app.rs`

- [ ] Add toast after `NewSet` (line 265): `self.push_toast("Set created", ToastSeverity::Info);`
- [ ] Add toast after `NewGroup` (line 290): `self.push_toast("Group created", ToastSeverity::Info);`
- [ ] Add toast after `RenameGroup` (line 296): `self.push_toast("Group renamed", ToastSeverity::Info);`
- [ ] Add toast in `handle_variable_action::Execute` (line 227): `self.push_toast("Variables updated", ToastSeverity::Info);`

- [ ] Commit: `fix: add toast notifications for create/update operations`

---

### T3. Empty-list focus migration

**Files:** `src/app.rs` (`on_detail_action` DeleteVariable / DeleteCommand)

- [ ] After `DeleteVariable` (line 345), after `clamp_selected`:
```rust
if ds.set.variables.is_empty() {
    ds.focus = DetailFocus::Name;
}
```

- [ ] After `DeleteCommand` (line 357), after `clamp_selected`:
```rust
if ds.set.commands.is_empty() {
    ds.focus = DetailFocus::Variables;
}
```

- [ ] Commit: `fix: switch detail focus away from empty list after deletion`

---

### T4. commit_edit() extraction

**Files:** `src/ui/detail_editor.rs`

- [ ] Add helper method to `DetailEditState`:
```rust
fn commit_edit<T>(&mut self, idx: usize, items: &mut Vec<T>, new_item: T, list: &mut ScrollableList) {
    if let Some(insert_pos) = self.insert_at.take() {
        items.insert(insert_pos, new_item);
        list.selected = insert_pos;
    } else {
        items[idx] = new_item;
        list.selected = idx;
    }
}
```

- [ ] Replace the insert-vs-edit block in `handle_variable_edit` Enter branch (both paths: `=`-split and no-`=`):
```rust
// Old (each path):
if let Some(insert_pos) = self.insert_at.take() {
    variables.insert(insert_pos, var);
    list.selected = insert_pos;
} else {
    variables[idx] = var;
    list.selected = idx;
}
// New:
self.commit_edit(idx, variables, var, list);
```

- [ ] Replace the insert-vs-edit block in `handle_command_edit` Enter branch:
```rust
// Old:
if let Some(insert_pos) = self.insert_at.take() {
    commands.insert(insert_pos, command);
    list.selected = insert_pos;
} else {
    commands[idx] = command;
    list.selected = idx;
}
// New:
self.commit_edit(idx, commands, command, list);
```

- [ ] Commit: `refactor: extract commit_edit helper to deduplicate insert-vs-edit logic`

---

### T5. Execution footer → render_status_bar()

**Files:** `src/ui/execution_screen.rs`

- [ ] Change footer layout from `Length(1)` to `Length(2)` (line 309):
```rust
let body_layout = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]);
```

- [ ] Replace manual Paragraph rendering (lines ~327-333) with:
```rust
render_status_bar(frame, footer_area, theme, footer_text);
```

- [ ] Ensure `render_status_bar` imported:
```rust
use crate::ui::components::{bordered_block, list_scrollbar_areas, render_scrollbar, render_status_bar};
```

- [ ] Commit: `fix: use render_status_bar for execution footer`

---

### T6. bordered_block_info() + dialog migration

**Files:** `src/ui/components.rs`, `src/ui/help_screen.rs`, `src/ui/variable_screen.rs`

- [ ] Add to `components.rs` after `bordered_block()`:
```rust
/// Create a bordered Block with accent_info color for overlay dialogs.
pub fn bordered_block_info<'a>(theme: &Theme, title: &'a str) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent_info))
        .title(title)
}
```

- [ ] Migrate `help_screen.rs` — replace manual Block with `bordered_block_info(theme, " Help ").style(...)`. Remove `Block`/`Borders` from imports.

- [ ] Migrate `variable_screen.rs` — replace manual Block with `bordered_block_info(theme, " Set Variables ").style(...)`. Remove `Block`/`Borders` from imports. Add `bordered_block_info` to `components` import.

- [ ] Commit: `refactor: add bordered_block_info for overlay dialog borders`

---

### T7. highlight_style normalization

**Files:** `src/ui/main_screen.rs`

**Decision:** Remove `highlight_style` everywhere, relying only on per-item inline styles (matching detail_screen pattern).

- [ ] Groups panel (line 192): change from `List::new(items).highlight_style(theme.selected_style(...))` to `List::new(items)`
- [ ] Sets panel (line 347): same change

- [ ] Commit: `refactor: remove redundant highlight_style from main screen lists`

---

### T8. selected_or_none() normalization

**Files:** `src/ui/main_screen.rs`

- [ ] Groups panel (line 191): change `with_selected(Some(self.group_list.selected))` to `with_selected(self.group_list.selected_or_none(data.groups.len()))`

- [ ] Sets panel (line 343-344): change existing `.filter()` chain to:
```rust
let selected = if self.active_panel == Panel::Sets {
    self.set_list.selected_or_none(sets.len())
} else {
    None
};
```

- [ ] Commit: `refactor: normalize selected_or_none usage`

---

### T9. render_items_list() extraction

**Files:** `src/ui/detail_screen.rs`

**Goal:** Replace `render_variables` (72 lines) and `render_commands` (84 lines) with calls to a single generic function. The function takes:
- The data slice length (for title, scrollbar)
- The edit state (for inline editing display)
- A closure `(index, is_editing, is_insert) -> (label, style)`
- The `ScrollableList` reference

- [ ] Add private method `render_items_list` to `DetailScreenState`:

```rust
fn render_items_list(
    &self,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &str,
    focused: bool,
    count: usize,
    list: &ScrollableList,
    editing_item: Option<usize>,
    label_fn: impl Fn(usize, bool, bool) -> (String, Style),
    preview_label: Option<String>,
    empty_text: &str,
) {
    let block = bordered_block(theme, title, focused);
    let inner = block.inner(area);
    frame.render_widget(&block, area);

    let (list_area, scrollbar_area) = list_scrollbar_areas(inner);

    let mut items: Vec<ListItem> = (0..count)
        .map(|i| {
            let is_editing = Some(i) == editing_item;
            let is_insert = self.edit_state.insert_at.is_some();
            let (label, style) = label_fn(i, is_editing, is_insert);
            ListItem::new(fill_row(Line::from(Span::styled(label, style)), style, list_area.width))
        })
        .collect();

    // Preview row for inserts
    if let Some(idx) = editing_item
        && self.edit_state.insert_at.is_some()
        && let Some(label) = preview_label
    {
        let style = Style::default()
            .fg(theme.accent_primary)
            .add_modifier(Modifier::BOLD);
        let preview = ListItem::new(fill_row(Line::from(Span::styled(label, style)), style, list_area.width));
        items.insert(
            self.edit_state.insert_at.unwrap_or(idx.min(items.len())),
            preview,
        );
    }

    if count == 0 {
        items.push(empty_hint(theme, empty_text));
    }

    let mut list_state = ratatui::widgets::ListState::default()
        .with_selected(list.selected_or_none(count));
    frame.render_stateful_widget(List::new(items), list_area, &mut list_state);

    render_scrollbar(frame, scrollbar_area, theme, count, list.selected);
}
```

- [ ] Rewrite `render_variables` to call the helper:

```rust
fn render_variables(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
    let count = self.set.variables.len();
    let title = format!(" Variables ({}) ", count);
    let preview = if self.edit_state.editing_variable.is_some()
        && self.edit_state.insert_at.is_some()
    {
        Some(format!("  ▶ {}", self.edit_state.edit_input.content))
    } else {
        None
    };

    self.render_items_list(
        frame, area, theme, &title,
        self.focus == DetailFocus::Variables,
        count, &self.variable_list,
        self.edit_state.editing_variable,
        |i, is_editing, is_insert| {
            let label = if is_editing && !is_insert {
                format!("  ▶ {}", self.edit_state.edit_input.content)
            } else {
                let v = &self.set.variables[i];
                format!("  {} = {}", v.name, v.default_value)
            };
            let style = if is_editing {
                Style::default()
                    .fg(theme.text_on_selected)
                    .bg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD)
            } else if i == self.variable_list.selected && self.focus == DetailFocus::Variables {
                theme.selected_style(theme.selection_bg_secondary)
            } else {
                theme.normal_style()
            };
            (label, style)
        },
        preview,
        " (empty — press a to add a variable) ",
    );

    // Cursor for inline variable editing
    if let Some(idx) = self.edit_state.editing_variable {
        let pos = self.edit_state.insert_at.unwrap_or(idx);
        render_inline_cursor(
            frame, /* list_area */ /* ... */, self.variable_list.offset,
            pos, &self.edit_state.edit_input,
            unicode_width::UnicodeWidthStr::width("  ▶ ") as u16,
        );
    }
}
```

Wait — the cursor code needs `list_area`. The helper function currently discards `list_area` after rendering. We need to return it or handle cursor inside the helper.

**Simpler approach:** Have `render_items_list` **return** `(list_area, scrollbar_area)` so the caller can add cursor positioning. OR pass an optional cursor closure.

Actually, the simplest approach: just compute `list_area` before calling the helper, and pass it in. But that requires computing the block.inner split twice.

**Revised:** Make `render_items_list` return `list_area` so the caller can do cursor:

```rust
fn render_items_list(...) -> Rect
```

Return `list_area`. Callers that need cursor use it; render_variables already computes it inline anyway.

- [ ] Rewrite `render_variables` to be ~20 lines (delegates to helper, adds cursor)
- [ ] Rewrite `render_commands` to be ~30 lines (same pattern, different label/position logic)
- [ ] Commit: `refactor: extract render_items_list to replace render_variables/render_commands duplication`

---

### Verification

```bash
cargo test
cargo clippy 2>&1 | grep '^error'
cargo build
```
