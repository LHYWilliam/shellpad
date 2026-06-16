# Picker as Side-by-Side Block — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move picker from inside Properties to an independent side-panel block with full vertical extent, deleting the broken vertical divider.

**Architecture:** `render_metadata` is cleaned up to render Properties as a standalone full-width block (no divider, no picker). `mod.rs` render() horizontally splits the metadata area into Properties (left) + Picker (right) when an Option is focused. Picker uses `bordered_block_info_zone` as a proper sibling block.

**Tech Stack:** Rust, Ratatui (no new dependencies)

---

### Task 1: Clean up render_metadata — delete divider, restore full-width

**Files:**
- Modify: `src/ui/detail_screen/render.rs`

- [ ] **Step 1: Remove vertical divider and picker integration from render_metadata**

Replace lines 134-217 (divider + picker area calculation + picker call) with full-width separator and labels:

```rust
        // Separator — full width
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" ── Options {} ", "─".repeat(sep_row.width.saturating_sub(12) as usize)),
                Style::default().fg(theme.text_disabled).add_modifier(Modifier::DIM),
            ))),
            sep_row,
        );

        // Group
        let group_name = self
            .groups
            .iter()
            .find(|g| g.id == self.set.group_id)
            .map(|g| g.name.as_str())
            .unwrap_or("(unknown)");
        let group_style = if self.focus == DetailFocus::Group {
            Style::default().fg(theme.accent_primary)
        } else {
            theme.normal_style()
        };
        let group_label = if self.focus == DetailFocus::Group {
            format!(" ◄ Group: {} ►", group_name)
        } else {
            format!(" Group: {}", group_name)
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(group_label, group_style))),
            group_row,
        );

        // Shell
        let shell_style = if self.focus == DetailFocus::Shell {
            Style::default().fg(theme.accent_primary)
        } else {
            theme.normal_style()
        };
        let shell_label = if self.focus == DetailFocus::Shell {
            format!(" ◄ Shell: {} ►", self.set.shell.label())
        } else {
            format!(" Shell: {}", self.set.shell.label())
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(shell_label, shell_style))),
            shell_row,
        );

        // Exec mode
        let mode_style = if self.focus == DetailFocus::ExecMode {
            Style::default().fg(theme.accent_primary)
        } else {
            theme.normal_style()
        };
        let mode_label = if self.focus == DetailFocus::ExecMode {
            format!(" ◄ Mode: {} ►", self.set.exec_mode.label())
        } else {
            format!(" Mode: {}", self.set.exec_mode.label())
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(mode_label, mode_style))),
            mode_row,
        );
    }
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles (picker no longer called from render_metadata — will be called from mod.rs in Task 2)

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add src/ui/detail_screen/render.rs
git commit -m "refactor: remove vertical divider and picker from render_metadata

Restore Properties block to full-width standalone rendering.
Picker will be rendered as a sibling block by the caller (mod.rs).
Labels use full rects instead of divider-constrained columns.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Add horizontal split in mod.rs, make picker a sibling block

**Files:**
- Modify: `src/ui/detail_screen/mod.rs`
- Modify: `src/ui/detail_screen/render.rs` (picker function stays, signature unchanged)

- [ ] **Step 1: Split metadata area horizontally when Option focused**

In `src/ui/detail_screen/mod.rs`, in the `render` method, replace the single metadata area with a conditional split. Currently (around line 68-82):

```rust
        // Split into top metadata and bottom command areas
        let layout = Layout::vertical([
            Constraint::Length(9), // Properties block (5 rows + borders)
            Constraint::Min(3),    // variables
            Constraint::Min(3),    // commands
            Constraint::Length(2), // status bar (separator + content)
        ]);
        let [meta_area, var_area, cmd_area, status_area] = layout.areas(inner);

        // Update scroll offsets (approx inner height = area - 2 for borders)
        self.variable_list
            .update_offset(var_area.height.saturating_sub(2) as usize);
        self.command_list
            .update_offset(cmd_area.height.saturating_sub(2) as usize);

        self.render_metadata(frame, meta_area, theme);
        self.render_variables(frame, var_area, theme);
        self.render_commands(frame, cmd_area, theme);
        self.render_status_bar(frame, status_area, theme);
