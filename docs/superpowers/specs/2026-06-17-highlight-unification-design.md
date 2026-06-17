# Highlight Style Unification — Design Spec

**Date:** 2026-06-17
**Status:** Approved
**Scope:** Unify all selection and editing highlights to two consistent styles

## Problem

The app currently uses 4 different highlight styles with no semantic consistency:

| Style | Where |
|-------|-------|
| Opaque blue bg + white text + BOLD | Groups list, inline edit fields |
| Opaque green bg + white text + BOLD | Sets list, Variables, Commands |
| Blue text, no bg | Name/WorkDir focused, Options ◄► focused |
| Gray bg + blue text | Picker items |

Edition modes (Name, WorkDir, Variables, Commands, Group rename) all use
opaque blue — visually indistinguishable from list selection.

## Solution

Two semantic highlight styles, exactly one meaning each:

1. **Selected** — translucent blue bg + white text + BOLD
2. **Editing** — translucent green bg + white text + BOLD

Picker keeps its current style (gray bg + blue text) — it is intentionally
different, functioning as an informational "you are here" indicator rather
than a primary selection highlight.

## Theme Changes

### Remove

```rust
pub selection_bg_primary: Color,   // opaque blue → replaced by selection_bg
pub selection_bg_secondary: Color, // opaque green → replaced by selection_bg
```

### Add

```rust
/// Translucent feel via low-saturation blue. Used for selected items
/// across all lists and Properties fields.
pub selection_bg: Color,   // ~Rgb(60, 70, 110)

/// Translucent feel via low-saturation green. Used for inline editing
/// (Name, WorkDir, Variables, Commands, Group rename).
pub editing_bg: Color,     // ~Rgb(50, 80, 60)
```

### Keep

```rust
pub surface_border: Color, // still used by Picker highlight + block borders
```

### Updated `selected_style`

```rust
pub fn selected_style(&self) -> Style {
    Style::default()
        .fg(self.text_on_selected)
        .bg(self.selection_bg)
        .add_modifier(Modifier::BOLD)
}
```

No longer takes a `bg` parameter — there's only one selection color.

### New `editing_style`

```rust
pub fn editing_style(&self) -> Style {
    Style::default()
        .fg(self.text_on_selected)
        .bg(self.editing_bg)
        .add_modifier(Modifier::BOLD)
}
```

## Call Site Changes

| Location | Current | New |
|----------|---------|-----|
| `main_screen/render.rs` Groups | `selected_style(selection_bg_primary)` | `selected_style()` |
| `main_screen/render.rs` Sets | `selected_style(selection_bg_secondary)` | `selected_style()` |
| `main_screen/render.rs` Group rename | (uses selected style) | `editing_style()` |
| `detail_screen/render.rs` Name focused | `fg(accent_primary)` | `selected_style()` |
| `detail_screen/render.rs` Name editing | (via `render_editable_field`) | `editing_style()` |
| `detail_screen/render.rs` WorkDir focused | `fg(accent_primary)` | `selected_style()` |
| `detail_screen/render.rs` WorkDir editing | (via `render_editable_field`) | `editing_style()` |
| `detail_screen/render.rs` Group ◄► | `fg(accent_primary)` | `selected_style()` |
| `detail_screen/render.rs` Shell ◄► | `fg(accent_primary)` | `selected_style()` |
| `detail_screen/render.rs` Mode ◄► | `fg(accent_primary)` | `selected_style()` |
| `detail_screen/render.rs` Variables/Commands | `list_item_style(..., selected)` | `selected_style()` |
| `detail_screen/render.rs` Inline edit | `list_item_style(..., editing)` | `editing_style()` |
| `detail_screen/render.rs` Picker | `bg(surface_border)` | **unchanged** |
| `variable_screen.rs` | `selected_style(selection_bg_primary)` | `selected_style()` |
| `render.rs` | `selected_style(selection_bg_secondary)` | `selected_style()` |

## `render_editable_field` changes

The helper currently takes `focused: bool, editing: bool` and computes the
style. Updated to use the two Theme methods:

```rust
let style = if editing {
    theme.editing_style()
} else if focused {
    theme.selected_style()
} else {
    theme.normal_style()
};
```

## Color Values (for `default_dark`)

```rust
selection_bg: Color::Rgb(60, 70, 110),   // muted indigo — visible on dark bg
editing_bg:   Color::Rgb(50, 85, 65),    // muted green — visibly distinct from blue
```

(Exact values to be refined via visual testing.)

## Picker — intentionally unchanged

Picker uses `bg(surface_border)` + `fg(accent_primary)` (blue text on gray).
This is a "context indicator" style — shows you which page of names you're
looking at, not a primary selection. It stays distinct from the list
selection style to avoid confusion.

## Files Affected

| File | Change |
|------|--------|
| `src/ui/theme.rs` | Replace `selection_bg_primary/secondary` with `selection_bg + editing_bg`, update methods |
| `src/ui/render.rs` | `list_item_style` use new theme methods, remove bg param |
| `src/ui/main_screen/render.rs` | Groups/Sets/rename use new methods |
| `src/ui/detail_screen/render.rs` | Properties Options, render_editable_field, lists |
| `src/ui/variable_screen.rs` | Use `selected_style()` |

Estimated: ~30 lines changed across 5 files. No handler or data model changes.
