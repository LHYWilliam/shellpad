# Regression Bug Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 3 regression bugs: command insert cursor stuck, Sets panel green residue, NewSet selection after creation.

**Architecture:** Three independent one-line fixes across 3 files. All are regressions from prior refactoring tasks.

**Tech Stack:** ratatui 0.30.1

---

### T1. Fix command insert cursor stuck at wrong position

**Files:** `src/ui/detail_screen.rs:349`

**Root cause:** T3 (DetailEditState → two InlineEdits) missed one `var_edit` → `cmd_edit` replacement. The `render_inline_cursor` call for commands reads from `self.var_edit.edit_input` instead of `self.cmd_edit.edit_input`, so the cursor always points to stale var_edit content.

- [ ] Replace:
```rust
            render_inline_cursor(
                frame, list_area, self.command_list.offset,
                pos, &self.var_edit.edit_input,
                unicode_width::UnicodeWidthStr::width(display_prefix.as_str()) as u16,
            );
```
With:
```rust
            render_inline_cursor(
                frame, list_area, self.command_list.offset,
                pos, &self.cmd_edit.edit_input,
                unicode_width::UnicodeWidthStr::width(display_prefix.as_str()) as u16,
            );
```

- [ ] Test: `cargo test 2>&1 | tail -3`
- [ ] Commit: `fix: command inline edit cursor reads from cmd_edit not var_edit`

---

### T2. Fix Sets panel green residue after deletion

**Files:** `src/ui/main_screen.rs`

**Root cause:** T7 removed `highlight_style` from `List` widgets. Without `highlight_style`, the `List` widget does not manage background clearing of parent area when items shift. When a selected item is deleted, the next item slides into its place — but since the next item only styles its own text (no full-area background), the old selected background lingers.

**Fix:** Restore `highlight_style` on both Groups and Sets `List` widgets.

- [ ] Groups panel (line 192):
```rust
        let list = List::new(items).highlight_style(theme.selected_style(theme.selection_bg_primary));
```

- [ ] Sets panel (line 350):
```rust
        let list = List::new(items).highlight_style(theme.selected_style(theme.selection_bg_secondary));
```

- [ ] Test: `cargo test 2>&1 | tail -3`
- [ ] Commit: `fix: restore highlight_style to prevent background residue after deletion`

---

### T3. Fix NewSet selection after creation

**Files:** `src/app.rs`

**Root cause:** T1 (insert after selected) changed `push` to `insert(si, ...)` but did not update `self.main_screen.set_list.selected` to point to the newly created set. Old code set `selected = len - 1` via the `push` path.

- [ ] After the `insert` line (line 266), add:
```rust
                    self.main_screen.set_list.selected = si;
```

Full block:
```rust
            MainScreenAction::NewSet(gi) => {
                if gi < self.data.groups.len() {
                    let gid = self.data.groups[gi].id;
                    let set = CommandSet::new("New Command Set".to_string(), gid);
                    let si = (self.main_screen.set_list.selected + 1).min(self.data.groups[gi].sets.len());
                    self.data.groups[gi].sets.insert(si, set.clone());
                    self.main_screen.set_list.selected = si;
                    self.auto_save();
                    ...
```

- [ ] Test: `cargo test 2>&1 | tail -3`
- [ ] Commit: `fix: select newly created set after insert`

---

### Verification

```bash
cargo test
cargo build
```
