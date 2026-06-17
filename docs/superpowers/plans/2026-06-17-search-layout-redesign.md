# Search Layout Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split search mode rendering into a standalone Search block (bordered input) above a Results block, removing embedded search line from render_set_panel.

**Architecture:** `mod.rs` `render()` splits `right_area` vertically into Search(3) + Results(Min) when `search_mode`. New `render_search_block` method draws the bordered input. `render_set_panel` is stripped of all `search_mode` branches — it's always a plain Sets block.

**Tech Stack:** Rust, Ratatui (no new dependencies)

---

### Task 1: Add render_search_block + split right_area in search mode

**Files:**
- Modify: `src/ui/main_screen/render.rs`
- Modify: `src/ui/main_screen/mod.rs`

- [ ] **Step 1: Add render_search_block method**

Add after `render_group_panel` (before `render_set_panel` around line 97) in `render.rs`:

```rust
    pub(crate) fn render_search_block(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
    ) {
        let inner = bordered_block_zone(frame, area, theme, " Search ", false);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" Search: {} ", self.search_input.content),
                Style::default().fg(theme.text_primary),
            ))),
            inner,
        );
        let prefix_width = unicode_width::UnicodeWidthStr::width(" Search: ");
        set_cursor_after_prefix(
            frame,
            &self.search_input.content,
            self.search_input.cursor,
            prefix_width as u16,
            inner,
        );
    }
```

- [ ] **Step 2: Split right_area when search_mode in mod.rs**

In `src/ui/main_screen/mod.rs`, replace lines 100-102:

```rust
        // Right panel: command sets
        let sets = self.visible_sets(data);
        self.render_set_panel(frame, right_area, data, &sets, theme);
```

With:

```rust
        // Right panel: Search + Results (search mode) or command sets (normal)
        if self.search_mode {
            let search_layout = Layout::vertical([
                Constraint::Length(3),  // Search block
                Constraint::Min(1),     // Results block
            ]);
            let [search_area, results_area] = search_layout.areas(right_area);
            let right_vis = results_area.height.saturating_sub(2) as usize;
            self.set_list.update_offset(right_vis);
            self.render_search_block(frame, search_area, theme);
            let sets = self.visible_sets(data);
            self.render_set_panel(frame, results_area, data, &sets, theme);
        } else {
            let sets = self.visible_sets(data);
            self.render_set_panel(frame, right_area, data, &sets, theme);
        }
```

Also remove the original `let right_vis = right_area.height.saturating_sub(2) as usize;` line (line 93) and the original `let sets = self.visible_sets(data);` line (line 101) since they're now inside the branches.

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/main_screen/render.rs src/ui/main_screen/mod.rs
git commit -m "feat: add standalone Search block above Results in search mode

Search mode now renders two independent blocks: Search (3 rows,
bordered input) + Results (Min rows, filtered sets). New
render_search_block method draws the bordered input field.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Strip search_mode branches from render_set_panel

**Files:**
- Modify: `src/ui/main_screen/render.rs`

- [ ] **Step 1: Replace render_set_panel body**

Replace the entire body of `render_set_panel` (lines 97-252) with a cleaned-up version that has no `search_mode` branches:

