# Picker as Side-by-Side Block — Design Spec

**Date:** 2026-06-17
**Status:** Approved
**Scope:** Move picker from inside Properties to an independent side-panel block

## Problem

Picker is rendered inside the Properties block, sharing vertical space with the
six property rows. The Options section only has 3 rows (Group + Shell + Mode),
which is too shallow for a bordered picker (borders consume 2, leaving 1 item).
The vertical divider `│` also fails because `Paragraph` does not repeat a single
character vertically.

## Solution

Delete the vertical divider. Split the Detail Screen's top area (metadata zone)
horizontally into two independent blocks:

- **Properties** (left, ~50% width) — Name, WorkDir, `── Options ──`, Group, Shell, Mode
- **Picker** (right, ~50% width) — only rendered when an Option is focused

Picker is no longer a child of Properties. It is a sibling block with its own
borders, title, and full vertical extent (same height as Properties, ~7+ rows
inner), solving the height clamp.

When no Option is focused (Name / WorkDir / Variables / Commands focus),
Properties expands to full width and the Picker block is hidden.

## Layout

### Option focused (Group/Shell/Mode)

```
┌ Edit: My Set ────────────────────────────────────────┐
│  ┌ Properties ───────┐  ┌ Groups ──────────────────┐ │
│  │  Name: My Set     │  │  Group 1                  │ │
│  │  WorkDir: /home   │  │  Group 2   ← 高亮         │ │
│  │  ── Options ─────  │  │  Group 3                  │ │
│  │  ◄ Group: Deploy ► │  │                           │ │
│  │  ◄ Shell: bash ►  │  │  ◀ 1/3 ▶                  │ │
│  │  ◄ Mode: Stop ►   │  │                           │ │
│  └───────────────────┘  └───────────────────────────┘ │
│  ┌ Variables (0) ───────────────────────────────────┐ │
│  ┌ Commands (0) ────────────────────────────────────┐ │
│  [status bar]                                         │
└───────────────────────────────────────────────────────┘
```

### Non-Option focused (Name / WorkDir / Variables / Commands)

```
┌ Edit: My Set ────────────────────────────────────────┐
│  ┌ Properties ──────────────────────────────────────┐ │
│  │  Name: My Set                                    │ │
│  │  WorkDir: /home                                  │ │
│  │  ── Options ────────────────────────────────────  │ │
│  │  Group: Deploy                                    │ │  ← 无箭头 (未聚焦)
│  │  Shell: bash                                      │ │
│  │  Mode: Stop                                       │ │
│  └──────────────────────────────────────────────────┘ │
│  ┌ Variables (0) ───────────────────────────────────┐ │
│  ┌ Commands (0) ────────────────────────────────────┐ │
│  [status bar]                                         │
└───────────────────────────────────────────────────────┘
```

## Render Dispatching

```rust
fn render_metadata(&self, frame, area, theme) {
    let props_focused = ...; // any property in focus
    let show_picker = matches!(self.focus, Group | Shell | ExecMode);

    if show_picker {
        let [props_area, picker_area] = horizontal_split(area, 1:2, 1:2);
        self.render_properties(frame, props_area, theme);
        self.render_picker(frame, picker_area, theme);
    } else {
        self.render_properties(frame, area, theme);
    }
}
```

Properties always gets rendered. Picker gets its own dedicated Rect with full
height. No layout hacks, no vertical divider, no height clamp.

## Picker Block

The picker is rendered as a standalone `bordered_block_zone`:

```rust
fn render_picker(&self, frame, area, theme) {
    let title = match self.focus {
        Group => " Groups ",
        Shell => " Shells ",
        ExecMode => " Exec Mode ",
        _ => return,
    };
    let inner = bordered_block_info_zone(frame, area, theme, title);

    // Pagination (same logic as before)
    // ...
    frame.render_stateful_widget(List::new(page_items), inner, &mut state);

    // Page indicator
    if has_footer {
        frame.render_widget(Paragraph::new(format!(" ◀ {}/{} ▶ ", page, total)), footer_area);
    }
}
```

Uses `bordered_block_info_zone` (blue-tinted accent_info border) consistent with
other overlay dialogs.

## Properties Block Changes

- Remove all picker-related code inside `render_metadata`
- Remove vertical divider rendering
- Restore `── Options ──` separator and label rows to full-width
- No change to Tab/↑/↓/←/→ interaction

## Height Budget

| Block | Allocation | Inner |
|-------|-----------|-------|
| Properties + Picker | `Length(9)` | 7 rows (unchanged) |
| Variables | `Min(3)` | ≥1 |
| Commands | `Min(3)` | ≥1 |
| Status bar | `Length(2)` | — |

Properties inner = 7 rows. Name(1) + WorkDir(1) + sep(1) + Group(1) + Shell(1) + Mode(1) = 6 rows, leaving 1 spare. Picker shares the same 7-row inner. With bordered_block (2 borders), usable = 5 rows. That fits `MAX_PICKER_ITEMS = 5` perfectly with no footer needed for ≤5 items. For >5 items, the footer takes 1 row, leaving 4 item rows.

If picker needs more space, increase `Length(9)` to `Length(10)`.

## Tab Navigation

Unchanged. Tab cycles Properties → Variables → Commands. ↑/↓ cycles within
Properties. ←/→ cycles option values. The picker is display-only — it follows
the active Option focus, no new navigation needed.

## Files Affected

| File | Change |
|------|--------|
| `src/ui/detail_screen/render.rs` | Extract picker from `render_metadata`, split area horizontally when Option focused |
| `src/ui/detail_screen/mod.rs` | No change needed |

## Tests

No new tests — pure rendering refactor. Existing handler, model, and integration
tests cover all interaction paths.
