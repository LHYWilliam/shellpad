# Wave 3: Interaction Feedback — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add toast notifications, execution progress bar, execution auto-scroll, search highlighting, and variable input focus indicators.

**Architecture:** Toast system is a new module added to app.rs render lifecycle. Gauge and auto-scroll modify execution_screen.rs rendering. Search highlighting modifies main_screen.rs text rendering. Variable focus modifies variable_screen.rs styling. All build on Waves 1-2 theme infrastructure.

**Tech Stack:** ratatui 0.30.1 (Gauge, Clear, Span), crossterm 0.29.0

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src/ui/notification.rs` | **Create** | Toast struct, ToastSeverity enum |
| `src/ui/mod.rs` | Modify | Register notification module |
| `src/app.rs` | Modify | Toast field, push_toast/clean_toasts, auto_save -> &mut self, toast overlay rendering |
| `src/ui/execution_screen.rs` | Modify | Gauge progress bar in header, auto-scroll tracking |
| `src/ui/main_screen.rs` | Modify | Search keyword highlighting |
| `src/ui/variable_screen.rs` | Modify | Focus indicator (background/foreground swap) |

---

### Task 1: Toast Notification System

**Files:**
- Create: `src/ui/notification.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/app.rs`

- [ ] **Create `src/ui/notification.rs`**

```rust
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToastSeverity {
    Success,
    Error,
    Info,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub severity: ToastSeverity,
    pub created_at: Instant,
}

impl Toast {
    pub fn new(message: impl Into<String>, severity: ToastSeverity) -> Self {
        Self {
            message: message.into(),
            severity,
            created_at: Instant::now(),
        }
    }
}
```

- [ ] **Register module in `src/ui/mod.rs`**

Add `pub mod notification;` after the theme module line.

- [ ] **Add Toast imports and field to `App` struct in `src/app.rs`**

After `use crate::ui::theme::Theme;` add:
```rust
use crate::ui::notification::{Toast, ToastSeverity};
```

Add field to `App` struct after `theme: Theme,`:
```rust
    toasts: Vec<Toast>,
```

Initialize in `Self {` block constructor:
```rust
    toasts: Vec::new(),
```

- [ ] **Change `auto_save()` signature and add toast feedback**

Replace the auto_save method:
```rust
    fn auto_save(&mut self) {
        match storage::save_app_data(&self.data) {
            Ok(()) => self.push_toast("Saved", ToastSeverity::Success),
            Err(e) => self.push_toast(format!("Save failed: {}", e), ToastSeverity::Error),
        }
    }
```

- [ ] **Add `push_toast()` and `clean_toasts()` methods to `App`**

```rust
    fn push_toast(&mut self, message: impl Into<String>, severity: ToastSeverity) {
        self.toasts.push(Toast::new(message, severity));
    }

    fn clean_toasts(&mut self) {
        const TOAST_DURATION: std::time::Duration = std::time::Duration::from_secs(3);
        self.toasts.retain(|t| t.created_at.elapsed() < TOAST_DURATION);
    }
```

- [ ] **Call `clean_toasts()` in the event loop**

In `run()`, before `terminal.draw(|f| self.render(f))?;` (line 74), add:
```rust
            self.clean_toasts();
```

- [ ] **Render toasts in `render()` method**

After the `self.variable_screen.render(...)` line (after line 154), add toast rendering:

```rust
        // Render toast notification (bottom-right overlay)
        if let Some(toast) = self.toasts.last() {
            let (toast_fg, toast_label) = match toast.severity {
                ToastSeverity::Success => (self.theme.accent_success, " ✓ "),
                ToastSeverity::Error => (self.theme.accent_error, " ✗ "),
                ToastSeverity::Info => (self.theme.accent_info, " ● "),
            };
            let toast_msg = format!("{}{}", toast_label, toast.message);
            let toast_width = (toast_msg.len() as u16 + 2).min(area.width.saturating_sub(2));
            let x = area.width.saturating_sub(toast_width + 1);
            let toast_area = Rect::new(x, title_area.y, toast_width, 1);

            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    toast_msg,
                    Style::default()
                        .fg(toast_fg)
                        .add_modifier(Modifier::BOLD),
                ))),
                toast_area,
            );
        }
