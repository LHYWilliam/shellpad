# InlineEdit<T> + Insert Fix — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unify creation to insert after selected item for all lists; extract `InlineEdit` generic struct from `DetailEditState`.

**Architecture:** T1 fixes `app.rs` for groups/sets creation. T2 replaces `DetailEditState` with `InlineEdit` in `components.rs`. T3 migrates `DetailScreenState` and `detail_editor.rs` to use two `InlineEdit` instances.

**Tech Stack:** ratatui 0.30.1

---

### T1. Unify creation to insert after selected

**Files:** `src/app.rs`

- [ ] `NewSet` (line ~265): change `push` to insert at `selected + 1`
- [ ] `NewGroup` (line ~288): change `push` to insert at `selected + 1`

**For `NewSet`:**
```rust
// Before:
self.data.groups[gi].sets.push(set.clone());
// After:
let si = (self.main_screen.set_list.selected + 1).min(self.data.groups[gi].sets.len());
self.data.groups[gi].sets.insert(si, set.clone());
```

**For `NewGroup`:**
```rust
// Before:
.push(crate::models::Group::new(format!("Group {}", n)));
// After:
let gi = (self.main_screen.group_list.selected + 1).min(self.data.groups.len());
self.data.groups.insert(gi, crate::models::Group::new(format!("Group {}", n)));
```

- [ ] Compile & test: `cargo test 2>&1 | tail -3`
- [ ] Commit: `fix: insert new groups/sets after selected item instead of at end`

---

### T2. Define `InlineEdit` in components.rs

**Files:** `src/ui/components.rs`

- [ ] Add `InlineEdit` struct + impl after `ScrollableList`:

```rust
/// Generic inline text-edit state for a list of T.
#[derive(Clone)]
pub struct InlineEdit {
    pub editing: Option<usize>,   // index of item being edited (or None)
    pub edit_input: TextInput,
    pub insert_at: Option<usize>, // Some(pos) = inserting new item at pos
}

impl InlineEdit {
    pub fn new() -> Self {
        Self {
            editing: None,
            edit_input: TextInput::new(String::new()),
            insert_at: None,
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing.is_some()
    }

    /// Commit the edit, either inserting at `insert_at` position or replacing at `idx`.
    pub fn commit<T>(&mut self, idx: usize, items: &mut Vec<T>, new_item: T, list: &mut ScrollableList) {
        if let Some(insert_pos) = self.insert_at.take() {
            items.insert(insert_pos, new_item);
            list.selected = insert_pos;
        } else {
            items[idx] = new_item;
            list.selected = idx;
        }
    }

    /// Cancel the current edit.
    pub fn cancel(&mut self) {
        self.insert_at = None;
        self.editing = None;
    }

    /// Handle a plain text key event (delegates to handle_text_input).
    pub fn handle_key(&mut self, key: Event) {
        handle_text_input(&mut self.edit_input, key);
    }

    /// Handle a key event for variable editing (protects "key=" prefix).
    pub fn handle_variable_edit(
        &mut self,
        key: KeyEvent,
        idx: usize,
        items: &mut Vec<Variable>,
        list: &mut ScrollableList,
    ) -> DetailScreenAction {
        match key.code {
            KeyCode::Enter => {
                let input = self.edit_input.content.clone();
                if let Some(eq_pos) = input.find('=') {
                    let name = input[..eq_pos].trim().to_string();
                    let value = input[eq_pos + 1..].trim().to_string();
                    self.commit(idx, items, Variable { name, default_value: value }, list);
                } else if !input.is_empty() {
                    self.commit(idx, items, Variable { name: input.trim().to_string(), default_value: String::new() }, list);
                }
                self.editing = None;
                DetailScreenAction::None
            }
            KeyCode::Esc => {
                self.cancel();
                DetailScreenAction::None
            }
            _ => {
                let n = items.len();
                if (n > 0 || self.insert_at.is_some()) && self.editing.is_some() {
                    let protect = self.edit_input.content.find('=').map_or(0, |p| p + 1);
                    match key.code {
                        KeyCode::Backspace => { if self.edit_input.cursor > protect { self.edit_input.delete_before(); } }
                        KeyCode::Delete => { if self.edit_input.cursor > protect { self.edit_input.delete_at(); } }
                        KeyCode::Left => { if self.edit_input.cursor > protect { self.edit_input.move_cursor_left(); } }
                        KeyCode::Right => self.edit_input.move_cursor_right(),
                        KeyCode::Home => self.edit_input.move_cursor_to_start(),
                        KeyCode::End => self.edit_input.move_cursor_to_end(),
                        _ => { handle_text_input(&mut self.edit_input, key); }
                    }
                }
                DetailScreenAction::None
            }
        }
    }
}
```

- [ ] This requires importing `crossterm::event::KeyEvent`, `crossterm::event::KeyCode`, `crate::models::Variable` from `components.rs`. But `components.rs` shouldn't depend on `models`. 

**Design fix:** Instead of putting `handle_variable_edit` inside `InlineEdit`, keep it as a free function in `detail_editor.rs` that takes `&mut InlineEdit` + a closure for Enter behavior. OR better: keep `InlineEdit` generic (no Variable/Command knowledge), put the `=` prefix protection as a parameter:

