use crate::models::AppData;
use crate::ui::components::ScrollableList;
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
    RenameGroup(usize),
    DeleteGroup(usize),
    Quit,
    Help,
}

pub struct MainScreenState {
    pub group_list: ScrollableList,
    pub set_list: ScrollableList,
    pub search_mode: bool,
    pub search_query: String,
    pub show_delete_dialog: bool,
    pub delete_target: Option<(usize, usize)>,
    pub delete_dialog_is_set: bool, // true = deleting a set, false = deleting a group
}

impl MainScreenState {
    pub fn new() -> Self {
        Self {
            group_list: ScrollableList::new(),
            set_list: ScrollableList::new(),
            search_mode: false,
            search_query: String::new(),
            show_delete_dialog: false,
            delete_target: None,
            delete_dialog_is_set: true,
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
        if self.search_mode && !self.search_query.is_empty() {
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

        // Status bar
        self.render_status_bar(frame, status_area);
    }

    fn render_group_panel(&self, frame: &mut Frame, area: Rect, data: &AppData) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Groups ");

        let inner = block.inner(area);

        let mut items: Vec<ListItem> = data
            .groups
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let prefix = if i == self.group_list.selected {
                    "▶ "
                } else {
                    "  "
                };
                let label = format!("{}{} ({})", prefix, g.name, g.sets.len());
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
        let title = if self.search_mode && !self.search_query.is_empty() {
            format!(" Search: {} ", self.search_query)
        } else {
            let group_name: &str = self
                .selected_group_idx(data)
                .map(|gi| data.groups[gi].name.as_str())
                .unwrap_or("Commands");
            format!(" {} ", group_name)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(title);

        let inner = block.inner(area);

        let items: Vec<ListItem> = sets
            .iter()
            .enumerate()
            .map(|(i, &(_, _, set))| {
                let shell_label = set.shell.label();
                let mode_label = match set.exec_mode {
                    crate::models::ExecMode::StopOnError => "🛑",
                    crate::models::ExecMode::ContinueOnError => "⏩",
                };
                let cmd_count = set.commands.len();
                let label = format!(
                    " {}  {}  [{}] ({} cmd)",
                    mode_label, set.name, shell_label, cmd_count
                );
                let style = if i == self.set_list.selected {
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
            .with_selected(if sets.is_empty() {
                None
            } else {
                Some(self.set_list.selected.min(sets.len().saturating_sub(1)))
            });
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
            " [↑/↓] Nav  [Enter] Run  [e] Edit  [n] New  [d] Delete  [g] Group  [/] Search  [?] Help  [q] Quit",
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

        // If delete dialog is showing, handle that first
        if self.show_delete_dialog {
            return match key.code {
                KeyCode::Tab | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.show_delete_dialog = false;
                    let target = self.delete_target.take();
                    if let Some((gi, si)) = target {
                        if self.delete_dialog_is_set {
                            MainScreenAction::DeleteSet(gi, si)
                        } else {
                            MainScreenAction::DeleteGroup(gi)
                        }
                    } else {
                        MainScreenAction::None
                    }
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.show_delete_dialog = false;
                    self.delete_target = None;
                    MainScreenAction::None
                }
                _ => MainScreenAction::None,
            };
        }

        // Search mode
        if self.search_mode {
            return match key.code {
                KeyCode::Esc => {
                    self.search_mode = false;
                    self.search_query.clear();
                    self.set_list.reset();
                    MainScreenAction::None
                }
                KeyCode::Enter => {
                    self.search_mode = false;
                    self.search_query.clear();
                    MainScreenAction::None
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.set_list.reset();
                    MainScreenAction::None
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.set_list.reset();
                    MainScreenAction::None
                }
                _ => MainScreenAction::None,
            };
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                // Navigate right panel if has items, else left panel
                let set_count = self.visible_sets(data).len();
                if set_count > 0 {
                    self.set_list.select_previous();
                } else {
                    self.group_list.select_previous();
                }
                MainScreenAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let set_count = self.visible_sets(data).len();
                if set_count > 0 {
                    self.set_list.select_next(set_count);
                } else {
                    self.group_list.select_next(data.groups.len());
                }
                MainScreenAction::None
            }
            KeyCode::Left => {
                // Focus left panel (groups)
                self.group_list.select_previous();
                if data.groups.len() > 0 {
                    self.group_list.selected = self.group_list.selected.min(data.groups.len() - 1);
                }
                self.set_list.reset();
                MainScreenAction::None
            }
            KeyCode::Right => {
                // Focus right panel (sets)
                if !data.groups.is_empty() {
                    self.set_list.reset();
                }
                MainScreenAction::None
            }
            KeyCode::Enter => {
                // Execute selected set
                if let Some((gi, si)) = self.selected_set_idx(data) {
                    MainScreenAction::ExecuteSet(gi, si)
                } else {
                    MainScreenAction::None
                }
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if let Some((gi, si)) = self.selected_set_idx(data) {
                    MainScreenAction::EditSet(gi, si)
                } else {
                    MainScreenAction::None
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if let Some(gi) = self.selected_group_idx(data) {
                    MainScreenAction::NewSet(gi)
                } else {
                    MainScreenAction::None
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if let Some((gi, si)) = self.selected_set_idx(data) {
                    self.show_delete_dialog = true;
                    self.delete_target = Some((gi, si));
                    self.delete_dialog_is_set = true;
                }
                MainScreenAction::None
            }
            KeyCode::Char('g') => {
                MainScreenAction::NewGroup
            }
            KeyCode::Char('R') => {
                if let Some(gi) = self.selected_group_idx(data) {
                    return MainScreenAction::RenameGroup(gi);
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
