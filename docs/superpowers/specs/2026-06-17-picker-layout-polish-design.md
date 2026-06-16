# Picker Layout Polish — Design Spec

**Date:** 2026-06-17
**Status:** Approved
**Scope:** Fix picker item layout: centered selection with dim peek rows, right-aligned footer

## Problem

Current picker renders items from top to bottom with no specific positioning of
the selected item. The page indicator is centered. This is functional but lacks
the polish of a hover-based UI pattern where the selection is visually pinned
and contextual neighbors are subtly shown.

## Solution

Pin the selected item at **row 3** (the 4th row of 7, 0-indexed position 2)
within the picker. Show up to 3 items above and below the selection, with the
outermost rows (row 0 and row 6) rendered in a dim gray to indicate they are
edge-of-context peeks. Empty slots (at list boundaries) are left blank. The
page indicator `◀ X/Y ▶` is right-aligned at the bottom-right corner.

## Layout Model

7 vertical rows within the picker, each 1 line tall:

```
Row 0: sel-3  →  dim gray  (if 0 ≤ idx < total, else empty)
Row 1: sel-2  →  normal    (if 0 ≤ idx < total, else empty)
Row 2: sel-1  →  normal    (if 0 ≤ idx < total, else empty)
Row 3: sel    →  accent_primary + highlight (always present when total > 0)
Row 4: sel+1  →  normal    (if 0 ≤ idx < total, else empty)
Row 5: sel+2  →  normal    (if 0 ≤ idx < total, else empty)
Row 6: sel+3  →  dim gray  (if 0 ≤ idx < total, else empty)
```

Footer: `◀ X/Y ▶`, right-aligned.

## Color Map

| Row | Style | Purpose |
|-----|-------|---------|
| 0, 6 | `text_disabled` + `DIM` modifier | Peek rows — show context at edges |
| 1, 2, 4, 5 | `normal_style()` (theme default) | Normal sibling items |
| 3 | `accent_primary` fg + `surface_border` bg (`List::highlight_style`) | Selected item |

## Examples

All assume `total = 7` items (A..G).

### sel = 3 (middle)

```
A  dim gray      ← sel-3 exists
B  normal        ← sel-2 exists
C  normal        ← sel-1 exists
D  blue highlight ← sel (position 3)
E  normal
F  normal
G  dim gray
            ◀ 1/1 ▶
```

### sel = 0 (head)

```
[empty]          ← sel-3 does not exist
[empty]          ← sel-2 does not exist
[empty]          ← sel-1 does not exist
A  blue highlight ← sel
B  normal
C  normal
D  dim gray
            ◀ 1/1 ▶
```

### sel = 6 (tail)

```
D  dim gray
E  normal
F  normal
G  blue highlight ← sel
[empty]          ← sel+1 does not exist
[empty]
[empty]
            ◀ 1/1 ▶
```

## Page Indicator

- Text: `◀ X/Y ▶`
- `X = current page = sel / max_items + 1` (1-based, max_items = 5)
- `Y = total pages = (total + max_items - 1) / max_items`
- Alignment: right (`Alignment::Right`)
- Style: `text_disabled` + `DIM`
- Footer is always rendered (even for ≤5 items — this provides a consistent 8th row)

## Implementation

Modify `render_picker` in `src/ui/detail_screen/render.rs`:

```rust
const MAX_VISIBLE: usize = 7; // 5 main + 2 peek rows
const SEL_POS: usize = 3;     // 0-indexed position 2 = row 3

// Build a Vec of 7 Option<ListItem>, one per row
let mut rows: Vec<Option<ListItem<'_>>> = vec![None; MAX_VISIBLE];

for offset in -3isize..=3isize {
    let idx = sel.wrapping_add(offset as usize);
    // handle negative offset via saturating logic
    ...
    if idx < total {
        let is_selected = offset == 0;
        let is_peek = offset == -3 || offset == 3;
        let label = format!(" {}", names[idx]);
        let style = if is_selected {
            Style::default().fg(theme.accent_primary)
        } else if is_peek {
            Style::default().fg(theme.text_disabled).add_modifier(Modifier::DIM)
        } else {
            theme.normal_style()
        };
        rows[(offset + 3) as usize] = Some(styled_list_item(label, style, area.width));
    }
}

let flat: Vec<ListItem<'_>> = rows.into_iter().filter_map(|r| r).collect();
let selected_row = 3 - (sel - (sel.saturating_sub(3))).min(...);
// ListState::select(Some(selected_row))
```

## Height Budget

| Component | Rows |
|-----------|------|
| Item rows (7) | 7 |
| Footer | 1 |
| Bordered block borders | 2 |
| **Total needed (inner)** | **8** |
| **Allocated (Length)** | **10** (inner = 8) |

Requires increasing `Length(9)` to `Length(10)` in mod.rs.

## Files Affected

| File | Change |
|------|--------|
| `src/ui/detail_screen/render.rs` | Rewrite `render_picker`: 7-row fixed layout, dim peek rows, right-aligned footer |
| `src/ui/detail_screen/mod.rs` | `Length(9)` → `Length(10)` for metadata area |

## Tests

No new tests — rendering-only change. Existing 208 tests cover all interaction paths.
