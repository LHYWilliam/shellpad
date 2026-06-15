use crate::models::{CommandSet, ExecMode, Group, ShellType};
use crate::ui::components::{set_cursor_after_prefix, ScrollableList, TextInput};
use crate::ui::detail_editor::DetailEditState;
use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
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
    pub edit_state: DetailEditState,
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
            edit_state: DetailEditState::new(),
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
            Constraint::Length(1), // status bar
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
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.surface_border))
            .title(" Properties ");

        let inner = block.inner(area);
        frame.render_widget(&block, area);

        // Name, Group+Shell, ExecMode in rows inside the block
        let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)]);
        let [name_row, gs_row, mode_row] = rows.areas(inner);

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
            Style::default().fg(theme.text_primary)
        };

        let shell_style = if self.focus == DetailFocus::Shell {
            Style::default().fg(theme.accent_primary)
        } else {
            Style::default().fg(theme.text_primary)
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
            Style::default().fg(theme.text_primary)
        };
        let mode_text = format!(" Mode: {}", self.set.exec_mode.label());
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(mode_text, mode_style))),
            mode_row,
        );
    }

    fn render_variables(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let var_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if self.focus == DetailFocus::Variables {
                theme.accent_primary
            } else {
                theme.surface_border
            }))
            .title(format!(
                " Variables ({}) ",
                self.set.variables.len()
            ));

        let inner = var_block.inner(area);
        frame.render_widget(&var_block, area);

        // Split into list + scrollbar
        let inner_layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
        let [list_area, scrollbar_area] = inner_layout.areas(inner);

        let mut items: Vec<ListItem> = self
            .set
            .variables
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let is_editing = Some(i) == self.edit_state.editing_variable;
                let is_insert = self.edit_state.insert_at.is_some();
                let label = if is_editing && !is_insert {
                    format!("  ▶ {}", self.edit_state.edit_input.content)
                } else {
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

        // Preview row only for new inserts (not for editing existing)
        if let Some(idx) = self.edit_state.editing_variable
            && self.edit_state.insert_at.is_some() {
                let label = format!("  ▶ {}", self.edit_state.edit_input.content);
                let style = Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD);
                let preview = ListItem::new(Line::from(Span::styled(label, style)));
                let pos = self.edit_state.insert_at.unwrap_or(idx.min(items.len()));
                items.insert(pos, preview);
            }

        let mut list_state = ratatui::widgets::ListState::default()
            .with_selected(if self.set.variables.is_empty() {
                None
            } else {
                Some(
                    self.variable_list
                        .selected
                        .min(self.set.variables.len().saturating_sub(1)),
                )
            });
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

        // Cursor for inline variable editing
        if let Some(idx) = self.edit_state.editing_variable {
            let item_y = list_area.y + idx.saturating_sub(self.variable_list.offset) as u16;
            if item_y < list_area.y + list_area.height {
                let prefix_width = unicode_width::UnicodeWidthStr::width("  ▶ ");
                set_cursor_after_prefix(
                    frame,
                    &self.edit_state.edit_input.content,
                    self.edit_state.edit_input.cursor,
                    prefix_width as u16,
                    Rect::new(list_area.x, item_y, list_area.width, 1),
                );
            }
        }
    }

    fn render_commands(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let cmd_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if self.focus == DetailFocus::Commands {
                theme.accent_primary
            } else {
                theme.surface_border
            }))
            .title(format!(
                " Commands ({}) ",
                self.set.commands.len()
            ));

        let inner = cmd_block.inner(area);
        frame.render_widget(&cmd_block, area);

        // Split into list + scrollbar
        let inner_layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
        let [list_area, scrollbar_area] = inner_layout.areas(inner);

        let mut items: Vec<ListItem> = self
            .set
            .commands
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                let pos = cmd.position;
                let is_editing = Some(i) == self.edit_state.editing_command;
                let is_insert = self.edit_state.insert_at.is_some();
                let display_pos = if is_editing {
                    self.edit_state.insert_at.unwrap_or(pos)
                } else if is_insert && i >= self.edit_state.insert_at.unwrap() {
                    pos + 1
                } else {
                    pos
                };
                let content = if is_editing && !is_insert {
                    self.edit_state.edit_input.content.as_str()
                } else {
                    cmd.command.as_str()
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

        // Preview row only for new inserts (not for editing existing)
        if let Some(idx) = self.edit_state.editing_command
            && self.edit_state.insert_at.is_some() {
                let pos = self.edit_state.insert_at.unwrap_or(idx);
                let label = format!("  #{}▶ {}", pos, self.edit_state.edit_input.content);
                let style = Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD);
                let preview = ListItem::new(Line::from(Span::styled(label, style)));
                let insert_pos = self.edit_state.insert_at.unwrap_or(idx.min(items.len()));
                items.insert(insert_pos, preview);
            }

        let mut list_state = ratatui::widgets::ListState::default()
            .with_selected(if self.set.commands.is_empty() {
                None
            } else {
                Some(
                    self.command_list
                        .selected
                        .min(self.set.commands.len().saturating_sub(1)),
                )
            });
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

        // Cursor for inline command editing
        if let Some(idx) = self.edit_state.editing_command {
            let item_y = list_area.y + idx.saturating_sub(self.command_list.offset) as u16;
            if item_y < list_area.y + list_area.height {
                let pos = self.edit_state.insert_at.unwrap_or(idx);
                let display_prefix = format!("  #{}▶ ", pos);
                let prefix_width = unicode_width::UnicodeWidthStr::width(display_prefix.as_str());
                set_cursor_after_prefix(
                    frame,
                    &self.edit_state.edit_input.content,
                    self.edit_state.edit_input.cursor,
                    prefix_width as u16,
                    Rect::new(list_area.x, item_y, list_area.width, 1),
                );
            }
        }
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
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
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {}  |  [Ctrl+S] Save  [Esc] Cancel", status),
                Style::default().fg(theme.text_secondary).add_modifier(Modifier::DIM),
            ))),
            area,
        );
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> DetailScreenAction {
        use crossterm::event::KeyCode;

        // Handle inline editing
        if let Some(idx) = self.edit_state.editing_variable {
            return self.edit_state.handle_variable_edit(
                key, idx, &mut self.set.variables, &mut self.variable_list,
            );
        }
        if let Some(idx) = self.edit_state.editing_command {
            return self.edit_state.handle_command_edit(
                key, idx, &mut self.set.commands, &mut self.command_list,
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
                        self.edit_state.edit_input = TextInput::new(format!("{}={}", v.name, v.default_value));
                        self.edit_state.editing_variable = Some(idx);
                    }
                    DetailFocus::Commands if !self.set.commands.is_empty() => {
                        let idx = self.command_list.selected.min(self.set.commands.len().saturating_sub(1));
                        self.edit_state.edit_input = TextInput::new(self.set.commands[idx].command.clone());
                        self.edit_state.editing_command = Some(idx);
                    }
                    _ => {}
                }
            }
            KeyCode::Char('a' | 'A') => {
                match self.focus {
                    DetailFocus::Variables => {
                        self.edit_state.edit_input = TextInput::new(String::new());
                        let pos = (self.variable_list.selected + 1)
                            .min(self.set.variables.len());
                        self.edit_state.insert_at = Some(pos);
                        self.edit_state.editing_variable = Some(self.set.variables.len());
                    }
                    DetailFocus::Commands => {
                        self.edit_state.edit_input = TextInput::new(String::new());
                        let pos = (self.command_list.selected + 1)
                            .min(self.set.commands.len());
                        self.edit_state.insert_at = Some(pos);
                        self.edit_state.editing_command = Some(self.set.commands.len());
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
        let variants = [ExecMode::StopOnError, ExecMode::ContinueOnError];
        let current = variants
            .iter()
            .position(|m| *m == self.set.exec_mode)
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(variants.len() as isize) as usize;
        self.set.exec_mode = variants[next];
    }
}