```

Note: `title_area` is in scope inside `render()` — the toast overlays the right side of the title bar.

- [ ] **Update all `auto_save()` call sites to reflect `&mut self`**

The `auto_save()` call sites are:
- Line 169: `self.auto_save()` in `handle_variable_action` — already `&mut self`
- Line 208: `self.auto_save()` in `NewSet` — already `&mut self`
- Line 222: `self.auto_save()` in `DeleteSet` — already `&mut self`
- Line 233: `self.auto_save()` in `NewGroup` — already `&mut self`
- Line 238: `self.auto_save()` in `RenameGroup` — already `&mut self`
- Line 253: `self.auto_save()` in `DeleteGroup` — already `&mut self`
- Line 275: `self.auto_save()` in `Save` — already `&mut self`
- Line 169: `self.auto_save()` in `handle_variable_action` — already `&mut self`

All callers already have `&mut self` access, so changing `auto_save(&self)` to `auto_save(&mut self)` won't cause issues.

- [ ] **Add toast on execution completion**

In `on_exec_action()`, when `BackToMain` is triggered (line 324-327), add a toast before killing:

```rust
            ExecutionScreenAction::BackToMain => {
                if let Some(ref es) = self.exec_screen {
                    if es.completed {
                        let summary = format!(
                            "Done: {}/{} completed",
                            es.succeeded + es.failed + es.skipped,
                            es.total,
                        );
                        self.push_toast(summary, ToastSeverity::Success);
                    }
                }
                self.kill_execution();
                self.mode = AppMode::Main;
            }
```

- [ ] **Add toast on delete**

In `on_main_action()`, after `DeleteSet` (line 214-223), add:
```rust
                self.push_toast("Set deleted", ToastSeverity::Info);
```

After `DeleteGroup` (line 240-254), add:
```rust
                self.push_toast("Group deleted", ToastSeverity::Info);
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/notification.rs src/ui/mod.rs src/app.rs
git commit -m "feat(ux): add toast notification system with save/delete/exec feedback"
```

---

### Task 2: Execution Progress Bar (Gauge)

**Files:**
- Modify: `src/ui/execution_screen.rs`

- [ ] **Add Gauge import**

Replace:
```rust
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
```
with:
```rust
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
```

- [ ] **Modify header layout to include Gauge**

Replace the header layout in `render()` (lines 156-157):

From:
```rust
        let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]);
        let [header_area, body_area] = vertical.areas(area);
```

To:
```rust
        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)]);
        let [header_area, gauge_area, body_area] = vertical.areas(area);
```

- [ ] **Add Gauge rendering between header and body**

After the header paragraph rendering (`frame.render_widget(header, header_area);`) and before the body rendering, add:

```rust
        // Gauge progress bar
        let completed_count = self.succeeded + self.failed + self.skipped;
        let progress = if self.total > 0 {
            completed_count as f64 / self.total as f64
        } else {
            0.0
        };
        let gauge_label = format!(" {}/{}  ", completed_count, self.total);
        let gauge = Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(theme.accent_success)
                    .bg(theme.surface),
            )
            .percent((progress * 100.0) as u16)
            .label(gauge_label);
        frame.render_widget(gauge, gauge_area);
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/execution_screen.rs
git commit -m "feat(ux): add execution progress bar (Gauge)"
```

---

### Task 3: Execution Auto-Scroll

**Files:**
- Modify: `src/ui/execution_screen.rs`

- [ ] **Add scroll tracking fields to `ExecutionScreenState`**

After `pub total_duration_ms: Option<u128>,` add:
```rust
    pub auto_scroll: bool,
    pub scroll_offset: usize,
```

Initialize in `new()` before the closing `}`:
```rust
    auto_scroll: true,
    scroll_offset: 0,
```

- [ ] **Update `reset_from()` to reset scroll state**

In `reset_from()`, add at the start:
```rust
    self.auto_scroll = true;
    self.scroll_offset = 0;
```

- [ ] **Calculate scroll offset based on current command position**

Add a helper method to `ExecutionScreenState`:
```rust
    /// Calculate the flat items Vec index for a given command index.
    fn items_offset_for_command(&self, cmd_idx: usize) -> usize {
        let mut offset = 0;
        for i in 0..cmd_idx.min(self.cmd_states.len()) {
            offset += 1; // command header
            offset += self.cmd_states[i].output_lines.len(); // output
            offset += 1; // separator
        }
        offset
    }
```

- [ ] **Update `process_events()` to track scroll**

In the `ExecutionEvent::Starting { index, command }` match arm, after updating `current_index`, add:
```rust
                if self.auto_scroll {
                    self.scroll_offset = self.items_offset_for_command(index);
                }
```

- [ ] **Convert List rendering to stateful with scroll offset**

Replace the list rendering (from after `let list = List::new(items);` to the end of scrollbar rendering):

```rust
        // Scrollbar
        let content_len = self.cmd_states.len();
        let mut scrollbar_state = ScrollbarState::new(content_len)
            .position(0);
        // ...
