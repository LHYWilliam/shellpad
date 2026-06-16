# Search Mode Interaction Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix three search mode bugs: no visible cursor, no Left/Right cursor movement, Enter not exiting search mode.

**Architecture:** Add `search_cursor: usize` to `MainScreenState`. When in search mode, render the search query with cursor in the status bar area (like rename mode). Handle Left/Right/Home/End in search mode. Fix Enter to always exit. All changes in `main_screen.rs`.

**Tech Stack:** ratatui 0.30.1, crossterm 0.29.0, unicode-width 0.2.2

---

### Task 1: Add search cursor state

**Files:**
- Modify: `src/ui/main_screen.rs`

- [ ] **Add `search_cursor` field to `MainScreenState`**

Replace the current definition (lines 64-72):
```rust
pub struct MainScreenState {
    pub group_list: ScrollableList,
    pub set_list: ScrollableList,
    pub active_panel: Panel,
    pub search_mode: bool,
    pub search_query: String,
    pub rename_mode: bool,
    pub rename_input: TextInput,
}
```
with:
```rust
pub struct MainScreenState {
    pub group_list: ScrollableList,
    pub set_list: ScrollableList,
    pub active_panel: Panel,
    pub search_mode: bool,
    pub search_query: String,
    pub search_cursor: usize,
    pub rename_mode: bool,
    pub rename_input: TextInput,
}
```

- [ ] **Initialize `search_cursor` in `new()`**

In the `Self {` block, add after `search_query: String::new(),`:
```rust
            search_cursor: 0,
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "feat(search): add search_cursor field to MainScreenState"
```

---

### Task 2: Show search query + cursor in status bar

**Files:**
- Modify: `src/ui/main_screen.rs`

- [ ] **Add `set_cursor_after_prefix` import**

Add `set_cursor_after_prefix` to the components import:
```rust
use crate::ui::components::{handle_text_input, set_cursor_after_prefix, ScrollableList, TextInput};
```

- [ ] **Render search query in status bar when in search mode**

In `render()`, find the rename mode block (lines 140-163). After the rename mode block and before the `else { self.render_status_bar(...) }`, add a search mode block:

Replace the status bar rendering section (from `if self.rename_mode {` to the else block):

Current code:
```rust
        // Status bar (or rename input when in rename mode)
        if self.rename_mode {
            let prefix = " Rename: ";
            let ren = &self.rename_input;
            let display = format!("{}{}", prefix, ren.content);
            let style = if ren.content.is_empty() {
                Style::default().fg(theme.text_disabled)
            } else {
                Style::default().fg(theme.text_primary)
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(display, style))),
                status_area,
            );
            let prefix_w = unicode_width::UnicodeWidthStr::width(prefix);
            let prefix_w2 = unicode_width::UnicodeWidthStr::width(prefix);
            set_cursor_after_prefix(
                frame,
                &ren.content,
                ren.cursor,
                prefix_w2 as u16,
                Rect::new(status_area.x, status_area.y, status_area.width, 1),
            );
        } else {
            self.render_status_bar(frame, status_area, theme);
        }
```

Replace with:
```rust
        // Status bar (or rename/search input)
        if self.rename_mode {
            let prefix = " Rename: ";
            let ren = &self.rename_input;
            let display = format!("{}{}", prefix, ren.content);
            let style = if ren.content.is_empty() {
                Style::default().fg(theme.text_disabled)
            } else {
                Style::default().fg(theme.text_primary)
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(display, style))),
                status_area,
            );
            let prefix_w = unicode_width::UnicodeWidthStr::width(prefix);
            set_cursor_after_prefix(
                frame,
                &ren.content,
                ren.cursor,
                prefix_w as u16,
                Rect::new(status_area.x, status_area.y, status_area.width, 1),
            );
        } else if self.search_mode {
            let prefix = " Search: ";
            let display = format!("{}{}", prefix, self.search_query);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    display,
                    Style::default().fg(theme.text_primary),
                ))),
                status_area,
            );
            let prefix_w = unicode_width::UnicodeWidthStr::width(prefix);
            set_cursor_after_prefix(
                frame,
                &self.search_query,
                self.search_cursor,
                prefix_w as u16,
                Rect::new(status_area.x, status_area.y, status_area.width, 1),
            );
        } else {
            self.render_status_bar(frame, status_area, theme);
        }
```

