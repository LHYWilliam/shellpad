# Three-layer Tab Navigation + Option Picker — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace 7-stop flat Tab cycle with 3-region routing (Properties ↔ Variables ↔ Commands). Add ◄/► arrow decorators and an on-focus picker panel for Options. ↑/↓ navigates within the active region.

**Architecture:** New `DetailRegion` helper enum maps each focus to one of three regions. Tab routes between regions (always resetting to first item). ↑/↓ dispatches to region-specific logic: Properties cycles through 5 fields, Variables/Commands navigate their lists. Options rendering is split horizontally with a picker column that shows available choices when an Option is focused.

**Tech Stack:** Rust, Ratatui, crossterm (no new dependencies)

---

### Task 1: Replace Tab cycle with 3-region routing

**Files:**
- Modify: `src/ui/detail_screen/handler.rs`

- [ ] **Step 1: Write failing tests for new Tab behavior**

Replace the existing exhaustive tab-cycle test and the BackTab test. After the `test_ctrl_up_ignored_when_not_vars_or_cmds_focus` test, add:

```rust
    // ---- 3-region Tab navigation ----
    #[test]
    fn test_tab_cycles_properties_to_variables_to_commands() {
        let mut state = make_state();
        assert_eq!(state.focus, DetailFocus::Name); // Properties first
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Variables);
        assert_eq!(state.variable_list.selected, 0);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Commands);
        assert_eq!(state.command_list.selected, 0);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Name); // wraps to Properties
    }

    #[test]
    fn test_backtab_cycles_commands_to_variables_to_properties() {
        let mut state = make_state();
        state.handle_key(make_key(KeyCode::BackTab)); // Name → Commands
        assert_eq!(state.focus, DetailFocus::Commands);
        assert_eq!(state.command_list.selected, 0);
        state.handle_key(make_key(KeyCode::BackTab)); // Commands → Variables
        assert_eq!(state.focus, DetailFocus::Variables);
        assert_eq!(state.variable_list.selected, 0);
        state.handle_key(make_key(KeyCode::BackTab)); // Variables → Properties
        assert_eq!(state.focus, DetailFocus::Name);
    }
```

And remove the old `test_tab_cycles_focus_forward` and `test_backtab_cycles_focus_backward` tests.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test ui::detail_screen::handler::tests::test_tab_cycles_properties_to_variables_to_commands`
Expected: FAIL — Tab still does 7-stop cycle

- [ ] **Step 3: Implement `DetailRegion` enum and helpers**

Add inside `impl DetailScreenState` block, before `handle_key`:

```rust
    enum DetailRegion { Properties, Variables, Commands }

    fn region(&self) -> DetailRegion {
        match self.focus {
            DetailFocus::Name | DetailFocus::WorkDir | DetailFocus::Group
            | DetailFocus::Shell | DetailFocus::ExecMode => DetailRegion::Properties,
            DetailFocus::Variables => DetailRegion::Variables,
            DetailFocus::Commands => DetailRegion::Commands,
        }
    }

    fn next_region(&mut self) {
        self.commit_name_edit();
        self.commit_workdir_edit();
        match self.region() {
            DetailRegion::Properties => {
                self.focus = DetailFocus::Variables;
                self.variable_list.selected = 0;
            }
            DetailRegion::Variables => {
                self.focus = DetailFocus::Commands;
                self.command_list.selected = 0;
            }
            DetailRegion::Commands => {
                self.focus = DetailFocus::Name;
            }
        }
    }

    fn prev_region(&mut self) {
        self.commit_name_edit();
        self.commit_workdir_edit();
        match self.region() {
            DetailRegion::Properties => {
                self.focus = DetailFocus::Commands;
                self.command_list.selected = 0;
            }
            DetailRegion::Variables => {
                self.focus = DetailFocus::Name;
            }
            DetailRegion::Commands => {
                self.focus = DetailFocus::Variables;
                self.variable_list.selected = 0;
            }
        }
    }
```

- [ ] **Step 4: Replace Tab/BackTab handlers**

Replace the entire Tab and BackTab blocks with:

```rust
            KeyCode::Tab | KeyCode::Char('\t') => {
                self.next_region();
            }
            KeyCode::BackTab => {
                self.prev_region();
            }
