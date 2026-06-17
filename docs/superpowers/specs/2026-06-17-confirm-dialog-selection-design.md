# Confirm Dialog — Button Selection Redesign

**Date:** 2026-06-17
**Status:** Approved
**Scope:** Replace y/n keyboard confirmation with ←/→ button selection + Enter

## Problem

Current delete confirmation uses `y` / `n` / `Esc` key dispatch. This is functional
but creates an inconsistent interaction model compared to the rest of the app
(which uses ←/→ and Enter for selection).

## Solution

Replace the inline `y — confirm  n / Esc — cancel` hint with two selectable
buttons: **Confirm** and **Cancel**. ←/→ toggles focus between them, Enter
executes the focused action. Esc is a shortcut for Cancel. Default focus is
Cancel (safe default).

## Layout

```
┌ Delete ──────────────────────────────┐
│                                       │
│  Delete set "deploy"?                 │
│                                       │
│     Confirm          Cancel           │  ← Cancel 默认高亮
│                                       │
│  ←/→ Select      Enter — Confirm      │
└───────────────────────────────────────┘
```

**Confirm 选中时：**
```
│     [Confirm]        Cancel           │  ← Confirm 高亮
```

## Interaction

| Key | Behavior |
|-----|----------|
| ← | Focus moves left: Cancel → Confirm |
| → | Focus moves right: Confirm → Cancel |
| Enter | Execute focused button |
| Esc | Execute Cancel (same as Cancel + Enter) |
| Other keys | Ignored |

Default focus: **Cancel** (safety — accidental Enter doesn't delete).

## New Types

### `src/action.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmChoice {
    Confirm,
    Cancel,
}
```

### `src/mode.rs`

`AppMode::ConfirmDelete` gains a `selected` field:

```rust
pub enum AppMode {
    // ...existing...
    ConfirmDelete {
        kind: DeleteKind,
        prev: Box<AppMode>,
        selected: ConfirmChoice,  // new — which button is focused
    },
}
```

## Handler Changes

### `src/app/handler.rs`

**`handle_action` — `RequestDelete`** initializes to Cancel:

```rust
AppAction::RequestDelete(kind) => {
    self.mode = AppMode::ConfirmDelete {
        kind,
        prev: Box::new(self.mode.clone()),
        selected: ConfirmChoice::Cancel,
    };
}
```

**`handle_key` — `ConfirmDelete`** replaces y/n with ←/→ + Enter:

```rust
AppMode::ConfirmDelete { kind, prev, selected } => {
    match key.code {
        KeyCode::Left => {
            // Focus moves left (Cancel → Confirm, Confirm → no-op)
            if *selected == ConfirmChoice::Cancel {
                self.mode = AppMode::ConfirmDelete {
                    kind: kind.clone(), prev: prev.clone(),
                    selected: ConfirmChoice::Confirm,
                };
            }
        }
        KeyCode::Right => {
            // Focus moves right (Confirm → Cancel, Cancel → no-op)
            if *selected == ConfirmChoice::Confirm {
                self.mode = AppMode::ConfirmDelete {
                    kind: kind.clone(), prev: prev.clone(),
                    selected: ConfirmChoice::Cancel,
                };
            }
        }
        KeyCode::Enter => {
            let action = if matches!(selected, ConfirmChoice::Confirm) {
                // dispatch delete action
                match kind { ... }
            } else {
                AppAction::None
            };
            self.mode = (**prev).clone();
            if !matches!(action, AppAction::None) {
                self.handle_action(action);
            }
        }
        KeyCode::Esc => {
            // Cancel — same as Cancel + Enter
            self.mode = (**prev).clone();
        }
        _ => {} // ignore
    }
}
```

## Render Changes

### `src/ui/confirm_dialog.rs`

Replace the `y — confirm  n / Esc — cancel` text with two styled buttons:

```rust
let confirm_style = if selected == ConfirmChoice::Confirm {
    theme.selected_style()
} else {
    theme.normal_style()
};
let cancel_style = if selected == ConfirmChoice::Cancel {
    theme.selected_style()
} else {
    theme.normal_style()
};

// Prompt line (unchanged)
// ...

// Button row
let button_row_y = inner.y + 3;
let confirm_label = if selected == ConfirmChoice::Confirm {
    "[Confirm]"
} else {
    " Confirm "
};
let cancel_label = if selected == ConfirmChoice::Cancel {
    "[Cancel]"
} else {
    " Cancel "
};
let buttons = format!("   {}      {}   ", confirm_label, cancel_label);
frame.render_widget(
    Paragraph::new(Line::from(buttons)).alignment(Alignment::Center),
    Rect::new(inner.x, button_row_y, inner.width, 1),
);

// Hint row
let hint = " ←/→ Select      Enter — Confirm ";
frame.render_widget(
    Paragraph::new(Line::from(Span::styled(hint, ...))).alignment(Center),
    Rect::new(inner.x, button_row_y + 2, inner.width, 1),
);
```

## Tests

| Test | What it verifies |
|------|-----------------|
| `confirm_dialog_default_focus_is_cancel` | New RequestDelete initializes with Cancel |
| `confirm_dialog_left_from_cancel_moves_to_confirm` | ← swaps to Confirm |
| `confirm_dialog_right_from_confirm_moves_to_cancel` | → swaps to Cancel |
| `confirm_dialog_enter_on_confirm_executes_delete` | Enter on Confirm → delete |
| `confirm_dialog_enter_on_cancel_does_not_delete` | Enter on Cancel → no-op |
| `confirm_dialog_esc_cancels` | Esc = Cancel |
| `confirm_dialog_boundary_noop` | ← on Confirm / → on Cancel no-op |

Existing `test_handler_delete_set` via `AppAction::DeleteSet` still passes (it bypasses the dialog entirely — internal use only).

## Files Affected

| File | Change |
|------|--------|
| `src/action.rs` | Add `ConfirmChoice` enum |
| `src/mode.rs` | Add `selected: ConfirmChoice` field |
| `src/app/handler.rs` | Replace y/n handler with ←/→ + Enter, test updates |
| `src/ui/confirm_dialog.rs` | Replace hint with button row, new styling |

Estimated: ~60 lines changed, ~50 lines of test updates.
