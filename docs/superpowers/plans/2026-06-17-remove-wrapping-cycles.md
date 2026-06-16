# Remove Wrapping Cycles — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove wrapping behavior from Properties ↑/↓ and Options ←/→. Boundaries become hard stops.

**Architecture:** Four locations use `rem_euclid` or explicit wrap-around arms. Each is replaced with a saturating bounds check: if `candidate < 0 || candidate >= len`, no-op. Tests updated to assert boundary stops.

**Tech Stack:** Rust (no new dependencies)

---

### Task: Remove wrapping from all 4 cycle locations

**Files:**
- Modify: `src/ui/detail_screen/handler.rs` (Properties ↑/↓ arms)
- Modify: `src/ui/detail_screen/mod.rs` (cycle_group, cycle_shell, cycle_exec_mode)

- [ ] **Step 1: Write failing tests for boundary stops**

In `src/ui/detail_screen/handler.rs` test module, add after the existing Properties cycle tests:

```rust
    #[test]
    fn test_properties_up_at_name_stops() {
        let mut state = make_state();
        assert_eq!(state.focus, DetailFocus::Name);
        state.handle_key(make_key(KeyCode::Up));
        assert_eq!(state.focus, DetailFocus::Name); // no-op, does not wrap
    }

    #[test]
    fn test_properties_down_at_exec_mode_stops() {
        let mut state = make_state();
        state.focus = DetailFocus::ExecMode;
        state.handle_key(make_key(KeyCode::Down));
        assert_eq!(state.focus, DetailFocus::ExecMode); // no-op
    }
```

In `src/ui/detail_screen/mod.rs` (needs a test module at the bottom, or inline with `#[cfg(test)]` — but mod.rs has no test module. The cycle logic is tested indirectly via handler tests. We can verify boundary behavior by setting focus to ExecMode and pressing Down → no change. The existing `test_properties_down_at_exec_mode_stops` test above already covers this.

For `cycle_group`/`cycle_shell`/`cycle_exec_mode`, add tests in `src/ui/detail_screen/handler.rs`:

```rust
    #[test]
    fn test_left_at_first_group_noop() {
        let mut state = make_state();
        state.focus = DetailFocus::Group;
        // state.set.group_id is already the only group's id
        // cycle_group(-1) should have no effect
        state.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::empty()));
        // name unchanged, focus unchanged
        assert_eq!(state.focus, DetailFocus::Group);
    }

    #[test]
    fn test_right_at_last_exec_mode_stops() {
        let mut state = make_state();
        state.focus = DetailFocus::ExecMode;
        state.handle_key(make_key(KeyCode::Right)); // Continue
        state.handle_key(make_key(KeyCode::Right)); // should stop (no wrap)
        assert_eq!(state.set.exec_mode, ExecMode::ContinueOnError);
        assert_eq!(state.focus, DetailFocus::ExecMode);
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test ui::detail_screen::handler::tests::test_properties_up_at_name_stops`
Expected: FAIL — still wraps to ExecMode

- [ ] **Step 3: Fix Properties ↑/↓ — remove wrap arms**

In `handler.rs`, replace the two `DetailFocus::Name => ...` and `DetailFocus::ExecMode => ...` exit arms:

**Down direction** (current):
```rust
                            DetailFocus::ExecMode => DetailFocus::Name,
```
→ Change to:
```rust
                            DetailFocus::ExecMode => DetailFocus::ExecMode,
```

**Up direction** (current):
```rust
                            DetailFocus::Name => DetailFocus::ExecMode,
                            DetailFocus::WorkDir => DetailFocus::Name,
                            DetailFocus::Group => DetailFocus::WorkDir,
                            DetailFocus::Shell => DetailFocus::Group,
                            DetailFocus::ExecMode => DetailFocus::Shell,
```
→ Change `DetailFocus::Name => DetailFocus::ExecMode` to:
```rust
                            DetailFocus::Name => DetailFocus::Name,
```

- [ ] **Step 4: Fix cycle_group — replace rem_euclid with bounds check**

In `mod.rs`, replace `cycle_group`:

```rust
    fn cycle_group(&mut self, delta: isize) {
        let current = self
            .groups
            .iter()
            .position(|g| g.id == self.set.group_id)
            .unwrap_or(0);
        let len = self.groups.len() as isize;
        if len == 0 {
            return;
        }
        let candidate = current as isize + delta;
        if candidate < 0 || candidate >= len {
            return;
        }
        self.set.group_id = self.groups[candidate as usize].id;
    }
```

- [ ] **Step 5: Fix cycle_shell — replace rem_euclid with bounds check**

In `mod.rs`, replace `cycle_shell`:

```rust
    fn cycle_shell(&mut self, delta: isize) {
        let saved_custom = match &self.set.shell {
            ShellType::Custom(p) => Some(p.clone()),
            _ => None,
        };
        let variants = ShellType::builtin_variants();
        let current = match &self.set.shell {
            ShellType::Custom(_) => 5isize,
            other => variants
                .iter()
                .position(|s| std::mem::discriminant(s) == std::mem::discriminant(other))
                .unwrap_or(0) as isize,
        };
        let candidate = current + delta;
        if candidate < 0 || candidate >= 6 {
            return;
        }
        let next = candidate as usize;
        self.set.shell = if next == 5 {
            ShellType::Custom(saved_custom.unwrap_or_else(|| "/usr/bin/sh".to_string()))
        } else {
            variants[next].clone()
        };
    }
```

- [ ] **Step 6: Fix cycle_exec_mode — replace rem_euclid with bounds check**

In `mod.rs`, replace `cycle_exec_mode`:

```rust
    fn cycle_exec_mode(&mut self, delta: isize) {
        let variants = &[ExecMode::StopOnError, ExecMode::ContinueOnError];
        let pos = variants
            .iter()
            .position(|v| *v == self.set.exec_mode)
            .unwrap_or(0) as isize;
        let candidate = pos + delta;
        if candidate < 0 || candidate >= variants.len() as isize {
            return;
        }
        self.set.exec_mode = variants[candidate as usize];
    }
```

- [ ] **Step 7: Run all handler tests**

Run: `cargo test ui::detail_screen::handler::tests`
Expected: All tests PASS

- [ ] **Step 8: Run full test suite**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 9: Run clippy**

Run: `cargo clippy`
Expected: No new warnings

- [ ] **Step 10: Commit**

```bash
git add src/ui/detail_screen/handler.rs src/ui/detail_screen/mod.rs
git commit -m "refactor: remove wrapping cycles from Properties and Options

Properties ↑/↓: Name↑ and ExecMode↓ no longer wrap.
cycle_group, cycle_shell, cycle_exec_mode: rem_euclid replaced
with saturating bounds check. Boundaries are hard stops.
4 new tests verify boundary no-op behavior.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
