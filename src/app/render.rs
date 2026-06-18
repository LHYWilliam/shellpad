use crate::config::MIN_TERMINAL_HEIGHT;
use crate::config::MIN_TERMINAL_WIDTH;
use crate::mode::AppMode;
use crate::ui::help_screen::draw_help;
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
            AppMode::ConfirmDelete { .. } => "Confirm",
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

        match &self.mode {
            AppMode::Main => {
                self.main_screen.render(
                    frame,
                    content_area,
                    &self.data,
                    &self.theme,
                    self.trash.len(),
                );
            }
            AppMode::Detail => {
                if let Some(ref mut ds) = self.detail_screen {
                    ds.render(frame, content_area, &self.theme);
                }
            }
            AppMode::Execution => {
                if let ExecutionState::Running { ref mut screen, .. } = self.execution_state {
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
                        if let ExecutionState::Running { ref mut screen, .. } = self.execution_state
                        {
                            screen.render(frame, content_area, &self.theme);
                        }
                    }
                    _ => {
                        self.main_screen.render(
                            frame,
                            content_area,
                            &self.data,
                            &self.theme,
                            self.trash.len(),
                        );
                    }
                }
                draw_help(frame, content_area, &self.theme);
            }
            AppMode::ConfirmDelete {
                kind,
                prev,
                selected,
            } => {
                // Render underlying screen based on the stored prev mode
                match prev.as_ref() {
                    AppMode::Detail => {
                        if let Some(ref mut ds) = self.detail_screen {
                            ds.render(frame, content_area, &self.theme);
                        }
                    }
                    AppMode::Execution => {
                        if let ExecutionState::Running { ref mut screen, .. } = self.execution_state
                        {
                            screen.render(frame, content_area, &self.theme);
                        }
                    }
                    _ => {
                        self.main_screen.render(
                            frame,
                            content_area,
                            &self.data,
                            &self.theme,
                            self.trash.len(),
                        );
                    }
                }
                crate::ui::confirm_dialog::draw_confirm_dialog(
                    frame,
                    content_area,
                    &self.theme,
                    kind,
                    *selected,
                );
            }
        }

        self.variable_screen
            .render(frame, content_area, &self.theme);

        // Render toast notifications — stacked bottom-right in content_area
        let toasts = &self.toasts.toasts;
        if !toasts.is_empty() {
            let max_w: u16 = toasts
                .iter()
                .map(|t| {
                    let msg_w = unicode_width::UnicodeWidthStr::width(t.message.as_str()) as u16;
                    msg_w + 8 // icon(2) + space + msg + title padding + borders
                })
                .max()
                .unwrap_or(20)
                .min(40);
            let toast_h = 3u16;
            let stack_h = toasts.len() as u16 * toast_h;
            let x = content_area.x + content_area.width.saturating_sub(max_w + 2);
            let y = content_area.y + content_area.height.saturating_sub(stack_h);

            for (i, toast) in toasts.iter().enumerate() {
                let row_y = y + i as u16 * toast_h;
                let area = Rect::new(x, row_y, max_w, toast_h);
                frame.render_widget(Clear, area);
                let title = format!(" {} {} ", toast.severity.icon(), toast.message);
                let block = crate::ui::render::bordered_block_info(&self.theme, &title);
                frame.render_widget(&block, area);
            }
        }
    }
}