```rust
    pub(crate) fn render_set_panel(
        &self,
        frame: &mut Frame,
        area: Rect,
        data: &AppData,
        sets: &[(usize, usize, &crate::models::CommandSet)],
        theme: &Theme,
    ) {
        let title = if self.search_mode {
            " Results ".to_string()
        } else {
            let name = self
                .selected_group_idx(data)
                .map(|gi| data.groups[gi].name.as_str())
                .unwrap_or("Commands");
            format!(" {} ", name)
        };

        let inner =
            bordered_block_zone(frame, area, theme, &title, self.active_panel == Panel::Sets);

        let (list_area, scrollbar_area) = list_scrollbar_areas(inner);

        let mut items: Vec<ListItem> = sets
            .iter()
            .enumerate()
            .map(|(i, &(gi, _, set))| {
                let shell_label = set.shell.label();
                let mode_label = match set.exec_mode {
                    crate::models::ExecMode::StopOnError => "🛑",
                    crate::models::ExecMode::ContinueOnError => "⏩",
                };
                let cmd_count = set.commands.len();
                let is_selected = i == self.set_list.selected && self.active_panel == Panel::Sets;
                let text_style = if is_selected {
                    theme.selected_style(theme.selection_bg_secondary)
                } else {
                    theme.normal_style()
                };

                let prefix = format!(" {}  ", mode_label);
                let suffix = format!("  [{}] ({} cmd)", shell_label, cmd_count);

                // Build name part with optional search highlighting
                let name_part: Vec<Span> =
                    if self.search_mode && !self.search_input.content.is_empty() && !is_selected {
                        let matches =
                            find_matches_case_insensitive(&set.name, &self.search_input.content);
                        if matches.is_empty() {
                            vec![Span::styled(set.name.clone(), text_style)]
                        } else {
                            let mut spans: Vec<Span> = Vec::new();
                            let mut last_end = 0usize;
                            for (match_start, match_end) in &matches {
                                if *match_start > last_end {
                                    spans.push(Span::styled(
                                        &set.name[last_end..*match_start],
                                        text_style,
                                    ));
                                }
                                spans.push(Span::styled(
                                    &set.name[*match_start..*match_end],
                                    Style::default()
                                        .fg(theme.accent_primary)
                                        .add_modifier(Modifier::BOLD),
                                ));
                                last_end = *match_end;
                            }
                            if last_end < set.name.len() {
                                spans.push(Span::styled(&set.name[last_end..], text_style));
                            }
                            spans
                        }
                    } else {
                        vec![Span::styled(set.name.clone(), text_style)]
                    };

                let mut parts = vec![Span::styled(prefix, text_style)];
                parts.extend(name_part);
                parts.push(Span::styled(suffix, text_style));

                // Right-aligned group name in search mode
                if self.search_mode {
                    let gname = data.groups.get(gi).map(|g| g.name.as_str()).unwrap_or("?");
                    let text_width: usize = parts
                        .iter()
                        .map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref()))
                        .sum();
                    let pad = list_area.width as usize;
                    let padding = pad.saturating_sub(text_width + gname.len() + 1);
                    if padding > 0 {
                        parts.push(Span::styled(" ".repeat(padding), text_style));
                    }
                    parts.push(Span::styled(gname, text_style));
                }

                let set_line = fill_row(Line::from(parts), text_style, list_area.width);
                ListItem::new(set_line)
            })
            .collect();

        if sets.is_empty() {
            items.push(empty_hint(theme, " (empty — press n to add a set) "));
        }

        let selected = if self.active_panel == Panel::Sets {
            self.set_list.selected_or_none(sets.len())
        } else {
            None
        };
        let mut list_state = ratatui::widgets::ListState::default().with_selected(selected);
        let list =
            List::new(items).highlight_style(theme.selected_style(theme.selection_bg_secondary));
        frame.render_stateful_widget(list, list_area, &mut list_state);

        render_scrollbar(
            frame,
            scrollbar_area,
            theme,
            sets.len(),
            selected.unwrap_or(0),
        );
    }
```

The key difference: the `title` is always derived from the group name (not `" Search "`), and the `inner` is always `list_scrollbar_areas(inner)` (no inline search line split).

- [ ] **Step 2: Remove unused import `Constraint` from render.rs**

The `Constraint` import is still used by `render_search_block`'s caller (mod.rs), not render.rs directly. Check if it's still needed in render.rs:

```bash
grep -c "Constraint" src/ui/main_screen/render.rs
```

If no `Constraint` usage remains, remove it from imports:

```rust
// remove: use ratatui::layout::{Constraint, Layout, Rect};
// keep:  use ratatui::layout::{Layout, Rect};
```

Actually — `Constraint` is not used in render.rs anymore. It was used by the search_line vertical split which is now in mod.rs. But the import line shows `Constraint` alongside `Layout` and `Rect`. `Layout` and `Rect` are still used throughout render.rs. Remove only `Constraint` from the import:

```rust
use ratatui::layout::{Layout, Rect};
```

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
git add src/ui/main_screen/render.rs
git commit -m "refactor: strip search_mode branches from render_set_panel

render_set_panel is now a plain Sets block — no inline search
input, no title override for search mode. Search layout is
handled by the caller (mod.rs) via right_area vertical split.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
