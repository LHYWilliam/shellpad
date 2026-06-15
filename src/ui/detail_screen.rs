use crate::models::{CommandSet, ExecMode, Group, ShellType};
use crate::ui::components::{
    bordered_block, empty_hint, fill_row, handle_text_input, InlineEdit, list_scrollbar_areas,
    render_inline_cursor, render_scrollbar, render_status_bar, set_cursor_after_prefix,
    ScrollableList, TextInput,
};
use crate::ui::detail_editor::{handle_command_edit, handle_variable_edit};
use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailFocus {
    Name,
    Group,
    Shell,
    ExecMode,
    Variables,
    Commands,
}

pub enum DetailScreenAction {
    None,
    Save(CommandSet),
    Cancel,
    DeleteVariable(usize),
    DeleteCommand(usize),
}

pub struct DetailScreenState {
    pub set: CommandSet,
    pub groups: Vec<Group>,
    pub name_input: TextInput,
    pub focus: DetailFocus,
    pub variable_list: ScrollableList,
    pub command_list: ScrollableList,
    pub editing_name: bool,
    pub var_edit: InlineEdit,
    pub cmd_edit: InlineEdit,
}

impl DetailScreenState {
    pub fn new(set: CommandSet, groups: Vec<Group>) -> Self {
        let name = set.name.clone();
        Self {
            set,
            groups,
            name_input: TextInput::new(name),
            focus: DetailFocus::Name,
            variable_list: ScrollableList::new(),
            command_list: ScrollableList::new(),
            editing_name: false,
            var_edit: InlineEdit::new(),
            cmd_edit: InlineEdit::new(),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent_info))
            .title(format!(" Edit: {} ",
                if self.editing_name { &self.name_input.content } else { &self.set.name }
            ));

        let inner = block.inner(area);
        frame.render_widget(&block, area);

        // Split into top metadata and bottom command areas
        let layout = Layout::vertical([
            Constraint::Length(8), // Properties block (4 rows + borders)
            Constraint::Min(3),    // variables
            Constraint::Min(3),    // commands
            Constraint::Length(2), // status bar (separator + content)
        ]);
        let [meta_area, var_area, cmd_area, status_area] = layout.areas(inner);

        // Update scroll offsets (approx inner height = area - 2 for borders)
        self.variable_list.update_offset(var_area.height.saturating_sub(2) as usize);
        self.command_list.update_offset(cmd_area.height.saturating_sub(2) as usize);

