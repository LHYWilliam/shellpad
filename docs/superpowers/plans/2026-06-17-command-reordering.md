# Command & Item Reordering — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add unified Ctrl+Up/Ctrl+Down reordering for all four list types (Groups, Sets, Variables, Commands) via a single `AppAction::Reorder` action with bounds-checked swap.

**Architecture:** New `ReorderKind` enum + `AppAction::Reorder` variant. Screen handlers return `Reorder(kind, dir)` on Ctrl+Up/Down. `App::handle_action` performs bounds check, `vec.swap`, selected-index update, and (for Commands) position renumbering. Ctrl+Up/Down arms are placed before plain Up/Down arms in match blocks so guards fire first.

**Tech Stack:** Rust, Ratatui, crossterm (no new dependencies)

---

### Task 1: Add `ReorderKind` enum and `AppAction::Reorder` variant

**Files:**
- Modify: `src/action.rs`

- [ ] **Step 1: Add ReorderKind and Reorder variant**

Insert after `DeleteKind` (around line 29), before `AppAction`:

```rust
/// What the user wants to reorder — identifies the target item and its position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReorderKind {
    Group(usize),
    Set(usize, usize),
    Variable(usize),
    Command(usize),
}
```

Add to `AppAction` enum (after `RequestDelete`):

```rust
    // === Confirmation ===
    RequestDelete(DeleteKind),

    // === Reordering ===
    Reorder(ReorderKind, isize), // direction: -1 up, +1 down
}
```

- [ ] **Step 2: Verify it compiles (with temporary match arm)**

Run: `cargo check`
Expected: E0004 — `AppAction::Reorder` not covered in `handle_action`. Add temporary arm:

```rust
            // Temporary: placeholder until Task 2
            AppAction::Reorder(_, _) => {}
```

Re-run: `cargo check`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add src/action.rs src/app/handler.rs
git commit -m "feat: add ReorderKind enum and AppAction::Reorder variant

Direction -1 (up) or +1 (down). Four targets: Group, Set,
Variable, Command. Placeholder handler arm added.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: App handler — `handle_action(Reorder)` + tests

**Files:**
- Modify: `src/app/handler.rs`

- [ ] **Step 1: Write failing tests**

Add at end of the test module, before closing `}`:

```rust
    // ---- Reorder ----
    #[test]
    fn test_reorder_group_up() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.data.groups.push(Group::new("Second".to_string()));
        app.handle_action(AppAction::Reorder(ReorderKind::Group(1), -1));
        assert_eq!(app.data.groups[0].name, "Second");
        assert_eq!(app.data.groups[1].name, "Deploy");
        assert_eq!(app.main_screen.group_list.selected, 0);
    }

    #[test]
    fn test_reorder_group_down() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.data.groups.push(Group::new("Second".to_string()));
        app.handle_action(AppAction::Reorder(ReorderKind::Group(0), 1));
        assert_eq!(app.data.groups[0].name, "Second");
        assert_eq!(app.data.groups[1].name, "Deploy");
        assert_eq!(app.main_screen.group_list.selected, 1);
    }

    #[test]
    fn test_reorder_group_up_boundary_noop() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::Reorder(ReorderKind::Group(0), -1));
        assert_eq!(app.data.groups[0].name, "Deploy");
    }

    #[test]
    fn test_reorder_set_up() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        let mut set2 = CommandSet::new("set2".to_string(), app.data.groups[0].id);
        set2.commands.push(crate::models::Command { position: 0, command: "cmd".to_string() });
        app.data.groups[0].sets.push(set2);
        app.handle_action(AppAction::Reorder(ReorderKind::Set(0, 1), -1));
        assert_eq!(app.data.groups[0].sets[0].name, "set2");
        assert_eq!(app.data.groups[0].sets[1].name, "Prod");
        assert_eq!(app.main_screen.set_list.selected, 0);
    }

    #[test]
    fn test_reorder_set_down_boundary_noop() {
        let mut app = make_app();
        app.data = make_data_with_one_group();
        app.handle_action(AppAction::Reorder(ReorderKind::Set(0, 0), 1));
        assert_eq!(app.data.groups[0].sets[0].name, "Prod");
    }

    #[test]
    fn test_reorder_variable_up() {
        use crate::models::Variable;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.variables.push(Variable { name: "a".to_string(), default_value: "".to_string() });
        set.variables.push(Variable { name: "b".to_string(), default_value: "".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::Reorder(ReorderKind::Variable(1), -1));
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.variables[0].name, "b");
        assert_eq!(ds.set.variables[1].name, "a");
        assert_eq!(ds.variable_list.selected, 0);
    }

    #[test]
    fn test_reorder_variable_up_boundary_noop() {
        use crate::models::Variable;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.variables.push(Variable { name: "a".to_string(), default_value: "".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::Reorder(ReorderKind::Variable(0), -1));
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.variables.len(), 1);
        assert_eq!(ds.set.variables[0].name, "a");
    }

    #[test]
    fn test_reorder_command_up_renumbers_positions() {
        use crate::models::Command;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(Command { position: 0, command: "echo first".to_string() });
        set.commands.push(Command { position: 1, command: "echo second".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::Reorder(ReorderKind::Command(1), -1));
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.commands[0].command, "echo second");
        assert_eq!(ds.set.commands[0].position, 0);
        assert_eq!(ds.set.commands[1].command, "echo first");
        assert_eq!(ds.set.commands[1].position, 1);
        assert_eq!(ds.command_list.selected, 0);
    }

    #[test]
    fn test_reorder_command_down() {
        use crate::models::Command;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(Command { position: 0, command: "a".to_string() });
        set.commands.push(Command { position: 1, command: "b".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::Reorder(ReorderKind::Command(0), 1));
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.commands[0].command, "b");
        assert_eq!(ds.set.commands[0].position, 0);
        assert_eq!(ds.set.commands[1].command, "a");
        assert_eq!(ds.set.commands[1].position, 1);
        assert_eq!(ds.command_list.selected, 1);
    }

    #[test]
    fn test_reorder_command_down_boundary_noop() {
        use crate::models::Command;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(Command { position: 0, command: "only".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        app.handle_action(AppAction::Reorder(ReorderKind::Command(0), 1));
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.commands.len(), 1);
        assert_eq!(ds.set.commands[0].command, "only");
    }
```

Add import to test module:
```rust
    use crate::action::{AppAction, DeleteKind, ReorderKind};
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test app::handler::tests::test_reorder_group_up`
Expected: FAIL — `Reorder` handler is a placeholder

- [ ] **Step 3: Implement Reorder handler in `handle_action`**

Replace the temporary placeholder:
```rust
            // Temporary: placeholder until Task 2
            AppAction::Reorder(_, _) => {}
```

with:

```rust
            AppAction::Reorder(kind, dir) => {
                let new_idx = |i: usize, len: usize| -> Option<usize> {
                    let c = i as isize + dir;
                    if c >= 0 && (c as usize) < len { Some(c as usize) } else { None }
                };
                match kind {
                    ReorderKind::Group(gi) => {
                        if let Some(ni) = new_idx(gi, self.data.groups.len()) {
                            self.data.groups.swap(gi, ni);
                            self.main_screen.group_list.selected = ni;
                            self.auto_save();
                            self.toasts.add("Group moved", ToastSeverity::Info);
                        }
                    }
                    ReorderKind::Set(gi, si) => {
                        if gi < self.data.groups.len()
                            && let Some(ni) = new_idx(si, self.data.groups[gi].sets.len())
                        {
                            self.data.groups[gi].sets.swap(si, ni);
                            self.main_screen.set_list.selected = ni;
                            self.auto_save();
                            self.toasts.add("Set moved", ToastSeverity::Info);
                        }
                    }
                    ReorderKind::Variable(idx) => {
                        if let Some(ref mut ds) = self.detail_screen
                            && let Some(ni) = new_idx(idx, ds.set.variables.len())
                        {
                            ds.set.variables.swap(idx, ni);
                            ds.variable_list.selected = ni;
                            self.auto_save();
                            self.toasts.add("Variable moved", ToastSeverity::Info);
                        }
                    }
                    ReorderKind::Command(idx) => {
                        if let Some(ref mut ds) = self.detail_screen
                            && let Some(ni) = new_idx(idx, ds.set.commands.len())
                        {
                            ds.set.commands.swap(idx, ni);
                            for (i, c) in ds.set.commands.iter_mut().enumerate() {
                                c.position = i;
                            }
                            ds.command_list.selected = ni;
                            self.auto_save();
                            self.toasts.add("Command moved", ToastSeverity::Info);
                        }
                    }
                }
            }
```

