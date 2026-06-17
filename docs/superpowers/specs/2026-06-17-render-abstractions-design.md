# Render Abstraction Extraction — Design Spec

**Date:** 2026-06-17
**Status:** Approved
**Scope:** Extract reusable rendering helpers for editable fields and list edit cursors

## Problem

Three areas of render.rs have duplicated logic across two call sites each:

1. **Inline edit field rendering** — Name and WorkDir rows share 45 lines of
   identical styling/fill_row/cursor logic. Adding a third editable text field
   would triple the duplication.

2. **List edit cursor positioning** — render_variables and render_commands each
   have an 11-line block that checks `InlineEdit::editing`, computes a position,
   and calls `render_inline_cursor`. Only the prefix string differs.

## Solution

Two new methods on `DetailScreenState` in `render.rs`:

### 1. `render_editable_field` — unified inline edit row

```rust
fn render_editable_field(
    &self,
    frame: &mut Frame,
    row: Rect,
    theme: &Theme,
    label: &str,        // "Name", "WorkDir"
    focused: bool,       // self.focus == DetailFocus::X
    editing: bool,       // self.editing_name / self.workdir_editing
    input: &TextInput,   // &self.name_input / &self.workdir_input
    display: &str,       // &self.set.name / self.set.working_dir mapped
    dim: bool,           // true when value is default/empty and unfocused
) {
    // 1. Compute focused/editing/normal style (same as current Name)
    // 2. Apply dim style to display text when dim && !focused
    // 3. Format " {label}: {text}", fill_row + render
    // 4. Cursor positioning when editing
}
```

**Call sites:**

```rust
// Name
self.render_editable_field(frame, name_row, theme, "Name",
    self.focus == DetailFocus::Name, self.editing_name,
    &self.name_input, &self.set.name, false);

// WorkDir
self.render_editable_field(frame, workdir_row, theme, "WorkDir",
    self.focus == DetailFocus::WorkDir, self.workdir_editing,
    &self.workdir_input,
    self.set.working_dir.as_deref().unwrap_or("(default — launcher CWD)"),
    self.set.working_dir.is_none());
```

### 2. `render_edit_cursor` — unified list edit cursor

```rust
fn render_edit_cursor(
    &self,
    frame: &mut Frame,
    list_area: Rect,
    edit: &InlineEdit,
    list: &ScrollableList,
    prefix: &str,
) {
    if let Some(idx) = edit.editing {
        let pos = edit.insert_at.unwrap_or(idx);
        render_inline_cursor(
            frame, list_area, list.offset, pos,
            &edit.edit_input,
            unicode_width::UnicodeWidthStr::width(prefix) as u16,
        );
    }
}
```

**Call sites:**

```rust
// Variables
self.render_edit_cursor(frame, list_area, &self.var_edit, &self.variable_list, "  ▶ ");

// Commands (prefix includes position)
if let Some(idx) = self.cmd_edit.editing {
    let pos = self.cmd_edit.insert_at.unwrap_or(idx);
    self.render_edit_cursor(frame, list_area, &self.cmd_edit, &self.command_list,
        &format!("  #{}▶ ", pos));
}
```

## Files Affected

| File | Change |
|------|--------|
| `src/ui/detail_screen/render.rs` | Add 2 methods, replace 4 call sites |

## Code removed

### `render_editable_field` replaces in render_metadata

**Name block** (~35 lines: style computation + fill_row + cursor) → 1 method call  
**WorkDir block** (~42 lines: style computation + fill_row + cursor + dim logic) → 1 method call

### `render_edit_cursor` replaces

**render_variables tail** (~11 lines) → 1 method call  
**render_commands tail** (~11 lines, prefix calc retained) → 1 method call

## Tests

No new tests — pure rendering extraction. Existing 228 tests verify behavior.
The method extraction must produce identical visual output.

## Handler — unchanged

`commit_name_edit` and `commit_workdir_edit` in handler.rs are NOT abstracted.
Their structural similarity (clone input → set field → reset flag) is offset by
the WorkDir empty-to-None mapping, making a shared closure less readable than
the two short functions.
