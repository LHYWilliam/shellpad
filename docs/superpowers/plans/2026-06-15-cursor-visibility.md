# Cursor Visibility Enhancement — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a visible blinking-bar cursor in all text input contexts across the TUI.

**Architecture:** Set cursor style once at terminal init; add a shared cursor-positioning helper to `components.rs`; add `frame.set_cursor_position()` calls in `detail_screen.rs` for name editing and inline variable/command editing.

**Tech Stack:** crossterm 0.29.0 (SetCursorStyle), ratatui 0.30.1, unicode-width 0.2.2

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src/tui.rs` | Modify | Set `BlinkingBar` cursor on init, reset to default on restore |
| `src/ui/components.rs` | Modify | Add `set_cursor_after_prefix()` helper; update `TextInput::render()` to use it |
| `src/ui/detail_screen.rs` | Modify | Add cursor positioning in name edit and inline variable/command edit |
| `src/ui/main_screen.rs` | Modify | Rename mode cursor → use shared helper |
| `src/ui/variable_screen.rs` | Modify | Variable dialog cursor → use shared helper |

---

### Task 1: Terminal Cursor Style

**Files:**
- Modify: `src/tui.rs`

- [ ] **Add `SetCursorStyle` import and apply to init/restore**

Replace the current imports and functions:
```rust
use crossterm::cursor::SetCursorStyle;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;

/// The TUI terminal instance.
pub type TuiTerminal = Terminal<CrosstermBackend<io::Stdout>>;

/// Initialize the terminal into raw mode + alternate screen.
pub fn init_terminal() -> io::Result<TuiTerminal> {
    enable_raw_mode()?;
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        SetCursorStyle::BlinkingBar,
    )?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    Ok(terminal)
}

/// Restore the terminal to normal mode.
pub fn restore_terminal() -> io::Result<()> {
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        SetCursorStyle::DefaultUserShape,
    )?;
    disable_raw_mode()?;
    Ok(())
}
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/tui.rs
git commit -m "feat(cursor): set BlinkingBar cursor on terminal init"
```

---

### Task 2: Cursor Positioning Helper

**Files:**
- Modify: `src/ui/components.rs`

- [ ] **Add `set_cursor_after_prefix()` function**

Add this at the end of `components.rs` (after the `handle_text_input` function):

```rust
/// Set the terminal cursor after a text prefix at the given row.
/// `prefix_display_width` is the display column width of the label before the editable content.
/// `content` is the full editable text, `cursor` is the byte offset within it.
pub fn set_cursor_after_prefix(
    frame: &mut Frame,
    content: &str,
    cursor: usize,
    prefix_display_width: u16,
    row: Rect,
) {
    let cursor_display = unicode_width::UnicodeWidthStr::width(
        &content[..cursor.min(content.len())],
    );
    frame.set_cursor_position((
        row.x + prefix_display_width + cursor_display as u16,
        row.y,
    ));
}
```

The four parameters are:
- `frame` — the ratatui frame for cursor positioning
- `content` — the editable text (`TextInput.content`)
- `cursor` — the cursor byte offset (`TextInput.cursor`)
- `prefix_display_width` — display columns of the label prefix (e.g., " Name: " = 7)
- `row` — the `Rect` of the row where the text is rendered

- [ ] **Update `TextInput::render()` to use the helper**

In `TextInput::render()`, replace the cursor positioning block (lines 107-112):

```rust
        if focused {
            let col = unicode_width::UnicodeWidthStr::width(&self.content[..self.cursor.min(self.content.len())]);
            let cursor_x = inner.x + col as u16;
            frame.set_cursor_position((cursor_x, inner.y));
        }
```

With:
```rust
        if focused {
            set_cursor_after_prefix(frame, &self.content, self.cursor, 0, inner);
        }