```

Replace the whole scrollbar section to use a selected position based on current_index:

```rust
        // Use ListState with offset for auto-scroll
        let mut list_state = ratatui::widgets::ListState::default()
            .with_offset(self.scroll_offset);
        frame.render_stateful_widget(List::new(items), content_area, &mut list_state);

        let content_len = self.cmd_states.len();
        let mut scrollbar_state = ScrollbarState::new(content_len)
            .position(self.current_index);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme.surface_border)),
            scrollbar_area,
            &mut scrollbar_state,
        );
```

Wait, I need to be more careful here. The scroll position for the scrollbar should track the current command index, not the flat items offset. The ScrollbarState tracks content_len as number of commands, and position as current command index.

Actually, the scrollbar previously tracked 0 position. Let me just use current_index for the scrollbar position now, which is more useful.

Let me re-read the current code to write the exact replacement.

The current code (after earlier edits) is:
```rust
        let list = List::new(items);
        frame.render_widget(list, content_area);

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

I need to change it to use ListState and update scrollbar position:

```rust
        // Use ListState with offset for auto-scroll
        let mut list_state = ratatui::widgets::ListState::default()
            .with_offset(self.scroll_offset);
        frame.render_stateful_widget(List::new(items), content_area, &mut list_state);

        // Scrollbar (track current command)
        let content_len = self.cmd_states.len();
        let scroll_pos = self.current_index.min(content_len.saturating_sub(1));
        let mut scrollbar_state = ScrollbarState::new(content_len)
            .position(scroll_pos);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme.surface_border)),
            scrollbar_area,
            &mut scrollbar_state,
        );
```

- [ ] **Add `z` key to toggle auto-scroll**

In `handle_key()`, add before the final `_` match arm:
```rust
            KeyCode::Char('z') => {
                self.auto_scroll = !self.auto_scroll;
                ExecutionScreenAction::None
            }
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/execution_screen.rs
git commit -m "feat(ux): add execution auto-scroll with z toggle"
```

---

### Task 4: Search Highlighting

**Files:**
- Modify: `src/ui/main_screen.rs`

**Goal:** When in search mode, highlight the matched query text in set names using a different color.

- [ ] **Find the search highlighting location**

In `render_set_panel()`, the sets are iterated to build `ListItem`s. Each set name is rendered inside `format!(" {}  {}  [{}] ({} cmd)", mode_label, set.name, shell_label, cmd_count)`. The search query is available as `self.search_query`.

- [ ] **Replace set name rendering with highlighted version**

In `render_set_panel()`, find the label building code:
```rust
                let mut label = format!(
                    " {}  {}  [{}] ({} cmd)",
                    mode_label, set.name, shell_label, cmd_count
                );
```

Replace with a version that uses `Span::styled` for the matched portion of the set name:

```rust
                // Build label with search highlighting
                let base_label = format!(
                    " {}  ",
                    mode_label,
                );
                let shell_part = format!("  [{}] ({} cmd)", shell_label, cmd_count);

                // For search highlighting, split the set name into matched/unmatched parts
                let (name_spans, suffix) = if self.search_mode && !self.search_query.is_empty() {
                    let query = self.search_query.as_str();
                    // Find all occurrences of the query in set.name (case-insensitive)
                    let lower_name = set.name.to_lowercase();
                    let lower_q = query.to_lowercase();
                    let mut spans = Vec::new();
                    let mut last_end = 0;
                    if let Some(pos) = lower_name[last_end..].find(&lower_q) {
                        let actual_pos = last_end + pos;
                        // Text before match
                        if actual_pos > 0 {
                            spans.push(Span::styled(
                                &set.name[..actual_pos],
                                Style::default().fg(theme.text_primary),
                            ));
                        }
                        // Matched text
                        spans.push(Span::styled(
                            &set.name[actual_pos..actual_pos + query.len()],
                            Style::default()
                                .fg(theme.accent_primary)
                                .add_modifier(Modifier::BOLD),
                        ));
                        last_end = actual_pos + query.len();
                    }
                    // Remaining text
                    if last_end < set.name.len() {
                        spans.push(Span::styled(
                            &set.name[last_end..],
                            Style::default().fg(theme.text_primary),
                        ));
                    }
                    if spans.is_empty() {
                        spans.push(Span::styled(
                            set.name.clone(),
                            Style::default().fg(theme.text_primary),
                        ));
                    }
                    (spans, String::new())
                } else {
                    (vec![Span::styled(
                        set.name.clone(),
                        Style::default().fg(theme.text_primary),
                    )], String::new())
                };

                // Build the full label as styled spans
                let mut parts = vec![Span::styled(
                    base_label,
                    Style::default().fg(if is_selected { theme.text_on_selected } else { theme.text_primary }),
                )];
                parts.extend(name_spans);
                parts.push(Span::styled(
                    shell_part,
                    Style::default().fg(if is_selected { theme.text_on_selected } else { theme.text_primary }),
                ));
```