- [ ] **Step 4: Run handler tests**

Run: `cargo test app::handler::tests::test_reorder`
Expected: All 10 reorder tests PASS

- [ ] **Step 5: Run full handler test module**

Run: `cargo test app::handler::tests`
Expected: All tests PASS (existing 39 + 10 new = 49)

- [ ] **Step 6: Commit**

```bash
git add src/app/handler.rs
git commit -m "feat: handle Reorder action for all four list types

Bounds-checked swap for Group, Set, Variable, Command.
Commands additionally renumber position fields after swap.
Selected index follows the moved item.
10 new tests covering up/down/boundary for all types.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Main Screen handler — Ctrl+Up/Down + tests + status bar

**Files:**
- Modify: `src/ui/main_screen/handler.rs`
- Modify: `src/ui/main_screen/render.rs`

- [ ] **Step 1: Write failing tests**

In the test module, add imports:
```rust
    use crate::action::{AppAction, DeleteKind, ReorderKind};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
```

Replace the existing `use crossterm::event::KeyCode;` line.

Add these tests after the existing handler tests:

```rust
    #[test]
    fn test_ctrl_up_returns_reorder_group() {
        let mut state = MainScreenState::new();
        state.active_panel = Panel::Groups;
        state.group_list.selected = 1;
        let mut data = make_data();
        data.groups.push(Group::new("G2".to_string()));
        let ctrl_up = KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL);
        let action = state.handle_key(ctrl_up, &data);
        assert!(matches!(action, AppAction::Reorder(ReorderKind::Group(1), -1)));
    }

    #[test]
    fn test_ctrl_down_returns_reorder_set() {
        let mut state = MainScreenState::new();
        state.active_panel = Panel::Sets;
        state.set_list.selected = 0;
        let mut data = make_data();
        let set2 = CommandSet::new("S2".to_string(), data.groups[0].id);
        data.groups[0].sets.push(set2);
        let ctrl_down = KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL);
        let action = state.handle_key(ctrl_down, &data);
        assert!(matches!(action, AppAction::Reorder(ReorderKind::Set(0, 0), 1)));
    }

    #[test]
    fn test_ctrl_up_ignored_in_groups_when_no_group_selected() {
        let mut state = MainScreenState::new();
        // empty data — selected_group_idx returns None
        let empty_data = AppData::empty();
        let ctrl_up = KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL);
        let action = state.handle_key(ctrl_up, &empty_data);
        assert!(matches!(action, AppAction::None));
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test ui::main_screen::handler::tests::test_ctrl_up_returns_reorder_group`
Expected: FAIL — still returns `AppAction::None`

- [ ] **Step 3: Add Ctrl+Up/Down match arms in handler**

Insert BEFORE the existing plain `KeyCode::Up | ...` arm (before line 74):

```rust
            KeyCode::Up if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                match self.active_panel {
                    Panel::Groups if let Some(gi) = self.selected_group_idx(data) => {
                        return AppAction::Reorder(ReorderKind::Group(gi), -1);
                    }
                    Panel::Sets if let Some((gi, si)) = self.selected_set_idx(data) => {
                        return AppAction::Reorder(ReorderKind::Set(gi, si), -1);
                    }
                    _ => {}
                }
                AppAction::None
            }
            KeyCode::Down if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                match self.active_panel {
                    Panel::Groups if let Some(gi) = self.selected_group_idx(data) => {
                        return AppAction::Reorder(ReorderKind::Group(gi), 1);
                    }
                    Panel::Sets if let Some((gi, si)) = self.selected_set_idx(data) => {
                        return AppAction::Reorder(ReorderKind::Set(gi, si), 1);
                    }
                    _ => {}
                }
                AppAction::None
            }
