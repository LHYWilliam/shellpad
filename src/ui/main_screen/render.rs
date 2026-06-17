use crate::ui::render::bordered_block_zone;
use crate::models::AppData;
use crate::ui::main_screen::search::find_matches_case_insensitive;
use crate::ui::main_screen::{MainScreenState, Panel};
use crate::ui::render::{
    empty_hint, fill_row, list_scrollbar_areas, render_inline_cursor, render_scrollbar,
    render_status_bar, set_cursor_after_prefix, styled_list_item,
};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};

impl MainScreenState {
    pub(crate) fn render_group_panel(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        data: &AppData,
        theme: &Theme,
    ) {
        let inner = bordered_block_zone(
            frame,
            area,
            theme,
            " Groups ",
            self.active_panel == Panel::Groups
        );

        // Split inner area into list + scrollbar
        let (list_area, scrollbar_area) = list_scrollbar_areas(inner);

        let avail = list_area.width as usize;
        let mut items: Vec<ListItem> = data
            .groups
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let marker = if i == self.group_list.selected {
                    "▶ "
                } else {
                    "  "
                };
                let display_name = if self.rename_mode && i == self.group_list.selected {
                    &self.rename_input.content
                } else {
                    &g.name
                };
                let name = format!("{}{}", marker, display_name);
                let count = format!("({})", g.sets.len());
                let name_width = unicode_width::UnicodeWidthStr::width(name.as_str());
                let pad = avail.saturating_sub(name_width + count.len());
                let label = format!("{}{:>pad$}{}", name, "", count, pad = pad);
                let style = if i == self.group_list.selected {
                    theme.selected_style(theme.selection_bg_primary)
                } else {
                    theme.normal_style()
                };
                styled_list_item(label, style, list_area.width)
            })
            .collect();

        if data.groups.is_empty() {
            items.push(empty_hint(theme, " (empty — press g to add) "));
        }

        let mut list_state = ratatui::widgets::ListState::default()
            .with_selected(self.group_list.selected_or_none(data.groups.len()));
        let list =
            List::new(items).highlight_style(theme.selected_style(theme.selection_bg_primary));
        frame.render_stateful_widget(list, list_area, &mut list_state);

        // Render scrollbar
        render_scrollbar(
            frame,
            scrollbar_area,
            theme,
            data.groups.len(),
            self.group_list.selected,
        );

        // Cursor for rename mode at the selected group name position
        if self.rename_mode && !data.groups.is_empty() {
            render_inline_cursor(
                frame,
                list_area,
                self.group_list.offset,
                self.group_list.selected,
                &self.rename_input,
                unicode_width::UnicodeWidthStr::width("▶ ") as u16,
            );
        }
    }

    pub(crate) fn render_search_block(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
    ) {
        let inner = bordered_block_zone(frame, area, theme, " Search ", false);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" Search: {} ", self.search_input.content),
                Style::default().fg(theme.text_primary),
            ))),
            inner,
        );
        let prefix_width = unicode_width::UnicodeWidthStr::width(" Search: ");
        set_cursor_after_prefix(
            frame,
            &self.search_input.content,
            self.search_input.cursor,
            prefix_width as u16,
            inner,
        );
    }

    pub(crate) fn render_set_panel(
        &self,
        frame: &mut Frame,
        area: Rect,
        data: &AppData,
        sets: &[(usize, usize, &crate::models::CommandSet)],
        theme: &Theme,
    ) {
        let title = if self.search_mode {
            " Search ".to_string()
        } else {
            let group_name: &str = self
                .selected_group_idx(data)
                .map(|gi| data.groups[gi].name.as_str())
                .unwrap_or("Commands");
            format!(" {} ", group_name)
        };

        let inner =
            bordered_block_zone(frame, area, theme, &title, self.active_panel == Panel::Sets);

        // When in search mode, split inner into search line + list area
        let (list_area, scrollbar_area) = if self.search_mode {
            let search_layout = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]);
            let [search_line, remaining] = search_layout.areas(inner);

            // Render search query line
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!(" Search: {} ", self.search_input.content),
                    Style::default().fg(theme.text_primary),
                ))),
                search_line,
            );

            // Cursor at end of search query
            let prefix_width = unicode_width::UnicodeWidthStr::width(" Search: ");
            set_cursor_after_prefix(
                frame,
                &self.search_input.content,
                self.search_input.cursor,
                prefix_width as u16,
                search_line,
            );

            // Split remaining into list + scrollbar
            let (list_area, sb_area) = list_scrollbar_areas(remaining);
            (list_area, sb_area)
        } else {
            // Original: split inner into list + scrollbar
            let (list_area, sb_area) = list_scrollbar_areas(inner);
            (list_area, sb_area)
        };

        let mut items: Vec<ListItem> = sets
            .iter()
            .enumerate()
            .map(|(i, &(gi, _, set))| {
                let shell_label = set.shell.label();
                let mode_label = match set.exec_mode {
                    crate::models::ExecMode::StopOnError => "🛑",
                    crate::models::ExecMode::ContinueOnError => "⏩",
                };
                let cmd_count = set.commands.len();
                let is_selected = i == self.set_list.selected && self.active_panel == Panel::Sets;
                let text_style = if is_selected {
                    theme.selected_style(theme.selection_bg_secondary)
                } else {
                    theme.normal_style()
                };

                let prefix = format!(" {}  ", mode_label);
                let suffix = format!("  [{}] ({} cmd)", shell_label, cmd_count);

                // Build name part with optional search highlighting
                let name_part: Vec<Span> =
                    if self.search_mode && !self.search_input.content.is_empty() && !is_selected {
                        let matches =
                            find_matches_case_insensitive(&set.name, &self.search_input.content);
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
                                    Style::default()
                                        .fg(theme.accent_primary)
                                        .add_modifier(Modifier::BOLD),
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
                    let text_width: usize = parts
                        .iter()
                        .map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref()))
                        .sum();
                    let pad = list_area.width as usize;
                    let padding = pad.saturating_sub(text_width + gname.len() + 1);
                    if padding > 0 {
                        parts.push(Span::styled(" ".repeat(padding), text_style));
                    }
                    parts.push(Span::styled(gname, text_style));
                }

                let set_line = fill_row(Line::from(parts), text_style, list_area.width);
                ListItem::new(set_line)
            })
            .collect();

        // Empty-state hint when no sets
        if sets.is_empty() {
            items.push(empty_hint(theme, " (empty — press n to add a set) "));
        }

        let selected = if self.active_panel == Panel::Sets {
            self.set_list.selected_or_none(sets.len())
        } else {
            None
        };
        let mut list_state = ratatui::widgets::ListState::default().with_selected(selected);
        let list =
            List::new(items).highlight_style(theme.selected_style(theme.selection_bg_secondary));
        frame.render_stateful_widget(list, list_area, &mut list_state);

        // Render scrollbar
        render_scrollbar(
            frame,
            scrollbar_area,
            theme,
            sets.len(),
            selected.unwrap_or(0),
        );
    }

    pub(crate) fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let text = if self.rename_mode {
            "[Enter] Confirm  [Esc] Cancel — renaming group"
        } else if self.search_mode {
            "[Enter] Confirm  [Esc] Cancel  [↑/↓] Nav — searching"
        } else {
            "[↑/↓] Nav  [←/→] Panel  [Ctrl+↑/↓] Move  [Enter] Run  [e] Edit  [n] New  [R] Rename  [d] Del set  [D] Del group  [g] New group  [/] Search  [q] Quit"
        };
        render_status_bar(frame, area, theme, text);
    }
}
