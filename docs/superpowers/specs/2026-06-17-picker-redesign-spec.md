# Picker Redesign — Design Spec

**Date:** 2026-06-17
**Status:** Approved
**Scope:** Restructure Properties block with vertical divider, centered picker, and pagination

## Problem

Current picker rendering has several issues:

1. **No visual separation** — labels and picker share undivided space, no clear boundary
2. **Picker positioning is rigid** — Ratio-based split feels unpolished; no centering
3. **No pagination** — long option lists (shells with 6 items, groups with many items) overflow the available area
4. **No panel feel** — picker is a raw text list without visual containment

## Solution

Introduce a vertical divider `│` running through the Options section, separating
the label column from the picker column. The picker is rendered as a bordered
panel (`bordered_block`) centered within the right column. Pagination limits to
5 items per page with an auto-scroll that follows the selected value, plus a
`◀ X/Y ▶` page indicator in the footer.

## Full Layout

```
┌ Properties ───────────────────────────────────────────┐
│  Name: My Set                                          │  ← row 0: inline edit, fill_row
│  WorkDir: /home/user/project                           │  ← row 1: inline edit, fill_row
│  ── Options ─────────────────────────────────────────  │  ← row 2: full width separator
│  ◄ Group: Deploy ►          │  ┌─────────────┐        │  ← row 3: label │ picker
│  ◄ Shell: bash ►            │  │  Group 1     │        │  ← row 4: label │ picker
│  ◄ Mode: Stop ►             │  │  Group 2 ←高亮│        │  ← row 5: label │ picker
│                             │  │  Group 3     │        │  ← …   :       │ picker
│                             │  │  ◀ 1/3 ▶    │        │  ←      :       │ picker
│                             │  └─────────────┘        │
└────────────────────────────────────────────────────────┘
```

## Requirements

### Layout

| # | Requirement | Detail |
|---|-------------|--------|
| L1 | 6 rows in Properties | Name, WorkDir, `── Options ──`, Group, Shell, Mode |
| L2 | `── Options ──` full width | Does not stop at the vertical divide |
| L3 | Vertical bar `│` spans from Group row to Properties bottom | Placed at ~2/3 width, using `surface_border` color |
| L4 | Labels rendered on the left of `│` | All 6 rows (Name/WorkDir/separator/Group/Shell/Mode) on the left |
| L5 | Picker column on the right of `│` | Allocated to the remaining horizontal space |
| L6 | Picker centered horizontally and vertically | Within the right column, have it centered |
| L7 | Picker wrapped in `bordered_block` | Title shows category name, `accent_info` border |

### Interaction

| # | Requirement | Detail |
|---|-------------|--------|
| I1 | Tab cycles 3 regions | Properties → Variables → Commands |
| I2 | ↑/↓ cycles within Properties | Name → WorkDir → Group → Shell → ExecMode |
| I3 | ←/→ cycles option values | Group/Shell/Mode (unchanged) |
| I4 | Tab into a region resets to first | Properties → Name, Variables → index 0, Commands → index 0 |

### Picker Pagination

| # | Requirement | Detail |
|---|-------------|--------|
| P1 | Max 5 items per page | `MAX_PICKER_ITEMS = 5` |
| P2 | Page auto-follows selected value | When ←/→ changes the selection, the page scrolls to show it |
| P3 | Footer page indicator | `◀ X/Y ▶` where X = current page, Y = total pages |
| P4 | Selected item highlight | `accent_primary` blue |
| P5 | No manual page navigation | No extra keybindings — page follows selection |
| P6 | Picker shows correct items per Option | Group: all groups; Shell: builtin variants + Custom; ExecMode: Stop, Continue |

### Visual

| # | Requirement | Detail |
|---|-------------|--------|
| V1 | Vertical divider `│` in `surface_border` color | Matches border tone |
| V2 | Picker border in `accent_info` | Blue-tinted dialog feel |
| V3 | Picker title shows current option | e.g. ` Groups `, ` Shells `, ` Exec Mode ` |
| V4 | Non-Option focus → no picker | Picker only renders when focus is Group/Shell/ExecMode |

## Implementation Notes

### Vertical divider rendering

The `│` is a 1-column-wide `Paragraph` rendered from the Group row to the
Properties block bottom (mode_row bottom). It is rendered every frame,
regardless of the focus.

```rust
let bar_x = area.x + (area.width * 2 / 3);
let bar_area = Rect::new(bar_x, group_row.y, 1, mode_row.y + mode_row.height - group_row.y);
frame.render_widget(
    Paragraph::new(Line::from(Span::styled(
        "│", Style::default().fg(theme.surface_border),
    ))),
    bar_area,
);
```

### Picker centering

Compute picker rect within the right column, then center it:

```rust
let picker_inner_w = right_col.width.min(20); // fixed width
let picker_inner_h = actual_items.min(MAX_PICKER_ITEMS) + 2 (borders) + 1 (page indicator);
let picker_area = centered_rect(right_col, picker_inner_w, picker_inner_h);
```

### Pagination logic

```rust
const MAX_PICKER_ITEMS: usize = 5;
let total_pages = total_items.div_ceil(MAX_PICKER_ITEMS);
let current_page = selected_idx / MAX_PICKER_ITEMS;

let start = current_page * MAX_PICKER_ITEMS;
let end = (start + MAX_PICKER_ITEMS).min(total_items);
let page_items: Vec<_> = items[start..end].to_vec();
```

### Properties block height

The 6 rows still fit within the allocated height. If the picker extends
beyond, it clips naturally (constrained by `Constraint::Min(3)` for the
variables/commands areas below).

## Tests

| Module | Test | Count |
|--------|------|-------|
| handler | ↑/↓ cycles through 5 Properties fields (existing) | already tested |
| handler | Tab 3-region cycle (existing) | already tested |

No new handler tests needed — rendering changes only. The pagination
logic is tested via visual inspection.

## Files Affected

| File | Change |
|------|--------|
| `src/ui/detail_screen/render.rs` | Vertical divider, picker centering, pagination, bordered picker |
| `src/ui/detail_screen/mod.rs` | No change needed |

Estimated: ~80 lines changed (refactor of existing picker/label code).
