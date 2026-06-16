# Delete Confirmation Dialog — Design Spec

**Date:** 2026-06-16
**Status:** Approved
**Scope:** All destructive delete operations across the app

## Problem

Pressing `d` or `D` immediately deletes data (Set, Group, Variable, Command) with
no confirmation. Data is auto-saved to JSON immediately after deletion, making
the loss irreversible. There is no undo mechanism.

## Solution

Add an `AppMode::ConfirmDelete` overlay (following the existing Help screen
pattern) that intercepts any delete request, shows a confirmation prompt, and
only executes the deletion on explicit `y` confirmation.

All four delete operations are covered: Set, Group, Variable, Command.

## Data Flow

```
Screen handle_key: user presses d/D
  → returns AppAction::RequestDelete(DeleteKind::Set { ... })

App::handle_action(RequestDelete):
  → self.mode = AppMode::ConfirmDelete { kind, prev: Box::new(self.mode.clone()) }

Next tick — App::handle_key matches ConfirmDelete:
  → y → construct AppAction::DeleteXxx → handle_action → restore prev mode
  → n / Esc → restore prev mode, discard delete

App::render matches ConfirmDelete:
  → render underlying screen (content beneath overlay)
  → render confirmation overlay (Clear + centered_rect + bordered_block_info)
```

## New / Modified Types

### `src/mode.rs`

Add `ConfirmDelete` variant to `AppMode`:

```rust
pub enum AppMode {
    Main,
    Detail,
    Execution,
    Help,
    ConfirmDelete { kind: DeleteKind, prev: Box<AppMode> },
}
```

`Box<AppMode>` stores the mode to restore after confirm/cancel. Always present
for this variant (never None), so no `Option` needed.

**Note:** Adding this variant means `AppMode` loses `Copy`. This requires
changing `match self.mode` to `match &self.mode` in `handle_key` (all existing
arms work unchanged with a reference match). The `prev` is stored inside the
variant — the `App.prev_mode` field remains used exclusively for Help overlay
restoration, avoiding any conflict.

### `src/action.rs`

Add `DeleteKind` enum and `RequestDelete` variant:

```rust
#[derive(Debug, Clone)]
pub enum DeleteKind {
    Set {
        group_index: usize,
        set_index: usize,
        set_name: String,
    },
    Group {
        group_index: usize,
        group_name: String,
        set_count: usize,
    },
    Variable {
        var_name: String,
    },
    Command {
        cmd_index: usize,
        cmd_preview: String,
    },
}

// Add to AppAction:
pub enum AppAction {
    // ... existing variants unchanged ...
    RequestDelete(DeleteKind),
}
```

Existing `DeleteSet`, `DeleteGroup`, `DeleteVariable`, `DeleteCommand` variants
are kept — they are the "actually execute deletion" actions, invoked internally
after confirmation.

### New file: `src/ui/confirm_dialog.rs`

Public function `draw_confirm_dialog(frame, area, theme, kind)` following the
same pattern as `draw_help`. Renders:

```
┌ Delete ──────────────────────────────────┐
│                                           │
│  Delete set "deploy-prod"?                │
│                                           │
│  y — confirm    n / Esc — cancel          │
│                                           │
└───────────────────────────────────────────┘
```

For Group delete:
```
Delete group "servers" and all 5 sets in it?
```

- Uses `bordered_block_info` with `accent_error` color (red border for danger)
- Dialog width: 50 columns or 70% of terminal, whichever is smaller
- Dialog height: fixed 7 lines
- Body text centered, using `theme.text_primary`
- Hint row "y — confirm    n / Esc — cancel" in `theme.text_disabled` dim

## Screen Handler Changes

### `src/ui/main_screen/handler.rs`

| Key | Context | Before | After |
|-----|---------|--------|-------|
| `d` | active_panel == Sets, set selected | `AppAction::DeleteSet(gi, si)` | `AppAction::RequestDelete(DeleteKind::Set { ... })` |
| `D` | active_panel == Groups, group selected | `AppAction::DeleteGroup(gi)` | `AppAction::RequestDelete(DeleteKind::Group { ... })` |

### `src/ui/detail_screen/handler.rs`

