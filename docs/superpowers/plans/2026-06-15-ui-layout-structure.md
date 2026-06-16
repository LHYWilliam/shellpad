# Wave 2: Layout & Visual Structure — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add title bar, scrollbar widgets, detail screen Properties section, execution screen command separators, and enhanced status bar styling.

**Architecture:** All changes build on the Theme system from Wave 1. Each layout addition is independent — tasks can be done in any order except Task 1 (title bar) which should be first since it changes the render area for all screens.

**Tech Stack:** ratatui 0.30.1 (Scrollbar, Gauge, Block, Layout), crossterm 0.29.0

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src/app.rs` | Modify | Add title bar rendering, adjust screen areas |
| `src/ui/main_screen.rs` | Modify | Add Scrollbar to Groups + Sets panels |
| `src/ui/detail_screen.rs` | Modify | Add Properties section block + Scrollbar |
| `src/ui/execution_screen.rs` | Modify | Add Scrollbar + command separators |

No new files needed for Wave 2.

---

### Task 1: Title Bar

**Files:**
- Modify: `src/app.rs` (lines 95-129, the `render()` method)

**Current state:** The `render()` method passes `area` (full terminal area) directly to each screen.

**Goal:** Add a 1-line title bar at the very top, shrink each screen's render area by 1 row.

- [ ] **Modify `render()` to add title bar**

Replace the current `render()` implementation (from `fn render(&mut self, frame: &mut Frame)` to the closing `}` after variable_screen render):

```rust
    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
            let warning = Paragraph::new(Line::from(format!(
                "Terminal too small: {}x{} (min: {}x{})",
                area.width, area.height, MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT
            )))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Red));
            frame.render_widget(warning, area);
            return;
        }

        // Split off title bar
        let layout = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]);
        let [title_area, content_area] = layout.areas(area);

        // Render title bar
        let mode_str = match self.mode {
            AppMode::Main => "Main",
            AppMode::Detail => "Edit",
            AppMode::Execution => "Run",
            AppMode::Help => "Help",
        };
        let group_count = self.data.groups.len();
        let set_count: usize = self.data.groups.iter().map(|g| g.sets.len()).sum();
        let title_text = format!(
            " Launcher  |  {}  |  {} groups, {} sets  |  ? Help  q Quit",
            mode_str, group_count, set_count,
        );
        let title_paragraph = Paragraph::new(Line::from(Span::styled(
            title_text,
            Style::default()
                .fg(theme.text_secondary)
                .add_modifier(Modifier::DIM),
        )));
        frame.render_widget(title_paragraph, title_area);

        // Render content in the remaining area (pass content_area instead of area)
        match self.mode {
            AppMode::Main => {
                self.main_screen.render(frame, content_area, &self.data, &self.theme);
            }
            AppMode::Detail => {
                if let Some(ref mut ds) = self.detail_screen {
                    ds.render(frame, content_area, &self.theme);
                }
            }
            AppMode::Execution => {
                if let Some(ref es) = self.exec_screen {
                    es.render(frame, content_area, &self.theme);
                }
            }
            AppMode::Help => {
                self.main_screen.render(frame, content_area, &self.data, &self.theme);
                draw_help(frame, content_area, &self.theme);
            }
        }

        self.variable_screen.render(frame, content_area, &self.theme);
    }
