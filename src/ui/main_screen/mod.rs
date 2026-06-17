use crate::models::AppData;
use crate::ui::theme::Theme;
use crate::ui::widget::{ScrollableList, TextInput};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};

pub(crate) mod handler;
pub(crate) mod render;
pub(crate) mod search;

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
    pub search_input: TextInput,
    pub rename_mode: bool,
    pub rename_input: TextInput,
}

impl Default for MainScreenState {
    fn default() -> Self {
        Self::new()
    }
}

impl MainScreenState {
    pub fn new() -> Self {
        Self {
            group_list: ScrollableList::new(),
            set_list: ScrollableList::new(),
            active_panel: Panel::Groups,
            search_mode: false,
            search_input: TextInput::new(String::new()),
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
    pub fn visible_sets<'a>(&'a self, data: &'a AppData) -> Vec<crate::models::FilterResult<'a>> {
        if self.search_mode {
            data.filter_sets(&self.search_input.content)
        } else if let Some(gi) = self.selected_group_idx(data) {
            data.groups[gi]
                .sets
                .iter()
                .enumerate()
                .map(|(si, s)| crate::models::FilterResult {
                    group_index: gi,
                    set_index: si,
                    set: s,
                    name_matches: Vec::new(),
                })
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
        self.group_list.update_offset(left_vis);
        // set_list offset updated below (depends on search/normal mode)

        // Left panel: groups
        self.render_group_panel(frame, left_area, data, theme);

        // Right panel: Search + Results (search mode) or command sets (normal)
        if self.search_mode {
            let search_layout = Layout::vertical([
                Constraint::Length(3), // Search block
                Constraint::Min(1),    // Results block
            ]);
            let [search_area, results_area] = search_layout.areas(right_area);
            let right_vis = results_area.height.saturating_sub(2) as usize;
            self.set_list.update_offset(right_vis);
            self.render_search_block(frame, search_area, theme);
            let sets = self.visible_sets(data);
            self.render_set_panel(frame, results_area, data, &sets, theme);
        } else {
            let right_vis = right_area.height.saturating_sub(2) as usize;
            self.set_list.update_offset(right_vis);
            let sets = self.visible_sets(data);
            self.render_set_panel(frame, right_area, data, &sets, theme);
        }

        // Status bar (key hints always visible)
        self.render_status_bar(frame, status_area, theme);
    }
}
