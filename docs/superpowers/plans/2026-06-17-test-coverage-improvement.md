# Test Coverage Improvement — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 16 new tests across 4 files covering execution lifecycle, cross-module CRUD persistence, and cross-group move validation.

**Architecture:** Phase 1: extract `make_data_with_one_group` into `test_utils.rs` for reuse. Add integration tests for CRUD → SaveSet persistence. Add `app.rs` tests for `do_execute_with` and `teardown_execution`. Add `app/execution.rs` tests for `ExecutionManager`.

**Tech Stack:** Rust, existing test infrastructure (`make_app`, `make_key`, `AppAction` dispatch)

---

### Task 1: Extract `make_data_with_one_group` to test_utils and refactor handlers

**Files:**
- Modify: `src/test_utils.rs`
- Modify: `src/app/handler.rs`

- [ ] **Step 1: Move helper to test_utils.rs**

Add after `make_app()` in `src/test_utils.rs`:

```rust
/// Create AppData with one group containing one set (no commands).
pub(crate) fn make_data_with_one_group() -> AppData {
    let mut g = crate::models::Group::new("Deploy".to_string());
    let set = crate::models::CommandSet::new("Prod".to_string(), g.id);
    g.sets.push(set);
    AppData { groups: vec![g] }
}
```

- [ ] **Step 2: Replace local impl in handler.rs**

In `src/app/handler.rs` test module, remove the local `make_data_with_one_group` function (line 367-371) and replace the import:

```rust
    use crate::test_utils::{make_app, make_data_with_one_group, make_key};
```

- [ ] **Step 3: Verify tests still pass**

Run: `cargo test app::handler::tests`
Expected: All 50 tests PASS

- [ ] **Step 4: Commit**