And change the `ListItem` construct to use `Line::from(parts)` instead of `Line::from(Span::styled(label, style))`:

```rust
                // Before:
                // ListItem::new(Line::from(Span::styled(label, style)))
                // After:
                ListItem::new(Line::from(parts))
```

**Important:** This changes the single-span-per-item approach to multi-span. The `is_selected` check must still apply — for selected items, all text should use `theme.text_on_selected` foreground.

Let me simplify. Instead of the complex multi-span approach with individual styling per segment, here's a simpler version:

```rust
                let base_label = format!(" {}  ", mode_label);
                let shell_part = format!("  [{}] ({} cmd)", shell_label, cmd_count);

                // Build name part with search highlighting
                let mut name_parts: Vec<Span> = if self.search_mode && !self.search_query.is_empty() {
                    let query = self.search_query.as_str();
                    let lower_name = set.name.to_lowercase();
                    let lower_q = query.to_lowercase();
                    let mut spans = Vec::new();
                    let mut last_end = 0;
                    while let Some(pos) = lower_name[last_end..].find(&lower_q) {
                        let match_start = last_end + pos;
                        if match_start > last_end {
                            spans.push(Span::styled(
                                &set.name[last_end..match_start],
                                Style::default().fg(theme.text_primary),
                            ));
                        }
                        let match_end = match_start + query.len();
                        spans.push(Span::styled(
                            &set.name[match_start..match_end],
                            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
                        ));
                        last_end = match_end;
                        if query.is_empty() { break; }
                    }
                    if last_end < set.name.len() {
                        spans.push(Span::styled(
                            &set.name[last_end..],
                            Style::default().fg(theme.text_primary),
                        ));
                    }
                    if spans.is_empty() {
                        spans.push(Span::styled(
                            set.name.clone(),
                            Style::default().fg(theme.text_primary),
                        ));
                    }
                    spans
                } else {
                    vec![Span::styled(
                        set.name.clone(),
                        Style::default().fg(theme.text_primary),
                    )]
                };

                let mut parts = vec![
                    Span::styled(base_label, Style::default().fg(theme.text_primary)),
                ];
                parts.append(&mut name_parts);
                parts.push(Span::styled(shell_part, Style::default().fg(theme.text_primary)));

                // If selected, apply selected style to all spans
                let is_selected = i == self.set_list.selected && self.active_panel == Panel::Sets;
                let line = if is_selected {
                    Line::from(parts).style(Style::default().fg(theme.text_on_selected).bg(theme.selection_bg_secondary).add_modifier(Modifier::BOLD))
                } else {
                    Line::from(parts)
                };
                ListItem::new(line)
```

Hmm, this is getting complex. Let me simplify even further. Since this search highlighting is in the set panel, and the set panel already uses `highlight_style` for selection, the simplest approach is to only worry about the spans for the text content, and let the highlight_style handle the selected state.

Actually, the simplest correct approach: only highlight in non-selected items. For selected items, the highlight_style overrides everything anyway. So:

```rust
                let is_selected = i == self.set_list.selected
                    && self.active_panel == Panel::Sets;
                let span = if self.search_mode && !self.search_query.is_empty() && !is_selected {
                    // Build multi-span with highlighting
                    let query = self.search_query.as_str();
                    let lower_name = set.name.to_lowercase();
                    let lower_q = query.to_lowercase();
                    let base = format!(" {}  ", mode_label);
                    let shell = format!("  [{}] ({} cmd)", shell_label, cmd_count);
                    let mut spans = vec![Span::styled(base, Style::default().fg(theme.text_primary))];

                    let mut last = 0usize;
                    while let Some(pos) = lower_name[last..].find(&lower_q) {
                        let start = last + pos;
                        if start > last {
                            spans.push(Span::styled(
                                &set.name[last..start],
                                Style::default().fg(theme.text_primary),
                            ));
                        }
                        let end = start + query.len();
                        spans.push(Span::styled(
                            &set.name[start..end],
                            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
                        ));
                        last = end;
                    }
                    if last < set.name.len() {
                        spans.push(Span::styled(
                            &set.name[last..],
                            Style::default().fg(theme.text_primary),
                        ));
                    }
                    spans.push(Span::styled(shell, Style::default().fg(theme.text_primary)));
                    ListItem::new(Line::from(spans))
                } else {
                    let label = format!(" {}  {}  [{}] ({} cmd)", mode_label, set.name, shell_label, cmd_count);
                    let style = if is_selected {
                        Style::default().fg(theme.text_on_selected).bg(theme.selection_bg_secondary).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.text_primary)
                    };
                    ListItem::new(Line::from(Span::styled(label, style)))
                };
                ListItem::new(item)   // oops, this is a bug, `span` is already a ListItem
```

