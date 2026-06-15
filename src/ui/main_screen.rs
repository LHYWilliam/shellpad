use crate::ui::theme::Theme;
use crate::models::AppData;
use crate::ui::components::{handle_text_input, set_cursor_after_prefix, ScrollableList, TextInput};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

/// Find case-insensitive matches of `query` in `text`, returning byte-offset pairs
/// into `text` that are guaranteed valid for slicing.
/// Uses character-level case folding to avoid to_lowercase() byte-length mismatch.
fn find_matches_case_insensitive<'a>(text: &'a str, query: &str) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }

    let text_chars: Vec<(usize, char)> = text.char_indices().collect();
    let query_lower: Vec<char> = query.chars().flat_map(|c| c.to_lowercase()).collect();
    let text_lower: Vec<char> = text.chars().map(|c| {
        c.to_lowercase().next().unwrap_or(c)
    }).collect();

    let text_len = text_chars.len();
    let q_len = query_lower.len();
    let mut matches = Vec::new();
    let mut i = 0;
    while i + q_len <= text_len {
        if text_lower[i..i + q_len] == query_lower[..] {
            let byte_start = text_chars[i].0;
            let byte_end = if i + q_len < text_len {
                text_chars[i + q_len].0
            } else {
                text.len()
            };
            matches.push((byte_start, byte_end));
            i += q_len;
        } else {
            i += 1;
        }
    }
    matches
}

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

    pub fn render(&mut self, frame: &mut Frame, area: Rect, data: &AppData, theme: &Theme) {
        let vertical = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]);
        let [main_area, status_area] = vertical.areas(area);

        let horizontal = Layout::horizontal([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)]);
        let [left_area, right_area] = horizontal.areas(main_area);

        // Update scroll offsets before rendering (approximate inner height = area - 2 for borders)
        let left_vis = left_area.height.saturating_sub(2) as usize;
        let right_vis = right_area.height.saturating_sub(2) as usize;
        self.group_list.update_offset(left_vis);
        self.set_list.update_offset(right_vis);

        // Left panel: groups
        self.render_group_panel(frame, left_area, data, theme);

        // Right panel: command sets
        let sets = self.visible_sets(data);
        self.render_set_panel(frame, right_area, data, &sets, theme);

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
            set_cursor_after_prefix(
                frame,
                &ren.content,
                ren.cursor,
                prefix_w as u16,
                Rect::new(status_area.x, status_area.y, status_area.width, 1),
            );
        } else {
            self.render_status_bar(frame, status_area, theme);
        }
    }

    fn render_group_panel(&mut self, frame: &mut Frame, area: Rect, data: &AppData, theme: &Theme) {
        let border_color = if self.active_panel == Panel::Groups {
            theme.accent_primary
        } else {
            theme.surface_border
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" Groups ");

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
    }

    fn render_set_panel(
        &self,
        frame: &mut Frame,
        area: Rect,
        data: &AppData,
        sets: &[(usize, usize, &crate::models::CommandSet)],
        theme: &Theme,
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
            theme.accent_primary
        } else {
            theme.surface_border
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(title);

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

                let prefix = format!(" {}  ", mode_label);
                let suffix = format!("  [{}] ({} cmd)", shell_label, cmd_count);

                // Build name part with optional search highlighting
                let name_part: Vec<Span> = if self.search_mode && !self.search_query.is_empty() && !is_selected {
                    let matches = find_matches_case_insensitive(&set.name, &self.search_query);
                    if matches.is_empty() {
                        vec![Span::styled(set.name.clone(), text_style)]
                    } else {
                        let mut spans: Vec<Span> = Vec::new();
                        let mut last_end = 0usize;
                        for (match_start, match_end) in &matches {
                            if *match_start > last_end {
                                spans.push(Span::styled(
                                    &set.name[last_end..*match_start],
                                    text_style,
                                ));
                            }
                            spans.push(Span::styled(
                                &set.name[*match_start..*match_end],
                                Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
                            ));
                            last_end = *match_end;
                        }
                        if last_end < set.name.len() {
                            spans.push(Span::styled(&set.name[last_end..], text_style));
                        }
                        spans
                    }
                } else {
                    vec![Span::styled(set.name.clone(), text_style)]
                };

                let mut parts = vec![Span::styled(prefix, text_style)];
                parts.extend(name_part);
                parts.push(Span::styled(suffix, text_style));

                // Right-aligned group name in search mode
                if self.search_mode {
                    let gname = data.groups.get(gi).map(|g| g.name.as_str()).unwrap_or("?");
                    let text_width: usize = parts.iter().map(|s| {
                        unicode_width::UnicodeWidthStr::width(s.content.as_ref())
                    }).sum();
                    let pad = list_area.width as usize;
                    let padding = pad.saturating_sub(text_width + gname.len() + 1);
                    if padding > 0 {
                        parts.push(Span::styled(" ".repeat(padding), text_style));
                    }
                    parts.push(Span::styled(gname, text_style));
                }

                ListItem::new(Line::from(parts))
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
    }

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
                if self.active_panel == Panel::Sets
                    && let Some((gi, si)) = self.selected_set_idx(data) {
                        return MainScreenAction::ExecuteSet(gi, si);
                    }
                MainScreenAction::None
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if self.active_panel == Panel::Sets
                    && let Some((gi, si)) = self.selected_set_idx(data) {
                        return MainScreenAction::EditSet(gi, si);
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
                if self.active_panel == Panel::Sets
                    && let Some((gi, si)) = self.selected_set_idx(data) {
                        return MainScreenAction::DeleteSet(gi, si);
                    }
                MainScreenAction::None
            }
            KeyCode::Char('D') => {
                if self.active_panel == Panel::Groups
                    && let Some(gi) = self.selected_group_idx(data) {
                        return MainScreenAction::DeleteGroup(gi);
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
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.search_query.clear();
                self.set_list.reset();
                self.active_panel = Panel::Sets;
                MainScreenAction::None
            }
            KeyCode::Char('?') => {
                MainScreenAction::Help
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    return MainScreenAction::Help;
                }
                MainScreenAction::None
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

#[cfg(test)]
mod tests {
    use super::find_matches_case_insensitive;

    #[test]
    fn test_find_matches_ascii() {
        let m = find_matches_case_insensitive("deploy backend", "deploy");
        assert_eq!(m, vec![(0, 6)]);
    }

    #[test]
    fn test_find_matches_case_insensitive_ascii() {
        let m = find_matches_case_insensitive("Deploy Backend", "deploy");
        assert_eq!(m, vec![(0, 6)]);
    }

    #[test]
    fn test_find_matches_no_match() {
        let m = find_matches_case_insensitive("hello world", "xyz");
        assert!(m.is_empty());
    }

    #[test]
    fn test_find_matches_empty_query() {
        let m = find_matches_case_insensitive("hello", "");
        assert!(m.is_empty());
    }

    #[test]
    fn test_find_matches_multiple() {
        let m = find_matches_case_insensitive("test test test", "test");
        assert_eq!(m.len(), 3);
        assert_eq!(m[0], (0, 4));
        assert_eq!(m[1], (5, 9));
        assert_eq!(m[2], (10, 14));
    }

    #[test]
    fn test_find_matches_partial_word() {
        // "deployment" — "ploy" starts at char index 2 (byte 2)
        let m = find_matches_case_insensitive("deployment", "ploy");
        assert_eq!(m, vec![(2, 6)]);
    }

    #[test]
    fn test_find_matches_unicode_safe() {
        // Use characters whose case-folding does NOT change byte length
        let m = find_matches_case_insensitive("Café", "café");
        assert_eq!(m, vec![(0, 5)]);
    }

    #[test]
    fn test_find_matches_eszett_roundtrip() {
        // ẞ (U+1E9E, capital sharp S, 3 bytes in UTF-8) → ß (U+00DF, 2 bytes)
        // The match byte positions come from char_indices of the original text
        let text = "STRAẞE";
        // Search for "E" at the end — should only match the last character
        let m = find_matches_case_insensitive(text, "e");
        assert_eq!(m.len(), 1);
        // The match should be the last character "E" (byte 6..7)
        assert_eq!(&text[m[0].0..m[0].1], "E");
    }

    #[test]
    fn test_find_matches_eszett_inside() {
        // ẞ to ß changes byte length: 3 bytes → 2 bytes
        // This test verifies we don't panic on such strings
        let text = "STRAẞE";
        let m = find_matches_case_insensitive(text, "e");
        assert!(!m.is_empty());
        for (start, end) in &m {
            // Every slice should be valid UTF-8 (will not panic)
            let _ = &text[*start..*end];
        }
    }
}