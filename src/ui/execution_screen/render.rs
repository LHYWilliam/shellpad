use super::{CmdStatus, ExecutionScreenState};
use crate::ui::render::bordered_block_zone;
use crate::ui::render::{list_scrollbar_areas, render_scrollbar, render_status_bar};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, List, ListItem, Paragraph};

impl ExecutionScreenState {
    pub(crate) fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ]);
        let [header_area, gauge_area, body_area] = vertical.areas(area);

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
        let progress = if self.total > 0 {
            completed_count as f64 / self.total as f64
        } else {
            0.0
        };
        let gauge_label = format!(
            "  {}/{}  {:.0}%  ",
            completed_count,
            self.total,
            progress * 100.0
        );
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(theme.accent_success).bg(theme.surface))
            .percent((progress * 100.0) as u16)
            .label(gauge_label);
        frame.render_widget(gauge, gauge_area);

        // Body: scrollable command output
        let mut items: Vec<ListItem> = Vec::new();

        for (i, state) in self.cmd_states.iter().enumerate() {
            // Command header
            let status_symbol = match state.status {
                CmdStatus::Pending => "⏳",
                CmdStatus::Running => "▶",
                CmdStatus::Success => "✅",
                CmdStatus::Failure => "❌",
                CmdStatus::Skipped => "⏭",
            };

            let status_color = match state.status {
                CmdStatus::Success => theme.accent_success,
                CmdStatus::Failure => theme.accent_error,
                CmdStatus::Running => theme.accent_warning,
                CmdStatus::Pending => theme.text_disabled,
                CmdStatus::Skipped => theme.text_disabled,
            };

            let duration_str = state.duration_ms.map(format_duration).unwrap_or_default();

            let header_style = if Some(i) == self.focus_index {
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

            // Output lines (indented)
            for line in &state.output_lines {
                let is_stderr = line.starts_with("[stderr]");
                items.push(ListItem::new(Line::from(Span::styled(
                    format!("   {}", line),
                    Style::default().fg(if is_stderr {
                        theme.accent_error
                    } else {
                        theme.text_primary
                    }),
                ))));
            }

            // Separator between commands
            if i + 1 < self.cmd_states.len() {
                let sep_width = area.width.saturating_sub(6) as usize;
                let separator = "╌".repeat(sep_width);
                items.push(ListItem::new(Line::from(Span::styled(
                    separator,
                    Style::default()
                        .fg(theme.text_disabled)
                        .add_modifier(Modifier::DIM),
                ))));
            }
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
        let footer_text = match (self.focus_index, self.completed, self.continue_from) {
            (Some(_), _, _) => "[←/→] Browse  [z] Follow  [q] Back",
            (None, true, None) => " [←/→] Browse  [r] Re-execute  [q] Back",
            (None, true, Some(_)) => " [←/→] Browse  [n] Continue from next  [r] Re-execute  [q] Back",
            (None, false, _) => " [←/→] Browse  [s] Skip  [z] Auto-scroll  [Ctrl+C] Interrupt  [q] Back",
        };

        let body_layout = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]);
        let [list_area, footer_area] = body_layout.areas(body_area);

        let list_inner = bordered_block_zone(frame, list_area, theme, " Output ", false);

        // Split list inner into content + scrollbar
        let (content_area, scrollbar_area) = list_scrollbar_areas(list_inner);

        // Scroll to focused command, or use auto-scroll offset
        let target_cmd = self.focus_index.unwrap_or(self.current_index);
        let scroll_offset = if self.focus_index.is_some() || self.auto_scroll {
            self.items_offset_for_command(target_cmd)
        } else {
            self.scroll_offset
        };
        let mut list_state = ratatui::widgets::ListState::default().with_offset(scroll_offset);
        frame.render_stateful_widget(List::new(items), content_area, &mut list_state);

        // Scrollbar tracks focused or current command
        render_scrollbar(
            frame,
            scrollbar_area,
            theme,
            self.cmd_states.len(),
            target_cmd,
        );

        render_status_bar(frame, footer_area, theme, footer_text);
    }
}

pub(crate) fn format_duration(d: u128) -> String {
    format!(" ({}.{:02}s)", d / 1000, d % 1000 / 10)
}
