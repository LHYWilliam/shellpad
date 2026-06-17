# Render Abstraction Extraction — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract two reusable rendering helpers: `render_editable_field` for Name/WorkDir rows and `render_edit_cursor` for Variables/Commands list cursors. ~60 lines net saved.

**Architecture:** Both helpers are private methods on `DetailScreenState` in `src/ui/detail_screen/render.rs`. Call sites replace inline code with single method calls. No handler or data model changes.

**Tech Stack:** Rust, Ratatui, unicode-width (existing dependencies)

---

### Task 1: Extract `render_editable_field` and `render_edit_cursor` helpers, replace call sites

**Files:**
- Modify: `src/ui/detail_screen/render.rs`

- [ ] **Step 1: Add the two helper methods**

Add after the `render_metadata` closing brace (after the separator/labels block, around line 180) and before `render_picker`:

```rust
    fn render_editable_field(
        &self,
        frame: &mut Frame,
        row: Rect,
        theme: &Theme,
        label: &str,
        focused: bool,
        editing: bool,
        input: &TextInput,
        display: &str,
        dim: bool,
    ) {
        let style = if focused {
            if editing {
                Style::default()
                    .fg(theme.text_on_selected)
                    .bg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.accent_primary)
            }
        } else {
            theme.normal_style()
        };

        let display_style = if dim && !focused {
            Style::default()
                .fg(theme.text_disabled)
                .add_modifier(Modifier::DIM)
        } else {
            style
        };

        let text = if editing {
            format!(" {}: {}", label, input.content)
        } else {
            format!(" {}: {}", label, display)
        };

        let line = fill_row(
            Line::from(Span::styled(text, display_style)),
            display_style,
            row.width,
        );
        frame.render_widget(Paragraph::new(line), row);

        if editing {
            let prefix_width = unicode_width::UnicodeWidthStr::width(format!(" {}: ", label).as_str());
            set_cursor_after_prefix(
                frame,
                &input.content,
                input.cursor,
                prefix_width as u16,
                row,
            );
        }
    }

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
                frame,
                list_area,
                list.offset,
                pos,
                &edit.edit_input,
                unicode_width::UnicodeWidthStr::width(prefix) as u16,
            );
        }
    }
```

Need to add `TextInput` to imports. Current import at line 3:

```rust
use crate::ui::widget::ScrollableList;
```

Change to:

```rust
use crate::ui::widget::{InlineEdit, ScrollableList, TextInput};
```

- [ ] **Step 2: Replace Name rendering (lines 47-84)**

Replace the entire Name block with:

```rust
        // Name
        self.render_editable_field(
            frame, name_row, theme, "Name",
            self.focus == DetailFocus::Name,
            self.editing_name,
            &self.name_input,
            &self.set.name,
            false,
        );
```

- [ ] **Step 3: Replace WorkDir rendering (lines 86-132)**

Replace the entire WorkDir block with:

```rust
        // WorkDir
        self.render_editable_field(
            frame, workdir_row, theme, "WorkDir",
            self.focus == DetailFocus::WorkDir,
            self.workdir_editing,
            &self.workdir_input,
            self.set.working_dir.as_deref().unwrap_or("(default — launcher CWD)"),
            self.set.working_dir.is_none(),
        );
```

- [ ] **Step 4: Replace Variables cursor block (lines 406-416)**

Replace:

```rust
        if let Some(idx) = self.var_edit.editing {
            let pos = self.var_edit.insert_at.unwrap_or(idx);
            render_inline_cursor(
                frame,
                list_area,
                self.variable_list.offset,
                pos,
                &self.var_edit.edit_input,
                unicode_width::UnicodeWidthStr::width("  ▶ ") as u16,
            );
        }
```

With:

```rust
        self.render_edit_cursor(frame, list_area, &self.var_edit, &self.variable_list, "  ▶ ");
```

- [ ] **Step 5: Replace Commands cursor block (lines 462-473)**

Replace:

```rust
        if let Some(idx) = self.cmd_edit.editing {
            let pos = self.cmd_edit.insert_at.unwrap_or(idx);
            let display_prefix = format!("  #{}▶ ", pos);
            render_inline_cursor(
                frame,
                list_area,
                self.command_list.offset,
                pos,
                &self.cmd_edit.edit_input,
                unicode_width::UnicodeWidthStr::width(display_prefix.as_str()) as u16,
            );
        }
```

With:

```rust
        if let Some(idx) = self.cmd_edit.editing {
            let pos = self.cmd_edit.insert_at.unwrap_or(idx);
            self.render_edit_cursor(frame, list_area, &self.cmd_edit, &self.command_list,
                &format!("  #{}▶ ", pos));
        }
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 7: Run tests**

Run: `cargo test`
Expected: All 228 tests PASS

- [ ] **Step 8: Run clippy**

Run: `cargo clippy`
Expected: No new warnings

- [ ] **Step 9: Commit**

```bash
git add src/ui/detail_screen/render.rs
git commit -m "refactor: extract render_editable_field and render_edit_cursor helpers

render_editable_field unifies Name and WorkDir row rendering
(style computation + fill_row + cursor). render_edit_cursor
unifies Variables and Commands list edit cursor positioning.
~65 lines net saved.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
