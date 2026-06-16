# Delete Confirmation Dialog — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a modal confirmation overlay for all four delete operations (Set, Group, Variable, Command) replacing immediate irreversible deletion.

**Architecture:** Follow the existing Help screen overlay pattern — new `AppMode::ConfirmDelete` variant stores the target `DeleteKind` and `prev` mode in a `Box`. Screens return `RequestDelete(DeleteKind)` instead of direct delete actions. The app handler routes `y`/`n`/`Esc` to execute or cancel. Rendering uses `Clear` + `centered_rect` + red-bordered dialog.

**Tech Stack:** Rust, Ratatui, crossterm (no new dependencies)

---

### Task 1: Add `DeleteKind` enum and `AppAction::RequestDelete` variant

**Files:**
- Modify: `src/action.rs`

- [ ] **Step 1: Add DeleteKind enum and RequestDelete variant**

```rust
//! Unified action enum for screen-to-app communication.
//!
//! All screens return [`AppAction`] variants from their `handle_key()` methods.
//! The `app::handler` module processes these centrally in `App::handle_action()`.

use crate::models::CommandSet;

/// What the user is about to delete — carries enough context
/// for the confirm dialog to render a descriptive prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        var_index: usize,
        var_name: String,
    },
    Command {
        cmd_index: usize,
        cmd_preview: String,
    },
}

/// Unified action enum returned by all screens.
/// The `app/handler.rs` handles all variants centrally.
pub enum AppAction {
    None,
    Quit,
    Help,

    // === Main screen ===
    ExecuteSet(usize, usize), // (group_index, set_index)
    EditSet(usize, usize),    // (group_index, set_index) — handler resolves data
    NewSet(usize),            // group_index
    DeleteSet(usize, usize),  // (group_index, set_index)
    NewGroup,
    RenameGroup(usize, String), // (group_index, new_name)
    DeleteGroup(usize),

    // === Detail screen ===
    SaveSet(CommandSet),
    CancelEdit,
    DeleteVariable(usize),
    DeleteCommand(usize),

    // === Execution screen ===
    SkipCurrent,
    ContinueFrom(usize),
    ReExec,
    BackToMain,

    // === Variable overlay ===
    ConfirmVariables, // handler reads from variable_screen.inputs
    CancelVariables,

    // === Confirmation ===
    RequestDelete(DeleteKind),
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully (all existing callers still work — DeleteSet, DeleteGroup, etc. variants are unchanged)

- [ ] **Step 3: Commit**

```bash
git add src/action.rs
git commit -m "feat: add DeleteKind enum and AppAction::RequestDelete variant

Prepares the action layer for delete confirmation flow.
Existing delete variants (DeleteSet, DeleteGroup, etc.) are kept
unchanged for internal use after confirmation.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Add `AppMode::ConfirmDelete` and handle `Copy` removal

**Files:**
- Modify: `src/mode.rs`

- [ ] **Step 1: Add ConfirmDelete variant, remove Copy derive**

```rust
/// Application screen modes — only one screen is active at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    /// Main list screen: groups on the left, command sets on the right
    Main,
    /// Detail / edit screen: full-screen form for editing a command set
    Detail,
    /// Full-screen execution view: real-time command output
    Execution,
    /// Help overlay: keyboard shortcuts reference
    Help,
    /// Confirmation overlay for destructive actions (delete Set/Group/Variable/Command)
    ConfirmDelete {
        /// What is being deleted (for prompt text)
        kind: crate::action::DeleteKind,
        /// Mode to restore after confirm/cancel
        prev: Box<AppMode>,
    },
}
```

**Key change:** Remove `Copy` from derive list. `Clone, PartialEq, Eq` remain. The `prev: Box<AppMode>` field stores the restore target — `App.prev_mode` stays dedicated to Help overlay restoration.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compile errors in files that use `Copy` semantics on `AppMode` — specifically `match self.mode` in `app/handler.rs` which moves the value. We'll fix that in Task 6.

The errors should be about moving `self.mode` in `match self.mode`. This is expected — Task 6 will change to `match &self.mode`.

- [ ] **Step 3: Commit**