| Key | Context | Before | After |
|-----|---------|--------|-------|
| `d` | focus == Variables, variable selected | `AppAction::DeleteVariable(idx)` | `AppAction::RequestDelete(DeleteKind::Variable { ... })` |
| `d` | focus == Commands, command selected | `AppAction::DeleteCommand(idx)` | `AppAction::RequestDelete(DeleteKind::Command { ... })` |

## App Handler Changes

### `src/app/handler.rs`

**`handle_key`:**
- Add a match arm for `AppMode::ConfirmDelete { ref kind, ref prev }` before the
  `AppMode::Help` arm.
- `y`/`Y`: map `kind` to the corresponding `DeleteXxx` action → restore mode
  from `prev` → call `handle_action(action)`.
- `n`/`N` / `Esc`: restore mode from `prev`, no action.
- Other keys: ignored (no-op).

**`handle_action`:**
- Add `AppAction::RequestDelete(kind)` arm: `self.mode =
  AppMode::ConfirmDelete { kind, prev: Box::new(self.mode.clone()) }`.
  The `App.prev_mode` field is NOT touched — it stays dedicated to Help overlay
  restoration.

**`?` Help during confirmation:** pressing `?` in ConfirmDelete mode invokes
Help as usual (global shortcut). Help stores `prev_mode = ConfirmDelete`. Exiting
Help restores to ConfirmDelete — the confirm dialog reappears. This is correct:
the user can view shortcuts and return to their pending decision.

## Render Changes

### `src/app/render.rs`

- Title bar: for `AppMode::ConfirmDelete`, display mode string `"Confirm"`.
- Mode match: add `AppMode::ConfirmDelete { ref prev, .. }` arm:
  - Render underlying screen from `prev` (same pattern as Help overlay).
  - Call `draw_confirm_dialog(frame, content_area, &self.theme, kind)`.

## Help Screen Update

### `src/ui/help_screen.rs`

No changes needed — the help content already lists `d`/`D` as delete keys. The
confirmation step is self-documenting via the on-screen prompt.

## Tests

All tests follow the convention in CLAUDE.md: explicit imports, `make_key`,
`make_app`, module-specific helpers in their own `#[cfg(test)]` block.

### Unit tests in `src/ui/confirm_dialog.rs`

None (pure rendering function, no testable logic).

### Handler tests in `src/app/handler.rs`

```
test_confirm_delete_y_executes_delete
test_confirm_delete_n_cancels
test_confirm_delete_esc_cancels
test_confirm_delete_other_key_ignored
test_request_delete_set_enters_confirm_mode
test_request_delete_group_enters_confirm_mode
test_request_delete_variable_enters_confirm_mode
test_request_delete_command_enters_confirm_mode
test_help_still_works_during_confirm_delete
```

### Handler tests in `src/ui/main_screen/handler.rs`

```
test_d_key_returns_request_delete_set    (was: test_d_key_returns_delete_set)
test_d_key_returns_request_delete_group  (was: test_D_key_returns_delete_group)
```

### Handler tests in `src/ui/detail_screen/handler.rs`

```
test_d_key_returns_request_delete_variable (was: test_d_deletes_variable)
test_d_key_returns_request_delete_command  (was: test_d_deletes_command)
```

### Integration tests in `src/integration_tests.rs`

```
test_delete_set_with_confirmation_flow
```

## Error Handling

No new error paths. All delete actions already handle out-of-bounds indices via
bounds checks. The confirm dialog is purely UI routing — if `kind` indices are
stale (race between request and confirm), the existing bounds checks in delete
handlers will silently no-op. This is acceptable since the tick-based event loop
prevents data races in practice.

## Files Affected

| File | Change |
|------|--------|
| `src/mode.rs` | Add `ConfirmDelete` variant |
| `src/action.rs` | Add `DeleteKind` enum + `RequestDelete` variant |
| `src/ui/confirm_dialog.rs` | **New file** — render function |
| `src/ui/mod.rs` | Register `confirm_dialog` module + re-export |
| `src/app/handler.rs` | Handle `RequestDelete` action + `ConfirmDelete` key dispatch |
| `src/app/render.rs` | Render `ConfirmDelete` mode + title bar |
| `src/ui/main_screen/handler.rs` | `d`/`D` → `RequestDelete` |
| `src/ui/detail_screen/handler.rs` | `d` → `RequestDelete` (variables + commands) |

Estimated total: ~150 lines of production code, ~80 lines of tests.