```

Change to:

```rust
        // Split into top metadata and bottom command areas
        let layout = Layout::vertical([
            Constraint::Length(9), // Properties block (5 rows + borders)
            Constraint::Min(3),    // variables
            Constraint::Min(3),    // commands
            Constraint::Length(2), // status bar (separator + content)
        ]);
        let [meta_area, var_area, cmd_area, status_area] = layout.areas(inner);

        // Update scroll offsets (approx inner height = area - 2 for borders)
        self.variable_list
            .update_offset(var_area.height.saturating_sub(2) as usize);
        self.command_list
            .update_offset(cmd_area.height.saturating_sub(2) as usize);

        // When an Option is focused, split metadata into Properties (left) + Picker (right)
        let show_picker = matches!(
            self.focus,
            DetailFocus::Group | DetailFocus::Shell | DetailFocus::ExecMode
        );
        if show_picker {
            let split = Layout::horizontal([
                Constraint::Ratio(1, 2),
                Constraint::Ratio(1, 2),
            ]);
            let [props_area, picker_area] = split.areas(meta_area);
            self.render_metadata(frame, props_area, theme);
            self.render_picker(frame, picker_area, theme);
        } else {
            self.render_metadata(frame, meta_area, theme);
        }

        self.render_variables(frame, var_area, theme);
        self.render_commands(frame, cmd_area, theme);
        self.render_status_bar(frame, status_area, theme);
```

Need to add imports for `Layout` and `Constraint` — they should already be imported in mod.rs. Also need `DetailFocus` — it's already in scope via `use super::{DetailFocus, DetailScreenState}` or similar. Let me verify what mod.rs imports look like.

Look at mod.rs imports:

```rust
use crate::models::{CommandSet, ExecMode, Group, ShellType};
use crate::ui::theme::Theme;
use crate::ui::widget::{InlineEdit, ScrollableList, TextInput};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};
```

`Constraint` and `Layout` are already imported. `DetailFocus` is in the same file. Good.

- [ ] **Step 2: Update render_picker to use bordered_block_info_zone**

In `render_picker`, replace the manual `bordered_block` + `centered_rect` logic with `bordered_block_info_zone`. The picker now gets its own full Rect, so no centering is needed — just use the full area:

```rust
    fn render_picker(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::layout::Alignment;

        let (names, selected_idx, title): (Vec<String>, Option<usize>, &str) = match self.focus {
            // ... same match arms as before ...
            _ => return,
        };

        let max_items: usize = 5;
        let total = names.len();
        let sel = selected_idx.unwrap_or(0);
        let total_pages = (total + max_items - 1) / max_items;
        let current_page = sel / max_items;
        let start = current_page * max_items;
        let end = (start + max_items).min(total);

        let inner = crate::ui::render::bordered_block_info_zone(frame, area, theme, title);

        let has_footer = total > max_items;
        let content_lines = (end - start).max(1);
        let footer_lines = if has_footer { 1 } else { 0 };
        let lines_layout = Layout::vertical([
            Constraint::Length(content_lines as u16),
            Constraint::Length(footer_lines),
        ]);
        let [list_area, footer_area] = lines_layout.areas(inner);

        let list_items: Vec<ListItem<'_>> = names[start..end].iter().enumerate().map(|(i, name)| {
            let is_current = start + i == sel;
            let style = if is_current {
                Style::default().fg(theme.accent_primary)
            } else {
                theme.normal_style()
            };
            styled_list_item(format!(" {}", name), style, list_area.width)
        }).collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(sel - start));
        frame.render_stateful_widget(
            List::new(list_items).highlight_style(
                Style::default().bg(theme.surface_border),
            ),
            list_area,
            &mut list_state,
        );

        if has_footer {
            let page_text = format!(" ◀ {}/{} ▶ ", current_page + 1, total_pages);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    page_text,
                    Style::default()
                        .fg(theme.text_disabled)
                        .add_modifier(Modifier::DIM),
                )))
                .alignment(Alignment::Center),
                footer_area,
            );
        }
    }
```

The `bordered_block_info_zone` already renders the block + borders and returns `inner`. No `Clear` needed since the picker now has its own dedicated Rect.

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 5: Run clippy**

Run: `cargo clippy`
Expected: No new warnings

- [ ] **Step 6: Commit**

```bash
git add src/ui/detail_screen/mod.rs src/ui/detail_screen/render.rs
git commit -m "feat: split Properties/Picker into side-by-side sibling blocks

meta_area is horizontally split when Option is focused:
Properties (left 50%) + Picker (right 50%) as sibling blocks.
Picker gets full vertical extent (7 inner rows), solving the
height clamp that limited display to 1 item. Uses
bordered_block_info_zone for consistent dialog styling.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
