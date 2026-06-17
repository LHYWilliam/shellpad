use crate::models::AppData;
use crate::ui::main_screen::{MainScreenState, Panel};
use crate::ui::render::bordered_block_zone;
use crate::ui::render::{
    empty_hint, fill_row, list_scrollbar_areas, render_inline_cursor, render_scrollbar,
    render_status_bar, set_cursor_after_prefix, styled_list_item,
};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::Rect;
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
            self.active_panel == Panel::Groups,
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
                let style = if self.rename_mode && i == self.group_list.selected {
                    theme.editing_style()
                } else if i == self.group_list.selected {
                    theme.selected_style()
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
        let list_highlight = if self.rename_mode {
            theme.editing_style()
        } else {
            theme.selected_style()
        };
        let list = List::new(items).highlight_style(list_highlight);
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

    pub(crate) fn render_search_block(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let inner = bordered_block_zone(frame, area, theme, " Search ", self.search_mode);
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
        results: &[crate::models::FilterResult<'_>],
        theme: &Theme,
    ) {
        let title = if self.search_mode {
            " Results ".to_string()
        } else {
            let name = self
                .selected_group_idx(data)
                .map(|gi| data.groups[gi].name.as_str())
                .unwrap_or("Commands");
            format!(" {} ", name)
        };

        let inner =
            bordered_block_zone(frame, area, theme, &title, self.active_panel == Panel::Sets);

        let (list_area, scrollbar_area) = list_scrollbar_areas(inner);

        let mut items: Vec<ListItem> = results
            .iter()
            .enumerate()
            .map(|(i, result)| {
                let set = result.set;
                let shell_label = set.shell.label();
                let mode_label = match set.exec_mode {
                    crate::models::ExecMode::StopOnError => "■",
                    crate::models::ExecMode::ContinueOnError => "→",
                };
                let cmd_count = set.commands.len();
                let is_selected = i == self.set_list.selected && self.active_panel == Panel::Sets;
                let text_style = if is_selected {
                    theme.selected_style()
                } else {
                    theme.normal_style()
                };

                let prefix = format!(" {}  ", mode_label);
                let suffix = format!("  [{}] ({} cmd)", shell_label, cmd_count);

                // Build name part with fuzzy match highlighting
                let name_part: Vec<Span> =
                    if !result.name_matches.is_empty() && !is_selected {
                        let mut spans: Vec<Span> = Vec::new();
                        let mut last_end = 0usize;
                        for (match_start, match_end) in &result.name_matches {
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
                    } else {
                        vec![Span::styled(set.name.clone(), text_style)]
                    };

                let mut parts = vec![Span::styled(prefix, text_style)];
                parts.extend(name_part);
                parts.push(Span::styled(suffix, text_style));

                // Right-aligned group name in search mode
                if self.search_mode {
                    let gname = data.groups.get(result.group_index).map(|g| g.name.as_str()).unwrap_or("?");
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
        if results.is_empty() {
            items.push(empty_hint(theme, " (empty — press n to add a set) "));
        }

        let selected = if self.active_panel == Panel::Sets {
            self.set_list.selected_or_none(results.len())
        } else {
            None
        };
        let mut list_state = ratatui::widgets::ListState::default().with_selected(selected);
        let list = List::new(items).highlight_style(theme.selected_style());
        frame.render_stateful_widget(list, list_area, &mut list_state);

        // Render scrollbar
        render_scrollbar(
            frame,
            scrollbar_area,
            theme,
            results.len(),
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