```bash
git add src/mode.rs
git commit -m "feat: add AppMode::ConfirmDelete variant, remove Copy derive

Box<AppMode> stores the restore mode for the confirmation overlay.
prev_mode field on App remains exclusive to Help overlay.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Create `confirm_dialog.rs` rendering module and `bordered_block_error` helper

**Files:**
- Create: `src/ui/confirm_dialog.rs`
- Modify: `src/ui/render.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Add `bordered_block_error` and `bordered_block_error_zone` helpers to render.rs**

Append before the `bordered_block` function (around line 56), after `bordered_block_info`:

```rust
/// Create a bordered Block with accent_error color for danger overlays.
pub fn bordered_block_error<'a>(theme: &Theme, title: &'a str) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent_error))
        .title(title)
}
```

And append after `bordered_block_info_zone` (around line 195):

```rust
/// Render a bordered error block onto the frame, then return the inner Rect.
pub fn bordered_block_error_zone(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &str,
) -> Rect {
    let block = bordered_block_error(theme, title);
    let inner = block.inner(area);
    frame.render_widget(&block, area);
    inner
}
```

- [ ] **Step 2: Create `src/ui/confirm_dialog.rs`**

```rust
use crate::action::DeleteKind;
use crate::ui::render::{bordered_block_error_zone, centered_rect};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

/// Render the delete confirmation overlay dialog.
pub fn draw_confirm_dialog(frame: &mut Frame, area: Rect, theme: &Theme, kind: &DeleteKind) {
    let prompt = match kind {
        DeleteKind::Set { set_name, .. } => {
            format!("Delete set \"{}\"?", set_name)
        }
        DeleteKind::Group {
            group_name, set_count, ..
        } => {
            if *set_count > 0 {
                format!(
                    "Delete group \"{}\" and all {} sets in it?",
                    group_name, set_count
                )
            } else {
                format!("Delete empty group \"{}\"?", group_name)
            }
        }
        DeleteKind::Variable { var_name, .. } => {
            format!("Delete variable \"{}\"?", var_name)
        }
        DeleteKind::Command { cmd_index, cmd_preview } => {
            let preview = if cmd_preview.len() > 40 {
                format!("{}...", &cmd_preview[..37])
            } else {
                cmd_preview.clone()
            };
            format!("Delete command #{} \"{}\"?", cmd_index, preview)
        }
    };

    let hint = " y — confirm    n / Esc — cancel ";

    let dialog_width = area.width.saturating_sub(8).min(50);
    let dialog_height = 7;
    let dialog_area = centered_rect(area, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let inner = bordered_block_error_zone(frame, dialog_area, theme, " Delete ");

    // Vertical layout: empty, prompt, empty, hint, empty
    let inner_center_y = inner.y + 1;
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &prompt,
            Style::default().fg(theme.text_primary),
        )))
        .alignment(Alignment::Center),
        Rect::new(inner.x, inner_center_y, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hint,
            Style::default()
                .fg(theme.text_disabled)
                .add_modifier(Modifier::DIM),
        )))
        .alignment(Alignment::Center),
        Rect::new(inner.x, inner_center_y + 2, inner.width, 1),
    );
}
```

- [ ] **Step 3: Register module in `src/ui/mod.rs`**

Add the `confirm_dialog` module declaration in alphabetical order (after `pub mod toast;` and before `pub mod render;`):

```rust
pub mod confirm_dialog;
pub mod detail_screen;
pub mod execution_screen;
pub mod help_screen;
pub mod main_screen;
pub mod render;
pub mod theme;
pub mod toast;
pub mod variable_screen;
pub mod widget;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully. The confirm_dialog module is not yet called from anywhere, so no integration issues.

- [ ] **Step 5: Commit**

```bash
git add src/ui/confirm_dialog.rs src/ui/render.rs src/ui/mod.rs
git commit -m "feat: add confirm_dialog overlay renderer and bordered_block_error helper

draw_confirm_dialog renders a centered red-bordered modal with
context-sensitive prompt text for all four DeleteKind variants.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: Update Main Screen handler — `d`/`D` → `RequestDelete`

**Files:**
- Modify: `src/ui/main_screen/handler.rs` (production code + tests)

- [ ] **Step 1: Write failing tests for the new behavior**

Replace the existing `test_d_returns_delete_set` and `test_big_d_returns_delete_group` tests with new ones that expect `RequestDelete`. At lines 252-268 of handler.rs:

