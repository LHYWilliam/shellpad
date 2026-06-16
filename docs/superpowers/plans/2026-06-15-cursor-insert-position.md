# Insert Cursor Position Fix — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix cursor positioned at end of list when inserting variable/command in the middle.

**Architecture:** Two one-line fixes in `detail_screen.rs` — both cursor positioning blocks use the sentinel `editing_variable`/`editing_command` (which is `vec.len()`) for Y calculation instead of `insert_at`.

**Tech Stack:** N/A

---

### Task 1: Fix Variable Insert Cursor Position

**Files:**
- Modify: `src/ui/detail_screen.rs` (line 286)

- [ ] **Fix Y position calculation**

Find:
```rust
        if let Some(idx) = self.edit_state.editing_variable {
            let item_y = list_area.y + idx.saturating_sub(self.variable_list.offset) as u16;
```

Replace with:
```rust
        if let Some(idx) = self.edit_state.editing_variable {
            let pos = self.edit_state.insert_at.unwrap_or(idx);
            let item_y = list_area.y + pos.saturating_sub(self.variable_list.offset) as u16;
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "fix: variable insert cursor position uses insert_at instead of sentinel"
```

---

### Task 2: Fix Command Insert Cursor Position

**Files:**
- Modify: `src/ui/detail_screen.rs` (line 407)

- [ ] **Fix Y position calculation**

Find:
```rust
        if let Some(idx) = self.edit_state.editing_command {
            let item_y = list_area.y + idx.saturating_sub(self.command_list.offset) as u16;
```

Replace with:
```rust
        if let Some(idx) = self.edit_state.editing_command {
            let pos = self.edit_state.insert_at.unwrap_or(idx);
            let item_y = list_area.y + pos.saturating_sub(self.command_list.offset) as u16;
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "fix: command insert cursor position uses insert_at instead of sentinel"
```

---

### Verification

- [ ] **Run full test suite**

```bash
cargo test
cargo build
```
