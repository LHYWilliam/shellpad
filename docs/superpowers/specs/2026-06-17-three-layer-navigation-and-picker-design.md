# Three-layer Tab Navigation + Option Picker — Design Spec

**Date:** 2026-06-17
**Status:** Approved
**Scope:** Restructure Detail Screen navigation and add visual picker for Options

## Problem

Detail Screen has a flat 7-stop Tab cycle (Name → WorkDir → Group → Shell →
ExecMode → Variables → Commands). Moving between sections requires up to 6 Tab
presses. Options (Group/Shell/Mode) provide no visual feedback when cycling —
just text that changes color to blue.

## Solution

Introduce a **three-layer Tab navigation** where Tab cycles between regions
(Properties → Variables → Commands) rather than individual fields. ↑/↓ navigates
within each region. Options gain a ◄/► arrow decoration and an on-focus picker
panel showing all available choices.

## Tab Cycle

```
[Properties] ──Tab──→ [Variables] ──Tab──→ [Commands] ──Tab──→ [Properties]
```

| Action | Behavior |
|--------|----------|
| Tab from Properties | Commit active inline edit, focus Variables (first variable) |
| Tab from Variables | Focus Commands (first command) |
| Tab from Commands | Focus Properties (Name — first field) |
| BackTab | Reverse direction: Properties ← Commands ← Variables ← Properties |

No "last focused" state is stored. Entering a region always focuses the first
item.

## ↑/↓ Navigation

| Region | ↑/↓ Behavior |
|--------|-------------|
| **Properties** | Cycle: `Name → WorkDir → Group → Shell → ExecMode` (wrap). Commit inline edit before moving |
| **Variables** | Unchanged — navigate list (prev/next variable) |
| **Commands** | Unchanged — navigate list (prev/next command) |

In Properties, ↑/↓ wraps at boundaries (from Name ↑ goes to ExecMode, from
ExecMode ↓ goes to Name).

## ←/→ Navigation

| Focus | ←/→ Behavior |
|-------|-------------|
| Group | Cycle available groups (unchanged) |
| Shell | Cycle available shells (unchanged) |
| ExecMode | Toggle StopOnError/ContinueOnError (unchanged) |
| Name / WorkDir | No-op (unchanged) |
| Variables / Commands | No-op (unchanged) |

## Picker Panel

### Layout

When an Option (Group/Shell/Mode) is focused, the Options section is split
horizontally: label column (left) and picker column (right).

```
┌ Options ───────────────────────────────────────┐
│  ◄ Group: Deploy ►     │  Group 1               │
│  Shell: bash           │  Group 2  ← 选中高亮   │
│  Mode: Stop            │  Group 3               │
└─────────────────────────────────────────────────┘
```

- **Label column** — 3 label rows (Group/Shell/Mode)
- **Picker column** — vertically extends to fill the Options section height
- **Only active** when the corresponding Option is focused
- Unfocused Options show no picker
- The picker area is rendered as a `Clear` overlay over the empty right column

### Picker Content by Focus

| Focus | Picker Items |
|-------|-------------|
| Group | All visible group names (from `self.groups`) |
| Shell | All built-in shells (SystemDefault, Bash, Zsh, Fish, PowerShell, Custom) |
| ExecMode | Stop on Error, Continue on Error |

The currently selected value is highlighted with `accent_primary` color on the
picker, matching the focused row color.

### Arrow Decoration

All three Options (Group/Shell/Mode) display `◄` and `►` when focused:

```
  ◄ Group: Deploy ►
  ◄ Shell: bash ►
  ◄ Mode: Stop on Error ►
```

Unfocused: plain text (no arrows).

## Tab/↑/↓ Handler Changes

### Core principle

```
match key.code {
    KeyCode::Tab => { self.commit_edits(); self.next_region(); }
    KeyCode::BackTab => { self.commit_edits(); self.prev_region(); }
    KeyCode::Up => { ... region-specific up ... }
    KeyCode::Down => { ... region-specific down ... }
    // ←/→, Enter, Esc, a, d, Ctrl+S unchanged per focus
}
```

### New region enum (internal to handler)