```rust
#[test]
fn test_d_returns_request_delete_set() {
    let mut state = MainScreenState::new();
    state.active_panel = Panel::Sets;
    let data = make_data();
    let action = state.handle_key(make_key(KeyCode::Char('d')), &data);
    assert!(
        matches!(action, AppAction::RequestDelete(DeleteKind::Set {
            group_index: 0,
            set_index: 0,
            ..
        }))
    );
}

#[test]
fn test_d_on_groups_panel_does_nothing() {
    let mut state = MainScreenState::new();
    state.active_panel = Panel::Groups;
    let data = make_data();
    let action = state.handle_key(make_key(KeyCode::Char('d')), &data);
    assert!(matches!(action, AppAction::None));
}

#[test]
fn test_big_d_returns_request_delete_group() {
    let mut state = MainScreenState::new();
    state.active_panel = Panel::Groups;
    let data = make_data();
    let action = state.handle_key(make_key(KeyCode::Char('D')), &data);
    assert!(
        matches!(action, AppAction::RequestDelete(DeleteKind::Group {
            group_index: 0,
            ..
        }))
    );
}
```

Also add the import for `DeleteKind` in the test module:
```rust
use crate::action::DeleteKind;
```

And add `DeleteKind` import at the top of the file (production code area):
```rust
use crate::action::{AppAction, DeleteKind};
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test ui::main_screen::handler::tests::test_d_returns_request_delete_set`
Expected: FAIL — still returns `DeleteSet(0, 0)` instead of `RequestDelete(...)`

- [ ] **Step 3: Update the `d` key handler (line 150-157)**

```rust
KeyCode::Char('d') => {
    if self.active_panel == Panel::Sets
        && let Some((gi, si)) = self.selected_set_idx(data)
    {
        let set_name = data.groups[gi].sets[si].name.clone();
        return AppAction::RequestDelete(DeleteKind::Set {
            group_index: gi,
            set_index: si,
            set_name,
        });
    }
    AppAction::None
}
```

- [ ] **Step 4: Update the `D` key handler (line 158-165)**

```rust
KeyCode::Char('D') => {
    if self.active_panel == Panel::Groups
        && let Some(gi) = self.selected_group_idx(data)
    {
        let group = &data.groups[gi];
        return AppAction::RequestDelete(DeleteKind::Group {
            group_index: gi,
            group_name: group.name.clone(),
            set_count: group.sets.len(),
        });
    }
    AppAction::None
}
```

- [ ] **Step 5: Run all main_screen handler tests**

Run: `cargo test ui::main_screen::handler::tests`
Expected: All 11 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/ui/main_screen/handler.rs
git commit -m "feat: main screen d/D returns RequestDelete instead of direct delete

DeleteSet/DeleteGroup actions are now gated behind confirmation overlay.
Tests updated to match new RequestDelete(DeleteKind::...) return values.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Update Detail Screen handler — `d` → `RequestDelete`

**Files:**
- Modify: `src/ui/detail_screen/handler.rs` (production code + tests)

- [ ] **Step 1: Write failing tests for the new behavior**

Replace `test_d_on_variables_returns_delete_variable` (line 305-314). Add a test for command delete:

```rust
#[test]
fn test_d_on_variables_returns_request_delete_variable() {
    let mut state = make_state();
    state.set.variables.push(crate::models::Variable {
        name: "x".to_string(),
        default_value: "y".to_string(),
    });
    state.focus = DetailFocus::Variables;
    let action = state.handle_key(make_key(KeyCode::Char('d')));
    assert!(
        matches!(action, AppAction::RequestDelete(DeleteKind::Variable {
            var_index: 0,
            ..
        }))
    );
}

#[test]
fn test_d_on_commands_returns_request_delete_command() {
    let mut state = make_state();
    state.set.commands.push(crate::models::Command {
        position: 0,
        command: "echo hi".to_string(),
    });
    state.focus = DetailFocus::Commands;
    let action = state.handle_key(make_key(KeyCode::Char('d')));
    assert!(
        matches!(action, AppAction::RequestDelete(DeleteKind::Command {
            cmd_index: 0,
            ..
        }))
    );
}
```

Add the import for `DeleteKind` in the test module:
```rust
use crate::action::DeleteKind;
```

And add `DeleteKind` import at the top of the production code:
```rust
use crate::action::{AppAction, DeleteKind};
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test ui::detail_screen::handler::tests::test_d_on_variables_returns_request_delete_variable`
Expected: FAIL — still returns `DeleteVariable(0)`

