# UI Sync & Consistency Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 7 UI synchronization and consistency issues across 3 files.

**Architecture:** Each fix is an isolated change in one file. Tasks are ordered by file: main_screen.rs first, then detail_screen.rs, then execution_screen.rs.

**Tech Stack:** ratatui 0.30.1, unicode-width 0.2.2

---

### Task 1: Group Rename Real-Time Sync

**Files:**
- Modify: `src/ui/main_screen.rs` (line 214 in `render_group_panel`)

- [ ] **Replace the name building in `render_group_panel`**

Find the map closure body in `render_group_panel`:
```rust
            .map(|(i, g)| {
                let marker = if i == self.group_list.selected { "▶ " } else { "  " };
                let name = format!("{}{}", marker, g.name);
```

Replace with:
```rust
            .map(|(i, g)| {
                let marker = if i == self.group_list.selected { "▶ " } else { "  " };
                let display_name = if self.rename_mode && i == self.group_list.selected {
                    &self.rename_input.content
                } else {
                    &g.name
                };
                let name = format!("{}{}", marker, display_name);
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "fix: group rename now updates list in real-time"
```

---

### Task 2: Detail Screen Outer Title Sync

**Files:**
- Modify: `src/ui/detail_screen.rs` (line 59 in `render`)

- [ ] **Replace the block title in `render()`**

Find line 59:
```rust
            .title(format!(" Edit: {} ", self.set.name));
```

Replace with:
```rust
            .title(format!(" Edit: {} ",
                if self.editing_name { &self.name_input.content } else { &self.set.name }
            ));
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "fix: detail screen border title updates during name editing"
```

---

### Task 3: Properties Block Focus Border

**Files:**
- Modify: `src/ui/detail_screen.rs` (in `render_metadata`)

- [ ] **Add focus-aware border to Properties block**

Find the Properties block in `render_metadata`:
```rust
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.surface_border))
            .title(" Properties ");
```

Replace with:
```rust
        let props_focused = matches!(self.focus, DetailFocus::Name | DetailFocus::Group | DetailFocus::Shell | DetailFocus::ExecMode);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if props_focused {
                theme.accent_primary
            } else {
                theme.surface_border
            }))
            .title(" Properties ");
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "feat: Properties block border lights up on focus"
```

---

### Task 4: Name Editing Background Highlight

**Files:**
- Modify: `src/ui/detail_screen.rs` (in `render_metadata`)

- [ ] **Add background highlight for name editing**

Find the name style block in `render_metadata`:
```rust
        let name_style = if is_name_focused {
            Style::default().fg(theme.accent_primary)
        } else {
            Style::default().fg(theme.text_primary)
        };
```

Replace with:
```rust
        let name_style = if is_name_focused {
            if self.editing_name {
                Style::default()
                    .fg(theme.text_on_selected)
                    .bg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.accent_primary)
            }
        } else {
            Style::default().fg(theme.text_primary)
        };
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "feat: name editing gets background highlight like inline editing"
```

---

### Task 5: Empty-State Guidance

**Files:**
- Modify: `src/ui/main_screen.rs` (Sets panel in `render_set_panel`)
- Modify: `src/ui/detail_screen.rs` (Variables + Commands lists)

- [ ] **Add empty-state hint for Sets panel**

In `render_set_panel()`, after the items collection and before the `selected` calculation, add:

```rust
        // Empty-state hint
        if sets.is_empty() {
            let hint = ListItem::new(Line::from(Span::styled(
                " (empty — press n to add a set) ",
                Style::default().fg(theme.text_disabled).add_modifier(Modifier::ITALIC),
            )));
            // Add it as the only item so the list renders with proper height
            // We insert it conditionally only when sets are empty
        }
```

Find the code block after items.collect() (around lines 351-357), and before `let selected = ...`, add:

```rust
        // Empty-state hint when no sets
        if sets.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                " (empty — press n to add a set) ",
                Style::default().fg(theme.text_disabled).add_modifier(Modifier::ITALIC),
            ))));
        }

        let selected = if !sets.is_empty() && self.active_panel == Panel::Sets {
```

Note: When sets is empty, `selected` will be `None` and the list renders with no selection — that's correct.

- [ ] **Add empty-state hint for Variables list**

In `render_variables()`, after the items map and preview-insert blocks, before the `list_state` creation, add:

```rust
        if self.set.variables.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                " (empty — press a to add a variable) ",
                Style::default().fg(theme.text_disabled).add_modifier(Modifier::ITALIC),
            ))));
        }
```

Insert before the `let mut list_state = ...` line (around line 239).

- [ ] **Add empty-state hint for Commands list**

In `render_commands()`, same pattern:

```rust
        if self.set.commands.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                " (empty — press a to add a command) ",
                Style::default().fg(theme.text_disabled).add_modifier(Modifier::ITALIC),
            ))));
        }
```

Insert before the `let mut list_state = ...` line.

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs src/ui/detail_screen.rs
git commit -m "feat: add empty-state guidance for Sets, Variables, Commands"
```

---

### Task 6: Detail Screen Status Bar Separator

**Files:**
- Modify: `src/ui/detail_screen.rs` (in `render_status_bar`)

- [ ] **Read the current status bar layout height and method**

First, find the layout height for the status bar in `render()` (around line 69):
```rust
            Constraint::Length(1), // status bar
```

Change to:
```rust
            Constraint::Length(2), // status bar (separator + content)
```

- [ ] **Update `render_status_bar()` to add separator**

Find the method (around line 395):
```rust
    fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_editing = self.edit_state.is_editing();
        let status: String = if is_editing {
            ...
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                ...
                Style::default().fg(theme.text_secondary).add_modifier(Modifier::DIM),
            ))),
            area,
        );
    }
```

Replace with:
```rust
    fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Top separator line
        let sep = "─".repeat(area.width as usize);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                sep,
                Style::default().fg(theme.surface_border),
            ))),
            Rect::new(area.x, area.y, area.width, 1),
        );

        // Status content
        let is_editing = self.edit_state.is_editing();
        let status: String = if is_editing {
            format!(" Editing: {}  [Enter] Confirm  [Esc] Cancel", self.edit_state.edit_input.content)
        } else {
            match self.focus {
                DetailFocus::Name => "[Enter] Edit name  [Tab] Next".into(),
                DetailFocus::Group => "[←/→] Change group  [Tab] Next".into(),
                DetailFocus::Shell => "[←/→] Change shell  [Tab] Next".into(),
                DetailFocus::ExecMode => "[←/→] Change mode  [Tab] Next".into(),
                DetailFocus::Variables => "[a] Add  [e] Edit  [d] Delete  [Tab] Next".into(),
                DetailFocus::Commands => "[a] Add  [e] Edit  [d] Delete  [Tab] Next".into(),
            }
        };
        let status_area = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1));
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {}  |  [Ctrl+S] Save  [Esc] Cancel", status),
                Style::default().fg(theme.text_secondary).add_modifier(Modifier::DIM),
            ))),
            status_area,
        );
    }
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "feat: add separator line to detail screen status bar"
```

---

### Task 7: Execution Screen Output Block Title

**Files:**
- Modify: `src/ui/execution_screen.rs` (around line 311-315)

- [ ] **Add title to the output block**

Find the block:
```rust
        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.surface_border));
```

Replace with:
```rust
        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.surface_border))
            .title(" Output ");
```

- [ ] **Compile & test**

```bash
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/execution_screen.rs
git commit -m "feat: add Output title to execution screen block"
```

---

### Verification

- [ ] **Run full test suite**

```bash
cargo test
```

Expected: All 60 tests pass.

- [ ] **Run clippy**

```bash
cargo clippy 2>&1 | grep '^error'
```

Expected: No errors.

- [ ] **Build**

```bash
cargo build
```
