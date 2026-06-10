use crate::models::{Command, CommandSet, ExecMode, Group, ShellType, Variable};
use crate::ui::components::{ScrollableList, TextInput};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
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
    pub editing_variable: Option<usize>,
    pub editing_command: Option<usize>,
    pub editing_name: bool,
    pub edit_input: TextInput,
    pub show_cancel_dialog: bool,
    pub delete_var_confirm: bool,
    pub delete_cmd_confirm: bool,
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
            editing_variable: None,
            editing_command: None,
            editing_name: false,
            edit_input: TextInput::new(String::new()),
            show_cancel_dialog: false,
            delete_var_confirm: false,
            delete_cmd_confirm: false,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!(" Edit: {} ", self.set.name));

        let inner = block.inner(area);
        frame.render_widget(&block, area);

        // Split into top metadata and bottom command areas
        let layout = Layout::vertical([
            Constraint::Length(6), // metadata (name, group, shell, mode)
            Constraint::Min(3),    // variables
            Constraint::Min(3),    // commands
            Constraint::Length(1), // status bar
        ]);
        let [meta_area, var_area, cmd_area, status_area] = layout.areas(inner);

        self.render_metadata(frame, meta_area);
        self.render_variables(frame, var_area);
        self.render_commands(frame, cmd_area);
        self.render_status_bar(frame, status_area);
    }

    fn render_metadata(&self, frame: &mut Frame, area: Rect) {
        // Name, Group, Shell, ExecMode in rows
        let rows = Layout::vertical([Constraint::Length(1); 4]);
        let [name_row, group_row, shell_row, mode_row] = rows.areas(area);

        // Name
        let name_style = if self.focus == DetailFocus::Name && !self.editing_name {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        let name_text = format!(" Name: {}", self.set.name);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(name_text, name_style))),
            name_row,
        );

        // Group
        let group_name = self
            .groups
            .iter()
            .find(|g| g.id == self.set.group_id)
            .map(|g| g.name.as_str())
            .unwrap_or("(unknown)");
        let group_style = if self.focus == DetailFocus::Group {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        let group_text = format!(" Group: {}", group_name);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(group_text, group_style))),
            group_row,
        );

        // Shell
        let shell_style = if self.focus == DetailFocus::Shell {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        let shell_text = format!(" Shell: {}", self.set.shell.label());
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(shell_text, shell_style))),
            shell_row,
        );

        // Exec mode
        let mode_style = if self.focus == DetailFocus::ExecMode {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        let mode_text = format!(" Mode: {}", self.set.exec_mode.label());
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(mode_text, mode_style))),
            mode_row,
        );
    }

    fn render_variables(&self, frame: &mut Frame, area: Rect) {
        let var_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if self.focus == DetailFocus::Variables {
                Color::Yellow
            } else {
                Color::DarkGray
            }))
            .title(format!(
                " Variables ({}) ",
                self.set.variables.len()
            ));

        let inner = var_block.inner(area);
        frame.render_widget(&var_block, area);

        let items: Vec<ListItem> = self
            .set
            .variables
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let label = format!("  {} = {}", v.name, v.default_value);
                let style = if Some(i) == self.editing_variable {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if i == self.variable_list.selected
                    && self.focus == DetailFocus::Variables
                {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

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
        frame.render_stateful_widget(List::new(items), inner, &mut list_state);
    }

    fn render_commands(&self, frame: &mut Frame, area: Rect) {
        let cmd_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if self.focus == DetailFocus::Commands {
                Color::Yellow
            } else {
                Color::DarkGray
            }))
            .title(format!(
                " Commands ({}) ",
                self.set.commands.len()
            ));

        let inner = cmd_block.inner(area);
        frame.render_widget(&cmd_block, area);

        let items: Vec<ListItem> = self
            .set
            .commands
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                let pos = cmd.position;
                let label = format!("  #{}  {}", pos, cmd.command);
                let style = if Some(i) == self.editing_command {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if i == self.command_list.selected
                    && self.focus == DetailFocus::Commands
                {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

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
        frame.render_stateful_widget(List::new(items), inner, &mut list_state);
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let status = match self.focus {
            DetailFocus::Name => "[Enter] Edit name  [Tab] Next",
            DetailFocus::Group => "[←/→] Change group  [Tab] Next",
            DetailFocus::Shell => "[←/→] Change shell  [Tab] Next",
            DetailFocus::ExecMode => "[←/→] Change mode  [Tab] Next",
            DetailFocus::Variables => "[a] Add  [e] Edit  [d] Delete  [Tab] Next",
            DetailFocus::Commands => "[a] Add  [e] Edit  [d] Delete  [Tab] Next",
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {}  |  [Ctrl+S] Save  [Esc] Cancel", status),
                Style::default().fg(Color::DarkGray),
            ))),
            area,
        );
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> DetailScreenAction {
        use crossterm::event::KeyCode;

        // Handle inline editing
        if let Some(idx) = self.editing_variable {
            return self.handle_variable_edit(key, idx);
        }
        if let Some(idx) = self.editing_command {
            return self.handle_command_edit(key, idx);
        }

        match key.code {
            KeyCode::Tab | KeyCode::Char('\t') => {
                let next = match self.focus {
                    DetailFocus::Name => DetailFocus::Group,
                    DetailFocus::Group => DetailFocus::Shell,
                    DetailFocus::Shell => DetailFocus::ExecMode,
                    DetailFocus::ExecMode => DetailFocus::Variables,
                    DetailFocus::Variables => DetailFocus::Commands,
                    DetailFocus::Commands => DetailFocus::Name,
                };
                self.focus = next;
            }
            KeyCode::BackTab => {
                let prev = match self.focus {
                    DetailFocus::Name => DetailFocus::Commands,
                    DetailFocus::Group => DetailFocus::Name,
                    DetailFocus::Shell => DetailFocus::Group,
                    DetailFocus::ExecMode => DetailFocus::Shell,
                    DetailFocus::Variables => DetailFocus::ExecMode,
                    DetailFocus::Commands => DetailFocus::Variables,
                };
                self.focus = prev;
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
                        self.edit_input = TextInput::new(format!("{}={}", v.name, v.default_value));
                        self.editing_variable = Some(idx);
                    }
                    DetailFocus::Commands if !self.set.commands.is_empty() => {
                        let idx = self.command_list.selected.min(self.set.commands.len().saturating_sub(1));
                        self.edit_input = TextInput::new(self.set.commands[idx].command.clone());
                        self.editing_command = Some(idx);
                    }
                    _ => {}
                }
            }
            KeyCode::Char('a' | 'A') => {
                match self.focus {
                    DetailFocus::Variables => {
                        self.edit_input = TextInput::new(String::new());
                        self.editing_variable = Some(self.set.variables.len());
                    }
                    DetailFocus::Commands => {
                        self.edit_input = TextInput::new(String::new());
                        self.editing_command = Some(self.set.commands.len());
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

    fn handle_variable_edit(&mut self, key: crossterm::event::KeyEvent, idx: usize) -> DetailScreenAction {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Enter => {
                // Parse "name=value" format
                let input = self.edit_input.content.clone();
                if let Some(eq_pos) = input.find('=') {
                    let name = input[..eq_pos].trim().to_string();
                    let value = input[eq_pos + 1..].trim().to_string();
                    let var = Variable {
                        name,
                        default_value: value,
                    };
                    if idx < self.set.variables.len() {
                        self.set.variables[idx] = var;
                    } else {
                        self.set.variables.push(var);
                    }
                } else if !input.is_empty() {
                    let var = Variable {
                        name: input.trim().to_string(),
                        default_value: String::new(),
                    };
                    if idx < self.set.variables.len() {
                        self.set.variables[idx] = var;
                    } else {
                        self.set.variables.push(var);
                    }
                }
                self.editing_variable = None;
            }
            KeyCode::Esc => {
                self.editing_variable = None;
            }
            KeyCode::Char(c) => {
                self.edit_input.insert_char(c);
            }
            KeyCode::Backspace => {
                self.edit_input.delete_before();
            }
            KeyCode::Delete => {
                self.edit_input.delete_at();
            }
            KeyCode::Left => {
                self.edit_input.move_cursor_left();
            }
            KeyCode::Right => {
                self.edit_input.move_cursor_right();
            }
            KeyCode::Home => {
                self.edit_input.move_cursor_to_start();
            }
            KeyCode::End => {
                self.edit_input.move_cursor_to_end();
            }
            _ => {}
        }
        DetailScreenAction::None
    }

    fn handle_command_edit(&mut self, key: crossterm::event::KeyEvent, idx: usize) -> DetailScreenAction {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Enter => {
                let cmd = self.edit_input.content.clone();
                let command = Command {
                    position: idx,
                    command: cmd,
                };
                if idx < self.set.commands.len() {
                    self.set.commands[idx] = command;
                } else {
                    self.set.commands.push(command);
                }
                // Re-index positions
                for (i, c) in self.set.commands.iter_mut().enumerate() {
                    c.position = i;
                }
                self.editing_command = None;
            }
            KeyCode::Esc => {
                self.editing_command = None;
            }
            KeyCode::Char(c) => {
                self.edit_input.insert_char(c);
            }
            KeyCode::Backspace => {
                self.edit_input.delete_before();
            }
            KeyCode::Delete => {
                self.edit_input.delete_at();
            }
            KeyCode::Left => {
                self.edit_input.move_cursor_left();
            }
            KeyCode::Right => {
                self.edit_input.move_cursor_right();
            }
            KeyCode::Home => {
                self.edit_input.move_cursor_to_start();
            }
            KeyCode::End => {
                self.edit_input.move_cursor_to_end();
            }
            _ => {}
        }
        DetailScreenAction::None
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
        let variants = ShellType::builtin_variants();
        let current = variants
            .iter()
            .position(|s| std::mem::discriminant(s) == std::mem::discriminant(&self.set.shell))
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(variants.len() as isize) as usize;
        // Preserve Custom variant if currently Custom
        match &self.set.shell {
            ShellType::Custom(_) => {
                if delta > 0 {
                    self.set.shell = ShellType::Fish;
                } else {
                    self.set.shell = ShellType::SystemDefault;
                }
            }
            _ => {
                self.set.shell = variants[next].clone();
            }
        }
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