- [ ] **Step 3: Update the `d` key handler for Variables (line 157-164)**

```rust
KeyCode::Char('d' | 'D') => match self.focus {
    DetailFocus::Variables if !self.set.variables.is_empty() => {
        let idx = self
            .variable_list
            .selected
            .min(self.set.variables.len().saturating_sub(1));
        let var_name = self.set.variables[idx].name.clone();
        return AppAction::RequestDelete(DeleteKind::Variable {
            var_index: idx,
            var_name,
        });
    }
    DetailFocus::Commands if !self.set.commands.is_empty() => {
        let idx = self
            .command_list
            .selected
            .min(self.set.commands.len().saturating_sub(1));
        let cmd_preview = self.set.commands[idx].command.clone();
        return AppAction::RequestDelete(DeleteKind::Command {
            cmd_index: idx,
            cmd_preview,
        });
    }
    _ => {}
},
```

- [ ] **Step 4: Run all detail_screen handler tests**

Run: `cargo test ui::detail_screen::handler::tests`
Expected: All 9 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/detail_screen/handler.rs
git commit -m "feat: detail screen d returns RequestDelete for variables and commands

DeleteVariable/DeleteCommand actions are now gated behind confirmation.
Tests updated to match new RequestDelete(DeleteKind::...) return values.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: Update App handler — `handle_action(RequestDelete)` + `handle_key(ConfirmDelete)`

**Files:**
- Modify: `src/app/handler.rs`

- [ ] **Step 1: Write failing tests for ConfirmDelete mode behavior**

Add these tests to the `#[cfg(test)] mod tests` block in `src/app/handler.rs`, before the closing `}`:

```rust
// ---- RequestDelete / ConfirmDelete ----
fn make_data_for_delete_test() -> AppData {
    let mut g = Group::new("Deploy".to_string());
    let mut set = CommandSet::new("Prod".to_string(), g.id);
    set.commands.push(crate::models::Command { position: 0, command: "echo hi".to_string() });
    g.sets.push(set);
    AppData { groups: vec![g] }
}

fn setup_detail_for_delete(app: &mut App) {
    app.data = make_data_for_delete_test();
    let set = app.data.groups[0].sets[0].clone();
    app.detail_screen = Some(DetailScreenState::new(set, app.data.groups.clone()));
    app.mode = AppMode::Detail;
}

#[test]
fn test_request_delete_set_enters_confirm_mode() {
    let mut app = make_app();
    app.data = make_data_for_delete_test();
    app.handle_action(AppAction::RequestDelete(DeleteKind::Set {
        group_index: 0,
        set_index: 0,
        set_name: "Prod".to_string(),
    }));
    assert!(
        matches!(app.mode, AppMode::ConfirmDelete { .. }),
        "Expected ConfirmDelete mode after RequestDelete"
    );
}

#[test]
fn test_request_delete_group_enters_confirm_mode() {
    let mut app = make_app();
    app.data = make_data_for_delete_test();
    app.handle_action(AppAction::RequestDelete(DeleteKind::Group {
        group_index: 0,
        group_name: "Deploy".to_string(),
        set_count: 1,
    }));
    assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
}

#[test]
fn test_request_delete_variable_enters_confirm_mode() {
    use crate::models::Variable;
    let mut app = make_app();
    let mut g = Group::new("G".to_string());
    let mut set = CommandSet::new("S".to_string(), g.id);
    set.variables.push(Variable { name: "host".to_string(), default_value: "".to_string() });
    g.sets.push(set);
    app.data = AppData { groups: vec![g] };
    let set_clone = app.data.groups[0].sets[0].clone();
    app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
    app.mode = AppMode::Detail;

    app.handle_action(AppAction::RequestDelete(DeleteKind::Variable {
        var_index: 0,
        var_name: "host".to_string(),
    }));
    assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
}

#[test]
fn test_request_delete_command_enters_confirm_mode() {
    let mut app = make_app();
    app.data = make_data_for_delete_test();
    let set = app.data.groups[0].sets[0].clone();
    app.detail_screen = Some(DetailScreenState::new(set, app.data.groups.clone()));
    app.mode = AppMode::Detail;

    app.handle_action(AppAction::RequestDelete(DeleteKind::Command {
        cmd_index: 0,
        cmd_preview: "echo hi".to_string(),
    }));
    assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
}

#[test]
fn test_confirm_delete_y_executes_delete_set() {
    let mut app = make_app();
    app.data = make_data_for_delete_test();
    // Enter ConfirmDelete mode directly
    app.mode = AppMode::ConfirmDelete {
        kind: DeleteKind::Set {
            group_index: 0,
            set_index: 0,
            set_name: "Prod".to_string(),
        },
        prev: Box::new(AppMode::Main),
    };
    // Simulate 'y' key press
    let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty());
    app.handle_key(y_key);
    assert!(app.data.groups[0].sets.is_empty());
    assert_eq!(app.mode, AppMode::Main);
}

#[test]
fn test_confirm_delete_y_executes_delete_group() {
    let mut app = make_app();
    app.data = make_data_for_delete_test();
    app.mode = AppMode::ConfirmDelete {
        kind: DeleteKind::Group {
            group_index: 0,
            group_name: "Deploy".to_string(),
            set_count: 1,
        },
        prev: Box::new(AppMode::Main),
    };
    let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty());
    app.handle_key(y_key);
    assert!(app.data.groups.is_empty());
    assert_eq!(app.mode, AppMode::Main);
}

#[test]
fn test_confirm_delete_y_executes_delete_variable() {
    use crate::models::Variable;
    let mut app = make_app();
    let mut g = Group::new("G".to_string());
    let mut set = CommandSet::new("S".to_string(), g.id);
    set.variables.push(Variable { name: "host".to_string(), default_value: "".to_string() });
    g.sets.push(set);
    app.data = AppData { groups: vec![g] };
    let set_clone = app.data.groups[0].sets[0].clone();
    app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));

    app.mode = AppMode::ConfirmDelete {
        kind: DeleteKind::Variable {
            var_index: 0,
            var_name: "host".to_string(),
        },
        prev: Box::new(AppMode::Detail),
    };
    let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty());
    app.handle_key(y_key);
    let ds = app.detail_screen.as_ref().unwrap();
    assert!(ds.set.variables.is_empty());
    assert_eq!(app.mode, AppMode::Detail);
}

#[test]
fn test_confirm_delete_y_executes_delete_command() {
    let mut app = make_app();
    app.data = make_data_for_delete_test();
    let set = app.data.groups[0].sets[0].clone();
    app.detail_screen = Some(DetailScreenState::new(set, app.data.groups.clone()));

    app.mode = AppMode::ConfirmDelete {
        kind: DeleteKind::Command {
            cmd_index: 0,
            cmd_preview: "echo hi".to_string(),
        },
        prev: Box::new(AppMode::Detail),
    };
    let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty());
    app.handle_key(y_key);
    let ds = app.detail_screen.as_ref().unwrap();
    assert!(ds.set.commands.is_empty());
    assert_eq!(app.mode, AppMode::Detail);
}

#[test]
fn test_confirm_delete_n_cancels() {
    let mut app = make_app();
    app.data = make_data_for_delete_test();
    app.mode = AppMode::ConfirmDelete {
        kind: DeleteKind::Set {
            group_index: 0,
            set_index: 0,
            set_name: "Prod".to_string(),
        },
        prev: Box::new(AppMode::Main),
    };
    let n_key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty());
    app.handle_key(n_key);
    // Set should NOT be deleted
    assert_eq!(app.data.groups[0].sets.len(), 1);
    assert_eq!(app.mode, AppMode::Main);
}

#[test]
fn test_confirm_delete_esc_cancels() {
    let mut app = make_app();
    app.data = make_data_for_delete_test();
    app.mode = AppMode::ConfirmDelete {
        kind: DeleteKind::Set {
            group_index: 0,
            set_index: 0,
            set_name: "Prod".to_string(),
        },
        prev: Box::new(AppMode::Main),
    };
    app.handle_key(make_key(KeyCode::Esc));
    // Set should NOT be deleted
    assert_eq!(app.data.groups[0].sets.len(), 1);
    assert_eq!(app.mode, AppMode::Main);
}

#[test]
fn test_confirm_delete_other_key_ignored() {
    let mut app = make_app();
    app.data = make_data_for_delete_test();
    app.mode = AppMode::ConfirmDelete {
        kind: DeleteKind::Set {
            group_index: 0,
            set_index: 0,
            set_name: "Prod".to_string(),
        },
        prev: Box::new(AppMode::Main),
    };
    // Press a key that is not y, n, or Esc
    app.handle_key(make_key(KeyCode::Char('x')));
    // Should still be in ConfirmDelete mode
    assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
    // Set should NOT be deleted
    assert_eq!(app.data.groups[0].sets.len(), 1);
}

#[test]
fn test_help_still_works_during_confirm_delete() {
    let mut app = make_app();
    app.data = make_data_for_delete_test();
    app.mode = AppMode::ConfirmDelete {
        kind: DeleteKind::Set {
            group_index: 0,
            set_index: 0,
            set_name: "Prod".to_string(),
        },
        prev: Box::new(AppMode::Main),
    };
    // Press '?' for help
    app.handle_key(make_key(KeyCode::Char('?')));
    assert_eq!(app.mode, AppMode::Help);
    assert_eq!(app.prev_mode, Some(AppMode::ConfirmDelete {
        kind: DeleteKind::Set {
            group_index: 0,
            set_index: 0,
            set_name: "Prod".to_string(),
        },
        prev: Box::new(AppMode::Main),
    }));
    // Dismiss Help — should restore to ConfirmDelete
    app.handle_key(make_key(KeyCode::Esc));
    assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));
}
```

