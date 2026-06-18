use super::{CmdStatus, ExecutionScreenState, MAX_OUTPUT_LINES};
use crate::ui::render::bordered_block_primary_zone;
use crate::ui::render::{empty_hint, list_scrollbar_areas, render_scrollbar, render_status_bar};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, List, ListItem, Paragraph};

impl ExecutionScreenState {
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
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
        let footer_text = match (
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
        };

        let body_layout = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]);
        let [list_area, footer_area] = body_layout.areas(body_area);

        let list_inner = bordered_block_primary_zone(frame, list_area, theme, " Output ");

        // Split list inner into content + scrollbar
        let (content_area, scrollbar_area) = list_scrollbar_areas(list_inner);

        let scroll_offset = self.scroll_offset(content_area.height);
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

pub(crate) fn format_duration(d: u128) -> String {
    format!(" ({}.{:02}s)", d / 1000, d % 1000 / 10)
}