        self.render_metadata(frame, meta_area, theme);
        self.render_variables(frame, var_area, theme);
        self.render_commands(frame, cmd_area, theme);
        self.render_status_bar(frame, status_area, theme);
    }

    fn render_metadata(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let props_focused = matches!(self.focus, DetailFocus::Name | DetailFocus::Group | DetailFocus::Shell | DetailFocus::ExecMode);
        let block = bordered_block(theme, " Properties ", props_focused);

        let inner = block.inner(area);
        frame.render_widget(&block, area);

        // Name, Group+Shell, ExecMode in rows inside the block
        let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)]);
        let [name_row, gs_row, mode_row] = rows.areas(inner);

        // Name
        let is_name_focused = self.focus == DetailFocus::Name;
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
            theme.normal_style()
        };
        let display_name = if self.editing_name {
            self.name_input.content.as_str()
        } else {
            self.set.name.as_str()
        };
        let name_text = format!(" Name: {}", display_name);
        let name_line = fill_row(Line::from(Span::styled(name_text, name_style)), name_style, name_row.width);
        frame.render_widget(
            Paragraph::new(name_line),
            name_row,
        );

        // Cursor for name editing
        if self.editing_name {
            let prefix_width = unicode_width::UnicodeWidthStr::width(" Name: ");
            set_cursor_after_prefix(
                frame,
                &self.name_input.content,
                self.name_input.cursor,
                prefix_width as u16,
                name_row,
            );
        }

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
            theme.normal_style()
        };

        let shell_style = if self.focus == DetailFocus::Shell {
            Style::default().fg(theme.accent_primary)
        } else {
            theme.normal_style()
        };

        let half_layout = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]);
        let [group_col, shell_col] = half_layout.areas(gs_row);
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

        // Exec mode (full width)
        let mode_style = if self.focus == DetailFocus::ExecMode {
            Style::default().fg(theme.accent_primary)
        } else {
            theme.normal_style()
        };
        let mode_text = format!(" Mode: {}", self.set.exec_mode.label());
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(mode_text, mode_style))),
            mode_row,
        );
    }

    /// Shared list renderer for Variables and Commands.
    /// `item_fn(index, is_editing) -> (label, style)` provides per-item content.
    /// `preview_label` is shown when `insert_at` is Some.
    /// Returns `list_area` for cursor positioning.
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
    {
        let block = bordered_block(theme, title, focused);
        let inner = block.inner(area);
        frame.render_widget(&block, area);

        let (list_area, scrollbar_area) = list_scrollbar_areas(inner);

        let mut items: Vec<ListItem> = (0..count)
            .map(|i| {
                let is_editing = Some(i) == editing_item;
                let (label, style) = item_fn(i, is_editing);
                ListItem::new(fill_row(Line::from(Span::styled(label, style)), style, list_area.width))
            })
            .collect();

        // Preview row for new inserts
        if let Some(idx) = editing_item
            && insert_at.is_some()
            && let Some(label) = &preview_label
        {
            let style = Style::default()
                .fg(theme.text_on_selected)
                .bg(theme.accent_primary)
                .add_modifier(Modifier::BOLD);
            let preview = ListItem::new(fill_row(Line::from(Span::styled(label.clone(), style)), style, list_area.width));
            let pos = insert_at.unwrap_or(idx.min(items.len()));
            items.insert(pos, preview);
        }

        if count == 0 {
            items.push(empty_hint(theme, empty_text));
        }

        let mut list_state = ratatui::widgets::ListState::default()
            .with_selected(list.selected_or_none(count));
        frame.render_stateful_widget(List::new(items), list_area, &mut list_state);

        render_scrollbar(frame, scrollbar_area, theme, count, list.selected);
        list_area
    }

    fn render_variables(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let count = self.set.variables.len();
        let preview = self.var_edit.insert_at.is_some().then(|| {
            format!("  ▶ {}", self.var_edit.edit_input.content)
        });
        let list_area = self.render_items_list(
            frame, area, theme,
            &format!(" Variables ({}) ", count),
            self.focus == DetailFocus::Variables,
            count, &self.variable_list,
            self.var_edit.editing,
            self.var_edit.insert_at,
            |i, is_editing| {
                let label = if is_editing {
                    format!("  ▶ {}", self.var_edit.edit_input.content)
                } else {
                    let v = &self.set.variables[i];
                    format!("  {} = {}", v.name, v.default_value)
                };
                let style = if is_editing {
                    Style::default()
                        .fg(theme.text_on_selected)
                        .bg(theme.accent_primary)
                        .add_modifier(Modifier::BOLD)
                } else if i == self.variable_list.selected
                    && self.focus == DetailFocus::Variables
                {
                    theme.selected_style(theme.selection_bg_secondary)
                } else {
                    theme.normal_style()
                };
                (label, style)
            },
            preview,
            " (empty — press a to add a variable) ",
        );

        if let Some(idx) = self.var_edit.editing {
            let pos = self.var_edit.insert_at.unwrap_or(idx);
            render_inline_cursor(
                frame, list_area, self.variable_list.offset,
                pos, &self.var_edit.edit_input,
                unicode_width::UnicodeWidthStr::width("  ▶ ") as u16,
            );
        }
    }

    fn render_commands(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let count = self.set.commands.len();
        let preview = self.cmd_edit.insert_at.is_some().then(|| {
            let pos = self.cmd_edit.insert_at.unwrap_or(0);
            format!("  #{}▶ {}", pos, self.cmd_edit.edit_input.content)
        });
        let list_area = self.render_items_list(
            frame, area, theme,
            &format!(" Commands ({}) ", count),
            self.focus == DetailFocus::Commands,
            count, &self.command_list,
            self.cmd_edit.editing,
            self.cmd_edit.insert_at,
            |i, is_editing| {
                let pos = self.set.commands[i].position;
                let is_insert = self.cmd_edit.insert_at.is_some();
                let display_pos = if is_editing {
                    self.cmd_edit.insert_at.unwrap_or(pos)
                } else if is_insert && i >= self.cmd_edit.insert_at.unwrap() {
                    pos + 1
                } else {
                    pos
                };
                let content = if is_editing {
                    self.cmd_edit.edit_input.content.as_str()
                } else {
                    self.set.commands[i].command.as_str()
                };
                let label = format!("  #{}  {}", display_pos, content);
                let style = if is_editing {
                    Style::default()
                        .fg(theme.text_on_selected)
                        .bg(theme.accent_primary)
                        .add_modifier(Modifier::BOLD)
                } else if i == self.command_list.selected
                    && self.focus == DetailFocus::Commands
                {
                    theme.selected_style(theme.selection_bg_secondary)
                } else {
                    theme.normal_style()
                };
                (label, style)
            },
            preview,
            " (empty — press a to add a command) ",
        );

        if let Some(idx) = self.cmd_edit.editing {
            let pos = self.cmd_edit.insert_at.unwrap_or(idx);
            let display_prefix = format!("  #{}▶ ", pos);
            render_inline_cursor(
                frame, list_area, self.command_list.offset,
                pos, &self.cmd_edit.edit_input,
                unicode_width::UnicodeWidthStr::width(display_prefix.as_str()) as u16,
            );
        }
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_editing = self.var_edit.is_editing() || self.cmd_edit.is_editing();
        let status: String = if is_editing {
            " [Enter] Confirm  [Esc] Cancel".into()
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
        let text = format!(" {}  |  [Ctrl+S] Save  [Esc] Cancel", status);
        render_status_bar(frame, area, theme, &text);
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> DetailScreenAction {
        use crossterm::event::KeyCode;

        // Handle inline editing
        if let Some(idx) = self.var_edit.editing {
            return handle_variable_edit(
                &mut self.var_edit, key, idx, &mut self.set.variables, &mut self.variable_list,
            );
        }
        if let Some(idx) = self.cmd_edit.editing {
            return handle_command_edit(
                &mut self.cmd_edit, key, idx, &mut self.set.commands, &mut self.command_list,
            );
        }

        match key.code {
            KeyCode::Tab | KeyCode::Char('\t') => {
                self.commit_name_edit();
                self.focus = match self.focus {
                    DetailFocus::Name => DetailFocus::Group,
                    DetailFocus::Group => DetailFocus::Shell,
                    DetailFocus::Shell => DetailFocus::ExecMode,
                    DetailFocus::ExecMode => DetailFocus::Variables,
                    DetailFocus::Variables => DetailFocus::Commands,
                    DetailFocus::Commands => DetailFocus::Name,
                };
            }
            KeyCode::BackTab => {
                self.commit_name_edit();
                self.focus = match self.focus {
                    DetailFocus::Name => DetailFocus::Commands,
                    DetailFocus::Group => DetailFocus::Name,
                    DetailFocus::Shell => DetailFocus::Group,
                    DetailFocus::ExecMode => DetailFocus::Shell,
                    DetailFocus::Variables => DetailFocus::ExecMode,
                    DetailFocus::Commands => DetailFocus::Variables,
                };
            }
            KeyCode::Up => {
                match self.focus {
                    DetailFocus::Variables => { self.variable_list.select_previous(); }
                    DetailFocus::Commands => { self.command_list.select_previous(); }
                    _ => {}
                }
            }
            KeyCode::Down => {
                match self.focus {
                    DetailFocus::Variables => { self.variable_list.select_next(self.set.variables.len()); }
                    DetailFocus::Commands => { self.command_list.select_next(self.set.commands.len()); }
                    _ => {}
                }
            }
            KeyCode::Left => {
                match self.focus {
                    DetailFocus::Group => { self.cycle_group(-1); }
                    DetailFocus::Shell => { self.cycle_shell(-1); }
                    DetailFocus::ExecMode => { self.cycle_exec_mode(-1); }
                    _ => {}
                }
            }
            KeyCode::Right => {
                match self.focus {
                    DetailFocus::Group => { self.cycle_group(1); }
                    DetailFocus::Shell => { self.cycle_shell(1); }
                    DetailFocus::ExecMode => { self.cycle_exec_mode(1); }
                    _ => {}
                }
            }
            KeyCode::Enter => {
                match self.focus {
                    DetailFocus::Name => {
                        if self.editing_name {
                            // Second Enter: confirm edit
                            self.set.name = self.name_input.content.clone();
                            self.editing_name = false;
                        } else {
                            // First Enter: start editing
                            self.name_input = TextInput::new(self.set.name.clone());
                            self.editing_name = true;
                        }
                    }
                    DetailFocus::Variables if !self.set.variables.is_empty() => {
                        let idx = self.variable_list.selected.min(self.set.variables.len().saturating_sub(1));
                        let v = &self.set.variables[idx];
                        self.var_edit.edit_input = TextInput::new(format!("{}={}", v.name, v.default_value));
                        self.var_edit.editing = Some(idx);
                    }
                    DetailFocus::Commands if !self.set.commands.is_empty() => {
                        let idx = self.command_list.selected.min(self.set.commands.len().saturating_sub(1));
                        self.cmd_edit.edit_input = TextInput::new(self.set.commands[idx].command.clone());
                        self.cmd_edit.editing = Some(idx);
                    }
                    _ => {}
                }
            }
            KeyCode::Char('a' | 'A') => {
                match self.focus {
                    DetailFocus::Variables => {
                        self.var_edit.edit_input = TextInput::new(String::new());
                        let pos = (self.variable_list.selected + 1)
                            .min(self.set.variables.len());
                        self.var_edit.insert_at = Some(pos);
                        self.var_edit.editing = Some(self.set.variables.len());
                        self.variable_list.selected = pos;
                    }
                    DetailFocus::Commands => {
                        self.cmd_edit.edit_input = TextInput::new(String::new());
                        let pos = (self.command_list.selected + 1)
                            .min(self.set.commands.len());
                        self.cmd_edit.insert_at = Some(pos);
                        self.cmd_edit.editing = Some(self.set.commands.len());
                        self.command_list.selected = pos;
                    }
                    _ => {}
                }
            }
            KeyCode::Char('d' | 'D') => {
                match self.focus {
                    DetailFocus::Variables if !self.set.variables.is_empty() => {
                        let idx = self.variable_list.selected.min(self.set.variables.len().saturating_sub(1));
                        return DetailScreenAction::DeleteVariable(idx);
                    }
                    DetailFocus::Commands if !self.set.commands.is_empty() => {
                        let idx = self.command_list.selected.min(self.set.commands.len().saturating_sub(1));
                        return DetailScreenAction::DeleteCommand(idx);
                    }
                    _ => {}
                }
            }
            KeyCode::Char('s') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                return DetailScreenAction::Save(self.set.clone());
            }
            KeyCode::Esc => {
                if self.editing_name {
                    self.editing_name = false;
                } else {
                    return DetailScreenAction::Cancel;
                }
            }
            _ => {}
        };

        // Handle name editing (Enter to confirm is handled in the outer match)
        if self.editing_name {
            handle_text_input(&mut self.name_input, key);
        }

        DetailScreenAction::None
    }

    fn commit_name_edit(&mut self) {
        if self.editing_name {
            self.set.name = self.name_input.content.clone();
            self.editing_name = false;
        }
    }


    fn cycle_group(&mut self, delta: isize) {
        let current = self
            .groups
            .iter()
            .position(|g| g.id == self.set.group_id)
            .unwrap_or(0);
        let len = self.groups.len();
        if len == 0 {
            return;
        }
        let next = (current as isize + delta).rem_euclid(len as isize) as usize;
        self.set.group_id = self.groups[next].id;
    }

    fn cycle_shell(&mut self, delta: isize) {
        // Build a list that includes Custom at the appropriate position
        // Build 6-element cycle: SystemDefault, Bash, Zsh, Fish, PowerShell, Custom(prev path)
        let saved_custom = match &self.set.shell {
            ShellType::Custom(p) => Some(p.clone()),
            _ => None,
        };
        let variants = ShellType::builtin_variants();
        let current = match &self.set.shell {
            ShellType::Custom(_) => 5usize,
            other => variants.iter().position(|s| std::mem::discriminant(s) == std::mem::discriminant(other))
                .unwrap_or(0),
        };
        let next = ((current as isize + delta).rem_euclid(6)) as usize;
        self.set.shell = if next == 5 {
            ShellType::Custom(saved_custom.unwrap_or_else(|| "/usr/bin/sh".to_string()))
        } else {
            variants[next].clone()
        };
    }

    fn cycle_exec_mode(&mut self, delta: isize) {
        self.set.exec_mode = cycle_enum(
            &[ExecMode::StopOnError, ExecMode::ContinueOnError],
            &self.set.exec_mode,
            delta,
        );
    }
}

/// Generic cycle helper for enum variants.
fn cycle_enum<T: Clone + PartialEq>(variants: &[T], current: &T, delta: isize) -> T {
    let pos = variants.iter().position(|v| *v == *current).unwrap_or(0);
    let next = (pos as isize + delta).rem_euclid(variants.len() as isize) as usize;
    variants[next].clone()
}
