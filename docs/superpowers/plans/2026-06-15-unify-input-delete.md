# Unify Input Handling & Delete Logic — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Standardize 3 repetitive code patterns across the project: search mode input, name edit dispatch, and delete-after selection adjustment.

**Architecture:** Replace bare `search_query: String + search_cursor: usize` with `TextInput`. Replace hand-rolled name edit key dispatch with `handle_text_input()` helper. Unify 4 lists' delete-selection logic to `if selected >= len { selected = len.saturating_sub(1) }`.

**Tech Stack:** N/A — pure code reorganization

---

### Task 1: Search Mode Uses TextInput

**Files:**
- Modify: `src/ui/main_screen.rs`

**Current state:** `search_query: String` and `search_cursor: usize` hand-rolled. ~40 lines of manual string manipulation in handle_key.

**Goal:** Replace with `search_input: TextInput`, use `handle_text_input()` for key dispatch.

- [ ] **Replace fields in `MainScreenState`**

Change:
```rust
    pub search_mode: bool,
    pub search_query: String,
    pub search_cursor: usize,
```
To:
```rust
    pub search_mode: bool,
    pub search_input: TextInput,
```

- [ ] **Update constructor**

Change:
```rust
            search_mode: false,
            search_query: String::new(),
            search_cursor: 0,
```
To:
```rust
            search_mode: false,
            search_input: TextInput::new(String::new()),
```

- [ ] **Update `visible_sets()`**

Change:
```rust
            data.filter_sets(&self.search_query)
```
To:
```rust
            data.filter_sets(&self.search_input.content)
```

- [ ] **Update `render_set_panel()` title**

