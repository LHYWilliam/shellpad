# Selection Background Fill — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the selection/active background color to the full row width by padding with styled trailing spaces.

**Architecture:** Add a `fill_row()` helper to `components.rs`; apply it at every selected/highlighted row render call-site across 3 screen files.

**Tech Stack:** ratatui 0.30.1, unicode-width 0.2.2

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src/ui/components.rs` | Modify | Add `fill_row()` helper function |
| `src/ui/main_screen.rs` | Modify | Apply to Groups items, Sets items, rename cursor |
| `src/ui/detail_screen.rs` | Modify | Apply to Name row, Variables items, Commands items |
| `src/ui/variable_screen.rs` | Modify | Apply to focused variable row |

---

### Task 1: Add `fill_row()` helper

**Files:**
- Modify: `src/ui/components.rs`

- [ ] **Add the helper function after `set_cursor_after_prefix`**

```rust
/// Pad a styled Line with trailing spaces up to `target_width` columns,
/// so that the background highlight extends to the full row width.
/// Uses `fill_style` for the padding spaces (typically the same style as the row).
pub fn fill_row(line: Line<'_>, fill_style: Style, target_width: u16) -> Line<'_> {
    let current: usize = line.spans.iter()
        .map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref()))
        .sum();
    let need = target_width.saturating_sub(current as u16) as usize;
    if need > 0 {
        let mut spans = line.spans;
        spans.push(Span::styled(" ".repeat(need), fill_style));
        Line::from(spans)
    } else {
        line
    }
}
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/components.rs
git commit -m "refactor: add fill_row helper for selection background extension"
```

---

### Task 2: Apply to main_screen.rs

**Files:**
- Modify: `src/ui/main_screen.rs`

Add `fill_row` to imports.

#### Groups panel item

Current (line ~180):
```rust
ListItem::new(Line::from(Span::styled(label, style)))
```

Replace with:
```rust
let line = fill_row(Line::from(Span::styled(label, style)), style, list_area.width);
ListItem::new(line)
```

#### Sets panel items

The sets panel builds `parts: Vec<Span>` (line ~340). After building `let mut parts = vec![...];` and before `ListItem::new(Line::from(parts))`:

```rust
let line = fill_row(Line::from(parts), text_style, list_area.width);
ListItem::new(line)
```

Note: when `is_selected`, `text_style` is `theme.selected_style(theme.selection_bg_secondary)` — that's the correct fill style because it has `.bg(blue)`. When not selected, `text_style` is `theme.normal_style()` which has no background — padding would be invisible (correct — no fill for non-selected rows).

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "feat: extend selection background to row width in main screen panels"
```

---

### Task 3: Apply to detail_screen.rs

**Files:**
- Modify: `src/ui/detail_screen.rs`

Add `fill_row` to imports.

#### Properties Name row

Current (line ~116):
```rust
let name_text = format!(" Name: {}", display_name);
frame.render_widget(
    Paragraph::new(Line::from(Span::styled(name_text, name_style))),
    name_row,
);
```

Replace with:
```rust
let name_text = format!(" Name: {}", display_name);
let name_line = fill_row(Line::from(Span::styled(name_text, name_style)), name_style, name_row.width);
frame.render_widget(
    Paragraph::new(name_line),
    name_row,
);
```

#### Variables items

Current (line ~230):
```rust
ListItem::new(Line::from(Span::styled(label, style)))
```

Replace with:
```rust
let line = fill_row(Line::from(Span::styled(label, style)), style, list_area.width);
ListItem::new(line)
```

Also for the preview row (insert mode preview, line ~244):
```rust
let preview = ListItem::new(Line::from(Span::styled(label, style)));
```

Replace with:
```rust
let p_line = fill_row(Line::from(Span::styled(label, style)), style, list_area.width);
let preview = ListItem::new(p_line);
```

#### Commands items (same pattern)

Current (line ~350):
```rust
ListItem::new(Line::from(Span::styled(label, style)))
```

Replace with:
```rust
let line = fill_row(Line::from(Span::styled(label, style)), style, list_area.width);
ListItem::new(line)
```

And the preview row:
```rust
let p_line = fill_row(Line::from(Span::styled(label, style)), style, list_area.width);
let preview = ListItem::new(p_line);
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "feat: extend selection background to row width in detail screen"
```

---

### Task 4: Apply to variable_screen.rs

**Files:**
- Modify: `src/ui/variable_screen.rs`

Add `fill_row` to imports.

#### Focused variable row

Current (line ~114):
```rust
let display = format!(" {} = {}", self.names[i], self.inputs[i].content);
frame.render_widget(
    Paragraph::new(Line::from(Span::styled(display, row_style))),
    row,
);
```

Replace with:
```rust
let display = format!(" {} = {}", self.names[i], self.inputs[i].content);
let filled_line = fill_row(Line::from(Span::styled(display, row_style)), row_style, row.width);
frame.render_widget(
    Paragraph::new(filled_line),
    row,
);
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/variable_screen.rs
git commit -m "feat: extend selection background to row width in variable dialog"
```

---

### Verification

```bash
cargo test
cargo clippy 2>&1 | grep '^error'
cargo build
```