```

Add `ReorderKind` import at top:
```rust
use crate::action::{AppAction, DeleteKind, ReorderKind};
```

- [ ] **Step 4: Run main_screen handler tests**

Run: `cargo test ui::main_screen::handler::tests`
Expected: All tests PASS (existing 11 + 3 new = 14)

- [ ] **Step 5: Update status bar text in render.rs**

In `src/ui/main_screen/render.rs`, line 260, change the else branch:

```rust
        let text = if self.rename_mode {
            "[Enter] Confirm  [Esc] Cancel — renaming group"
        } else if self.search_mode {
            "[Enter] Confirm  [Esc] Cancel  [↑/↓] Nav — searching"
        } else {
            "[↑/↓] Nav  [←/→] Panel  [Ctrl+↑/↓] Move  [Enter] Run  [e] Edit  [n] New  [R] Rename  [d] Del set  [D] Del group  [g] New group  [/] Search  [q] Quit"
        };
```

- [ ] **Step 6: Commit**

```bash
git add src/ui/main_screen/handler.rs src/ui/main_screen/render.rs
git commit -m "feat: main screen Ctrl+Up/Down reorders groups and sets

Ctrl+Up/Down arms placed before plain Up/Down so guards match first.
Status bar updated with [Ctrl+↑/↓] Move hint.
3 new tests: reorder group, reorder set, empty-data no-op.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: Detail Screen handler — Ctrl+Up/Down + tests + status bar

**Files:**
- Modify: `src/ui/detail_screen/handler.rs`
- Modify: `src/ui/detail_screen/render.rs`

- [ ] **Step 1: Write failing tests**

Add `ReorderKind` to test imports:
```rust
    use crate::action::{AppAction, DeleteKind, ReorderKind};
```

Add these tests after the existing ones:

```rust
    #[test]
    fn test_ctrl_up_returns_reorder_variable() {
        let mut state = make_state();
        state.set.variables.push(crate::models::Variable {
            name: "x".to_string(),
            default_value: "y".to_string(),
        });
        state.set.variables.push(crate::models::Variable {
            name: "z".to_string(),
            default_value: "w".to_string(),
        });
        state.focus = DetailFocus::Variables;
        state.variable_list.selected = 1;
        let ctrl_up = KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL);
        let action = state.handle_key(ctrl_up);
        assert!(matches!(action, AppAction::Reorder(ReorderKind::Variable(1), -1)));
    }

    #[test]
    fn test_ctrl_down_returns_reorder_command() {
        let mut state = make_state();
        state.set.commands.push(crate::models::Command {
            position: 0,
            command: "c1".to_string(),
        });
        state.set.commands.push(crate::models::Command {
            position: 1,
            command: "c2".to_string(),
        });
        state.focus = DetailFocus::Commands;
        state.command_list.selected = 0;
        let ctrl_down = KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL);
        let action = state.handle_key(ctrl_down);
        assert!(matches!(action, AppAction::Reorder(ReorderKind::Command(0), 1)));
    }

    #[test]
    fn test_ctrl_up_ignored_when_not_vars_or_cmds_focus() {
        let mut state = make_state();
        state.focus = DetailFocus::Name;
        let ctrl_up = KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL);
        let action = state.handle_key(ctrl_up);
        assert!(matches!(action, AppAction::None));
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test ui::detail_screen::handler::tests::test_ctrl_up_returns_reorder_variable`
Expected: FAIL — not yet implemented

- [ ] **Step 3: Add Ctrl+Up/Down match arms in handler**

Insert BEFORE the existing `KeyCode::Up` arm (before line 56):

```rust
            KeyCode::Up if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                match self.focus {
                    DetailFocus::Variables if !self.set.variables.is_empty() => {
                        let idx = self
                            .variable_list
                            .selected
                            .min(self.set.variables.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Variable(idx), -1);
                    }
                    DetailFocus::Commands if !self.set.commands.is_empty() => {
                        let idx = self
                            .command_list
                            .selected
                            .min(self.set.commands.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Command(idx), -1);
                    }
                    _ => {}
                }
            }
            KeyCode::Down if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                match self.focus {
                    DetailFocus::Variables if !self.set.variables.is_empty() => {
                        let idx = self
                            .variable_list
                            .selected
                            .min(self.set.variables.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Variable(idx), 1);
                    }
                    DetailFocus::Commands if !self.set.commands.is_empty() => {
                        let idx = self
                            .command_list
                            .selected
                            .min(self.set.commands.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Command(idx), 1);
                    }
                    _ => {}
                }
            }
```

