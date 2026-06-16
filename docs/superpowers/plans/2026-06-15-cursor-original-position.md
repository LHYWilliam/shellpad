# Cursor at Original Position + Empty Variables Fix — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move rename/search cursor from status bar to original content position; fix empty-variables insert bug.

**Architecture:** Remove status bar rename/search blocks in `main_screen.rs`. Add cursor in `render_group_panel` for rename. Move search query from Block title to a Paragraph inside the panel with cursor. Fix `detail_editor.rs` guard to allow input during empty-variable insert.

**Tech Stack:** ratatui 0.30.1, unicode-width 0.2.2

---

### Task 1: Rename Cursor at Group Name

**Files:**
- Modify: `src/ui/main_screen.rs`

- [ ] **Remove rename mode from status bar**

Find the status bar section (the if-else chain starting around line 144). Replace the entire block (from `if self.rename_mode {` to the closing `}` of `else if self.search_mode`) with just the `else` branch:

```rust
        // Status bar
        if self.rename_mode || self.search_mode {
            // Show nothing — cursor is rendered at the original position
        } else {
            self.render_status_bar(frame, status_area, theme);
        }
```

Wait — the status bar should still show key hints in rename/search mode. Let me keep a simpler version. Replace:

```rust
        // Status bar (or rename/search input)
        if self.rename_mode {
            let prefix = " Rename: ";
            let ren = &self.rename_input;
            let display = format!("{}{}", prefix, ren.content);
            let style = if ren.content.is_empty() {
                Style::default().fg(theme.text_disabled)
            } else {
                Style::default().fg(theme.text_primary)
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(display, style))),
                status_area,
            );
            let prefix_w = unicode_width::UnicodeWidthStr::width(prefix);
            set_cursor_after_prefix(
                frame,
                &ren.content,
                ren.cursor,
                prefix_w as u16,
                Rect::new(status_area.x, status_area.y, status_area.width, 1),
            );
        } else if self.search_mode {
            let prefix = " Search: ";
            let display = format!("{}{}", prefix, self.search_query);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    display,
                    Style::default().fg(theme.text_primary),
                ))),
                status_area,
            );
            let prefix_w = unicode_width::UnicodeWidthStr::width(prefix);
            set_cursor_after_prefix(
                frame,
                &self.search_query,
                self.search_cursor,
                prefix_w as u16,
                Rect::new(status_area.x, status_area.y, status_area.width, 1),
            );
        } else {
            self.render_status_bar(frame, status_area, theme);
        }
```

With:
```rust
        // Status bar (key hints always visible)
        self.render_status_bar(frame, status_area, theme);
```

- [ ] **Add rename cursor in `render_group_panel()`**

In `render_group_panel()`, after the scrollbar rendering block and before the closing `}`, add:

```rust
        // Cursor for rename mode at the selected group name position
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

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "fix: move rename cursor from status bar to group name in list"
```

---

### Task 2: Search Cursor Inside Panel

**Files:**
- Modify: `src/ui/main_screen.rs` (in `render_set_panel`)

- [ ] **Change the set panel title when in search mode**

Find the title building at the top of `render_set_panel`:
```rust
        let title = if self.search_mode {
            format!(" Search: {} ", self.search_query)
        } else {
```

Replace with:
```rust
        let title = if self.search_mode {
            " Search ".to_string()
        } else {
```

- [ ] **Add search query line + cursor inside the block**

In `render_set_panel()`, after `frame.render_widget(&block, area);` and before the `// Split inner into list + scrollbar` comment, add the search query rendering.

Replace the block from `let inner = block.inner(area);` through the inner_layout split:

Current:
```rust
        let inner = block.inner(area);
        frame.render_widget(&block, area);

        // Split inner into list + scrollbar
        let inner_layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
        let [list_area, scrollbar_area] = inner_layout.areas(inner);
```

Replace with:
```rust
        let inner = block.inner(area);
        frame.render_widget(&block, area);

        // When in search mode, split inner into search line + list area
        let (list_area, scrollbar_area) = if self.search_mode {
            let search_layout = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]);
            let [search_line, remaining] = search_layout.areas(inner);

            // Render search query line
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!(" Search: {} ", self.search_query),
                    Style::default().fg(theme.text_primary),
                ))),
                search_line,
            );

            // Cursor at end of search query
            let prefix_width = unicode_width::UnicodeWidthStr::width(" Search: ");
            set_cursor_after_prefix(
                frame,
                &self.search_query,
                self.search_cursor,
                prefix_width as u16,
                search_line,
            );

            // Split remaining area into list + scrollbar
            let list_layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
            let [list_area, sb_area] = list_layout.areas(remaining);
            (list_area, sb_area)
        } else {
            // Original: split inner into list + scrollbar
            let list_layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
            let [list_area, sb_area] = list_layout.areas(inner);
            (list_area, sb_area)
        };
```

Note: The `list_area` and `scrollbar_area` variables change from being `let` bindings to being returned from an `if` expression. All code below that uses `list_area` and `scrollbar_area` will continue to work since they're still in scope.

Also need to add `set_cursor_after_prefix` to imports if not already there — it was added in a previous commit, so it should already be imported.

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "fix: move search cursor from status bar to panel-internal query line"
```

---

### Task 3: Fix Empty Variables Insert Bug

**Files:**
- Modify: `src/ui/detail_editor.rs` (line 71)

- [ ] **Fix the `n > 0` guard**

Find this in `handle_variable_edit`:
```rust
            _ => {
                let n = variables.len();
                if n > 0 && self.editing_variable.is_some() {
                    // Protect "key=" prefix from deletion
```

Replace with:
```rust
            _ => {
                let n = variables.len();
                if (n > 0 || self.insert_at.is_some()) && self.editing_variable.is_some() {
```

- [ ] **Also fix empty-state interaction in render_variables**

The `render_variables` in `detail_screen.rs` adds an empty-state hint item. When inserting the first variable, the hint item is rendered before the preview row. This is fine — the preview row appears after the hint.

But we should ensure the `list_state` works correctly. Currently when variables is empty, `list_state.selected = None` which is correct for an empty list. When inserting, the preview row at index 0 is still visible.

No additional code change needed here — the empty-state hint shows, and when the user presses `a`, the preview row replaces the hint visually (the hint and preview are both in `items`).

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_editor.rs
git commit -m "fix: allow keyboard input when inserting first variable into empty list"
```

---

### Verification

- [ ] **Run full test suite**

```bash
cargo test
```

Expected: All 60 tests pass.

- [ ] **Run clippy**

```bash
cargo clippy 2>&1 | grep '^error'
```

Expected: No errors.

- [ ] **Build**

```bash
cargo build
```