Change:
```rust
            " Search ".to_string()
```
(already hardcoded — no change needed, but verify it's correct)

- [ ] **Update search query line rendering**

Find the search query line in `render_set_panel()`:
```rust
                    format!(" Search: {} ", self.search_query),
```
Replace with:
```rust
                    format!(" Search: {} ", self.search_input.content),
```

And the cursor positioning:
```rust
                set_cursor_after_prefix(
                    frame,
                    &self.search_query,
                    self.search_cursor,
                    ...
```

Replace with:
```rust
                set_cursor_after_prefix(
                    frame,
                    &self.search_input.content,
                    self.search_input.cursor,
                    ...
```

- [ ] **Replace the search mode key handler**

Find the search mode `handle_key` block (currently a hand-rolled match with Left, Right, Home, End, Char, Backspace, Delete). Replace the ENTIRE search mode key handler:

```rust
        // Search mode
        if self.search_mode {
            return match key.code {
                KeyCode::Esc => {
                    self.search_mode = false;
                    self.search_input = TextInput::new(String::new());
                    self.set_list.reset();
                    self.active_panel = Panel::Groups;
                    MainScreenAction::None
                }
                KeyCode::Enter => {
                    let results = data.filter_sets(&self.search_input.content);
                    if let Some((gi, si, _)) = results.get(self.set_list.selected) {
                        self.group_list.selected = *gi;
                        self.set_list.selected = *si;
                        self.active_panel = Panel::Sets;
                    }
                    self.search_mode = false;
                    self.search_input = TextInput::new(String::new());
                    MainScreenAction::None
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.set_list.select_previous();
                    MainScreenAction::None
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    let n = data.filter_sets(&self.search_input.content).len();
                    self.set_list.select_next(n);
                    MainScreenAction::None
                }
                _ => {
                    handle_text_input(&mut self.search_input, key);
                    self.active_panel = Panel::Sets;
                    self.set_list.reset();
                    MainScreenAction::None
                }
            };
        }
```

Key changes:
- Uses `handle_text_input(&mut self.search_input, key)` for all text handling
- All cursor movement (Left/Right/Home/End) handled automatically
- `self.search_input.content` replaces `self.search_query` everywhere
- `self.search_input.cursor` replaces `self.search_cursor`

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "refactor: search mode uses TextInput and handle_text_input"
```

---

### Task 2: Name Edit Uses handle_text_input

**Files:**
- Modify: `src/ui/detail_screen.rs`

**Current state:** Name editing key dispatch (lines ~602-623) manually calls `insert_char`, `delete_before`, etc. on `self.name_input`.

**Goal:** Replace with `handle_text_input(&mut self.name_input, key)`.

- [ ] **Add import for `handle_text_input`**

Find the components import:
```rust
use crate::ui::components::{set_cursor_after_prefix, ScrollableList, TextInput};
```
Add `handle_text_input`:
```rust
use crate::ui::components::{handle_text_input, set_cursor_after_prefix, ScrollableList, TextInput};
```

- [ ] **Replace the name editing dispatch block**

Find the name editing section in `handle_key` (the block after `// Handle name editing`):
```rust
        // Handle name editing (Enter to confirm is handled in the outer match)
        if self.editing_name {
            match key.code {
                KeyCode::Char(c) => {
                    self.name_input.insert_char(c);
                }
                KeyCode::Backspace => {
                    self.name_input.delete_before();
                }
                KeyCode::Delete => {
                    self.name_input.delete_at();
                }
                KeyCode::Left => {
                    self.name_input.move_cursor_left();
                }
                KeyCode::Right => {
                    self.name_input.move_cursor_right();
                }
                KeyCode::Home => {
                    self.name_input.move_cursor_to_start();
                }
                KeyCode::End => {
                    self.name_input.move_cursor_to_end();
                }
                _ => {}
            }
        }
```

Replace with:
```rust
        // Handle name editing (Enter to confirm is handled in the outer match)
        if self.editing_name {
            handle_text_input(&mut self.name_input, key);
        }
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "refactor: name editing uses handle_text_input helper"
```

---

### Task 3: Unify Delete-Selection Logic

**Files:**
- Modify: `src/app.rs`

**Current state:**
- Groups/Sets: `if selected >= len { selected = len.saturating_sub(1) }`
- Variables/Commands: `list.selected = list.selected.min(len.saturating_sub(1))`

**Goal:** Variables and Commands use the same `if selected >= len` pattern as Groups/Sets.

- [ ] **Change Variables delete logic**

Find:
```rust
            DetailScreenAction::DeleteVariable(idx) => {
                if let Some(ref mut ds) = self.detail_screen
                    && idx < ds.set.variables.len()
                {
                    ds.set.variables.remove(idx);
                    let last = ds.set.variables.len().saturating_sub(1);
                    ds.variable_list.selected = ds.variable_list.selected.min(last);
                }
            }
```

Replace with:
```rust
            DetailScreenAction::DeleteVariable(idx) => {
                if let Some(ref mut ds) = self.detail_screen
                    && idx < ds.set.variables.len()
                {
                    ds.set.variables.remove(idx);
                    if ds.variable_list.selected >= ds.set.variables.len() {
                        ds.variable_list.selected =
                            ds.set.variables.len().saturating_sub(1);
                    }
                }
            }
```

- [ ] **Change Commands delete logic**

Find:
```rust
            DetailScreenAction::DeleteCommand(idx) => {
                if let Some(ref mut ds) = self.detail_screen
                    && idx < ds.set.commands.len()
                {
                    ds.set.commands.remove(idx);
                    for (i, c) in ds.set.commands.iter_mut().enumerate() {
                        c.position = i;
                    }
                    let last = ds.set.commands.len().saturating_sub(1);
                    ds.command_list.selected = ds.command_list.selected.min(last);
                }
            }
```

Replace with:
```rust
            DetailScreenAction::DeleteCommand(idx) => {
                if let Some(ref mut ds) = self.detail_screen
                    && idx < ds.set.commands.len()
                {
                    ds.set.commands.remove(idx);
                    for (i, c) in ds.set.commands.iter_mut().enumerate() {
                        c.position = i;
                    }
                    if ds.command_list.selected >= ds.set.commands.len() {
                        ds.command_list.selected =
                            ds.set.commands.len().saturating_sub(1);
                    }
                }
            }
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/app.rs
git commit -m "refactor: unify delete-selection logic across all 4 lists"
```

---

### Verification

- [ ] **Run full test suite**

```bash
cargo test
```

- [ ] **Run clippy**

```bash
cargo clippy 2>&1 | grep '^error'
```

- [ ] **Build**

```bash
cargo build
```
