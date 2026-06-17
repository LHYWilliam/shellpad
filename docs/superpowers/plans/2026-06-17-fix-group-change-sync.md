# Fix: Group change not synced to Main Screen — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the bug where changing a set's group in Detail Screen (←/→ on Group) and saving leaves the set orphaned in the old group's Vec.

**Architecture:** `SaveSet` handler detects `set.group_id != current_group.id`. If different: remove from old group, insert at end of target group, update `main_screen.group_list.selected` and `set_list.selected`, clamp old group's `set_list`.

**Tech Stack:** Rust

---

### Task: Fix SaveSet to move set across groups when group_id changed

**Files:**
- Modify: `src/app/handler.rs`

- [ ] **Step 1: Write failing test**

In `src/app/handler.rs` test module, add after `test_handler_save_set`:

```rust
    #[test]
    fn test_handler_save_set_moves_to_new_group() {
        let mut app = make_app();
        // Two groups, Deploy has one set
        let mut g1 = Group::new("Deploy".to_string());
        let set = CommandSet::new("Prod".to_string(), g1.id);
        g1.sets.push(set);
        let g2 = Group::new("Infra".to_string());
        app.data = AppData { groups: vec![g1, g2] };
        let set = app.data.groups[0].sets[0].clone();
        let groups = app.data.groups.clone();
        app.detail_screen = Some(DetailScreenState::new(set, groups));
        app.mode = AppMode::Detail;

        // Change group from Deploy to Infra
        let mut saved = app.data.groups[0].sets[0].clone();
        saved.group_id = app.data.groups[1].id;
        app.handle_action(AppAction::SaveSet(saved));

        // Should be moved to Infra
        assert!(app.detail_screen.is_none());
        assert_eq!(app.mode, AppMode::Main);
        assert!(app.data.groups[0].sets.is_empty()); // Deploy empty
        assert_eq!(app.data.groups[1].sets.len(), 1); // Infra has 1
        assert_eq!(app.data.groups[1].sets[0].name, "Prod");
        assert_eq!(app.data.groups[1].sets[0].group_id, app.data.groups[1].id);
        assert_eq!(app.main_screen.group_list.selected, 1); // Infra selected
        assert_eq!(app.main_screen.set_list.selected, 0); // first (only) set selected
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test app::handler::tests::test_handler_save_set_moves_to_new_group`
Expected: FAIL — set stays in Deploy (groups[0]) with wrong group_id

- [ ] **Step 3: Fix SaveSet handler**

Replace lines 173-184:

```rust
            // ---- Detail screen ----
            AppAction::SaveSet(set) => {
                let sid = set.id;
                let mut moved_gi = None;
                let mut moved_si = None;
                for (gi, group) in self.data.groups.iter_mut().enumerate() {
                    if let Some(pos) = group.sets.iter().position(|s| s.id == sid) {
                        if set.group_id != group.id {
                            // Group changed — remove from old, insert into new
                            group.sets.remove(pos);
                            if let Some(target) = self
                                .data
                                .groups
                                .iter_mut()
                                .find(|g| g.id == set.group_id)
                            {
                                target.sets.push(set);
                                moved_gi = self.data.groups.iter().position(|g| g.id == target.id);
                                moved_si = Some(target.sets.len().saturating_sub(1));
                            }
                        } else {
                            // Same group — replace in place
                            group.sets[pos] = set;
                            group.sets[pos].updated_at = chrono::Utc::now();
                        }
                        break;
                    }
                }
                // Update main screen selection if moved
                if let (Some(gi), Some(si)) = (moved_gi, moved_si) {
                    self.main_screen.group_list.selected = gi;
                    self.main_screen.set_list.selected = si;
                }
                self.detail_screen = None;
                self.mode = AppMode::Main;
                self.auto_save();
                self.toasts.add("Command set saved", ToastSeverity::Info);
            }
```

Note: `moved_gi` is computed as `self.data.groups.iter().position(|g| g.id == target.id)` — this works because `target` was obtained from `iter_mut()` on `self.data.groups`, so the index is correct. The code accesses `self.data.groups` while `target` borrows it... this will conflict.

Fix: find the target index differently:

```rust
            AppAction::SaveSet(set) => {
                let sid = set.id;
                let mut old_gi = None;
                let mut old_pos = None;
                // Find old position
                for (gi, group) in self.data.groups.iter().enumerate() {
                    if let Some(pos) = group.sets.iter().position(|s| s.id == sid) {
                        old_gi = Some(gi);
                        old_pos = Some(pos);
                        break;
                    }
                }
                let mut target_gi = None;
                let mut new_si = None;
                if let (Some(gi), Some(pos)) = (old_gi, old_pos) {
                    if set.group_id != self.data.groups[gi].id {
                        // Group changed — remove from old group
                        self.data.groups[gi].sets.remove(pos);
                        self.main_screen.set_list.clamp_selected(self.data.groups[gi].sets.len());
                        // Insert at end of target group
                        if let Some(ti) = self.data.groups.iter().position(|g| g.id == set.group_id) {
                            let mut moved = set;
                            moved.updated_at = chrono::Utc::now();
                            self.data.groups[ti].sets.push(moved);
                            target_gi = Some(ti);
                            new_si = Some(self.data.groups[ti].sets.len().saturating_sub(1));
                        }
                    } else {
                        // Same group — replace in place
                        self.data.groups[gi].sets[pos] = set;
                        self.data.groups[gi].sets[pos].updated_at = chrono::Utc::now();
                    }
                }
                // Update main screen selection if moved
                if let (Some(tgi), Some(si)) = (target_gi, new_si) {
                    self.main_screen.group_list.selected = tgi;
                    self.main_screen.set_list.selected = si;
                }
                self.detail_screen = None;
                self.mode = AppMode::Main;
                self.auto_save();
                self.toasts.add("Command set saved", ToastSeverity::Info);
            }
```

- [ ] **Step 4: Run handler tests**

Run: `cargo test app::handler::tests`
Expected: All tests PASS (49 + 1 new = 50)

- [ ] **Step 5: Run full test suite + clippy**

Run: `cargo test && cargo clippy`
Expected: All tests PASS, no new warnings

- [ ] **Step 6: Commit**

```bash
git add src/app/handler.rs
git commit -m "fix: move set to target group end when group_id changed in Detail

SaveSet now detects group_id mismatch and physically moves the set
from the old group's Vec to the end of the target group's Vec.
Updates main_screen group_list.selected and set_list.selected.
Clamps old group's set_list after removal.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