```

- [ ] **Step 5: Implement Properties ↑/↓**

Replace the existing plain `KeyCode::Up` arm (the part inside `match self.focus { DetailFocus::Variables => ... DetailFocus::Commands => ... _ => {} }`) and the `KeyCode::Down` arm with region-aware dispatch. The full replacement for both Up and Down arms:

```rust
            KeyCode::Up if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                match self.focus {
                    DetailFocus::Variables if !self.set.variables.is_empty() => {
                        let idx = self.variable_list.selected
                            .min(self.set.variables.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Variable(idx), -1);
                    }
                    DetailFocus::Commands if !self.set.commands.is_empty() => {
                        let idx = self.command_list.selected
                            .min(self.set.commands.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Command(idx), -1);
                    }
                    _ => {}
                }
            }
            KeyCode::Up => {
                match self.region() {
                    DetailRegion::Properties => {
                        self.commit_name_edit();
                        self.commit_workdir_edit();
                        self.focus = match self.focus {
                            DetailFocus::Name => DetailFocus::ExecMode,
                            DetailFocus::WorkDir => DetailFocus::Name,
                            DetailFocus::Group => DetailFocus::WorkDir,
                            DetailFocus::Shell => DetailFocus::Group,
                            DetailFocus::ExecMode => DetailFocus::Shell,
                            _ => self.focus,
                        };
                    }
                    DetailRegion::Variables => {
                        self.variable_list.select_previous();
                    }
                    DetailRegion::Commands => {
                        self.command_list.select_previous();
                    }
                }
            }
            KeyCode::Down if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                match self.focus {
                    DetailFocus::Variables if !self.set.variables.is_empty() => {
                        let idx = self.variable_list.selected
                            .min(self.set.variables.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Variable(idx), 1);
                    }
                    DetailFocus::Commands if !self.set.commands.is_empty() => {
                        let idx = self.command_list.selected
                            .min(self.set.commands.len().saturating_sub(1));
                        return AppAction::Reorder(ReorderKind::Command(idx), 1);
                    }
                    _ => {}
                }
            }
            KeyCode::Down => {
                match self.region() {
                    DetailRegion::Properties => {
                        self.commit_name_edit();
                        self.commit_workdir_edit();
                        self.focus = match self.focus {
                            DetailFocus::Name => DetailFocus::WorkDir,
                            DetailFocus::WorkDir => DetailFocus::Group,
                            DetailFocus::Group => DetailFocus::Shell,
                            DetailFocus::Shell => DetailFocus::ExecMode,
                            DetailFocus::ExecMode => DetailFocus::Name,
                            _ => self.focus,
                        };
                    }
                    DetailRegion::Variables => {
                        self.variable_list.select_next(self.set.variables.len());
                    }
                    DetailRegion::Commands => {
                        self.command_list.select_next(self.set.commands.len());
                    }
                }
            }
```

Note: The Ctrl+Up/Ctrl+Down arms must stay BEFORE the plain Up/Down arms so that Ctrl variants match first.

Also remove the old separate `KeyCode::Up => match self.focus { ... }` and `KeyCode::Down => match self.focus { ... }` blocks (they have been subsumed into the above).

- [ ] **Step 6: Run all handler tests**

Run: `cargo test ui::detail_screen::handler::tests`
Expected: All tests PASS (existing count - 2 removed + 2 new = 17)

- [ ] **Step 7: Commit**

```bash
git add src/ui/detail_screen/handler.rs
git commit -m "feat: replace 7-stop Tab cycle with 3-region routing

Tab cycles between Properties, Variables, Commands (reset to first).
↑/↓ navigates within region: Properties cycles fields, Variables/
Commands navigate lists. Ctrl+Up/Down reorder preserved.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Arrow decorators for Options

**Files:**
- Modify: `src/ui/detail_screen/render.rs`

- [ ] **Step 1: Wrap Option labels with ◄ ► arrows when focused**

In `render_metadata`, update the Group, Shell, and ExecMode rendering. The Group label (line ~164):