Wait, I'm calling the variable `span` but it's actually a `ListItem`. Let me clean this up. Actually let me just write the final version clearly in the plan.

Let me simplify the plan. Since the existing code builds a `label: String` and wraps it in `Span::styled(label, style)`, and I need to change it to potentially use multiple spans... Let me present the cleanest approach.

Actually, I'll write it as a simple code replacement in the plan. The key insight is: when in search mode, instead of `Span::styled(format!(...), style)`, build a `Line` from multiple `Span`s where the matched portion of the name uses `accent_primary` + `BOLD`.

Let me use a cleaner approach for the plan:

- [ ] **Replace the set label building with search-aware spans**

In `render_set_panel()`, find the map closure that builds each item. Replace the label and style building with:

```rust
                let is_selected = i == self.set_list.selected
                    && self.active_panel == Panel::Sets;

                let mut parts: Vec<Span> = Vec::new();

                // Mode emoji
                let mode_label = match set.exec_mode {
                    crate::models::ExecMode::StopOnError => "🛑",
                    crate::models::ExecMode::ContinueOnError => "⏩",
                };
                parts.push(Span::styled(
                    format!(" {}  ", mode_label),
                    text_style,
                ));

                // Set name with optional search highlighting
                if self.search_mode && !self.search_query.is_empty() && !is_selected {
                    let query = self.search_query.as_str();
                    let lower_name = set.name.to_lowercase();
                    let lower_q = query.to_lowercase();
                    let mut last = 0usize;
                    let mut has_match = false;
                    while let Some(pos) = lower_name[last..].find(&lower_q) {
                        has_match = true;
                        let match_start = last + pos;
                        if match_start > last {
                            parts.push(Span::styled(
                                &set.name[last..match_start],
                                text_style,
                            ));
                        }
                        let match_end = math_start + query.len(); // sic: match_start typo
                        parts.push(Span::styled(
                            &set.name[match_start..match_end],
                            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
                        ));
                        last = match_end;
                    }
                    if last < set.name.len() {
                        parts.push(Span::styled(&set.name[last..], text_style));
                    }
                    if !has_match {
                        parts.push(Span::styled(set.name.clone(), text_style));
                    }
                } else {
                    parts.push(Span::styled(set.name.clone(), text_style));
                }

                // Shell and command count
                let shell_label = set.shell.label();
                let cmd_count = set.commands.len();
                parts.push(Span::styled(
                    format!("  [{}] ({} cmd)", shell_label, cmd_count),
                    text_style,
                ));

                // Group name in search mode (right-aligned)
                if self.search_mode {
                    let gname = data.groups.get(gi).map(|g| g.name.as_str()).unwrap_or("?");
                    let text_width: usize = parts.iter().map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref())).sum();
                    let pad = list_area_width.saturating_sub(text_width + gname.len() + 1);
                    if pad > 0 {
                        parts.push(Span::styled(
                            " ".repeat(pad),
                            text_style,
                        ));
                    }
                    parts.push(Span::styled(gname, text_style));
                }

                let line = if is_selected {
                    Line::from(parts).style(
                        Style::default()
                            .fg(theme.text_on_selected)
                            .bg(theme.selection_bg_secondary)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Line::from(parts)
                };
                ListItem::new(line)
```

Actually this is getting way too complex for a plan. Let me drastically simplify. The whole search highlighting is about changing the color of matched text in the set name. Let me just show the essential change in the plan without reproducing the entire function.

Let me simplify the plan to just show the key code change and describe the rest. This is a writing plan, not the actual implementation.

Let me rewrite the plan concisely.<｜end▁of▁thinking｜>

<｜｜DSML｜｜tool_calls>
<｜｜DSML｜｜invoke name="Write">
<｜｜DSML｜｜parameter name="content" string="true"># Wave 3: Interaction Feedback — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add toast notifications, execution progress bar, execution auto-scroll, search highlighting, and variable input focus indicators.

**Architecture:** Each feature is independent — modify one file per task. Toast system is the only cross-cutting change (new module + app.rs modifications).