```rust
enum DetailRegion { Properties, Variables, Commands }

fn current_region(&self) -> DetailRegion {
    match self.focus {
        DetailFocus::Name | DetailFocus::WorkDir | DetailFocus::Group
        | DetailFocus::Shell | DetailFocus::ExecMode => DetailRegion::Properties,
        DetailFocus::Variables => DetailRegion::Variables,
        DetailFocus::Commands => DetailRegion::Commands,
    }
}

fn next_region(&mut self) {
    match self.current_region() {
        DetailRegion::Properties => {
            self.focus = DetailFocus::Variables;
            self.variable_list.selected = 0; // first variable
        }
        DetailRegion::Variables => {
            self.focus = DetailFocus::Commands;
            self.command_list.selected = 0; // first command
        }
        DetailRegion::Commands => {
            self.focus = DetailFocus::Name;
        }
    }
}

fn prev_region(&mut self) {
    match self.current_region() {
        DetailRegion::Properties => {
            self.focus = DetailFocus::Commands;
            self.command_list.selected = 0;
        }
        DetailRegion::Variables => {
            self.focus = DetailFocus::Name;
        }
        DetailRegion::Commands => {
            self.focus = DetailFocus::Variables;
            self.variable_list.selected = 0;
        }
    }
}
```

### Properties ↑/↓

Cycle through 5 Property fields:

```rust
DetailFocus::Name => DetailFocus::WorkDir,
DetailFocus::WorkDir => DetailFocus::Group,
DetailFocus::Group => DetailFocus::Shell,
DetailFocus::Shell => DetailFocus::ExecMode,
DetailFocus::ExecMode => DetailFocus::Name,
```

Before changing focus, `commit_name_edit()` and `commit_workdir_edit()` are
called to flush any in-progress inline editing.

## Render Changes

### Layout

Options section (`render_metadata` after the separator and 3 option rows) gains
a horizontal split on the Options block area:

```rust
let options_layout = Layout::horizontal([
    Constraint::Ratio(1, 2), // label column
    Constraint::Ratio(1, 2), // picker column
]);
let [label_col, picker_col] = options_layout.areas(options_area);
```

Where `options_area` is the rect covering all 3 option rows plus any
extension space for the picker.

### Options label rendering with arrows

For each Option (Group/Shell/Mode):

```rust
let arrow_left = if is_focused { "◄ " } else { "" };
let arrow_right = if is_focused { " ►" } else { "" };
let label = format!("{}{}{}", arrow_left, value_text, arrow_right);
```

### Picker rendering

When an Option is focused, the picker column renders a vertical list of all
available values for that Option type. The currently selected value is
highlighted.

```rust
fn render_picker(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
    let items: Vec<ListItem> = match self.focus {
        DetailFocus::Group => {
            self.groups.iter().map(|g| {
                let selected = g.id == self.set.group_id;
                styled_list_item(g.name.clone(), picker_style(selected, theme), area.width)
            }).collect()
        }
        DetailFocus::Shell => {
            let mut variants = ShellType::builtin_variants();
            // + Custom variant
            ...
        }
        DetailFocus::ExecMode => {
            // Stop on Error, Continue on Error
        }
        _ => return, // no picker for non-Options
    };
    frame.render_widget(Clear, area);
    // render List widget with items
}
```

## Visual States Summary

| Element | Unfocused | Focused |
|---------|-----------|---------|
| Name label | Normal color | accent_primary color |
| WorkDir label | Normal / dim (None) | accent_primary color |
| Group/Shell/Mode label | Normal color | `◄ ... ►` accent_primary |
| Picker column | Empty (no render) | List of all values, selected highlighted |

## Variables/Commands Transition

When Tab enters Variables or Commands, the respective list's `selected` is set
to 0 (first item). This is a behavioral change from the current behavior where
Tab preserved the previously selected index.

## Tests

| Module | Test | Count |
|--------|------|-------|
| handler | Tab cycles Properties → Variables → Commands → Properties | 1 |
| handler | ↑/↓ cycles through 5 Properties fields | 1 |
| handler | Entering region resets to first item | 3 |
| handler | Picker shows correct items per Option type | 3 |

## Files Affected

| File | Change |
|------|--------|
| `src/ui/detail_screen/handler.rs` | Replace Tab cycle with 3-zone routing, add Properties ↑/↓ |
| `src/ui/detail_screen/render.rs` | Arrow decorators, horizontal split, picker rendering |
| `src/ui/detail_screen/mod.rs` | No structural changes (option rows already separated) |

Estimated: ~200 lines production code, ~40 lines tests.
