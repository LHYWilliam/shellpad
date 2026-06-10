use crate::models::AppData;
use crate::ui::components::{handle_text_input, ScrollableList, TextInput};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

pub enum MainScreenAction {
    None,
    ExecuteSet(usize, usize),     // (group_index, set_index)
    EditSet(usize, usize),        // (group_index, set_index)
    NewSet(usize),                // group_index
    DeleteSet(usize, usize),      // (group_index, set_index)
    NewGroup,
    RenameGroup(usize, String),
    DeleteGroup(usize),
    Quit,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Groups,
    Sets,
}

pub struct MainScreenState {
    pub group_list: ScrollableList,
    pub set_list: ScrollableList,
    pub active_panel: Panel,
    pub search_mode: bool,
    pub search_query: String,
    pub rename_mode: bool,
    pub rename_input: TextInput,
}

impl MainScreenState {
    pub fn new() -> Self {
        Self {
            group_list: ScrollableList::new(),
            set_list: ScrollableList::new(),
            active_panel: Panel::Groups,
            search_mode: false,
            search_query: String::new(),
            rename_mode: false,
            rename_input: TextInput::new(String::new()),
        }
    }

    /// Get the currently selected group index, if any.
    pub fn selected_group_idx(&self, data: &AppData) -> Option<usize> {
        if self.group_list.selected < data.groups.len() {
            Some(self.group_list.selected)
        } else {
            None
        }
    }

    /// Get the currently selected command set indices, if any.
    pub fn selected_set_idx(&self, data: &AppData) -> Option<(usize, usize)> {
        let gi = self.selected_group_idx(data)?;
        if self.set_list.selected < data.groups[gi].sets.len() {
            Some((gi, self.set_list.selected))
        } else {
            None
        }
    }