**Tech Stack:** ratatui 0.30.1 (Gauge, Clear, Span, ListState)

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src/ui/notification.rs` | **Create** | Toast struct, ToastSeverity enum |
| `src/ui/mod.rs` | Modify | Register notification module |
| `src/app.rs` | Modify | Toast field, push_toast/clean_toasts, auto_save -> &mut self, toast overlay rendering |
| `src/ui/execution_screen.rs` | Modify | Gauge progress bar in header, auto-scroll tracking, z toggle |
| `src/ui/main_screen.rs` | Modify | Search keyword highlighting in set panel |
| `src/ui/variable_screen.rs` | Modify | Focus indicator background color on Tab-switched inputs |

---

### Task 1: Toast Notification System

**Files:**
- Create: `src/ui/notification.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/app.rs`

- [ ] **Create `src/ui/notification.rs`**

```rust
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToastSeverity {
    Success,
    Error,
    Info,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub severity: ToastSeverity,
    pub created_at: Instant,
}

impl Toast {
    pub fn new(message: impl Into<String>, severity: ToastSeverity) -> Self {
        Self {
            message: message.into(),
            severity,
            created_at: Instant::now(),
        }
    }
}
```

- [ ] **Register module in `src/ui/mod.rs`**

Add `pub mod notification;` after `pub mod theme;`.

- [ ] **Add Toast imports and field to `App`**

In `src/app.rs`, after `use crate::ui::theme::Theme;`:
```rust
use crate::ui::notification::{Toast, ToastSeverity};
```

Add field to `App` struct after `theme: Theme,`:
```rust
    toasts: Vec<Toast>,
```

Initialize in `Self {`:
```rust
    toasts: Vec::new(),
```

- [ ] **Change `auto_save()` to `&mut self` and push toasts**

Replace:
```rust
    fn auto_save(&self) {
        if let Err(e) = storage::save_app_data(&self.data) {
            eprintln!("Auto-save failed: {}", e);
        }
    }
```

With:
```rust
    fn auto_save(&mut self) {
        match storage::save_app_data(&self.data) {
            Ok(()) => self.push_toast("Saved", ToastSeverity::Success),
            Err(e) => self.push_toast(format!("Save failed: {}", e), ToastSeverity::Error),
        }
    }
```

All callers of `auto_save()` already have `&mut self` — no changes needed.

- [ ] **Add push_toast/clean_toasts + call clean_toasts in event loop**

Methods on `App`:
```rust
    fn push_toast(&mut self, message: impl Into<String>, severity: ToastSeverity) {
        self.toasts.push(Toast::new(message, severity));
    }

    fn clean_toasts(&mut self) {
        const TOAST_DURATION: std::time::Duration = std::time::Duration::from_secs(3);
        self.toasts.retain(|t| t.created_at.elapsed() < TOAST_DURATION);
    }
```

In `run()`, before `terminal.draw(|f| self.render(f))?;` on line 74, add:
```rust
            self.clean_toasts();
```

- [ ] **Render toasts in title bar (right-aligned overlay)**

In `render()`, after `self.variable_screen.render(...)` (line 154), add:
```rust
        // Render toast notification (right side of title bar)
        if let Some(toast) = self.toasts.last() {
            let (toast_fg, toast_label) = match toast.severity {
                ToastSeverity::Success => (self.theme.accent_success, " ✓ "),
                ToastSeverity::Error => (self.theme.accent_error, " ✗ "),
                ToastSeverity::Info => (self.theme.accent_info, " ● "),
            };
            let toast_msg = format!("{}{}", toast_label, toast.message);
            let toast_width = (toast_msg.len() as u16 + 2).min(area.width.saturating_sub(4));
            let x = (area.width.saturating_sub(toast_width)) / 2;
            let toast_area = Rect::new(x, title_area.y, toast_width, 1);
            // Clear the title bar area where toast will be rendered
            frame.render_widget(Clear, toast_area);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    toast_msg,
                    Style::default().fg(toast_fg).add_modifier(Modifier::BOLD),
                ))),
                toast_area,
            );
        }
```

Add `use ratatui::widgets::Clear;` to the imports (already used in help_screen but may not be in app.rs).

- [ ] **Add toast on execution completion**

In `on_exec_action()`, `BackToMain` arm (line 324-327):
```rust
            ExecutionScreenAction::BackToMain => {
                if let Some(ref es) = self.exec_screen && es.completed {
                    let summary = format!(
                        "Done: {}/{}",
                        es.succeeded + es.failed + es.skipped,
                        es.total,
                    );
                    self.push_toast(summary, ToastSeverity::Success);
                }
                self.kill_execution();
                self.mode = AppMode::Main;
            }
```

- [ ] **Add toast on DeleteSet and DeleteGroup**

In `on_main_action()`, after `DeleteSet` block (after line 216):
```rust
                self.push_toast("Set deleted", ToastSeverity::Info);
```

After `DeleteGroup` block (after line 244):
```rust
                self.push_toast("Group deleted", ToastSeverity::Info);
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

Expected: No errors, 51 tests pass.

- [ ] **Commit**