```

The `0` prefix width is correct because `TextInput` content starts right at the inner area's left edge (no additional prefix).

- [ ] **Commit**

```bash
git add src/ui/components.rs
git commit -m "feat(cursor): add set_cursor_after_prefix helper, update TextInput::render"
```

---

### Task 3: Detail Screen Name Editing Cursor

**Files:**
- Modify: `src/ui/detail_screen.rs`

- [ ] **Add cursor positioning in `render_metadata()` for name editing**

In the `render_metadata()` function, after rendering the name Paragraph (after line 112), add:

```rust
        // Cursor for name editing
        if self.editing_name {
            let prefix_width = unicode_width::UnicodeWidthStr::width(" Name: ");
            set_cursor_after_prefix(
                frame,
                &self.name_input.content,
                self.name_input.cursor,
                prefix_width as u16,
                name_row,
            );
        }
```

Add the import for `set_cursor_after_prefix`:
```rust
use crate::ui::components::set_cursor_after_prefix;
```

- [ ] **Compile**

```bash
cargo check 2>&1 | grep error
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "feat(cursor): add cursor to detail screen name editing"
```

---

### Task 4: Detail Screen Inline Edit Cursor

**Files:**
- Modify: `src/ui/detail_screen.rs`

**Goal:** When editing a variable or command inline, show the cursor at the correct position inside the list.

The inline edit renders as a `ListItem` in a `List` widget. We can't position the cursor inside `ListItem` during rendering, but we can calculate where that row appears on screen. The editing row is at:

- `list_area.y + editing_index_into_visible_items` (approximately)
- More precisely: `var_block.inner(area).y + 1 + editing_idx` (1 for the block's top border)

But this is tricky because the list may be scrolled. Let me use a simpler approach: the `list_area` already accounts for scroll offset. The editing variable/command is at position `editing_idx` in the items vector, but it may be scrolled out of view.

For the initial implementation, use the scroll offset:
- Item visual position = `list_area.y + editing_idx - scroll_offset`
- This works only if the item is visible (within list_area height)

Let me use the `var_area` (the area passed to render_variables) which includes the block borders:

```rust
// In render_variables, after the list rendering:
if self.edit_state.is_editing() {
    let editing_idx = self.edit_state.editing_variable
        .or(self.edit_state.editing_command)
        .unwrap_or(0);
    let row_y = var_area.y + 1 + editing_idx as u16;
    let prefix_width = unicode_width::UnicodeWidthStr::width("  ▶ ");
    set_cursor_after_prefix(
        frame,
        &self.edit_state.edit_input.content,
        self.edit_state.edit_input.cursor,
        prefix_width as u16,
        Rect::new(var_area.x + 1, row_y, var_area.width.saturating_sub(2), 1),
    );
}
```

But wait - `render_variables` and `render_commands` are separate functions. The edit state can be editing either a variable OR a command, not both at once. The cursor should only show for the active edit.

Actually, the approach is simpler: put the cursor logic directly inside `render_variables` and `render_commands` where `is_editing` is already checked. The editing row's visual position can be derived from the block's inner area.

For `render_variables`, after rendering the List and scrollbar (after the scrollbar block), add:

```rust
        // Cursor for inline variable editing
        if let Some(idx) = self.edit_state.editing_variable {
            // Calculate the visual row of the editing item within the list
            let item_y = list_area.y + idx.saturating_sub(self.variable_list.offset) as u16;
            if item_y < list_area.y + list_area.height {
                let prefix_width = unicode_width::UnicodeWidthStr::width("  ▶ ");
                set_cursor_after_prefix(
                    frame,
                    &self.edit_state.edit_input.content,
                    self.edit_state.edit_input.cursor,
                    prefix_width as u16,
                    Rect::new(list_area.x, item_y, list_area.width, 1),
                );
            }
        }
```

For `render_commands`, similarly after its scrollbar block:

```rust
        // Cursor for inline command editing
        if let Some(idx) = self.edit_state.editing_command {
            let item_y = list_area.y + idx.saturating_sub(self.command_list.offset) as u16;
            if item_y < list_area.y + list_area.height {
                let prefix_width = unicode_width::UnicodeWidthStr::width("  #0▶ ");
                // For insert mode, the display has "#N▶" prefix; for edit mode, "#0▶" is actually 
                // rendered as "  #0  " in non-edit mode but for editing we use "  ▶ " prefix
                // Actually during editing, the variable label is "  ▶ content"
                // and command label is "  #N▶ content"
                // Let's compute actual prefix width from the rendered label
                let pos = self.edit_state.insert_at.unwrap_or(idx);
                let display_prefix = format!("  #{}▶ ", pos);
                let prefix_width = unicode_width::UnicodeWidthStr::width(&display_prefix);
                set_cursor_after_prefix(
                    frame,
                    &self.edit_state.edit_input.content,
                    self.edit_state.edit_input.cursor,
                    prefix_width as u16,
                    Rect::new(list_area.x, item_y, list_area.width, 1),
                );
            }
        }