Add the necessary imports to the test module:
```rust
use crate::action::DeleteKind;
use crossterm::event::KeyModifiers;
```

And add imports at the top of the file (production area):
```rust
use crate::action::{AppAction, DeleteKind};
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test app::handler::tests::test_request_delete_set_enters_confirm_mode`
Expected: FAIL — `RequestDelete` variant not handled in `handle_action`

- [ ] **Step 3: Add `RequestDelete` handler in `handle_action`**

In `handle_action`, after the `AppAction::Quit` arm (line 52), add:

```rust
AppAction::RequestDelete(kind) => {
    self.mode = AppMode::ConfirmDelete {
        kind,
        prev: Box::new(self.mode.clone()),
    };
}
```

- [ ] **Step 4: Change `match self.mode` to `match &self.mode` in `handle_key`**

On line 26, change:
```rust
match self.mode {
```
to:
```rust
match &self.mode {
```

- [ ] **Step 5: Add `ConfirmDelete` match arm in `handle_key`**

Add before the `AppMode::Help` arm (before line 43):

```rust
AppMode::ConfirmDelete { kind, prev } => {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let action = match kind {
                DeleteKind::Set {
                    group_index, set_index, ..
                } => AppAction::DeleteSet(*group_index, *set_index),
                DeleteKind::Group { group_index, .. } => {
                    AppAction::DeleteGroup(*group_index)
                }
                DeleteKind::Variable { var_index, .. } => {
                    AppAction::DeleteVariable(*var_index)
                }
                DeleteKind::Command { cmd_index, .. } => {
                    AppAction::DeleteCommand(*cmd_index)
                }
            };
            self.mode = (**prev).clone();
            self.handle_action(action);
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            self.mode = (**prev).clone();
        }
        _ => {} // Ignore all other keys during confirmation
    }
}
```

- [ ] **Step 6: Fix Help mode `self.mode` move issue**

On line 55, in the `AppAction::Help` handler, change:
```rust
self.prev_mode = Some(self.mode);
```
to:
```rust
self.prev_mode = Some(self.mode.clone());
```

This is needed because `AppMode` is no longer `Copy`, so moving `self.mode` into `Some(...)` would prevent the immediate reassignment `self.mode = AppMode::Help` from being valid in all borrow-checker scenarios.

- [ ] **Step 7: Run all app handler tests**

Run: `cargo test app::handler::tests`
Expected: All tests PASS (existing 26 + 12 new = 38 tests)

- [ ] **Step 8: Commit**