```

Need to add import for `Modifier` (check if already used). Also need `Span` (already imported).

- [ ] **Compile check**

```bash
cargo check 2>&1 | grep error
```

Expected: No errors.

- [ ] **Commit**

```bash
git add src/app.rs
git commit -m "feat(layout): add title bar with mode and stats"
```

---

### Task 2: Main Screen Scrollbars

**Files:**
- Modify: `src/ui/main_screen.rs`

**Goal:** Add `Scrollbar` widget to the Groups panel (left) and Sets panel (right).

- [ ] **Add Scrollbar import**

Replace the ratatui widgets import:
```rust
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
```
with:
```rust
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
```

- [ ] **Add scrollbar to `render_group_panel()`**

After the `frame.render_widget(&block, area);` line (line 144) and before getting `inner`, change the approach:

The key insight: the block is already rendered. After that, we need to split `inner` into list area + scrollbar area.

Replace the block of code from `let inner = block.inner(area);` to the end of the function, adding scrollbar:

```rust
        let inner = block.inner(area);
        frame.render_widget(&block, area);

        // Split inner area into list + scrollbar
        let inner_layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
        let [list_area, scrollbar_area] = inner_layout.areas(inner);

        let avail = list_area.width as usize;
        let mut items: Vec<ListItem> = data
            .groups
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let marker = if i == self.group_list.selected { "▶ " } else { "  " };
                let name = format!("{}{}", marker, g.name);
                let count = format!("({})", g.sets.len());
                let name_width = unicode_width::UnicodeWidthStr::width(name.as_str());
                let pad = avail.saturating_sub(name_width + count.len());
                let label = format!("{}{:>pad$}{}", name, "", count, pad = pad);
                let style = if i == self.group_list.selected {
                    Style::default()
                        .fg(theme.text_on_selected)
                        .bg(theme.selection_bg_primary)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text_primary)
                };
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

        if data.groups.is_empty() {
            items.push(
                ListItem::new(Line::from(Span::styled(
                    " (empty — press g to add) ",
                    Style::default().fg(theme.text_disabled).add_modifier(Modifier::ITALIC),
                ))),
            );
        }

        let mut list_state = ratatui::widgets::ListState::default()
            .with_selected(Some(self.group_list.selected));
        let list = List::new(items).highlight_style(
            Style::default()
                .fg(theme.text_on_selected)
                .bg(theme.selection_bg_primary)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_stateful_widget(list, list_area, &mut list_state);

        // Render scrollbar
        let content_len = data.groups.len();
        let mut scrollbar_state = ScrollbarState::new(content_len)
            .position(self.group_list.selected);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme.surface_border)),
            scrollbar_area,
            &mut scrollbar_state,
        );