```rust
        let group_label = if self.focus == DetailFocus::Group {
            format!(" ◄ Group: {} ►", group_name)
        } else {
            format!(" Group: {}", group_name)
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(group_label, group_style))),
            group_col,
        );
```

The Shell label (line ~171):

```rust
        let shell_label = if self.focus == DetailFocus::Shell {
            format!(" ◄ Shell: {} ►", self.set.shell.label())
        } else {
            format!(" Shell: {}", self.set.shell.label())
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(shell_label, shell_style))),
            shell_col,
        );
```

The ExecMode label (line ~183):

```rust
        let mode_label = if self.focus == DetailFocus::ExecMode {
            format!(" ◄ Mode: {} ►", self.set.exec_mode.label())
        } else {
            format!(" Mode: {}", self.set.exec_mode.label())
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(mode_label, mode_style))),
            mode_row,
        );
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add src/ui/detail_screen/render.rs
git commit -m "feat: add ◄ ► arrow decorators on focused Options

Group, Shell, ExecMode display ◄/► arrows when focused
to indicate ←/→ cycling is available.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Options picker panel

**Files:**
- Modify: `src/ui/detail_screen/render.rs`

- [ ] **Step 1: Restructure Options area layout**

In `render_metadata`, the Options section (from separator downwards) is currently 3 rows. We need to split the area horizontally. But the separator and three label rows are all rendered with independent `Layout::vertical` areas. The simplest approach: calculate an `options_total_area` covering the separator + 3 rows + available space below, then split it horizontally.

However, this is difficult with the current flat row-based layout. A pragmatic approach: compute the picker column from the separator row's right half, extending vertically down. The separator row itself is the reference point.

Replace the separator rendering with a horizontal split:

```rust
        // Options section: left labels, right picker
        // sep_row is the separator row — use its right half for picker
        // The Options rows (gs_row, mode_row) use the left half
        let picker_width = sep_row.width.saturating_sub(sep_row.width * 2 / 3);
        let label_width = sep_row.width.saturating_sub(picker_width);
        let opts_layout = Layout::horizontal([
            Constraint::Length(label_width),
            Constraint::Length(picker_width),
        ]);
        let [label_col, picker_col] = opts_layout.areas(sep_row);

        // Separator (full label width)
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" ── Options {} ", "─".repeat(label_col.width.saturating_sub(12) as usize)),
                Style::default().fg(theme.text_disabled).add_modifier(Modifier::DIM),
            ))),
            label_col,
        );

        // Adjust gs_row and mode_row to use label_col area
        let gs_left = Rect::new(label_col.x, gs_row.y, label_col.width, gs_row.height);
        let mode_left = Rect::new(label_col.x, mode_row.y, label_col.width, mode_row.height);

        let half_layout = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]);
        let [group_col, shell_col] = half_layout.areas(gs_left);
```

And update Group/Shell rendering to use `gs_left` based areas. Update Mode rendering:

```rust
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(mode_label, mode_style))),
            mode_left,
        );
