use super::{CmdStatus, ExecutionScreenState, MAX_OUTPUT_LINES, SearchState};
use crate::ui::render::bordered_block_primary_zone;
use crate::ui::render::bordered_block_zone;
use crate::ui::render::{
    empty_hint, list_scrollbar_areas, render_scrollbar, render_status_bar, set_cursor_after_prefix,
};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, List, ListItem, Paragraph};
use std::collections::HashSet;

/// Pure function: build the search bar text from query + match state.
fn search_bar_text(query: &str, matches: &[usize], current: usize) -> String {
    let count_hint = if query.is_empty() {
        String::new()
    } else if matches.is_empty() {
        " (no matches)".to_string()
    } else {
        format!(" ({}/{})", current + 1, matches.len())
    };
    format!(" Search: {}{} ", query, count_hint)
}

impl ExecutionScreenState {
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let search_height: u16 = if matches!(self.search, SearchState::Active { .. }) {
            3
        } else {
            0
        };

        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(search_height),
            Constraint::Min(1),
        ]);
        let [header_area, gauge_area, search_area, body_area] = vertical.areas(area);

        // Header
        let status_text = if self.completed {
            "Completed"
        } else {
            "Running..."
        };
        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" Executing: {} ", self.set_name),
                Style::default()
                    .fg(theme.accent_info)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" [{}]", status_text),
                Style::default().fg(if self.completed {
                    theme.accent_success
                } else {
                    theme.accent_warning
                }),
            ),
        ]));
        frame.render_widget(header, header_area);

        // Gauge progress bar
        let completed_count = self.succeeded + self.failed + self.skipped;
        let pct: u16 = if self.total > 0 {
            let raw = completed_count as f64 / self.total as f64;
            ((raw * 100.0) as u16).min(100)
        } else {
            0
        };
        let gauge_label = format!(
            "  {}/{}  {}%  ",
            completed_count.min(self.total),
            self.total,
            pct
        );
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(theme.accent_success).bg(theme.surface))
            .percent(pct)
            .label(gauge_label);
        frame.render_widget(gauge, gauge_area);

        // Search bar (only visible when search is active)
        if search_height > 0 {
            self.render_search_bar(frame, search_area, theme);
        }

        // Body: scrollable command output
        let mut items: Vec<ListItem> = Vec::new();

        // Pre-compute separator strings (loop-invariant)
        let sep_width = area.width.saturating_sub(6) as usize;
        let thin_sep = "─".repeat(sep_width);
        let thick_sep = "═".repeat(sep_width);

        for (i, state) in self.cmd_states.iter().enumerate() {
            // Command header
            let status_symbol = match (&state.status, state.exit_code) {
                (CmdStatus::Pending, _) if state.defer => "▽".to_string(),
                (CmdStatus::Pending, _) => "○".to_string(),
                (CmdStatus::Running, _) => "▶".to_string(),
                (CmdStatus::Success, _) => "✓".to_string(),
                (CmdStatus::Failure, Some(code)) => format!("[{}]", code),
                (CmdStatus::Failure, None) => "✕".to_string(),
                (CmdStatus::Skipped, _) => "~".to_string(),
            };

            let status_color = match state.status {
                CmdStatus::Success => theme.accent_success,
                CmdStatus::Failure => theme.accent_error,
                CmdStatus::Running => theme.accent_warning,
                CmdStatus::Pending => theme.text_disabled,
                CmdStatus::Skipped => theme.text_disabled,
            };

            let duration_str = state.duration_ms.map(format_duration).unwrap_or_default();

            let header_style = if self.browsing_index() == Some(i) {
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD)
            };

            items.push(ListItem::new(Line::from(Span::styled(
                format!(" {} $ {}{}", status_symbol, state.command, duration_str),
                header_style,
            ))));

            // Truncation marker — shown once above output lines
            if state.truncated {
                items.push(ListItem::new(Line::from(Span::styled(
                    format!(
                        "   ─ (output truncated, showing last {} lines) ─",
                        MAX_OUTPUT_LINES
                    ),
                    Style::default()
                        .fg(theme.text_disabled)
                        .add_modifier(Modifier::DIM),
                ))));
            }

            let match_set: HashSet<usize> =
                if let SearchState::Active { matches, .. } = &self.search {
                    matches.iter().copied().collect()
                } else {
                    HashSet::new()
                };

            let current_match_idx: Option<usize> =
                if let SearchState::Active {
                    current, matches, ..
                } = &self.search
                {
                    matches.get(*current).copied()
                } else {
                    None
                };

            // Output lines (indented)
            for (li, line) in (items.len()..).zip(state.output_lines.iter()) {
                let is_stderr = line.starts_with("[stderr]");
                let is_match = match_set.contains(&li);
                let is_current_match = current_match_idx == Some(li);

                let line_style = if is_current_match {
                    Style::default()
                        .fg(theme.text_on_selected)
                        .bg(theme.accent_primary)
                        .add_modifier(Modifier::BOLD)
                } else if is_match {
                    Style::default().fg(theme.accent_primary).bg(theme.surface)
                } else {
                    Style::default().fg(if is_stderr {
                        theme.accent_error
                    } else {
                        theme.text_primary
                    })
                };

                items.push(ListItem::new(Line::from(Span::styled(
                    format!("   {}", line),
                    line_style,
                ))));
            }

            // Separator between commands
            if i + 1 < self.cmd_states.len() {
                let is_defer_boundary = !state.defer && self.cmd_states[i + 1].defer;
                if is_defer_boundary {
                    items.push(ListItem::new(Line::from("")));
                }
                let (sep, fg, modif) = if is_defer_boundary {
                    (&thick_sep, theme.accent_info, Modifier::BOLD)
                } else {
                    (&thin_sep, theme.text_disabled, Modifier::DIM)
                };
                items.push(ListItem::new(Line::from(Span::styled(
                    sep.clone(),
                    Style::default().fg(fg).add_modifier(modif),
                ))));
            }
        }

        // Empty-state hint when no commands
        if self.cmd_states.is_empty() {
            items.push(empty_hint(theme, " (no commands — press q to go back) "));
        }

        // Summary at bottom if completed
        if self.completed {
            let total_dur = self
                .total_duration_ms
                .map(format_duration)
                .unwrap_or_default();
            items.push(ListItem::new(Line::from("")));
            items.push(ListItem::new(Line::from(Span::styled(
                format!(
                    " {} / {} completed, {} succeeded, {} failed, {} skipped{}",
                    self.succeeded + self.failed + self.skipped,
                    self.total,
                    self.succeeded,
                    self.failed,
                    self.skipped,
                    total_dur
                ),
                Style::default()
                    .fg(theme.accent_info)
                    .add_modifier(Modifier::BOLD),
            ))));
        }

        // Footer with key hints
        let footer_text = if matches!(self.search, SearchState::Active { .. }) {
            "[Esc] Cancel  [Enter] Exit  [↑/↓] Next match"
        } else {
            match (
                self.browsing_index(),
                self.paused,
                self.deferring,
                self.completed,
            ) {
            (Some(_), _, _, _) => {
                "[←/→] Browse  [↑/↓] Scroll  [PgUp/PgDn] Page  [z] Follow  [q] Back"
            }
            (None, _, true, false) => {
                "[←/→] Browse                                    Deferred commands running..."
            }
            (None, true, false, false) => "[n] Continue  [Ctrl+C] Abort  [←/→] Browse",
            (None, false, false, false) => "[s] Skip  [Ctrl+C] Abort  [←/→] Browse  [z] Follow",
            (None, _, _, true) => {
                "[←/→] Browse  [↑/↓] Scroll  [PgUp/PgDn] Page  [r] Re-execute  [q] Back"
            }
        }
    };

        let body_layout = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]);
        let [list_area, footer_area] = body_layout.areas(body_area);

        let list_inner = bordered_block_primary_zone(frame, list_area, theme, " Output ");

        // Split list inner into content + scrollbar
        let (content_area, scrollbar_area) = list_scrollbar_areas(list_inner);

        let scroll_offset = self.scroll_offset(content_area.height);
        self.last_offset = scroll_offset;
        self.visible_height = content_area.height as usize;
        let mut list_state = ratatui::widgets::ListState::default().with_offset(scroll_offset);
        frame.render_stateful_widget(List::new(items), content_area, &mut list_state);

        // Scrollbar tracks focused or current command
        render_scrollbar(
            frame,
            scrollbar_area,
            theme,
            self.items_total(),
            scroll_offset,
        );

        render_status_bar(frame, footer_area, theme, footer_text);
    }
}

impl ExecutionScreenState {
    fn render_search_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if let SearchState::Active {
            input,
            matches,
            current,
            ..
        } = &self.search
        {
            let inner = bordered_block_zone(frame, area, theme, " Search ", true);
            let text = search_bar_text(&input.content, matches, *current);
            let style = if matches.is_empty() && !input.content.is_empty() {
                Style::default().fg(theme.accent_error)
            } else {
                Style::default().fg(theme.accent_info)
            };
            let para = Paragraph::new(Line::from(Span::styled(text, style)));
            frame.render_widget(para, inner);

            let prefix_width = unicode_width::UnicodeWidthStr::width(" Search: ");
            set_cursor_after_prefix(
                frame,
                &input.content,
                input.cursor,
                prefix_width as u16,
                inner,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_bar_text_no_matches() {
        let text = search_bar_text("error", &[], 0);
        assert!(text.contains("(no matches)"));
    }

    #[test]
    fn test_search_bar_text_with_matches() {
        let text = search_bar_text("line", &[1, 3, 5], 1);
        assert!(text.contains("(2/3)"));
    }
}

pub(crate) fn format_duration(d: u128) -> String {
    format!(" ({}.{:02}s)", d / 1000, d % 1000 / 10)
}
