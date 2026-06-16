# Picker Layout Polish — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pin selected item at row 3 of a 7-row picker, with dim gray peek rows top and bottom, and right-aligned footer.

**Architecture:** `render_picker` body is rewritten to build 7 `Option<ListItem>` rows from offsets -3..=3 around `sel`. Valid indices render as styled items (row 0/6 = dim, row 3 = accent_primary highlight, else = normal). `ListState::select` tracks the visual position of `sel`. Footer is right-aligned. Metadata area height bumps from 9 to 10 to fit 7 items + footer + 2 borders.

**Tech Stack:** Rust, Ratatui (no new dependencies)

---

### Task 1: Bump metadata area height

**Files:**
- Modify: `src/ui/detail_screen/mod.rs`

- [ ] **Step 1: Increase metadata block height**

Line 74: `Constraint::Length(9)` → `Constraint::Length(10)`:

```rust
        let layout = Layout::vertical([
            Constraint::Length(10), // Properties block + picker
            Constraint::Min(3),    // variables
            Constraint::Min(3),    // commands
            Constraint::Length(2), // status bar (separator + content)
        ]);
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add src/ui/detail_screen/mod.rs
git commit -m "fix: bump metadata area height from Length(9) to Length(10)

Picker needs 8 inner rows (7 items + footer) after borders.
Variables/Commands Min(3) still guaranteed by terminal minimum.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Rewrite render_picker with 7-row layout, dim peek rows, right-aligned footer

**Files:**
- Modify: `src/ui/detail_screen/render.rs`

- [ ] **Step 1: Replace render_picker body**

Replace the entire function body (lines 198-300):

```rust
    pub(crate) fn render_picker(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::layout::Alignment;

        let (names, selected_idx, title): (Vec<String>, Option<usize>, &str) = match self.focus {
            DetailFocus::Group => {
                let idx = self.groups.iter().position(|g| g.id == self.set.group_id);
                let names = self.groups.iter().map(|g| g.name.clone()).collect();
                (names, idx, " Groups ")
            }
            DetailFocus::Shell => {
                let variants = ShellType::builtin_variants();
                let saved_custom = match &self.set.shell {
                    ShellType::Custom(p) => Some(p.clone()),
                    _ => None,
                };
                let mut names = Vec::new();
                let mut selected_idx = None;
                for (i, v) in variants.iter().enumerate() {
                    let selected = std::mem::discriminant(&self.set.shell)
                        == std::mem::discriminant(v);
                    if selected { selected_idx = Some(i); }
                    names.push(match v {
                        ShellType::SystemDefault => "System Default".to_string(),
                        ShellType::Custom(_) => unreachable!(),
                        _ => v.label(),
                    });
                }
                if let Some(ref path) = saved_custom {
                    if matches!(&self.set.shell, ShellType::Custom(_)) {
                        selected_idx = Some(names.len());
                    }
                    names.push(format!("Custom: {}", path));
                } else {
                    names.push("Custom".to_string());
                }
                (names, selected_idx, " Shells ")
            }
            DetailFocus::ExecMode => {
                let modes = ["Stop on Error", "Continue on Error"];
                let idx = if self.set.exec_mode == ExecMode::StopOnError {
                    Some(0)
                } else {
                    Some(1)
                };
                let names = modes.iter().map(|s| s.to_string()).collect();
                (names, idx, " Exec Mode ")
            }
            _ => return,
        };

        let total = names.len();
        let sel = selected_idx.unwrap_or(0);
        let inner = crate::ui::render::bordered_block_info_zone(frame, area, theme, title);

        // 7 rows per page: fixed layout, row 3 is always sel
        const VISIBLE: usize = 7;
        const SEL_ROW: isize = 3; // 0-indexed position 2 in the visible window
        let sel_isize = sel as isize;
        let total_isize = total as isize;

        // Build 7 rows: offsets -3..=3 around sel
        let mut items: Vec<ListItem<'_>> = Vec::new();
        let mut sel_visual = None;
        for visual_row in 0..VISIBLE {
            let offset = visual_row as isize - SEL_ROW;
            let idx = sel_isize + offset;
            let in_bounds = idx >= 0 && idx < total_isize;
            if in_bounds {
                let i = idx as usize;
                let is_selected = offset == 0;
                let is_peek = offset == -SEL_ROW || offset == SEL_ROW;
                let style = if is_selected {
                    Style::default().fg(theme.accent_primary)
                } else if is_peek {
                    Style::default()
                        .fg(theme.text_disabled)
                        .add_modifier(Modifier::DIM)
                } else {
                    theme.normal_style()
                };
                if is_selected {
                    sel_visual = Some(items.len());
                }
                items.push(styled_list_item(format!(" {}", names[i]), style, inner.width));
            } else {
                // Empty row — use a blank ListItem so layout stays 7 rows
                items.push(styled_list_item(
                    String::new(), theme.normal_style(), inner.width,
                ));
                if offset == 0 {
                    sel_visual = Some(items.len() - 1);
                }
            }
        }

        let lines_layout = Layout::vertical([
            Constraint::Length(VISIBLE as u16),
            Constraint::Length(1), // footer
        ]);
        let [list_area, footer_area] = lines_layout.areas(inner);

        let mut list_state = ratatui::widgets::ListState::default();
        if total > 0 {
            list_state.select(sel_visual);
        }
        frame.render_stateful_widget(
            List::new(items).highlight_style(
                Style::default().bg(theme.surface_border),
            ),
            list_area,
            &mut list_state,
        );

        // Footer — right-aligned
        let max_items: usize = 5;
        let total_pages = (total.saturating_sub(1) / max_items) + 1;
        let current_page = sel / max_items + 1;
        let page_text = format!(" ◀ {}/{} ▶ ", current_page, total_pages);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                page_text,
                Style::default()
                    .fg(theme.text_disabled)
                    .add_modifier(Modifier::DIM),
            )))
            .alignment(Alignment::Right),
            footer_area,
        );
    }
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 4: Run clippy**

Run: `cargo clippy`
Expected: No new warnings

- [ ] **Step 5: Commit**

```bash
git add src/ui/detail_screen/render.rs
git commit -m "feat: pin selected item at row 3 with dim peek rows

Picker uses 7-row fixed layout with sel at row 3 (0-idx 2).
Rows 0 and 6 show dim gray peek items for edge-of-context.
Footer ◀ X/Y ▶ right-aligned. Empty rows at list boundaries.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
