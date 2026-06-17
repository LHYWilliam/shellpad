# Confirm Dialog — Button Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace y/n keyboard confirmation with ←/→ button selection between Confirm/Cancel + Enter to execute.

**Architecture:** New `ConfirmChoice` enum with `Confirm`/`Cancel` variants. `AppMode::ConfirmDelete` gains `selected: ConfirmChoice` field. Handler ←/→ toggles focus, Enter dispatches. Render replaces y/n hint with styled button row.

**Tech Stack:** Rust, Ratatui

---

### Task 1: Add ConfirmChoice enum + update AppMode + RequestDelete init

**Files:**
- Modify: `src/action.rs`
- Modify: `src/mode.rs`
- Modify: `src/app/handler.rs`

- [ ] **Step 1: Add ConfirmChoice to action.rs**

In `src/action.rs`, before the `AppAction` enum:

```rust
/// Which button is focused in the delete confirmation dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmChoice {
    Confirm,
    Cancel,
}
```

- [ ] **Step 2: Add selected field to AppMode::ConfirmDelete**

In `src/mode.rs`, update the variant:

```rust
ConfirmDelete {
    kind: crate::action::DeleteKind,
    prev: Box<AppMode>,
    selected: crate::action::ConfirmChoice,  // ← new
},
```

- [ ] **Step 3: Init selected in RequestDelete handler**

In `src/app/handler.rs`, update `AppAction::RequestDelete(kind)` to:

```rust
AppAction::RequestDelete(kind) => {
    self.mode = AppMode::ConfirmDelete {
        kind,
        prev: Box::new(self.mode.clone()),
        selected: ConfirmChoice::Cancel,
    };
}
```

Add `use crate::action::ConfirmChoice;` to imports at top:
```rust
use crate::action::{AppAction, ConfirmChoice, DeleteKind, ReorderKind};
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: E0004 — `AppMode::ConfirmDelete` arms need `selected` field in match patterns. Expected — fixed in Task 2.

- [ ] **Step 5: Commit**

```bash
git add src/action.rs src/mode.rs src/app/handler.rs
git commit -m "feat: add ConfirmChoice enum and selected field to ConfirmDelete

RequestDelete initializes with ConfirmChoice::Cancel (safe default).

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Update handler + tests

**Files:**
- Modify: `src/app/handler.rs`

- [ ] **Step 1: Write failing tests**

Replace existing 6 confirm_delete tests (lines 858-1002: `test_confirm_delete_y_*`, `test_confirm_delete_n_cancels`, `test_confirm_delete_esc_cancels`, `test_confirm_delete_other_key_ignored`) with new 7 tests:

```rust
    #[test]
    fn test_confirm_dialog_default_focus_is_cancel() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::RequestDelete(DeleteKind::Set {
            group_index: 0, set_index: 0, set_name: "P".to_string(),
        }));
        if let AppMode::ConfirmDelete { selected, .. } = &app.mode {
            assert!(matches!(selected, ConfirmChoice::Cancel));
        } else {
            panic!("expected ConfirmDelete");
        }
    }

    #[test]
    fn test_confirm_dialog_left_to_confirm_enter_deletes() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set { group_index: 0, set_index: 0, set_name: "P".to_string() },
            prev: Box::new(AppMode::Main),
            selected: ConfirmChoice::Cancel,
        };
        app.handle_key(make_key(KeyCode::Left)); // Cancel → Confirm
        app.handle_key(make_key(KeyCode::Enter)); // Confirm executes delete
        assert!(app.data.groups[0].sets.is_empty());
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_dialog_right_to_cancel_enter_noop() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set { group_index: 0, set_index: 0, set_name: "P".to_string() },
            prev: Box::new(AppMode::Main),
            selected: ConfirmChoice::Confirm,
        };
        app.handle_key(make_key(KeyCode::Right)); // Confirm → Cancel
        app.handle_key(make_key(KeyCode::Enter)); // Cancel = no-op
        assert_eq!(app.data.groups[0].sets.len(), 1);
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_dialog_enter_on_confirm_deletes() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set { group_index: 0, set_index: 0, set_name: "P".to_string() },
            prev: Box::new(AppMode::Main),
            selected: ConfirmChoice::Confirm,
        };
        app.handle_key(make_key(KeyCode::Enter));
        assert!(app.data.groups[0].sets.is_empty());
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_dialog_enter_on_cancel_noop() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set { group_index: 0, set_index: 0, set_name: "P".to_string() },
            prev: Box::new(AppMode::Main),
            selected: ConfirmChoice::Cancel,
        };
        app.handle_key(make_key(KeyCode::Enter));
        assert_eq!(app.data.groups[0].sets.len(), 1);
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_dialog_esc_cancels() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set { group_index: 0, set_index: 0, set_name: "P".to_string() },
            prev: Box::new(AppMode::Main),
            selected: ConfirmChoice::Cancel,
        };
        app.handle_key(make_key(KeyCode::Esc));
        assert_eq!(app.data.groups[0].sets.len(), 1);
        assert_eq!(app.mode, AppMode::Main);
    }

    #[test]
    fn test_confirm_dialog_arrow_boundary_noop() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.mode = AppMode::ConfirmDelete {
            kind: DeleteKind::Set { group_index: 0, set_index: 0, set_name: "P".to_string() },
            prev: Box::new(AppMode::Main),
            selected: ConfirmChoice::Confirm,
        };
        app.handle_key(make_key(KeyCode::Left)); // already at Confirm, no further left
        // still in ConfirmDelete mode
        assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
        // data still intact
        assert_eq!(app.data.groups[0].sets.len(), 1);
    }
```

Also fix the `test_request_delete_*_enters_confirm_mode` tests (4 tests, lines 796-854) — the patterns must match `selected: _`:

```rust
assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
// This already works since `..` ignores `selected`. No change needed.
```

Update `test_help_still_works_during_confirm_delete` (line 998):
```rust
// Add selected field to the confirm delete setup
app.mode = AppMode::ConfirmDelete {
    kind: DeleteKind::Set { ... },
    prev: Box::new(AppMode::Main),
    selected: ConfirmChoice::Cancel,
};
```

Add `use crate::action::ConfirmChoice;` to test imports.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test app::handler::tests::test_confirm_dialog_left_to_confirm_enter_deletes`
Expected: FAIL — ←/→ not yet handled

- [ ] **Step 3: Replace handler key dispatch**

Replace the current `AppMode::ConfirmDelete { kind, prev }` match arm (lines 44-68) with:

```rust
            AppMode::ConfirmDelete { kind, prev, ref selected } => {
                match key.code {
                    KeyCode::Left => {
                        if matches!(selected, ConfirmChoice::Cancel) {
                            self.mode = AppMode::ConfirmDelete {
                                kind: kind.clone(), prev: prev.clone(),
                                selected: ConfirmChoice::Confirm,
                            };
                        }
                    }
                    KeyCode::Right => {
                        if matches!(selected, ConfirmChoice::Confirm) {
                            self.mode = AppMode::ConfirmDelete {
                                kind: kind.clone(), prev: prev.clone(),
                                selected: ConfirmChoice::Cancel,
                            };
                        }
                    }
                    KeyCode::Enter => {
                        let action = if matches!(selected, ConfirmChoice::Confirm) {
                            match kind {
                                DeleteKind::Set { group_index, set_index, .. } => {
                                    AppAction::DeleteSet(*group_index, *set_index)
                                }
                                DeleteKind::Group { group_index, .. } => {
                                    AppAction::DeleteGroup(*group_index)
                                }
                                DeleteKind::Variable { var_index, .. } => {
                                    AppAction::DeleteVariable(*var_index)
                                }
                                DeleteKind::Command { cmd_index, .. } => {
                                    AppAction::DeleteCommand(*cmd_index)
                                }
                            }
                        } else {
                            AppAction::None
                        };
                        self.mode = (**prev).clone();
                        if !matches!(action, AppAction::None) {
                            self.handle_action(action);
                        }
                    }
                    KeyCode::Esc => {
                        self.mode = (**prev).clone();
                    }
                    _ => {}
                }
            }
