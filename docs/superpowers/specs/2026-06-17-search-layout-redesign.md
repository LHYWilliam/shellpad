# Search Layout Redesign — Design Spec

**Date:** 2026-06-17
**Status:** Approved
**Scope:** Restructure search mode rendering: separate Search block + Results block

## Problem

Current search mode embeds the search input line inside the Sets panel:
`render_set_panel` detects `search_mode`, splits `inner` vertically into
`search_line + remaining`, and renders the input at the top. This has two
issues:

1. The input line has no **bordered block** around it — it's a bare `Paragraph`
   within the Sets block's inner area
2. The Search block **replaces** the Sets block title group name, losing context

## Solution

When search mode is active, split the right panel vertically into two
independent `bordered_block_zone` blocks:

- **Search** (top, 3 rows) — input field with borders and cursor
- **Results** (bottom, Min) — filtered sets rendered like the normal Sets block

Both blocks are standalone — Search has its own `bordered_block_zone` labeled
`" Search "`, and Results has `" Results "`.

When search mode is **not** active, the right panel renders the normal Sets
block (unchanged from current behavior).

## Full Layout

### Search mode

```
┌ Groups ────────┐ ┌ Search ──────────────────────────┐
│                 │ │  Search: deploy                  │  ← 3 rows (borders + 1 inner)
│                 │ └──────────────────────────────────┘
│                 │ ┌ Results ─────────────────────────┐
│                 │ │  🛑 Prod       [bash](2 cmd)      │  ← Min rows
│                 │ │  ⏩ Testing    [bash](1 cmd)      │
└─────────────────┘ └──────────────────────────────────┘
```

### Normal mode (no change)

```
┌ Groups ────────┐ ┌ <group name> ────────────────────┐
│                 │ │  🛑 Prod       [bash](2 cmd)     │
│                 │ │  ⏩ Testing    [bash](1 cmd)     │
└─────────────────┘ └──────────────────────────────────┘
```

## Implementation

### Main screen render changes

Remove the `search_line` inline rendering from `render_set_panel`. Instead, add
a new method `render_search_block` that draws the Search block with
`bordered_block_zone`.

Update the main render entry point: when `search_mode`, split the right panel
vertically:

```rust
if self.search_mode {
    let search_layout = Layout::vertical([
        Constraint::Length(3),  // Search block
        Constraint::Min(1),     // Results block
    ]);
    let [search_area, results_area] = search_layout.areas(right_area);
    self.render_search_block(frame, search_area, theme);
    self.render_set_panel(frame, results_area, data, &sets, theme);
} else {
    self.render_set_panel(frame, right_area, data, &sets, theme);
}
```

### `render_set_panel` cleanup

Remove all `search_mode` branches. The function always renders a standard Sets
block (title from group name, no inline search line). The `highlight_style` and
`is_selected` logic stays — selection highlighting works identically.

### `render_search_block` — new method

```rust
fn render_search_block(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
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

### Interaction

No change. `/` enters search mode, Esc exits, Enter confirms, ↑/↓ navigates
results. The search input handles text input via the existing handler.

## Render.tsc cleanup

The right panel is currently rendered inside the groups-to-sets horizontal
split. `render_set_panel` currently receives the right panel area. The only
change is that `render_set_panel` no longer has `search_mode` branches — it's
always a normal Sets block. The search-specific layout is handled by the
caller (`render` in `main_screen/render.rs`, or wherever the horizontal split
is done).

## Status bar

No change — `"[Enter] Confirm  [Esc] Cancel  [↑/↓] Nav — searching"` is still
appropriate and rendered by the same code path.

## Files Affected

| File | Change |
|------|--------|
| `src/ui/main_screen/render.rs` | Add `render_search_block`, remove search branches from `render_set_panel` |
| `src/ui/main_screen/mod.rs` (or caller) | Split right panel when `search_mode` |

No handler, data model, or interaction changes.