```

Key changes:
- `avail` uses `list_area.width` instead of `inner.width`
- List renders into `list_area` not `inner`
- Scrollbar renders into `scrollbar_area`

- [ ] **Add scrollbar to `render_set_panel()`**

Similar change. After `frame.render_widget(&block, area);` and `let inner = block.inner(area);`, split inner:

Replace from `let inner = block.inner(area);` to end of function:

```rust
        let inner = block.inner(area);
        frame.render_widget(&block, area);

        // Split inner into list + scrollbar
        let inner_layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
        let [list_area, scrollbar_area] = inner_layout.areas(inner);

        let items: Vec<ListItem> = sets
            .iter()
            .enumerate()
            .map(|(i, &(gi, _, set))| {
                let shell_label = set.shell.label();
                let mode_label = match set.exec_mode {
                    crate::models::ExecMode::StopOnError => "🛑",
                    crate::models::ExecMode::ContinueOnError => "⏩",
                };
                let cmd_count = set.commands.len();
                let mut label = format!(
                    " {}  {}  [{}] ({} cmd)",
                    mode_label, set.name, shell_label, cmd_count
                );
                if self.search_mode {
                    let gname = data.groups.get(gi).map(|g| g.name.as_str()).unwrap_or("?");
                    let avail = list_area.width as usize;
                    let pad = avail.saturating_sub(label.len() + gname.len() + 1);
                    label = format!("{}{:>pad$}{}", label, "", gname, pad = pad);
                }
                let is_selected = i == self.set_list.selected
                    && self.active_panel == Panel::Sets;
                let style = if is_selected {
                    Style::default()
                        .fg(theme.text_on_selected)
                        .bg(theme.selection_bg_secondary)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text_primary)
                };
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

        let selected = if !sets.is_empty() && self.active_panel == Panel::Sets {
            Some(self.set_list.selected.min(sets.len().saturating_sub(1)))
        } else {
            None
        };
        let mut list_state = ratatui::widgets::ListState::default()
            .with_selected(selected);
        let list = List::new(items).highlight_style(
            Style::default()
                .fg(theme.text_on_selected)
                .bg(theme.selection_bg_secondary)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_stateful_widget(list, list_area, &mut list_state);

        // Render scrollbar
        let content_len = sets.len();
        let scroll_pos = selected.unwrap_or(0);
        let mut scrollbar_state = ScrollbarState::new(content_len)
            .position(scroll_pos);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme.surface_border)),
            scrollbar_area,
            &mut scrollbar_state,
        );
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "feat(layout): add scrollbar widgets to main screen panels"
```

---

### Task 3: Detail Screen Scrollbars

**Files:**
- Modify: `src/ui/detail_screen.rs`

**Goal:** Add Scrollbar to Variables and Commands sections.

- [ ] **Add Scrollbar import**

Replace:
```rust
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
```
with:
```rust
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
```

- [ ] **Add scrollbar to `render_variables()`**

Find the block inside `render_variables()` that starts with `let inner = var_block.inner(area);` and ends with `frame.render_stateful_widget(List::new(items), inner, &mut list_state);`

Replace the rendering section (from `let inner = var_block.inner(area);` to the closing `}` of the function):

```rust
        let inner = var_block.inner(area);
        frame.render_widget(&var_block, area);

        // Split into list + scrollbar
        let inner_layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
        let [list_area, scrollbar_area] = inner_layout.areas(inner);

        // ... (all the items building code stays exactly the same, just use list_area instead of inner)

        frame.render_stateful_widget(List::new(items), list_area, &mut list_state);

        // Scrollbar
        let content_len = self.set.variables.len();
        let scroll_pos = self.variable_list.selected.min(content_len.saturating_sub(1));
        let mut scrollbar_state = ScrollbarState::new(content_len)
            .position(scroll_pos);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme.surface_border)),
            scrollbar_area,
            &mut scrollbar_state,
        );
```

- [ ] **Add scrollbar to `render_commands()`**

Same pattern as variables. Replace the rendering section after `frame.render_widget(&cmd_block, area);`:

```rust
        let inner = cmd_block.inner(area);
        frame.render_widget(&cmd_block, area);

        // Split into list + scrollbar
        let inner_layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
        let [list_area, scrollbar_area] = inner_layout.areas(inner);

        // ... (all the items building code stays the same, use list_area)

        frame.render_stateful_widget(List::new(items), list_area, &mut list_state);

        // Scrollbar
        let content_len = self.set.commands.len();
        let scroll_pos = self.command_list.selected.min(content_len.saturating_sub(1));
        let mut scrollbar_state = ScrollbarState::new(content_len)
            .position(scroll_pos);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme.surface_border)),
            scrollbar_area,
            &mut scrollbar_state,
        );
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "feat(layout): add scrollbar widgets to detail screen variables and commands lists"
```

---

### Task 4: Execution Screen Scrollbar

**Files:**
- Modify: `src/ui/execution_screen.rs`

**Goal:** Add Scrollbar to the command output list.

- [ ] **Add Scrollbar import**

Replace:
```rust
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
```
with:
```rust
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
```

- [ ] **Add scrollbar to execution output list**

Find the block that renders the list (lines 267-271):
```rust
        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.surface_border));
        let list = List::new(items).block(list_block);
        frame.render_widget(list, list_area);
```

Replace with:
```rust
        // Split list area into list + scrollbar
        let list_inner_layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
        let [list_inner, scrollbar_area] = list_inner_layout.areas(list_area);

        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.surface_border));
        let list = List::new(items).block(list_block);
        frame.render_widget(list, list_inner);

        // Scrollbar
        let content_len = self.cmd_states.len();
        let mut scrollbar_state = ScrollbarState::new(content_len)
            .position(0);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme.surface_border)),
            scrollbar_area,
            &mut scrollbar_state,
        );