```

- [ ] **Step 4: Run handler tests**

Run: `cargo test app::handler::tests`
Expected: All tests PASS (50 old - 6 replaced + 7 new = 51)

- [ ] **Step 5: Commit**

```bash
git add src/app/handler.rs
git commit -m "feat: replace y/n with ←/→ button selection in confirm dialog

Left/Right toggles between Confirm and Cancel. Enter executes
focused button. Esc = Cancel. Default focus is Cancel (safe).
7 new tests, 6 old y/n tests replaced.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Update confirm dialog render

**Files:**
- Modify: `src/ui/confirm_dialog.rs`

- [ ] **Step 1: Replace render with button-based layout**

Replace the hint and bottom layout (lines 43-74) with:

```rust
    let dialog_width = area.width.saturating_sub(8).min(50);
    let dialog_height = 7;
    let dialog_area = centered_rect(area, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let block = bordered_block_error(theme, " Delete ");
    let inner = block.inner(dialog_area);
    frame.render_widget(&block, dialog_area);

    // Prompt
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &prompt,
            Style::default().fg(theme.text_primary),
        )))
        .alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    // Button row
    let confirm_style = if matches!(selected, ConfirmChoice::Confirm) {
        theme.selected_style()
    } else {
        theme.normal_style()
    };
    let cancel_style = if matches!(selected, ConfirmChoice::Cancel) {
        theme.selected_style()
    } else {
        theme.normal_style()
    };

    let buttons = Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled(if matches!(selected, ConfirmChoice::Confirm) {
            "[Confirm]"
        } else {
            " Confirm "
        },
        confirm_style),
        Span::styled("      ", Style::default()),
        Span::styled(if matches!(selected, ConfirmChoice::Cancel) {
            "[Cancel]"
        } else {
            " Cancel "
        },
        cancel_style),
    ]);
    frame.render_widget(
        Paragraph::new(buttons).alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
    );

    // Hint
    let hint = " ←/→ Select      Enter — Confirm ";
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hint,
            Style::default()
                .fg(theme.text_disabled)
                .add_modifier(Modifier::DIM),
        )))
        .alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + 5, inner.width, 1),
    );
```

Update function signature to accept `selected`:

```rust
pub fn draw_confirm_dialog(
    frame: &mut Frame, area: Rect, theme: &Theme,
    kind: &DeleteKind, selected: ConfirmChoice,
) {
```

Add import at top:
```rust
use crate::action::{ConfirmChoice, DeleteKind};
```

Update `src/app/render.rs` call site:
```rust
AppMode::ConfirmDelete { ref kind, ref prev, ref selected } => {
    // ... render underlying screen ...
    crate::ui::confirm_dialog::draw_confirm_dialog(
        frame, content_area, &self.theme, kind, *selected,
    );
}
```

Also add `ConfirmChoice` import to render.rs:
```rust
use crate::action::ConfirmChoice;
```

- [ ] **Step 2: Verify compilation and tests**

Run: `cargo check && cargo test`
Expected: All tests PASS

- [ ] **Step 3: Run clippy**

Run: `cargo clippy`
Expected: No new warnings

- [ ] **Step 4: Commit**

```bash
git add src/ui/confirm_dialog.rs src/app/render.rs
git commit -m "feat: render confirm dialog with selectable buttons

Two buttons [Confirm] [Cancel] with selected_style highlight.
←/→ hint in footer. selected: ConfirmChoice parameter drives
which button is highlighted.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