```rust
/// Handle key with an optional prefix-protection byte position.
/// If `protect_prefix` is Some(pos), Backspace/Delete/Left are blocked
/// when the cursor is at or before `pos`.
pub fn handle_key_protected(
    &mut self,
    key: crossterm::event::KeyEvent,
    protect_prefix: Option<usize>,
) {
    let protect = protect_prefix.unwrap_or(0);
    match key.code {
        KeyCode::Backspace => { if self.edit_input.cursor > protect { self.edit_input.delete_before(); } }
        KeyCode::Delete => { if self.edit_input.cursor > protect { self.edit_input.delete_at(); } }
        KeyCode::Left => { if self.edit_input.cursor > protect { self.edit_input.move_cursor_left(); } }
        KeyCode::Right => self.edit_input.move_cursor_right(),
        KeyCode::Home => self.edit_input.move_cursor_to_start(),
        KeyCode::End => self.edit_input.move_cursor_to_end(),
        _ => { handle_text_input(&mut self.edit_input, key); }
    }
}
```

This way `InlineEdit` stays in `components.rs` with no model dependency. The `Enter`/`Esc` handling and `=` prefix setup stay in `detail_editor.rs`.

- [ ] Commit: `refactor: add InlineEdit generic struct to components.rs`

---

### T3. Replace DetailEditState with two InlineEdit instances

**Files:** `src/ui/detail_editor.rs`, `src/ui/detail_screen.rs`

- [ ] **Replace `detail_editor.rs`** — rewrite `handle_variable_edit` and `handle_command_edit` as free functions taking `&mut InlineEdit`:

```rust
use crate::models::{Command, Variable};
use crate::ui::components::{handle_text_input, InlineEdit, ScrollableList};
use crate::ui::detail_screen::DetailScreenAction;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_variable_edit(
    edit: &mut InlineEdit,
    key: KeyEvent,
    idx: usize,
    variables: &mut Vec<Variable>,
    list: &mut ScrollableList,
) -> DetailScreenAction {
    match key.code {
        KeyCode::Enter => {
            let input = edit.edit_input.content.clone();
            if let Some(eq_pos) = input.find('=') {
                let name = input[..eq_pos].trim().to_string();
                let value = input[eq_pos + 1..].trim().to_string();
                let var = Variable { name, default_value: value };
                edit.commit(idx, variables, var, list);
            } else if !input.is_empty() {
                let var = Variable { name: input.trim().to_string(), default_value: String::new() };
                edit.commit(idx, variables, var, list);
            }
            edit.editing = None;
            DetailScreenAction::None
        }
        KeyCode::Esc => {
            edit.cancel();
            DetailScreenAction::None
        }
        _ => {
            let n = variables.len();
            if (n > 0 || edit.insert_at.is_some()) && edit.editing.is_some() {
                edit.handle_key_protected(key, edit.edit_input.content.find('=').map(|p| p + 1));
            }
            DetailScreenAction::None
        }
    }
}

pub fn handle_command_edit(
    edit: &mut InlineEdit,
    key: KeyEvent,
    idx: usize,
    commands: &mut Vec<Command>,
    list: &mut ScrollableList,
) -> DetailScreenAction {
    match key.code {
        KeyCode::Enter => {
            let cmd = edit.edit_input.content.clone();
            edit.commit(idx, commands, Command { position: idx, command: cmd }, list);
            for (i, c) in commands.iter_mut().enumerate() {
                c.position = i;
            }
            edit.editing = None;
            DetailScreenAction::None
        }
        KeyCode::Esc => {
            edit.cancel();
            DetailScreenAction::None
        }
        _ => {
            edit.handle_key(key);
            DetailScreenAction::None
        }
    }
}
```

Wait — `InlineEdit::handle_key` takes a `crossterm::event::KeyEvent` — need to import. The free function `InlineEdit::handle_key` dispatches via `handle_text_input()`.

Actually I realize `handle_key_protected` and `handle_key` should be methods on `InlineEdit`.

- [ ] **Update `DetailScreenState`** — replace `pub edit_state: DetailEditState` with:

```rust
    pub var_edit: InlineEdit,
    pub cmd_edit: InlineEdit,
```

- [ ] **Update all .editing_variable → .var_edit.editing** etc. in detail_screen.rs

- [ ] **Update all .editing_command → .cmd_edit.editing** etc.

- [ ] **Update detail_screen.rs `handle_key`** — replace calls to `self.edit_state.handle_variable_edit` with free function calls:

```rust
            handle_variable_edit(&mut self.var_edit, key, idx, &mut self.set.variables, &mut self.variable_list)
```
```rust
            handle_command_edit(&mut self.cmd_edit, key, idx, &mut self.set.commands, &mut self.command_list)
```

- [ ] **Update all `self.edit_state.` → `self.var_edit.` or `self.cmd_edit.`** throughout detail_screen.rs

- [ ] Remove `DetailEditState` struct.

- [ ] Compile & test: `cargo test 2>&1 | tail -3`
- [ ] Commit: `refactor: replace DetailEditState with two InlineEdit instances`

---

### Verification

```bash
cargo test
cargo clippy 2>&1 | grep '^error'
cargo build
```