```bash
git add src/test_utils.rs src/app/handler.rs
git commit -m "refactor: extract make_data_with_one_group into test_utils

Shared helper for creating AppData with 1 group + 1 set.
Replaces local copies in handler tests.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Integration tests — CRUD persistence (6 new tests)

**Files:**
- Modify: `src/integration_tests.rs`

- [ ] **Step 1: Add integration tests**

Add after `test_working_directory_lifecycle` (before the closing `}`):

```rust
    // ------------------------------------------------------------------
    // 5.9 SaveSet with group change moves to target
    // ------------------------------------------------------------------
    #[test]
    fn test_save_set_with_group_change_moves_to_target() {
        let mut app = make_app();
        let mut g1 = Group::new("Deploy".to_string());
        let set = CommandSet::new("Prod".to_string(), g1.id);
        g1.sets.push(set);
        let g2 = Group::new("Infra".to_string());
        app.data = AppData { groups: vec![g1, g2] };
        let set = app.data.groups[0].sets[0].clone();
        let groups = app.data.groups.clone();
        app.detail_screen = Some(DetailScreenState::new(set, groups));
        app.mode = AppMode::Detail;

        let mut saved = app.data.groups[0].sets[0].clone();
        saved.group_id = app.data.groups[1].id;
        app.handle_action(AppAction::SaveSet(saved));

        assert_eq!(app.mode, AppMode::Main);
        assert!(app.data.groups[0].sets.is_empty());
        assert_eq!(app.data.groups[1].sets.len(), 1);
        assert_eq!(app.data.groups[1].sets[0].group_id, app.data.groups[1].id);
    }

    // ------------------------------------------------------------------
    // 5.10 SaveSet with name change persists
    // ------------------------------------------------------------------
    #[test]
    fn test_save_set_with_name_change_persists() {
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        g.sets.push(CommandSet::new("Old".to_string(), g.id));
        app.data = AppData { groups: vec![g] };
        let set = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        let mut saved = app.data.groups[0].sets[0].clone();
        saved.name = "New Name".to_string();
        app.handle_action(AppAction::SaveSet(saved));

        assert_eq!(app.data.groups[0].sets[0].name, "New Name");
    }

    // ------------------------------------------------------------------
    // 5.11 Add variable + Save persists
    // ------------------------------------------------------------------
    #[test]
    fn test_add_variable_then_save_persists() {
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.variables.push(Variable { name: "host".to_string(), default_value: "".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        let mut saved = app.data.groups[0].sets[0].clone();
        saved.variables.push(Variable { name: "port".to_string(), default_value: "8080".to_string() });
        app.handle_action(AppAction::SaveSet(saved));

        assert_eq!(app.data.groups[0].sets[0].variables.len(), 2);
        assert_eq!(app.data.groups[0].sets[0].variables[1].name, "port");
    }

    // ------------------------------------------------------------------
    // 5.12 Delete variable + Save persists
    // ------------------------------------------------------------------
    #[test]
    fn test_delete_variable_then_save_persists() {
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.variables.push(Variable { name: "a".to_string(), default_value: "".to_string() });
        set.variables.push(Variable { name: "b".to_string(), default_value: "".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        let mut saved = app.data.groups[0].sets[0].clone();
        saved.variables.remove(0);
        app.handle_action(AppAction::SaveSet(saved));

        assert_eq!(app.data.groups[0].sets[0].variables.len(), 1);
        assert_eq!(app.data.groups[0].sets[0].variables[0].name, "b");
    }

    // ------------------------------------------------------------------
    // 5.13 Add command + Save persists
    // ------------------------------------------------------------------
    #[test]
    fn test_add_command_then_save_persists() {
        use crate::models::Command;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(Command { position: 0, command: "echo hi".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        let mut saved = app.data.groups[0].sets[0].clone();
        saved.commands.push(Command { position: 1, command: "echo bye".to_string() });
        app.handle_action(AppAction::SaveSet(saved));

        assert_eq!(app.data.groups[0].sets[0].commands.len(), 2);
        assert_eq!(app.data.groups[0].sets[0].commands[1].command, "echo bye");
    }

    // ------------------------------------------------------------------
    // 5.14 Delete command + Save persists
    // ------------------------------------------------------------------
    #[test]
    fn test_delete_command_then_save_persists() {
        use crate::models::Command;
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(Command { position: 0, command: "a".to_string() });
        set.commands.push(Command { position: 1, command: "b".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        let mut saved = app.data.groups[0].sets[0].clone();
        saved.commands.remove(0);
        app.handle_action(AppAction::SaveSet(saved));

        assert_eq!(app.data.groups[0].sets[0].commands.len(), 1);
        assert_eq!(app.data.groups[0].sets[0].commands[0].command, "b");
    }
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test integration_tests::tests::test_save_set_with_group_change_moves_to_target`
Expected: PASS

- [ ] **Step 3: Run all integration tests**

Run: `cargo test integration_tests::tests`
Expected: All tests PASS (existing 8 + 6 new = 14)

- [ ] **Step 4: Commit**

```bash
git add src/integration_tests.rs
git commit -m "test: add 6 CRUD persistence integration tests

Cover SaveSet with group change, name change, add/delete
variable, add/delete command. Verifies data reaches the
AppData model correctly after Detail Screen edits.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: app.rs tests — execution lifecycle (4 tests)

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add test module**

Add after the `Drop` impl (end of file):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Command, CommandSet, Group};
    use crate::test_utils::{make_app, make_data_with_one_group};
    use crate::ui::execution_screen::ExecutionScreenState;

    #[test]
    fn test_do_execute_with_launches_execution() {
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.commands.push(Command { position: 0, command: "echo ok".to_string() });
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };

        app.do_execute_with(0, 0, 0);

        assert_eq!(app.mode, AppMode::Execution);
        assert!(matches!(app.execution_state, ExecutionState::Running { .. }));
        if let ExecutionState::Running { pending_set, .. } = &app.execution_state {
            assert_eq!(*pending_set, (0, 0));
        }
    }

    #[test]
    fn test_do_execute_with_out_of_bounds_noop() {
        let mut app = make_app();
        app.do_execute_with(5, 5, 0);
        assert_eq!(app.mode, AppMode::Main);
        assert!(matches!(app.execution_state, ExecutionState::Idle { .. }));
    }

    #[test]
    fn test_teardown_execution_keep_screen_false_transitions_to_idle() {
        let mut app = make_app();
        let cmds = vec![Command { position: 0, command: "echo ok".to_string() }];
        app.execution_state = ExecutionState::Running {
            screen: Box::new(ExecutionScreenState::new("t".to_string(), &cmds)),
            manager: crate::app::execution::ExecutionManager::new(),
            pending_set: (0, 0),
        };
        app.mode = AppMode::Execution;

        app.teardown_execution(false, false);

        assert!(matches!(app.execution_state, ExecutionState::Idle { pending_set: None }));
    }

    #[test]
    fn test_teardown_execution_keep_screen_true_preserves() {
        let mut app = make_app();
        let cmds = vec![Command { position: 0, command: "echo ok".to_string() }];
        app.execution_state = ExecutionState::Running {
            screen: Box::new(ExecutionScreenState::new("t".to_string(), &cmds)),
            manager: crate::app::execution::ExecutionManager::new(),
            pending_set: (0, 0),
        };
        app.mode = AppMode::Execution;

        app.teardown_execution(true, true);

        // keep_screen=true means execution state stays Running
        assert!(matches!(app.execution_state, ExecutionState::Running { .. }));
    }
}
```

- [ ] **Step 2: Run app tests**

Run: `cargo test app::tests`
Expected: 4 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "test: add 4 execution lifecycle tests for app.rs

Cover do_execute_with (launches Running + noop OOB),
teardown_execution (transitions to Idle + keep_screen preserves).

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: app/execution.rs tests — ExecutionManager (3 tests)

**Files:**
- Modify: `src/app/execution.rs`

- [ ] **Step 1: Add test module**

Add after the `impl ExecutionManager` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionEvent;
    use crate::models::{Command, ExecMode, ShellCommand, Variable};
    use std::sync::mpsc;

    fn test_shell_cmd() -> ShellCommand {
        ShellCommand { program: "echo".to_string(), flag: "-n".to_string() }
    }

    #[test]
    fn test_execution_manager_start_sets_channel_and_handle() {
        let mut mgr = ExecutionManager::new();
        let cmds = vec![Command { position: 0, command: "ok".to_string() }];
        mgr.start(cmds, ExecMode::StopOnError, vec![], test_shell_cmd(), 0, None);

        assert!(mgr.rx.is_some());
        assert!(mgr.handle.is_some());
        assert!(!mgr.kill_signal.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn test_execution_manager_kill_flips_signal_and_nulls_rx() {
        let mut mgr = ExecutionManager::new();
        let cmds = vec![Command { position: 0, command: "echo ok".to_string() }];
        mgr.start(cmds, ExecMode::StopOnError, vec![], test_shell_cmd(), 0, None);

        mgr.kill();

        assert!(mgr.kill_signal.load(std::sync::atomic::Ordering::Relaxed));
        assert!(mgr.rx.is_none());
        // handle is None after join (done or joined by kill)
    }

    #[test]
    fn test_execution_manager_kill_twice_is_safe() {
        let mut mgr = ExecutionManager::new();
        let cmds = vec![Command { position: 0, command: "echo ok".to_string() }];
        mgr.start(cmds, ExecMode::StopOnError, vec![], test_shell_cmd(), 0, None);

        mgr.kill();
        mgr.kill(); // should not panic

        assert!(mgr.rx.is_none());
    }
}
```

- [ ] **Step 2: Run execution tests**

Run: `cargo test app::execution::tests`
Expected: 3 tests PASS

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: All tests PASS (212 + 4 + 6 + 3 = 225)

- [ ] **Step 4: Run clippy**

Run: `cargo clippy`
Expected: No new warnings

- [ ] **Step 5: Commit**

```bash
git add src/app/execution.rs
git commit -m "test: add 3 ExecutionManager lifecycle tests

Cover start (rx/handle/kill_signal state), kill (signal flip
+ rx null), and double-kill safety (no panic).

Co-Authored-By: Claude <noreply@anthropic.com>"
```
