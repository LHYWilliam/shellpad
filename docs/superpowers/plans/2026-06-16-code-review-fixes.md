# Code Review Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 3 bugs found during code review of the refactoring branch.

**Architecture:** One fix per file. Bug 1 fixes missing lower-bound check in the shared helper AND migrates 3 existing call sites to use it. Bug 2 fixes two missed `list_scrollbar_areas()` substitutions. Bug 3 reverts dialog borders from `bordered_block()` back to inline `accent_info`.

---

### Task 1: Fix `render_inline_cursor()` lower-bound guard + migrate call sites

**Files:**
- Modify: `src/ui/components.rs` (`render_inline_cursor`)
- Modify: `src/ui/main_screen.rs` (rename cursor → use `render_inline_cursor`)
- Modify: `src/ui/detail_screen.rs` (variable + command cursor → use `render_inline_cursor`)

- [ ] **Step 1: Fix lower-bound guard in `render_inline_cursor()`**

In `src/ui/components.rs`, replace:
```rust
    let item_y = list_area.y + item_index.saturating_sub(list_offset) as u16;
    if item_y < list_area.y + list_area.height {
```
with:
```rust
    let item_y = list_area.y + item_index.saturating_sub(list_offset) as u16;
    if item_index >= list_offset && item_y < list_area.y + list_area.height {
```

- [ ] **Step 2: Migrate rename cursor in `main_screen.rs` to `render_inline_cursor()`**

Find the rename cursor block in `render_group_panel()` (current pattern at line ~198-210):
```rust
        if self.rename_mode && !data.groups.is_empty() {
            let offset = self.group_list.offset;
            let selected = self.group_list.selected;
            let item_y = list_area.y + selected.saturating_sub(offset) as u16;
            if item_y < list_area.y + list_area.height {
                let prefix_width = unicode_width::UnicodeWidthStr::width("▶ ");
                set_cursor_after_prefix(
                    frame,
                    &self.rename_input.content,
                    self.rename_input.cursor,
                    prefix_width as u16,
                    Rect::new(list_area.x, item_y, list_area.width, 1),
                );
            }
        }
```
Replace with:
```rust
        if self.rename_mode && !data.groups.is_empty() {
            render_inline_cursor(
                frame, list_area, self.group_list.offset,
                self.group_list.selected, &self.rename_input,
                unicode_width::UnicodeWidthStr::width("▶ ") as u16,
            );
        }
```

Add `render_inline_cursor` to the components import:
```rust
use crate::ui::components::{
    bordered_block, empty_hint, handle_text_input, list_scrollbar_areas, render_inline_cursor,
    render_scrollbar, render_status_bar, set_cursor_after_prefix, ScrollableList, TextInput,
};
```

- [ ] **Step 3: Migrate variable inline cursor in `detail_screen.rs` to `render_inline_cursor()`**

Find the variable inline cursor block in `render_variables()` (around lines 249-261):
```rust
        if let Some(idx) = self.edit_state.editing_variable {
            let pos = self.edit_state.insert_at.unwrap_or(idx);
            let item_y = list_area.y + pos.saturating_sub(self.variable_list.offset) as u16;
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
Replace with:
```rust
        if let Some(idx) = self.edit_state.editing_variable {
            let pos = self.edit_state.insert_at.unwrap_or(idx);
            render_inline_cursor(
                frame, list_area, self.variable_list.offset,
                pos, &self.edit_state.edit_input,
                unicode_width::UnicodeWidthStr::width("  ▶ ") as u16,
            );
        }