```bash
git add src/ui/notification.rs src/ui/mod.rs src/app.rs
git commit -m "feat(ux): add toast notification system with save/delete/exec feedback"
```

---

### Task 2: Execution Progress Bar (Gauge)

**Files:**
- Modify: `src/ui/execution_screen.rs`

- [ ] **Add Gauge import**

Replace `use ratatui::widgets::{` line: add `Gauge,` to the list.

- [ ] **Modify header layout: change `[Length(3), Min(1)]` to `[Length(1), Length(1), Min(1)]`**

Replace:
```rust
        let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]);
        let [header_area, body_area] = vertical.areas(area);
```
with:
```rust
        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)]);
        let [header_area, gauge_area, body_area] = vertical.areas(area);
```

- [ ] **Add Gauge rendering after header**

After `frame.render_widget(header, header_area);` and before the body, add:
```rust
        // Gauge progress bar
        let completed_count = self.succeeded + self.failed + self.skipped;
        let progress = if self.total > 0 {
            completed_count as f64 / self.total as f64
        } else {
            0.0
        };
        let gauge_label = format!("  {}/{}  {:.0}%  ", completed_count, self.total, progress * 100.0);
        let gauge = Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(theme.accent_success)
                    .bg(theme.surface),
            )
            .percent((progress * 100.0) as u16)
            .label(gauge_label);
        frame.render_widget(gauge, gauge_area);
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/execution_screen.rs
git commit -m "feat(ux): add execution progress bar (Gauge)"
```

---

### Task 3: Execution Auto-Scroll

**Files:**
- Modify: `src/ui/execution_screen.rs`

- [ ] **Add scroll tracking fields to `ExecutionScreenState`**

After `pub total_duration_ms: Option<u128>,`:
```rust
    pub auto_scroll: bool,
    pub scroll_offset: usize,
```

Initialize in `new()` (add before `}`):
```rust
    auto_scroll: true,
    scroll_offset: 0,
```

- [ ] **Add helper method to calculate items offset for a command**

```rust
    /// Calculate the flat items Vec index for a given command index.
    fn items_offset_for_command(&self, cmd_idx: usize) -> usize {
        let mut offset = 0;
        for i in 0..cmd_idx.min(self.cmd_states.len()) {
            offset += 1; // command header line
            offset += self.cmd_states[i].output_lines.len(); // output lines
            offset += 1; // separator line
        }
        offset
    }
```

- [ ] **Update `process_events()` to set scroll_offset on Starting**

In the `ExecutionEvent::Starting { index, command }` arm (after line 107 `self.current_index = index;`):
```rust
                        if self.auto_scroll {
                            self.scroll_offset = self.items_offset_for_command(index);
                        }
```

- [ ] **Reset scroll state in `reset_from()`**

At the start of `reset_from()`:
```rust
        self.auto_scroll = true;
        self.scroll_offset = 0;
```

- [ ] **Convert flat List render to stateful render with offset + scrollbar position**

Replace the block of code from `let list = List::new(items);` through the scrollbar rendering:
```rust
        // Use ListState with offset for auto-scroll
        let mut list_state = ratatui::widgets::ListState::default()
            .with_offset(self.scroll_offset);
        frame.render_stateful_widget(List::new(items), content_area, &mut list_state);

        // Scrollbar tracks current command position
        let content_len = self.cmd_states.len();
        let scroll_pos = self.current_index.min(content_len.saturating_sub(1));
        let mut scrollbar_state = ScrollbarState::new(content_len)
            .position(scroll_pos);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme.surface_border)),
            scrollbar_area,
            &mut scrollbar_state,
        );
```

- [ ] **Add `z` key to toggle auto-scroll in `handle_key()`**

After the `KeyCode::Char('r')` arm, add:
```rust
            KeyCode::Char('z') => {
                self.auto_scroll = !self.auto_scroll;
                ExecutionScreenAction::None
            }
```

- [ ] **Update footer text to show z toggle**

In the footer text, change `[s] Skip current` to `[s] Skip  [z] Auto-scroll`:
```rust
        let footer_text = if self.completed {
            ...
        } else {
            " [q] Back to main  [s] Skip current  [z] Auto-scroll  [Ctrl+C] Interrupt"
        };
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/execution_screen.rs
git commit -m "feat(ux): add execution auto-scroll with z toggle"
```

---

### Task 4: Search Highlighting

**Files:**
- Modify: `src/ui/main_screen.rs`

**Goal:** When in search mode (`self.search_mode` is true and `self.search_query` non-empty), highlight matched text in set names by changing its foreground to `accent_primary` + BOLD.

- [ ] **In `render_set_panel()`, replace the label building for search highlighting**