```

Note: For commands, the prefix includes the position number (e.g., "  #0▶ "). For variables, the prefix is always "  ▶ ".

Also need to add this import:
- `use ratatui::layout::Rect;` — check if already imported (yes, line 4: `use ratatui::layout::{Constraint, Layout, Rect};`)
- `use crate::ui::components::set_cursor_after_prefix;` — already added in Task 3

- [ ] **Compile**

```bash
cargo check 2>&1 | grep error
```

- [ ] **Test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "feat(cursor): add cursor to detail screen inline variable/command editing"
```

---

### Task 5: Migrate Existing Cursor Code to Helper

**Files:**
- Modify: `src/ui/main_screen.rs`
- Modify: `src/ui/variable_screen.rs`

- [ ] **Update rename mode cursor in `main_screen.rs`**

In `main_screen.rs` `render()` method, find the rename mode cursor positioning block (lines 121-126):

```rust
            let prefix_w = unicode_width::UnicodeWidthStr::width(prefix);
            let content_w = unicode_width::UnicodeWidthStr::width(&ren.content[..ren.cursor.min(ren.content.len())]);
            frame.set_cursor_position((
                status_area.x + prefix_w as u16 + content_w as u16,
                status_area.y,
            ));
```

Replace with:
```rust
            let prefix_w = unicode_width::UnicodeWidthStr::width(prefix);
            set_cursor_after_prefix(
                frame,
                &ren.content,
                ren.cursor,
                prefix_w as u16,
                Rect::new(status_area.x, status_area.y, status_area.width, 1),
            );
```

Add import:
```rust
use crate::ui::components::set_cursor_after_prefix;
```

- [ ] **Update variable screen cursor in `variable_screen.rs`**

In `variable_screen.rs` `render()` method, find the cursor positioning block (lines 127-134):

```rust
                let prefix_w = unicode_width::UnicodeWidthStr::width(" ") // leading space
                    + unicode_width::UnicodeWidthStr::width(self.names[i].as_str())
                    + unicode_width::UnicodeWidthStr::width(" = ");
                let content_w = unicode_width::UnicodeWidthStr::width(
                    &self.inputs[i].content[..self.inputs[i].cursor
                        .min(self.inputs[i].content.len())],
                );
                frame.set_cursor_position((
                    inner.x + prefix_w as u16 + content_w as u16,
                    inner.y + i as u16,
                ));
```

Replace with:
```rust
                let prefix_w = unicode_width::UnicodeWidthStr::width(" ") // leading space
                    + unicode_width::UnicodeWidthStr::width(self.names[i].as_str())
                    + unicode_width::UnicodeWidthStr::width(" = ");
                set_cursor_after_prefix(
                    frame,
                    &self.inputs[i].content,
                    self.inputs[i].cursor,
                    prefix_w as u16,
                    Rect::new(inner.x, inner.y + i as u16, inner.width, 1),
                );
```

Add import:
```rust
use crate::ui::components::set_cursor_after_prefix;
```

Also add `use ratatui::layout::Rect;` if not already imported (check — yes, already imported on line 4).

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs src/ui/variable_screen.rs
git commit -m "refactor(cursor): migrate existing cursor code to shared helper"
```

---

### Task 6: Final Verification

- [ ] **Run full test suite**

```bash
cargo test
```

Expected: All tests pass (60+).

- [ ] **Run clippy**

```bash
cargo clippy 2>&1 | grep '^error'
```

Expected: No errors.

- [ ] **Build**

```bash
cargo build
```
