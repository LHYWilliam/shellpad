# Command & Item Reordering — Design Spec

**Date:** 2026-06-16
**Status:** Approved
**Scope:** Reorder Groups, Sets, Commands, and Variables via Ctrl+Up/Down

## Problem

Users cannot reorder items in any list across the app. Commands must be deleted
and re-added to change execution order. Groups, Sets, and Variables similarly
have no reordering mechanism. This is tedious for Command Sets with many
commands.

## Solution

Add unified `Ctrl+Up` / `Ctrl+Down` reordering across all four list types. A
single `AppAction::Reorder(ReorderKind, isize)` action handles all cases.
Direction is `-1` (up) or `+1` (down). Boundary checks prevent moving off-list.

## Data Flow

```
Screen handler: Ctrl+Up/Down in listed context
  → returns AppAction::Reorder(kind, direction)

App::handle_action(Reorder(kind, dir)):
  → bounds check → swap in vec → update selected index
  → renumber positions (Commands only) → auto_save → toast
```

Commands are special: after swap, `position` fields are renumbered to match
Vec index (same pattern as `DeleteCommand`).

## New Types

### `src/action.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReorderKind {
    Group(usize),           // group_index
    Set(usize, usize),      // (group_index, set_index)
    Variable(usize),        // index in variables vec
    Command(usize),         // index in commands vec
}

// Added to AppAction:
pub enum AppAction {
    // ...existing variants...
    Reorder(ReorderKind, isize),  // direction: -1 up, +1 down
}
```

## Trigger Matrix

| Shortcut | Screen | Focus/Context | Produces |
|----------|--------|---------------|----------|
| `Ctrl+Up` | Main | Groups panel | `Reorder(Group(gi), -1)` |
| `Ctrl+Down` | Main | Groups panel | `Reorder(Group(gi), +1)` |
| `Ctrl+Up` | Main | Sets panel | `Reorder(Set(gi, si), -1)` |
| `Ctrl+Down` | Main | Sets panel | `Reorder(Set(gi, si), +1)` |
| `Ctrl+Up` | Detail | Variables focus | `Reorder(Variable(idx), -1)` |
| `Ctrl+Down` | Detail | Variables focus | `Reorder(Variable(idx), +1)` |
| `Ctrl+Up` | Detail | Commands focus | `Reorder(Command(idx), -1)` |
| `Ctrl+Down` | Detail | Commands focus | `Reorder(Command(idx), +1)` |

## Handler Logic (unified)

All four arms share the same pattern:

1. Bounds check: `(idx as isize + dir) >= 0 && (idx as isize + dir) < len`
2. `vec.swap(idx, new_idx)`
3. Update `list.selected = new_idx`
4. For Commands: renumber `c.position = i` for all
5. `auto_save()` + toast

Boundary behavior:
- 0 items: no-op
- 1 item: no-op
- First item + Up: no-op
- Last item + Down: no-op
- Edit mode active: no-op (Ctrl+Up/Down is ignored during inline editing)

## Screen Handler Changes

### `src/ui/main_screen/handler.rs`

Two new match arms (in active panel context):

```rust
KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
    match self.active_panel {
        Panel::Groups if let Some(gi) = self.selected_group_idx(data) => {
            AppAction::Reorder(ReorderKind::Group(gi), -1)
        }
        Panel::Sets if let Some((gi, si)) = self.selected_set_idx(data) => {
            AppAction::Reorder(ReorderKind::Set(gi, si), -1)
        }
        _ => AppAction::None,
    }
}
KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
    // same pattern, direction +1
}
```

### `src/ui/detail_screen/handler.rs`

Two new match arms (in focus context):

```rust
KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
    match self.focus {
        DetailFocus::Variables if !self.set.variables.is_empty() => {
            AppAction::Reorder(ReorderKind::Variable(
                self.variable_list.selected.min(self.set.variables.len() - 1)), -1)
        }
        DetailFocus::Commands if !self.set.commands.is_empty() => {
            AppAction::Reorder(ReorderKind::Command(
                self.command_list.selected.min(self.set.commands.len() - 1)), -1)
        }
        _ => AppAction::None,
    }
}
// Same for Down, direction +1
```

Note: `Ctrl` modifier check via `key.modifiers.contains(KeyModifiers::CONTROL)`.
These arms are placed before the plain `Up`/`Down` arms so Ctrl variants match
first.

## Status Bar Updates

### Detail Screen — Commands focus

```
[a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Ctrl+↑/↓] Move  [Tab] Next  |  [Ctrl+S] Save
```

### Detail Screen — Variables focus (same addition)

### Main Screen (modify `render_status_bar` text or main_screen render)

The main screen already renders context-sensitive hints. No status bar block;
hints are integrated in the existing bottom hint area. Add move hints:

Groups panel: `[g] New  [R] Rename  [D] Delete  [Ctrl+↑/↓] Move`
Sets panel: `[n] New  [e] Edit  [d] Delete  [Ctrl+↑/↓] Move`

## Help Screen Update

Add under "Main Screen":
```
    Ctrl+Up/Down    Reorder group or set
```
Under "Detail Screen":
```
    Ctrl+Up/Down    Reorder variable or command
```

## Error Handling

No new error paths. Bounds checks prevent panics. If indices are stale (between
request and execution), bounds check no-ops the action — same pattern as delete
handlers.

## Files Affected

| File | Change |
|------|--------|
| `src/action.rs` | Add `ReorderKind` enum + `Reorder` variant |
| `src/app/handler.rs` | Handle `Reorder` action + 15 new tests |
| `src/ui/main_screen/handler.rs` | Ctrl+Up/Down → `Reorder` + 4 new tests |
| `src/ui/detail_screen/handler.rs` | Ctrl+Up/Down → `Reorder` + 4 new tests |
| `src/ui/detail_screen/render.rs` | Status bar text update |
| `src/ui/help_screen.rs` | Add move shortcuts |

Estimated: ~120 lines production code, ~70 lines tests.
