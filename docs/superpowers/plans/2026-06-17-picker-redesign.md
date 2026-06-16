# Picker Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add vertical divider `│` separating labels from a centered bordered picker with 5-item pagination and page indicator `◀ X/Y ▶`.

**Architecture:** Properties block gains a vertical divider at 2/3 width spanning the Options rows. Labels render left of the divider, picker renders right — centered using `centered_rect`. `render_picker` is rewritten with `MAX_PICKER_ITEMS = 5`, `bordered_block`, `ListState::select`, and a footer page indicator. No handler or data model changes.

**Tech Stack:** Rust, Ratatui (no new dependencies)

---

### Task 1: Add vertical divider and restructure label rendering

**Files:**
- Modify: `src/ui/detail_screen/render.rs`

- [ ] **Step 1: Replace Options section layout with divider-based approach**

Remove the `opts_area` / `Layout::horizontal` / `labels_area` / `picker_area` / `sep_left` / `*_left` rects (lines 134-159). Replace with:

```rust
        // Vertical divider — 2/3 width, from group_row down to mode_row bottom
        let bar_x = inner.x + inner.width * 2 / 3;
        let label_width = (bar_x.saturating_sub(inner.x)).max(20);
        let right_x = bar_x + 1;
        let right_width = inner.x + inner.width - right_x;
        let vec_down = mode_row.y + mode_row.height - group_row.y;

        // Vertical divider
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "│", Style::default().fg(theme.surface_border),
            ))),
            Rect::new(bar_x, group_row.y, 1, vec_down),
        );

        // Separator — full width
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" ── Options {} ",
                    "─".repeat(sep_row.width.saturating_sub(12) as usize)),
                Style::default().fg(theme.text_disabled).add_modifier(Modifier::DIM),
            ))),
            sep_row,
        );

        // Label rects — left of divider
        let group_col = Rect::new(inner.x, group_row.y, label_width, group_row.height);
        let shell_col = Rect::new(inner.x, shell_row.y, label_width, shell_row.height);
        let mode_col = Rect::new(inner.x, mode_row.y, label_width, mode_row.height);
```

- [ ] **Step 2: Update label rendering to use new col rects**

Replace `group_left` → `group_col`, `shell_left` → `shell_col`, `mode_left` → `mode_col` in the existing Group/Shell/Mode rendering blocks (lines 177-211). Only the rect variable names change:

```rust
        let group_label = if self.focus == DetailFocus::Group {
            format!(" ◄ Group: {} ►", group_name)
        } else {
            format!(" Group: {}", group_name)
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(group_label, group_style))),
            group_col,
        );

        // ... same for shell (shell_col), mode (mode_col) ...
```

- [ ] **Step 3: Update picker call to use right column rect**

Add picker area calculation and call (replaces the `if matches!(...)` block at lines 214-216):

```rust
        // Picker — right column, only when an Option is focused
        if matches!(self.focus, DetailFocus::Group | DetailFocus::Shell | DetailFocus::ExecMode) {
            let picker_col = Rect::new(right_x, group_row.y, right_width, vec_down);
            self.render_picker(frame, picker_col, theme);
        }
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 5: Run tests**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/ui/detail_screen/render.rs
git commit -m "refactor: add vertical divider to Properties Options section

Vertical bar │ at 2/3 width spans from group_row to mode_row
bottom. Separator ── Options ── is full-width. Labels render
left of divider, picker area isolated to right column.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Rewrite render_picker with bordered_block, centering, pagination

**Files:**
- Modify: `src/ui/detail_screen/render.rs`

- [ ] **Step 1: Rewrite render_picker**

Replace the entire `render_picker` method (lines 219-299) with the new version:

```rust
    const MAX_PICKER_ITEMS: usize = 5;

    fn render_picker(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use crate::ui::render::{bordered_block, centered_rect};
        use ratatui::alignment::Alignment;

        let (items, selected_idx, title): (Vec<String>, Option<usize>, &str) = match self.focus {
            DetailFocus::Group => {
                let idx = self.groups.iter().position(|g| g.id == self.set.group_id);
                let items = self.groups.iter().map(|g| g.name.clone()).collect();
                (items, idx, " Groups ")
            }
            DetailFocus::Shell => {
                let variants = ShellType::builtin_variants();
                let saved_custom = match &self.set.shell {
                    ShellType::Custom(p) => Some(p.clone()),
                    _ => None,
                };
                let mut items = Vec::new();
                let mut selected_idx = None;
                for (i, v) in variants.iter().enumerate() {
                    let selected = std::mem::discriminant(&self.set.shell)
                        == std::mem::discriminant(v);
                    if selected { selected_idx = Some(i); }
                    items.push(match v {
                        ShellType::SystemDefault => "System Default".to_string(),
                        ShellType::Custom(_) => unreachable!(),
                        _ => v.label(),
                    });
                }
                if let Some(ref path) = saved_custom {
                    if matches!(&self.set.shell, ShellType::Custom(_)) {
                        selected_idx = Some(items.len());
                    }
                    items.push(format!("Custom: {}", path));
                } else {
                    items.push("Custom".to_string());
                }
                (items, selected_idx, " Shells ")
            }
            DetailFocus::ExecMode => {
                let modes = ["Stop on Error", "Continue on Error"];
                let idx = if self.set.exec_mode == ExecMode::StopOnError {
                    Some(0)
                } else {
                    Some(1)
                };
                let items = modes.iter().map(|s| s.to_string()).collect();
                (items, idx, " Exec Mode ")
            }
            _ => return,
        };

        let total = items.len();
        let sel = selected_idx.unwrap_or(0);
        let total_pages = (total + MAX_PICKER_ITEMS - 1) / MAX_PICKER_ITEMS;
        let current_page = sel / MAX_PICKER_ITEMS;
        let start = current_page * MAX_PICKER_ITEMS;
        let end = (start + MAX_PICKER_ITEMS).min(total);

        let has_footer = total > MAX_PICKER_ITEMS;
        let content_lines = (end - start).max(1);
        let footer_lines = if has_footer { 1 } else { 0 };
        let picker_inner_h = content_lines as u16 + footer_lines + 2; // +2 borders

        let picker_w = area.width.min(24);
        let picker_h = picker_inner_h;
        let picker_rect = centered_rect(area, picker_w, picker_h);

        frame.render_widget(Clear, picker_rect);

        let block = bordered_block(theme, title, true);
        let inner = block.inner(picker_rect);
        frame.render_widget(&block, picker_rect);

        let lines_layout = Layout::vertical([
            Constraint::Length(content_lines as u16),
            Constraint::Length(footer_lines),
        ]);
        let [list_area, footer_area] = lines_layout.areas(inner);

        // Page items
        let list_items: Vec<ListItem<'_>> = items[start..end].iter().enumerate().map(|(i, name)| {
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
        frame.render_stateful_widget(List::new(list_items), list_area, &mut list_state);

        // Page indicator
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

- [ ] **Step 2: Remove unused imports**

The old import `use ratatui::widgets::{Clear, List, ListItem, Paragraph};` stays. Remove `use crate::models::{ExecMode, ShellType};` from line 1 if no longer needed at the top level (it's used in `render_picker`, so keep).

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
git add src/ui/detail_screen/render.rs
git commit -m "feat: centered bordered picker with 5-item pagination

Picker wrapped in bordered_block, centered in right column.
MAX_PICKER_ITEMS=5 with page indicator ◀ X/Y ▶.
Page auto-follows selected value via ←/→ cycling.
ListState::select highlights current item.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