```

Note: The execution screen uses `render_widget(list)` not `render_stateful_widget(list)` since it doesn't track `ListState` — it builds a flat `Vec<ListItem>`. The scrollbar position is set to 0 (top) since there's no interactive scrolling in execution mode.

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/execution_screen.rs
git commit -m "feat(layout): add scrollbar to execution screen output"
```

---

### Task 5: Detail Screen Properties Section

**Files:**
- Modify: `src/ui/detail_screen.rs`

**Goal:** Wrap the 4 metadata rows (Name, Group, Shell, ExecMode) in a bordered Block with title "Properties".

- [ ] **Modify `render_metadata()`**

Replace the current `render_metadata` function:

```rust
    fn render_metadata(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.surface_border))
            .title(" Properties ");

        let inner = block.inner(area);
        frame.render_widget(&block, area);

        // Name, Group, Shell, ExecMode in rows inside the block
        let rows = Layout::vertical([Constraint::Length(1); 4]);
        let [name_row, group_row, shell_row, mode_row] = rows.areas(inner);

        // Name
        let is_name_focused = self.focus == DetailFocus::Name;
        let name_style = if is_name_focused {
            Style::default().fg(theme.accent_primary)
        } else {
            Style::default().fg(theme.text_primary)
        };
        let display_name = if self.editing_name {
            self.name_input.content.as_str()
        } else {
            self.set.name.as_str()
        };
        let name_text = format!(" Name: {}", display_name);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(name_text, name_style))),
            name_row,
        );

        // Group and Shell on the same row (side by side)
        let group_name = self
            .groups
            .iter()
            .find(|g| g.id == self.set.group_id)
            .map(|g| g.name.as_str())
            .unwrap_or("(unknown)");
        let group_style = if self.focus == DetailFocus::Group {
            Style::default().fg(theme.accent_primary)
        } else {
            Style::default().fg(theme.text_primary)
        };

        let shell_style = if self.focus == DetailFocus::Shell {
            Style::default().fg(theme.accent_primary)
        } else {
            Style::default().fg(theme.text_primary)
        };

        // Split the row into two halves
        let half_layout = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]);
        let [group_col, shell_col] = half_layout.areas(group_row);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" Group: {}", group_name),
                group_style,
            ))),
            group_col,
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" Shell: {}", self.set.shell.label()),
                shell_style,
            ))),
            shell_col,
        );

        // Mode (full width)
        let mode_style = if self.focus == DetailFocus::ExecMode {
            Style::default().fg(theme.accent_primary)
        } else {
            Style::default().fg(theme.text_primary)
        };
        let mode_text = format!(" Mode: {}", self.set.exec_mode.label());
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(mode_text, mode_style))),
            mode_row,
        );
    }
```

Also need to adjust the layout in `render()` — the metadata section height. Currently it's `Constraint::Length(6)` for 4 rows with some padding. Now the Properties block has borders (2 rows overhead), so it needs `Length(8)`.

Change in `render()`:
```rust
        let layout = Layout::vertical([
            Constraint::Length(6), // metadata (name, group, shell, mode)
```
to:
```rust
            Constraint::Length(8), // Properties block (4 rows + borders)
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "feat(layout): wrap detail screen metadata in Properties section block"
```

---

### Task 6: Execution Screen Command Separators

**Files:**
- Modify: `src/ui/execution_screen.rs`

**Goal:** Replace blank line separators between commands with visible `╌` separator lines.

- [ ] **Replace blank line separator with visible separator**

Find this code (around line 225):
```rust
            // Separator between commands
            if i + 1 < self.cmd_states.len() {
                items.push(ListItem::new(Line::from("")));
            }
```