```

- [ ] **Step 4: Migrate command inline cursor in `detail_screen.rs` to `render_inline_cursor()`**

Find the command inline cursor block in `render_commands()` (around lines 338-354):
```rust
        if let Some(idx) = self.edit_state.editing_command {
            let pos = self.edit_state.insert_at.unwrap_or(idx);
            let item_y = list_area.y + pos.saturating_sub(self.command_list.offset) as u16;
            if item_y < list_area.y + list_area.height {
                let pos = self.edit_state.insert_at.unwrap_or(idx);
                let display_prefix = format!("  #{}▶ ", pos);
                let prefix_width = unicode_width::UnicodeWidthStr::width(display_prefix.as_str());
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
Replace with:
```rust
        if let Some(idx) = self.edit_state.editing_command {
            let pos = self.edit_state.insert_at.unwrap_or(idx);
            let display_prefix = format!("  #{}▶ ", pos);
            render_inline_cursor(
                frame, list_area, self.command_list.offset,
                pos, &self.edit_state.edit_input,
                unicode_width::UnicodeWidthStr::width(display_prefix.as_str()) as u16,
            );
        }
```

Add `render_inline_cursor` to the components import:
```rust
use crate::ui::components::{
    bordered_block, empty_hint, handle_text_input, list_scrollbar_areas, render_inline_cursor,
    render_scrollbar, render_status_bar, set_cursor_after_prefix, ScrollableList, TextInput,
};
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/components.rs src/ui/main_screen.rs src/ui/detail_screen.rs
git commit -m "fix: add lower-bound guard to render_inline_cursor, migrate 3 call sites"
```

---

### Task 2: Fix missed `list_scrollbar_areas()` in `main_screen.rs`

**Files:**
- Modify: `src/ui/main_screen.rs`

- [ ] **Replace both inline Layout::horizontal patterns in `render_set_panel`**

In `render_set_panel()`, replace the search-mode branch:
```rust
            // Split remaining into list + scrollbar
            let list_layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
            let [list_area, sb_area] = list_layout.areas(remaining);
            (list_area, sb_area)
```
with:
```rust
            // Split remaining into list + scrollbar
            let (list_area, sb_area) = list_scrollbar_areas(remaining);
            (list_area, sb_area)
```

And the non-search-mode branch:
```rust
            // Original: split inner into list + scrollbar
            let list_layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
            let [list_area, sb_area] = list_layout.areas(inner);
            (list_area, sb_area)
```
with:
```rust
            // Original: split inner into list + scrollbar
            let (list_area, sb_area) = list_scrollbar_areas(inner);
            (list_area, sb_area)
```

Now check if `Layout` and `Constraint` imports are still needed elsewhere. Grep for `Constraint::` in the file — if only used in the search layout split (line 247: `Layout::vertical([Constraint::Length(1), Constraint::Min(1)])`), keep the import. If all other uses are gone, remove the unused import.

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "fix: replace missed list_scrollbar_areas in render_set_panel"
```

---

### Task 3: Fix dialog border color regression

**Files:**
- Modify: `src/ui/help_screen.rs`
- Modify: `src/ui/variable_screen.rs`

**Root Cause:** `bordered_block(theme, "...", false)` uses `theme.surface_border` (gray in dark theme), but dialogs previously used `theme.accent_info` (sky blue). `bordered_block()` only has two modes (focused/not-focused) and cannot express a third accent color.

**Fix:** Revert dialog borders to inline Block construction with `theme.accent_info`, keeping `centered_rect()` (which works correctly).

- [ ] **Fix `help_screen.rs`**

Replace:
```rust
    let block = bordered_block(theme, " Help ", false)
        .style(Style::default().bg(theme.surface));
```
with:
```rust
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent_info))
        .title(" Help ")
        .style(Style::default().bg(theme.surface));
```

Remove `bordered_block` from import since it's no longer used:
```rust
use crate::ui::components::centered_rect;
```
Remove `Rating` if it was left alone — check the imports: `use ratatui::widgets::{Clear, Paragraph};` — need to add `Block, Borders` back:
```rust
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
```

- [ ] **Fix `variable_screen.rs`**

Replace:
```rust
    let block = bordered_block(theme, " Set Variables ", false)
        .style(Style::default().bg(theme.surface));
```
with:
```rust
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent_info))
        .title(" Set Variables ")
        .style(Style::default().bg(theme.surface));
```

Remove `bordered_block` from import:
```rust
use crate::ui::components::{centered_rect, handle_text_input, set_cursor_after_prefix, TextInput};
```
Add `Block, Borders` back to ratatui imports:
```rust
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/help_screen.rs src/ui/variable_screen.rs
git commit -m "fix: restore accent_info border color for overlay dialogs"
```

---

### Verification

```bash
cargo test
cargo clippy 2>&1 | grep '^error'
cargo build
```
