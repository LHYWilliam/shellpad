# Insert Preview Fix — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix command insert preview not displaying due to hardcoded `self.var_edit.insert_at` in shared renderer.

**Architecture:** Add `insert_at` parameter to `render_items_list`; callers pass their respective `InlineEdit.insert_at`. Single file change.

---

### Task 1: Fix `render_items_list` to accept `insert_at` parameter

**Files:** `src/ui/detail_screen.rs`

- [ ] **Add `insert_at` parameter to `render_items_list` signature**

Line 193-208: add `insert_at: Option<usize>` between `editing_item` and `item_fn`:

```rust
    fn render_items_list<F>(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        title: &str,
        focused: bool,
        count: usize,
        list: &ScrollableList,
        editing_item: Option<usize>,
        insert_at: Option<usize>,
        item_fn: F,
        preview_label: Option<String>,
        empty_text: &str,
    ) -> Rect
    where
        F: Fn(usize, bool) -> (String, Style),
```

- [ ] **Replace hardcoded `self.var_edit.insert_at` with the parameter**

Line 226-233: change `self.var_edit.insert_at` → `insert_at`:

```rust
        // Before:
        if let Some(idx) = editing_item
            && self.var_edit.insert_at.is_some()
            && let Some(label) = &preview_label
        {
            ...
            let pos = self.var_edit.insert_at.unwrap_or(idx.min(items.len()));
```

```rust
        // After:
        if let Some(idx) = editing_item
            && insert_at.is_some()
            && let Some(label) = &preview_label
        {
            ...
            let pos = insert_at.unwrap_or(idx.min(items.len()));
```

- [ ] **Update `render_variables` call site (line 254)**

Pass `self.var_edit.insert_at`:

```rust
        let list_area = self.render_items_list(
            frame, area, theme,
            &format!(" Variables ({}) ", count),
            self.focus == DetailFocus::Variables,
            count, &self.variable_list,
            self.var_edit.editing,
            self.var_edit.insert_at,
            |i, is_editing| { ... },
            preview,
            " (empty — press a to add a variable) ",
        );
```

- [ ] **Update `render_commands` call site (line 301)**

Pass `self.cmd_edit.insert_at`:

```rust
        let list_area = self.render_items_list(
            frame, area, theme,
            &format!(" Commands ({}) ", count),
            self.focus == DetailFocus::Commands,
            count, &self.command_list,
            self.cmd_edit.editing,
            self.cmd_edit.insert_at,
            |i, is_editing| { ... },
            preview,
            " (empty — press a to add a command) ",
        );
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "fix: parameterize insert_at in render_items_list instead of hardcoding var_edit"
```

---

### Verification

```bash
cargo test
cargo build
```