Add `ReorderKind` to production imports at top:
```rust
use crate::action::{AppAction, DeleteKind, ReorderKind};
```

- [ ] **Step 4: Run detail_screen handler tests**

Run: `cargo test ui::detail_screen::handler::tests`
Expected: All tests PASS (existing 9 + 3 new = 12)

- [ ] **Step 5: Update status bar text in render.rs**

In `src/ui/detail_screen/render.rs`, lines 301-306, update Variables and Commands hint:

```rust
            (false, DetailFocus::Variables) => {
                "[a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Ctrl+↑/↓] Move  [Tab] Next  |  [Ctrl+S] Save"
            }
            (false, DetailFocus::Commands) => {
                "[a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Ctrl+↑/↓] Move  [Tab] Next  |  [Ctrl+S] Save"
            }
```

- [ ] **Step 6: Commit**

```bash
git add src/ui/detail_screen/handler.rs src/ui/detail_screen/render.rs
git commit -m "feat: detail screen Ctrl+Up/Down reorders variables and commands

Ctrl+Up/Down arms placed before plain Up/Down so guards match first.
Status bar updated with [Ctrl+↑/↓] Move hint for Variables and Commands.
3 new tests: reorder variable, reorder command, wrong-focus no-op.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Help screen update + integration test

**Files:**
- Modify: `src/ui/help_screen.rs`
- Modify: `src/integration_tests.rs`

- [ ] **Step 1: Add shortcuts to help screen**

In `src/ui/help_screen.rs`, add before `[↑/↓] Nav` in Main Screen section (line 18):

```rust
        Line::from("    Ctrl+Up/Down   Reorder group or set"),
```

Add in Detail Screen section (after `←/→` line, around line 33):

```rust
        Line::from("    Ctrl+Up/Down   Reorder variable or command"),
```

- [ ] **Step 2: Add integration test**

In `src/integration_tests.rs`, add after the delete confirmation test:

```rust
    // ------------------------------------------------------------------
    // 5.7 Command reorder flow
    // ------------------------------------------------------------------
    #[test]
    fn test_reorder_command_flow() {
        use crate::action::ReorderKind;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(crate::models::Command { position: 0, command: "first".to_string() });
        set.commands.push(crate::models::Command { position: 1, command: "second".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        // Move second command up
        app.handle_action(AppAction::Reorder(ReorderKind::Command(1), -1));
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.commands[0].command, "second");
        assert_eq!(ds.set.commands[0].position, 0);
        assert_eq!(ds.set.commands[1].command, "first");
        assert_eq!(ds.set.commands[1].position, 1);

        // Move first command down (back to original order)
        app.handle_action(AppAction::Reorder(ReorderKind::Command(0), 1));
        let ds2 = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds2.set.commands[0].command, "first");
        assert_eq!(ds2.set.commands[1].command, "second");
    }
```

- [ ] **Step 3: Run integration test**

Run: `cargo test integration_tests::tests::test_reorder_command_flow`
Expected: PASS

- [ ] **Step 4: Run full test suite**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 5: Run clippy**

Run: `cargo clippy`
Expected: No new warnings (pre-existing 2 warnings OK)

- [ ] **Step 6: Commit**

```bash
git add src/ui/help_screen.rs src/integration_tests.rs
git commit -m "feat: add reorder shortcuts to help screen and integration test

Help screen now shows Ctrl+Up/Down under Main and Detail sections.
Integration test covers full command reorder round-trip with position
renumbering verification.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: Mark feature as completed in memory

**Files:**
- Modify: `/home/william/.claude/projects/-home-william-Code-Rust-launcher/memory/feature-gap-priorities.md`

- [ ] **Step 1: Mark #6 as done**

Change:
```
- [ ] **#6 命令行重排序（上移/下移）** (~120 行)
```
to:
```
- [x] **#6 命令行重排序（上移/下移）** (~200 行)
  完成: 2026-06-17
```

- [ ] **Step 2: Commit memory update**

```bash
git add /home/william/.claude/projects/-home-william-Code-Rust-launcher/memory/feature-gap-priorities.md
git commit -m "docs: mark item reordering as completed in memory

Co-Authored-By: Claude <noreply@anthropic.com>"
```
