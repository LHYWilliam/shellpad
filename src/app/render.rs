use crate::config::MIN_TERMINAL_HEIGHT;
use crate::config::MIN_TERMINAL_WIDTH;
use crate::mode::AppMode;
use crate::ui::help_screen::draw_help;
use crate::ui::toast::ToastSeverity;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use super::{App, ExecutionState};

impl App {
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
            let warning = Paragraph::new(Line::from(format!(
                "Terminal too small: {}x{} (min: {}x{})",
                area.width, area.height, MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT
            )))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Red));
            frame.render_widget(warning, area);
            return;
        }

        // Split off title bar
        let layout = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]);
        let [title_area, content_area] = layout.areas(area);

        // Render title bar
        let mode_str = match self.mode {
            AppMode::Main => "Main",
            AppMode::Detail => "Edit",
            AppMode::Execution => "Run",
            AppMode::Help => "Help",
        };
        let group_count = self.data.groups.len();
        let set_count: usize = self.data.groups.iter().map(|g| g.sets.len()).sum();
        let title_text = format!(
            " Launcher  |  {}  |  {} groups, {} sets  |  ? Help  q Quit",
            mode_str, group_count, set_count,
        );
        let title_paragraph = Paragraph::new(Line::from(Span::styled(
            title_text,
            Style::default()
                .fg(self.theme.text_secondary)
                .add_modifier(Modifier::DIM),
        )));
        frame.render_widget(title_paragraph, title_area);

        match self.mode {
            AppMode::Main => {
                self.main_screen
                    .render(frame, content_area, &self.data, &self.theme);
            }
            AppMode::Detail => {
                if let Some(ref mut ds) = self.detail_screen {
                    ds.render(frame, content_area, &self.theme);
                }
            }
            AppMode::Execution => {
                if let ExecutionState::Running { ref screen, .. } = self.execution_state {
                    screen.render(frame, content_area, &self.theme);
                }
            }
            AppMode::Help => {
                match self.prev_mode {
                    Some(AppMode::Detail) => {
                        if let Some(ref mut ds) = self.detail_screen {
                            ds.render(frame, content_area, &self.theme);
                        }
                    }
                    Some(AppMode::Execution) => {
                        if let ExecutionState::Running { ref screen, .. } = self.execution_state {
                            screen.render(frame, content_area, &self.theme);
                        }
                    }
                    _ => {
                        self.main_screen
                            .render(frame, content_area, &self.data, &self.theme);
                    }
                }
                draw_help(frame, content_area, &self.theme);
            }
        }

        self.variable_screen
            .render(frame, content_area, &self.theme);

        // Render toast notification (centered on title bar)
        if let Some(toast) = self.toasts.toasts.last() {
            let (toast_fg, toast_label) = match toast.severity {
                ToastSeverity::Success => (self.theme.accent_success, " ✓ "),
                ToastSeverity::Error => (self.theme.accent_error, " ✗ "),
                ToastSeverity::Info => (self.theme.accent_info, " ● "),
            };
            let toast_msg = format!("{}{}", toast_label, toast.message);
            let toast_display_width = unicode_width::UnicodeWidthStr::width(toast_msg.as_str());
            let toast_width = (toast_display_width as u16 + 2).min(area.width.saturating_sub(4));
            let x = (area.width.saturating_sub(toast_width)) / 2;
            let toast_area = Rect::new(x, title_area.y, toast_width, 1);
            frame.render_widget(Clear, toast_area);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    toast_msg,
                    Style::default().fg(toast_fg).add_modifier(Modifier::BOLD),
                ))),
                toast_area,
            );
        }
    }
}
