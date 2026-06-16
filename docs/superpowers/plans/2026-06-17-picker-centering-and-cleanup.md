# Picker: Remove Paginator + Center Items — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove useless page indicator and center picker items horizontally using unicode-width.

**Architecture:** Footer row and page indicator are deleted; metadata height reverts to 9. Each picker item is padded with computed left-whitespace so the widest item is centered within the picker inner width. `fill_row` (already called by `styled_list_item`) handles right-padding automatically.

**Tech Stack:** Rust, unicode-width (existing dependency)

---

### Task 1: Delete footer, revert metadata height to 9

**Files:**
- Modify: `src/ui/detail_screen/render.rs`
- Modify: `src/ui/detail_screen/mod.rs`

- [ ] **Step 1: Remove footer rendering and vertical layout split**

In `render_picker`, replace the footer-aware layout with a single list area. Current (lines after `items` build):

```rust
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
```

Replace with:

```rust
        let mut list_state = ratatui::widgets::ListState::default();
        if total > 0 {
            list_state.select(sel_visual);
        }
        frame.render_stateful_widget(
            List::new(items).highlight_style(
                Style::default().bg(theme.surface_border),
            ),
            inner,
            &mut list_state,
        );
```

Also remove the `use ratatui::layout::Alignment;` import line at the top of the function — no longer needed.

- [ ] **Step 2: Revert metadata height**

In `src/ui/detail_screen/mod.rs`:

```rust
            Constraint::Length(9), // Properties block + picker
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/detail_screen/render.rs src/ui/detail_screen/mod.rs
git commit -m "refactor: remove useless page indicator, revert metadata height

Page indicator ◀ X/Y ▶ made no sense with sliding-window layout.
Picker now renders 7 items directly into inner area. Metadata
height back to Length(9). Removed Alignment import.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Center picker items horizontally

**Files:**
- Modify: `src/ui/detail_screen/render.rs`

- [ ] **Step 1: Add unicode-width padding to each item**

In `render_picker`, replace the loop where `items` are built. Currently each item is built as:

```rust
items.push(styled_list_item(format!(" {}", names[i]), style, inner.width));
```

Replace all three `items.push(styled_list_item(...))` calls with a helper that computes centered padding. Add a helper closure before the loop:

```rust
        let center_label = |name: &str, style: Style, width: u16| -> ListItem<'static> {
            let raw = format!(" {}", name);
            let raw_w = unicode_width::UnicodeWidthStr::width(raw.as_str()) as u16;
            let left_pad = (width.saturating_sub(raw_w)) / 2;
            let label = format!("{}{}", " ".repeat(left_pad as usize), raw);
            styled_list_item(label, style, width)
        };
```

Then replace the three `styled_list_item(format!(" {}", names[i]), ...)` calls with `center_label(&names[i], ...)`.

The empty-row case (out-of-bounds idx) stays blank — `styled_list_item(String::new(), ...)` is already invisible.

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
git commit -m "feat: center picker items horizontally via unicode-width padding

Each item computes its unicode display width, then pads left
space so the element is centered within the picker column.
fill_row (called by styled_list_item) handles right padding.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