Replace with:
```rust
            // Separator between commands
            if i + 1 < self.cmd_states.len() {
                let separator = "╌".repeat(area.width.saturating_sub(4) as usize);
                items.push(ListItem::new(Line::from(Span::styled(
                    separator,
                    Style::default().fg(theme.text_disabled).add_modifier(Modifier::DIM),
                ))));
            }
```

Note: `area` is available in the outer scope. If not, it may need to be passed. Actually, `area` is a parameter of the `render()` function and is available in the closure. Let's check — the for loop iterates `self.cmd_states.iter().enumerate()` within the render function where `area` is in scope, so this should work.

Wait, actually `area` is a `Rect` parameter and it is available. But `area.width` gives the full screen width. We need to use something that accounts for the block border. Actually, the list items are rendered inside the block which has borders (2 chars overhead) and the scrollbar (1 char). Let me use a fixed approach — just use `area.width.saturating_sub(6)` to be safe:

```rust
                let separator_width = list_area.width.saturating_sub(2) as usize;
                let separator = "╌".repeat(separator_width);
```

But `list_area` is defined later... Hmm, let me think. The separator is inside the List widget, and the List is rendered inside the block's inner area. The block's inner area is determined by the borders (2 chars). So the width available inside the list is `list_area.width - 2`. But we don't have `list_area` yet at this point.

A simpler approach: just use a fixed-width separator that's reasonable, or pass the available width. Actually, `list_area` is defined at line 265. The separator is built in the items loop which is BEFORE the list_area definition. So we can't reference it.

Let me use `area.width.saturating_sub(6)` — area is terminal width, minus 4 for borders (2 left + 2 right) and 2 for scrollbar and safety margin:

```rust
                let sep_width = area.width.saturating_sub(6) as usize;
                let separator = "╌".repeat(sep_width);
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/execution_screen.rs
git commit -m "feat(layout): add visible command separator lines in execution screen"
```

---

### Task 7: Status Bar Styling Enhancement

**Files:**
- Modify: `src/ui/main_screen.rs` (the `render_status_bar()` method)

**Goal:** Add background fill and top separator line to the main screen status bar.

- [ ] **Add import for `Line` if not already present**

Check current imports — `Line` is already used in `main_screen.rs`. The import is: `use ratatui::text::{Line, Span};` — yes, it's there.

- [ ] **Enhance `render_status_bar()`**

Replace the current method:

```rust
    fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = Line::from(Span::styled(
            " [↑/↓] Nav  [←/→] Panel  [Enter] Run  [e] Edit  [n] New  [d] Del set  [Shift+D] Del group  [g] Group  [/] Search  [?] Help  [q] Quit",
            Style::default().fg(theme.text_secondary).add_modifier(Modifier::DIM),
        ));
        frame.render_widget(Paragraph::new(text), area);
    }
```

with:

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

        // Status bar content
        let text = Line::from(Span::styled(
            " [↑/↓] Nav  [←/→] Panel  [Enter] Run  [e] Edit  [n] New  [d] Del set  [Shift+D] Del group  [g] Group  [/] Search  [?] Help  [q] Quit",
            Style::default().fg(theme.text_secondary).add_modifier(Modifier::DIM),
        ));
        let status_area = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1));
        frame.render_widget(Paragraph::new(text), status_area);
    }
```

This adds a `─` line across the top of the status bar and shifts the content down by 1.

**Note:** This also requires adjusting the layout in `render()` to account for the extra line. The `Constraint::Length(1)` for the status bar should become `Constraint::Length(2)`.

In the `render()` method, change:
```rust
        let vertical = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]);
```
to:
```rust
        let vertical = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]);
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "feat(layout): enhance status bar with top separator line"
```

---

### Task 8: Final Verification

- [ ] **Run full test suite**

```bash
cargo test
```

Expected: All 51 tests pass.

- [ ] **Run clippy**

```bash
cargo clippy 2>&1 | grep -E '^error'
```

Expected: No errors (only pre-existing warnings).

- [ ] **Build release**

```bash
cargo build
```