Note: The original code had a redundant `let prefix_w = ...` that was duplicated. I've cleaned it up — only one `prefix_w` calculation now.

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "feat(search): show search query with cursor in status bar"
```

---

### Task 3: Fix search mode keyboard handling

**Files:**
- Modify: `src/ui/main_screen.rs`

- [ ] **Replace the search mode key handler**

Find the search mode key handler (lines 430-472). Replace the entire block from `if self.search_mode {` to the closing `};`:

Current code:
```rust
        // Search mode
        if self.search_mode {
            return match key.code {
                KeyCode::Esc => {
                    self.search_mode = false;
                    self.search_query.clear();
                    self.set_list.reset();
                    self.active_panel = Panel::Groups;
                    MainScreenAction::None
                }
                KeyCode::Enter => {
                    let results = data.filter_sets(&self.search_query);
                    if let Some((gi, si, _)) = results.get(self.set_list.selected) {
                        self.group_list.selected = *gi;
                        self.set_list.selected = *si;
                        self.search_mode = false;
                        self.active_panel = Panel::Sets;
                    }
                    // If no results matched, stay in search mode
                    MainScreenAction::None
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.set_list.select_previous();
                    MainScreenAction::None
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    let n = data.filter_sets(&self.search_query).len();
                    self.set_list.select_next(n);
                    MainScreenAction::None
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.active_panel = Panel::Sets;
                    self.set_list.reset();
                    MainScreenAction::None
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.active_panel = Panel::Sets;
                    self.set_list.reset();
                    MainScreenAction::None
                }
                _ => MainScreenAction::None,
            };
        }
```

Replace with:
```rust
        // Search mode
        if self.search_mode {
            return match key.code {
                KeyCode::Esc => {
                    self.search_mode = false;
                    self.search_query.clear();
                    self.search_cursor = 0;
                    self.set_list.reset();
                    self.active_panel = Panel::Groups;
                    MainScreenAction::None
                }
                KeyCode::Enter => {
                    self.search_mode = false;
                    // Try to select the highlighted result
                    let results = data.filter_sets(&self.search_query);
                    if let Some((gi, si, _)) = results.get(self.set_list.selected) {
                        self.group_list.selected = *gi;
                        self.set_list.selected = *si;
                        self.active_panel = Panel::Sets;
                    }
                    self.search_query.clear();
                    self.search_cursor = 0;
                    MainScreenAction::None
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.set_list.select_previous();
                    MainScreenAction::None
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    let n = data.filter_sets(&self.search_query).len();
                    self.set_list.select_next(n);
                    MainScreenAction::None
                }
                KeyCode::Left => {
                    if self.search_cursor > 0 {
                        self.search_cursor = self.search_query[..self.search_cursor]
                            .floor_char_boundary(self.search_cursor - 1);
                    }
                    MainScreenAction::None
                }
                KeyCode::Right => {
                    let len = self.search_query.len();
                    let pos = self.search_query.floor_char_boundary(self.search_cursor);
                    if let Some(ch) = self.search_query[pos..].chars().next() {
                        self.search_cursor = (pos + ch.len_utf8()).min(len);
                    }
                    MainScreenAction::None
                }
                KeyCode::Home => {
                    self.search_cursor = 0;
                    MainScreenAction::None
                }
                KeyCode::End => {
                    self.search_cursor = self.search_query.len();
                    MainScreenAction::None
                }
                KeyCode::Char(c) => {
                    let pos = self.search_query.floor_char_boundary(self.search_cursor);
                    self.search_query.insert(pos, c);
                    self.search_cursor = pos + c.len_utf8();
                    self.active_panel = Panel::Sets;
                    self.set_list.reset();
                    MainScreenAction::None
                }
                KeyCode::Backspace => {
                    let pos = self.search_query.floor_char_boundary(self.search_cursor);
                    if pos > 0 {
                        let prev = self.search_query[..pos - 1].floor_char_boundary(pos - 1);
                        self.search_query.remove(prev);
                        self.search_cursor = prev;
                    }
                    self.active_panel = Panel::Sets;
                    self.set_list.reset();
                    MainScreenAction::None
                }
                KeyCode::Delete => {
                    let pos = self.search_query.floor_char_boundary(self.search_cursor);
                    if pos < self.search_query.len() {
                        self.search_query.remove(pos);
                        self.search_cursor = pos;
                    }
                    MainScreenAction::None
                }
                _ => MainScreenAction::None,
            };
        }
```

Key changes:
- `Enter`: **always** exits search mode (`self.search_mode = false`) and clears query. If a result is selected, jumps to it.
- `Left`/`Right`: moves `search_cursor` forward/backward with `floor_char_boundary` for Unicode safety
- `Home`/`End`: jumps to start/end
- `Char(c)`: **inserts** at cursor position (not just `.push()`)
- `Backspace`: **deletes before cursor** (not just `.pop()`)
- `Delete`: **deletes at cursor** (new functionality)

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "fix(search): add cursor movement, Enter always exits, insert at cursor"
```

---

### Task 4: Final Verification

- [ ] **Run full test suite**

```bash
cargo test
```

Expected: All tests pass (60+).

- [ ] **Run clippy**

```bash
cargo clippy 2>&1 | grep '^error'
```

Expected: No errors.

- [ ] **Build**

```bash
cargo build
```