```

- [ ] **Step 2: Add picker rendering function**

Add a method `render_picker` to `DetailScreenState` (can be standalone function or impl method):

```rust
    fn render_picker(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem<'_>> = match self.focus {
            DetailFocus::Group => self.groups.iter().enumerate().map(|(i, g)| {
                let selected = g.id == self.set.group_id;
                let style = if selected {
                    Style::default().fg(theme.accent_primary)
                } else {
                    theme.normal_style()
                };
                styled_list_item(format!(" {}", g.name), style, area.width)
            }).collect(),
            DetailFocus::Shell => {
                let variants = ShellType::builtin_variants();
                let saved_custom = match &self.set.shell {
                    ShellType::Custom(p) => Some(p.clone()),
                    _ => None,
                };
                let mut result = Vec::new();
                for v in &variants {
                    let selected = std::mem::discriminant(&self.set.shell) == std::mem::discriminant(v);
                    let label = match v {
                        ShellType::SystemDefault => "System Default".to_string(),
                        ShellType::Custom(_) => unreachable!(),
                        _ => v.label(),
                    };
                    let style = if selected {
                        Style::default().fg(theme.accent_primary)
                    } else {
                        theme.normal_style()
                    };
                    result.push(styled_list_item(format!(" {}", label), style, area.width));
                }
                // Custom
                if let Some(ref path) = saved_custom {
                    let selected = matches!(&self.set.shell, ShellType::Custom(_));
                    let style = if selected {
                        Style::default().fg(theme.accent_primary)
                    } else {
                        theme.normal_style()
                    };
                    result.push(styled_list_item(
                        format!(" Custom: {}", path), style, area.width,
                    ));
                } else {
                    let style = theme.normal_style();
                    result.push(styled_list_item(" Custom", style, area.width));
                }
                result
            }
            DetailFocus::ExecMode => vec![
                styled_list_item(
                    " Stop on Error",
                    if self.set.exec_mode == ExecMode::StopOnError {
                        Style::default().fg(theme.accent_primary)
                    } else { theme.normal_style() },
                    area.width,
                ),
                styled_list_item(
                    " Continue on Error",
                    if self.set.exec_mode == ExecMode::ContinueOnError {
                        Style::default().fg(theme.accent_primary)
                    } else { theme.normal_style() },
                    area.width,
                ),
            ],
            _ => return, // no picker for non-Options
        };

        frame.render_widget(Clear, area);
        let inner = bordered_block_zone(frame, area, theme, " Options ", false);
        let mut state = ratatui::widgets::ListState::default();
        frame.render_stateful_widget(List::new(items), inner, &mut state);
    }
```

- [ ] **Step 3: Call picker from render_metadata**

Add picker rendering call at end of `render_metadata`. The picker area extends from the separator row down: `Rect::new(picker_col.x, picker_col.y, picker_col.width, sep_row.height + gs_row.height + mode_row.height)`.

Actually, a simpler approach: the picker column naturally occupies the full vertical extent of the Options section because we split the `sep_row` area and use it for the separator. But for the picker we need the full height.

Let me simplify: create the picker area as `Rect::new(picker_col.x, sep_row.y, picker_col.width, gs_row.y + gs_row.height - sep_row.y + mode_row.height)`. The vertical extent covers the separator through the end of the last options row.

Add after the Options rendering:

```rust
        // Picker column
        if matches!(self.focus, DetailFocus::Group | DetailFocus::Shell | DetailFocus::ExecMode) {
            let picker_area = Rect::new(
                picker_col.x, sep_row.y,
                picker_col.width,
                mode_row.y + mode_row.height - sep_row.y,
            );
            self.render_picker(frame, picker_area, theme);
        }
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 5: Run tests**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/ui/detail_screen/render.rs
git commit -m "feat: add on-focus picker panel for Options

Picker column on right side shows all available Group/Shell/Mode
choices when the corresponding Option is focused. Selected value
highlighted with accent_primary. Uses Clear widget overlay.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: Update status bar hints

**Files:**
- Modify: `src/ui/detail_screen/render.rs`

- [ ] **Step 1: Update Properties status bar hints**

In the status bar, update the Name, WorkDir, Group, Shell, and ExecMode hints to reflect the new ↑/↓ navigation:

```rust
            (false, DetailFocus::Name) => "[Enter] Edit  [↑/↓] Navigate  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::WorkDir) => "[Enter] Edit  [↑/↓] Navigate  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::Group) => "[←/→] Change  [↑/↓] Navigate  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::Shell) => "[←/→] Change  [↑/↓] Navigate  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::ExecMode) => "[←/→] Change  [↑/↓] Navigate  [Tab] Next  |  [Ctrl+S] Save",
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add src/ui/detail_screen/render.rs
git commit -m "feat: update status bar hints for 3-region navigation

Properties fields show [↑/↓] Navigate instead of [Tab] Next.
Options show [←/→] Change for cycling.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Full suite verification and clippy

**Files:**
- Modify: None (verification only)

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 2: Run clippy**

Run: `cargo clippy`
Expected: No new warnings (pre-existing warnings OK)

- [ ] **Step 3: Commit (verification pass)**

Skip commit — no code changes.