    /// Get all sets visible in current view (accounting for search).
    pub fn visible_sets<'a>(&'a self, data: &'a AppData) -> Vec<(usize, usize, &'a crate::models::CommandSet)> {
        if self.search_mode {
            data.filter_sets(&self.search_query)
        } else if let Some(gi) = self.selected_group_idx(data) {
            data.groups[gi]
                .sets
                .iter()
                .enumerate()
                .map(|(si, s)| (gi, si, s))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, data: &AppData) {
        let vertical = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]);
        let [main_area, status_area] = vertical.areas(area);

        let horizontal = Layout::horizontal([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)]);
        let [left_area, right_area] = horizontal.areas(main_area);

        // Left panel: groups
        self.render_group_panel(frame, left_area, data);

        // Right panel: command sets
        let sets = self.visible_sets(data);
        self.render_set_panel(frame, right_area, data, &sets);

        // Status bar (or rename input when in rename mode)
        if self.rename_mode {
            let prefix = " Rename: ";
            let ren = &self.rename_input;
            let display = format!("{}{}", prefix, ren.content);
            let style = if ren.content.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(display, style))),
                status_area,
            );
            frame.set_cursor_position((
                status_area.x + prefix.len() as u16 + ren.cursor as u16,
                status_area.y,
            ));
        } else {
            self.render_status_bar(frame, status_area);
        }
    }

    fn render_group_panel(&self, frame: &mut Frame, area: Rect, data: &AppData) {
        let border_color = if self.active_panel == Panel::Groups {
            Color::Yellow
        } else {
            Color::Cyan
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" Groups ");

        let inner = block.inner(area);
        frame.render_widget(&block, area);

        let avail = inner.width as usize;
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
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

        if data.groups.is_empty() {
            items.push(
                ListItem::new(Line::from(Span::styled(
                    " (empty — press g to add) ",
                    Style::default().fg(Color::DarkGray),
                ))),
            );
        }

        // Adjust offset
        let _vis_height = inner.height as usize;
        let mut list_state = ratatui::widgets::ListState::default()
            .with_selected(Some(self.group_list.selected));
        let list = List::new(items).highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_stateful_widget(list, inner, &mut list_state);
    }

    fn render_set_panel(
        &self,
        frame: &mut Frame,
        area: Rect,
        data: &AppData,
        sets: &[(usize, usize, &crate::models::CommandSet)],
    ) {
        let title = if self.search_mode {
            format!(" Search: {} ", self.search_query)
        } else {
            let group_name: &str = self
                .selected_group_idx(data)
                .map(|gi| data.groups[gi].name.as_str())
                .unwrap_or("Commands");
            format!(" {} ", group_name)
        };

        let border_color = if self.active_panel == Panel::Sets {
            Color::Yellow
        } else {
            Color::Cyan
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(title);

        let inner = block.inner(area);
        frame.render_widget(&block, area);

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
                    let avail = inner.width as usize;
                    let pad = avail.saturating_sub(label.len() + gname.len() + 1);
                    label = format!("{}{:>pad$}{}", label, "", gname, pad = pad);
                }
                let is_selected = i == self.set_list.selected
                    && self.active_panel == Panel::Sets;
                let style = if is_selected {
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

        let selected = if !sets.is_empty() && self.active_panel == Panel::Sets {
            Some(self.set_list.selected.min(sets.len().saturating_sub(1)))
        } else {
            None
        };
        let mut list_state = ratatui::widgets::ListState::default()
            .with_selected(selected);
        let list = List::new(items).highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_stateful_widget(list, inner, &mut list_state);
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let text = Line::from(Span::styled(
            " [↑/↓] Nav  [←/→] Panel  [Enter] Run  [e] Edit  [n] New  [d] Del set  [Shift+D] Del group  [g] Group  [/] Search  [?] Help  [q] Quit",
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(Paragraph::new(text), area);
    }

    /// Handle a key event, returning an action.
    pub fn handle_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        data: &AppData,
    ) -> MainScreenAction {
        use crossterm::event::KeyCode;

        // Rename mode (takes priority over search)
        if self.rename_mode {
            return match key.code {
                KeyCode::Enter => {
                    let name = self.rename_input.content.clone();
                    let gi = self.group_list.selected;
                    self.rename_mode = false;
                    MainScreenAction::RenameGroup(gi, name)
                }
                KeyCode::Esc => {
                    self.rename_mode = false;
                    MainScreenAction::None
                }
                _ => {
                    handle_text_input(&mut self.rename_input, key);
                    MainScreenAction::None
                }
            };
        }

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
                    }
                    self.search_mode = false;
                    self.active_panel = Panel::Sets;
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

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                match self.active_panel {
                    Panel::Groups => self.group_list.select_previous(),
                    Panel::Sets => {
                        if self.visible_sets(data).is_empty() {
                            self.active_panel = Panel::Groups;
                        } else {
                            self.set_list.select_previous();
                        }
                    }
                }
                MainScreenAction::None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                match self.active_panel {
                    Panel::Groups => self.group_list.select_next(data.groups.len()),
                    Panel::Sets => {
                        let n = self.visible_sets(data).len();
                        if n == 0 {
                            self.active_panel = Panel::Groups;
                        } else {
                            self.set_list.select_next(n);
                        }
                    }
                }
                MainScreenAction::None
            }
            KeyCode::Left => {
                match self.active_panel {
                    Panel::Sets => self.active_panel = Panel::Groups,
                    Panel::Groups => { /* already on the leftmost panel */ }
                }
                MainScreenAction::None
            }
            KeyCode::Right => {
                match self.active_panel {
                    Panel::Groups => {
                        let has_sets = self.selected_group_idx(data)
                            .map(|gi| !data.groups[gi].sets.is_empty())
                            .unwrap_or(false);
                        if has_sets {
                            self.active_panel = Panel::Sets;
                        }
                    }
                    Panel::Sets => { /* already on the rightmost panel */ }
                }
                MainScreenAction::None
            }
            KeyCode::Enter => {
                if self.active_panel == Panel::Sets {
                    if let Some((gi, si)) = self.selected_set_idx(data) {
                        return MainScreenAction::ExecuteSet(gi, si);
                    }
                }
                MainScreenAction::None
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if self.active_panel == Panel::Sets {
                    if let Some((gi, si)) = self.selected_set_idx(data) {
                        return MainScreenAction::EditSet(gi, si);
                    }
                }
                MainScreenAction::None
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if let Some(gi) = self.selected_group_idx(data) {
                    MainScreenAction::NewSet(gi)
                } else {
                    MainScreenAction::None
                }
            }
            KeyCode::Char('d') => {
                if self.active_panel == Panel::Sets {
                    if let Some((gi, si)) = self.selected_set_idx(data) {
                        return MainScreenAction::DeleteSet(gi, si);
                    }
                }
                MainScreenAction::None
            }
            KeyCode::Char('D') => {
                if self.active_panel == Panel::Groups {
                    if let Some(gi) = self.selected_group_idx(data) {
                        return MainScreenAction::DeleteGroup(gi);
                    }
                }
                MainScreenAction::None
            }
            KeyCode::Char('g') => {
                MainScreenAction::NewGroup
            }
            KeyCode::Char('R') => {
                if self.active_panel == Panel::Groups
                    && let Some(gi) = self.selected_group_idx(data) {
                    let current = data.groups[gi].name.clone();
                    self.rename_mode = true;
                    self.rename_input = TextInput::new(current);
                }
                MainScreenAction::None
            }
            KeyCode::Char('G') => {
                // Shift+G not handled here
                MainScreenAction::None
            }
            KeyCode::Delete | KeyCode::Char('x') => {
                // Delete group with D
                MainScreenAction::None
            }
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.search_query.clear();
                self.set_list.reset();
                MainScreenAction::None
            }
            KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Char('H') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    MainScreenAction::Help
                } else {
                    MainScreenAction::None
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                if key.code == KeyCode::Esc {
                    MainScreenAction::None
                } else {
                    MainScreenAction::Quit
                }
            }
            _ => MainScreenAction::None,
        }
    }
}