```bash
git add src/app/handler.rs
git commit -m "feat: handle RequestDelete action and ConfirmDelete key dispatch

- handle_action: RequestDelete transitions to ConfirmDelete mode
- handle_key: ConfirmDelete mode routes y/n/Esc to execute or cancel
- match &self.mode (was match self.mode) due to AppMode losing Copy
- Keep prev_mode exclusive to Help overlay

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: Update App render — `ConfirmDelete` mode

**Files:**
- Modify: `src/app/render.rs`

- [ ] **Step 1: Add ConfirmDelete mode rendering**

After the `AppMode::Execution` arm (line 68) and before `AppMode::Help` (line 69), add:

```rust
AppMode::ConfirmDelete { ref kind, ref prev } => {
    // Render underlying screen based on the stored prev mode
    match prev.as_ref() {
        AppMode::Detail => {
            if let Some(ref mut ds) = self.detail_screen {
                ds.render(frame, content_area, &self.theme);
            }
        }
        AppMode::Execution => {
            if let ExecutionState::Running { ref screen, .. } = self.execution_state {
                screen.render(frame, content_area, &self.theme);
            }
        }
        _ => {
            self.main_screen
                .render(frame, content_area, &self.data, &self.theme);
        }
    }
    crate::ui::confirm_dialog::draw_confirm_dialog(
        frame, content_area, &self.theme, kind,
    );
}
```

- [ ] **Step 2: Update title bar mode string**

In the title bar match (line 34-39), add the ConfirmDelete entry:

```rust
let mode_str = match self.mode {
    AppMode::Main => "Main",
    AppMode::Detail => "Edit",
    AppMode::Execution => "Run",
    AppMode::Help => "Help",
    AppMode::ConfirmDelete { .. } => "Confirm",
};
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully. All modules connected.

- [ ] **Step 4: Verify full test suite**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/app/render.rs
git commit -m "feat: render ConfirmDelete overlay with underlying screen backdrop

Following the Help screen pattern: render the prev-mode screen beneath
the Clear+centered_rect dialog. Title bar shows 'Confirm'.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 8: Integration test and final verification

**Files:**
- Modify: `src/integration_tests.rs`

- [ ] **Step 1: Add integration test for delete confirmation flow**

Add this test to `src/integration_tests.rs`:

```rust
#[test]
fn test_delete_set_with_confirmation_flow() {
    use crate::action::{AppAction, DeleteKind};
    use crate::app::App;
    use crate::mode::AppMode;
    use crate::models::{AppData, CommandSet, Group};
    use crate::test_utils::{make_app, make_key};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = make_app();
    // Set up data with one group and one set
    let mut group = Group::new("Test".to_string());
    let set = CommandSet::new("target-set".to_string(), group.id);
    group.sets.push(set);
    app.data = AppData { groups: vec![group] };

    // Step 1: Request delete via action
    app.handle_action(AppAction::RequestDelete(DeleteKind::Set {
        group_index: 0,
        set_index: 0,
        set_name: "target-set".to_string(),
    }));
    assert!(
        matches!(app.mode, AppMode::ConfirmDelete { .. }),
        "Should enter ConfirmDelete mode"
    );

    // Step 2: Press 'n' to cancel — set should remain
    let n_key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty());
    app.handle_key(n_key);
    assert_eq!(app.mode, AppMode::Main);
    assert_eq!(app.data.groups[0].sets.len(), 1);

    // Step 3: Request delete again, this time confirm with 'y'
    app.handle_action(AppAction::RequestDelete(DeleteKind::Set {
        group_index: 0,
        set_index: 0,
        set_name: "target-set".to_string(),
    }));
    assert!(matches!(app.mode, AppMode::ConfirmDelete { .. }));

    let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty());
    app.handle_key(y_key);
    assert_eq!(app.mode, AppMode::Main);
    assert!(app.data.groups[0].sets.is_empty());
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test integration_tests::test_delete_set_with_confirmation_flow`
Expected: PASS

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 4: Run clippy**

Run: `cargo clippy`
Expected: No warnings

- [ ] **Step 5: Commit**

```bash
git add src/integration_tests.rs
git commit -m "test: add integration test for delete confirmation flow

Covers full round-trip: RequestDelete → ConfirmDelete mode → cancel (n)
and confirm (y) → back to Main with data mutated or preserved.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 9: Mark feature as completed in memory

**Files:**
- Modify: `/home/william/.claude/projects/-home-william-Code-Rust-launcher/memory/feature-gap-priorities.md`

- [ ] **Step 1: Mark #1 as done**

Change line in `feature-gap-priorities.md`:
```
- [ ] **#1 删除确认对话框** (~100 行)
```
to:
```
- [x] **#1 删除确认对话框** (~150 行)
  完成: 2026-06-16
```

- [ ] **Step 2: Commit memory update**

```bash
git add /home/william/.claude/projects/-home-william-Code-Rust-launcher/memory/feature-gap-priorities.md
git commit -m "docs: mark delete confirmation dialog as completed in memory

Co-Authored-By: Claude <noreply@anthropic.com>"
```
