# Status Bar Editing Text Removal + Sets Delete Fix — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove redundant "Editing: content" from detail screen status bar; fix Sets delete selection logic.

**Architecture:** Two independent one-line fixes in different files.

**Tech Stack:** N/A — pure logic fixes

---

### Task 1: Remove Editing Content from Status Bar

**Files:**
- Modify: `src/ui/detail_screen.rs`

- [ ] **Replace the editing status bar text**

Find in `render_status_bar`:
```rust
        let status: String = if is_editing {
            format!(" Editing: {}  [Enter] Confirm  [Esc] Cancel", self.edit_state.edit_input.content)
```

Replace with:
```rust
        let status: String = if is_editing {
            " [Enter] Confirm  [Esc] Cancel".into()
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "fix: remove redundant editing content from detail screen status bar"
```

---

### Task 2: Fix Sets Delete Selection Logic

**Files:**
- Modify: `src/app.rs`

- [ ] **Replace `set_list.reset()` with proper selection adjustment**

Find the `DeleteSet` handler:
```rust
            MainScreenAction::DeleteSet(gi, si) => {
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    self.data.groups[gi].sets.remove(si);
                    self.main_screen.set_list.reset();
                    if self.data.groups[gi].sets.is_empty() {
                        self.main_screen.active_panel = Panel::Groups;
                    }
                    self.auto_save();
                    self.push_toast("Set deleted", ToastSeverity::Info);
                }
            }
```

Replace with:
```rust
            MainScreenAction::DeleteSet(gi, si) => {
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    self.data.groups[gi].sets.remove(si);
                    if self.main_screen.set_list.selected >= self.data.groups[gi].sets.len() {
                        self.main_screen.set_list.selected =
                            self.data.groups[gi].sets.len().saturating_sub(1);
                    }
                    if self.data.groups[gi].sets.is_empty() {
                        self.main_screen.active_panel = Panel::Groups;
                    }
                    self.auto_save();
                    self.push_toast("Set deleted", ToastSeverity::Info);
                }
            }
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/app.rs
git commit -m "fix: Sets delete selection logic — keep/advance instead of reset to 0"
```

---

### Verification

- [ ] **Run full suite**

```bash
cargo test
cargo build
```