Find the `ListItem` construction inside the `.map(|(i, &(gi, _, set))| {` closure. Currently it builds a single `format!(...)` string and wraps it in `Span::styled(label, style)`.

Replace from the `let mut label = format!(...)` through the `Line::from(Span::styled(label, style))` with:

```rust
                let is_selected = i == self.set_list.selected
                    && self.active_panel == Panel::Sets;
                let text_style = if is_selected {
                    Style::default()
                        .fg(theme.text_on_selected)
                        .bg(theme.selection_bg_secondary)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text_primary)
                };

                let mode_label = match set.exec_mode {
                    crate::models::ExecMode::StopOnError => "🛑",
                    crate::models::ExecMode::ContinueOnError => "⏩",
                };
                let prefix = format!(" {}  ", mode_label);
                let shell_label = set.shell.label();
                let cmd_count = set.commands.len();
                let suffix = format!("  [{}] ({} cmd)", shell_label, cmd_count);

                // Build name part with search highlighting
                let name_part = if self.search_mode && !self.search_query.is_empty() && !is_selected {
                    let query = self.search_query.as_str();
                    let lower_name = set.name.to_lowercase();
                    let lower_q = query.to_lowercase();
                    let mut spans: Vec<Span> = Vec::new();
                    let mut last = 0usize;
                    while let Some(pos) = lower_name[last..].find(&lower_q) {
                        let match_start = last + pos;
                        if match_start > last {
                            spans.push(Span::styled(&set.name[last..match_start], text_style));
                        }
                        let match_end = match_start + query.len();
                        spans.push(Span::styled(
                            &set.name[match_start..match_end],
                            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
                        ));
                        last = match_end;
                        if query.is_empty() { break; }
                    }
                    if last < set.name.len() {
                        spans.push(Span::styled(&set.name[last..], text_style));
                    }
                    if spans.is_empty() {
                        spans.push(Span::styled(set.name.clone(), text_style));
                    }
                    spans
                } else {
                    vec![Span::styled(set.name.clone(), text_style)]
                };

                let mut parts = vec![Span::styled(prefix, text_style)];
                parts.extend(name_part);
                parts.push(Span::styled(suffix, text_style));

                // Right-aligned group name in search mode
                if self.search_mode {
                    // Calculate total width of spans so far
                    let gname = data.groups.get(gi).map(|g| g.name.as_str()).unwrap_or("?");
                    let text_width: usize = parts.iter().map(|s| {
                        unicode_width::UnicodeWidthStr::width(s.content.as_ref())
                    }).sum();
                    let avail = list_area.width as usize;
                    let pad = avail.saturating_sub(text_width + gname.len() + 1);
                    if pad > 0 {
                        parts.push(Span::styled(" ".repeat(pad), text_style));
                    }
                    parts.push(Span::styled(gname, text_style));
                }

                ListItem::new(Line::from(parts))
```

**Note:** `list_area` is available in the scope — it's the list area variable from the scrollbar split.

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "feat(ux): add search keyword highlighting in set panel"
```

---

### Task 5: Variable Input Focus Indicator

**Files:**
- Modify: `src/ui/variable_screen.rs`

**Goal:** Currently Tab-switching between variable inputs only changes text color (Yellow vs White). Enhance to also change background color, making the focus unambiguous.

- [ ] **Enhance `render()` method's variable row rendering**

Find this code in `render()`:
```rust
        for i in 0..count {
            let focus = i == self.focus;
            let color = if focus { theme.accent_primary } else { theme.text_primary };
            let row = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
            let display = format!(" {} = {}", self.names[i], self.inputs[i].content);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    display,
                    Style::default().fg(color),
                ))),
                row,
            );
```

Replace with:
```rust
        for i in 0..count {
            let focused = i == self.focus;
            let row_style = if focused {
                Style::default()
                    .fg(theme.text_on_selected)
                    .bg(theme.selection_bg_primary)
            } else {
                Style::default().fg(theme.text_primary)
            };
            let row = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
            let display = format!(" {} = {}", self.names[i], self.inputs[i].content);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(display, row_style))),
                row,
            );
```

Also update cursor rendering: rename the local `focus` variable to `focused` and update the condition accordingly.

The cursor positioning code (the `if focus { ... }` block) should check `focused` instead.

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/ui/variable_screen.rs
git commit -m "feat(ux): add visual focus indicator for variable input fields"
```

---

### Task 6: Final Verification

- [ ] **Run full test suite**

```bash
cargo test
```

Expected: All 51 tests pass.

- [ ] **Run clippy**

```bash
cargo clippy 2>&1 | grep '^error'
```

Expected: No errors.

- [ ] **Build**

```bash
cargo build
```
