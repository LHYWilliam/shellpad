# Test Coverage Improvement — Design Spec

**Date:** 2026-06-17
**Status:** Approved
**Scope:** Fill critical test gaps in `app.rs`, `app/execution.rs`, `integration_tests.rs`, and `app/toast.rs`

## Problem

212 tests cover handlers, executor, widgets, and models well. However, three
areas with complex state transitions have zero or near-zero test coverage:

1. **`app.rs` — `do_execute_with` / `teardown_execution`** (0 tests): Execution
   lifecycle transitions (Idle → Running → Idle), working_dir passing, kill
   signal propagation, Drop handler. Any regression here is a crash.

2. **`app/execution.rs` — `ExecutionManager`** (0 tests): `start()`, `kill()`,
   `kill_signal` atomic state. Thread join behavior after kill.

3. **`integration_tests.rs`** (8 tests): 8 tests means zero cross-module
   coverage for the kind of state-consistency bugs we just fixed (group change
   not syncing to main screen). Every data mutation path through handler →
   auto_save → UI state needs at least one end-to-end test.

## Plan: 19 new tests across 3 files

### `src/app.rs` — 4 new tests

| # | Test | What it verifies |
|---|------|-----------------|
| 1 | `test_do_execute_with_launches_execution` | Calling `do_execute_with(0,0,0)` sets `ExecutionState::Running`, `mode = Execution` |
| 2 | `test_do_execute_with_passes_working_dir` | `set.working_dir = Some(...)` reaches `ExecutionManager::start` |
| 3 | `test_teardown_execution_mark_skipped` | `teardown_execution(true, true)` calls `screen.mark_remaining_as_skipped()` + transitions to Idle |
| 4 | `test_drop_teardowns_running_execution` | `Drop` impl calls `manager.kill()` when `ExecutionState::Running` |

### `src/app/execution.rs` — 3 new tests

| # | Test | What it verifies |
|---|------|-----------------|
| 5 | `test_execution_manager_start_sets_channel` | After `start()`, `rx` is `Some`, `handle` is `Some`, `kill_signal` is `false` |
| 6 | `test_execution_manager_kill_flips_signal_and_joins` | `kill()` sets `kill_signal = true`, `rx = None`, `handle = None` after join |
| 7 | `test_execution_manager_kill_twice_is_safe` | Calling `kill()` twice does not panic (handle is `None` on second call) |

### `src/integration_tests.rs` — 9 new tests

| # | Test | What it verifies |
|---|------|-----------------|
| 8 | `test_save_set_with_group_change_moves_to_target` | Same as the bug we just fixed — end-to-end validation |
| 9 | `test_save_set_with_name_change_persists` | Change name in Detail → SaveSet → `app.data.groups[...].name` matches |
| 10 | `test_add_variable_then_save_persists` | Add variable in Detail → SaveSet → stored in data |
| 11 | `test_delete_variable_then_save_persists` | Delete variable in Detail → SaveSet → removed from data |
| 12 | `test_add_command_then_save_persists` | Add command in Detail → SaveSet → stored in data |
| 13 | `test_delete_command_then_save_persists` | Delete command in Detail → SaveSet → removed from data |
| 14 | `test_delete_group_clears_active_panel` | Delete last group → `active_panel` reverts to Groups |
| 15 | `test_delete_last_set_in_group_switches_panel` | Delete last set → `active_panel` switches from Sets to Groups |
| 16 | `test_new_set_inserts_after_selected` | NewSet inserts at `selected + 1` |

### `src/app/toast.rs` — 3 new tests (later phase)

| # | Test | What it verifies |
|---|------|-----------------|
| 17 | `test_toast_add_and_retrieve` | `add()` pushes onto `toasts` vec |
| 18 | `test_toast_expiry_cleans_old` | `clean_expired()` removes expired toasts |
| 19 | `test_toast_severity_mapping` | Info/Success/Error map correctly |

---

**Phase 1: apps + execution + integration (16 tests)**
**Phase 2: toast (3 tests)** — lower priority

## Test patterns

All tests use existing helpers:

| Helper | File | Purpose |
|--------|------|---------|
| `make_app()` | `src/test_utils.rs` | Create minimal App |
| `make_data_with_one_group()` | `src/app/handler.rs` (move to `test_utils.rs`) | Create AppData with 1 group + 1 set |
| `make_key(KeyCode)` | `src/test_utils.rs` | Create KeyEvent |

For `do_execute_with` tests: commands must be echo-only (no side effects). Use `["echo ok"]` with `ExecMode::StopOnError`.

For `integration_tests`: use `handle_action(AppAction::Xxx)` to bypass key event simulation — actions are the public API, and this is how the app dispatches internally.

## Files Affected

| File | Change |
|------|--------|
| `src/app.rs` | Add `#[cfg(test)]` module with 4 tests |
| `src/app/execution.rs` | Add `#[cfg(test)]` module with 3 tests |
| `src/integration_tests.rs` | Add 9 new tests |
| `src/test_utils.rs` | Extract `make_data_with_one_group` helper |

Estimated: ~250 lines of test code. No production code changes.
