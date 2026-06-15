use crate::executor::ExecutionEvent;
use crate::ui::components::{bordered_block, list_scrollbar_areas, render_scrollbar};
use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, List, ListItem, Paragraph};
use ratatui::Frame;
use std::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdStatus {
    Pending,
    Running,
    Success,
    Failure,
    Skipped,
}

pub(crate) struct CmdState {
    pub(crate) status: CmdStatus,
    command: String,
    output_lines: Vec<String>,
    duration_ms: Option<u128>,
}

pub enum ExecutionScreenAction {
    BackToMain,
    Interrupt,
    Skip,
    Continue,
    Reexecute,
    None,
}

pub struct ExecutionScreenState {
    pub set_name: String,
    pub cmd_states: Vec<CmdState>,
    pub current_index: usize,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub completed: bool,
    pub continue_from: Option<usize>,
    pub total_duration_ms: Option<u128>,
    pub auto_scroll: bool,
    pub scroll_offset: usize,
}

impl ExecutionScreenState {
    pub fn new(set_name: String, commands: &[crate::models::Command]) -> Self {
        let cmd_states: Vec<CmdState> = commands
            .iter()
            .map(|c| CmdState {
                status: CmdStatus::Pending,
                command: c.command.clone(),
                output_lines: Vec::new(),
                duration_ms: None,
            })
            .collect();

        Self {
            set_name,
            total: cmd_states.len(),
            cmd_states,
            current_index: 0,
            succeeded: 0,
            failed: 0,
            skipped: 0,
            completed: false,
            continue_from: None,
            total_duration_ms: None,
            auto_scroll: true,
            scroll_offset: 0,
        }
    }

    /// Calculate the flat items Vec index for a given command index.
    fn items_offset_for_command(&self, cmd_idx: usize) -> usize {
        let mut offset = 0;
        for i in 0..cmd_idx.min(self.cmd_states.len()) {
            offset += 1; // command header line
            offset += self.cmd_states[i].output_lines.len(); // output lines
            offset += 1; // separator line
        }
        offset
    }

    /// Mark all remaining Pending commands as Skipped.
    /// Called after the execution thread is stopped (Skip or Interrupt).
    pub fn mark_remaining_as_skipped(&mut self) {
        self.completed = true;
        for (i, state) in self.cmd_states.iter_mut().enumerate() {
            if state.status == CmdStatus::Pending {
                state.status = CmdStatus::Skipped;
                self.skipped += 1;
                if self.continue_from.is_none() {
                    self.continue_from = Some(i);
                }
            }
        }
    }

    /// Reset the screen for continuing execution from a skip point.
    pub fn reset_from(&mut self, start_from: usize) {
        self.auto_scroll = true;
        self.scroll_offset = 0;
        for state in self.cmd_states[start_from..].iter_mut() {
            if state.status == CmdStatus::Skipped {
                state.status = CmdStatus::Pending;
            }
        }
        self.completed = false;
        self.continue_from = None;
    }

    /// Process events from the execution channel.
    pub fn process_events(&mut self, rx: &mpsc::Receiver<ExecutionEvent>) {
        while let Ok(event) = rx.try_recv() {
            match event {
                ExecutionEvent::Starting { index, command } => {
                    if index < self.cmd_states.len() {
                        self.cmd_states[index].status = CmdStatus::Running;
                        self.cmd_states[index].command = command;
                        self.current_index = index;
                        if self.auto_scroll {
                            self.scroll_offset = self.items_offset_for_command(index);
                        }
                    }
                }
                ExecutionEvent::StdoutLine { index, line } => {
                    if index < self.cmd_states.len() {
                        self.cmd_states[index].output_lines.push(line);
                    }
                }
                ExecutionEvent::StderrLine { index, line } => {
                    if index < self.cmd_states.len() {
                        self.cmd_states[index]
                            .output_lines
                            .push(format!("[stderr] {}", line));
                    }
                }
                ExecutionEvent::Finished {
                    index,
                    success,
                    duration_ms,
                } => {
                    if index < self.cmd_states.len() {
                        self.cmd_states[index].status = if success {
                            self.succeeded += 1;
                            CmdStatus::Success
                        } else {
                            self.failed += 1;
                            CmdStatus::Failure
                        };
                        self.cmd_states[index].duration_ms = Some(duration_ms);
                    }
                }
                ExecutionEvent::CompletedAll {
                    total: _,
                    succeeded: _,
                    failed: _,
                    total_duration_ms,
                } => {
                    self.completed = true;
                    self.total_duration_ms = Some(total_duration_ms);
                }
                ExecutionEvent::Interrupted { last_index: _ } => {
                    self.completed = true;
                }
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)]);
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
        let gauge_label = format!("  {}/{}  {:.0}%  ", completed_count, self.total, progress * 100.0);
        let gauge = Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(theme.accent_success)
                    .bg(theme.surface),
            )
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

            let duration_str = state
                .duration_ms
                .map(|d| format_duration(d))
                .unwrap_or_default();

            items.push(ListItem::new(Line::from(Span::styled(
                format!(" {} $ {}{}", status_symbol, state.command, duration_str),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ))));

            // Output lines (indented)
            for line in &state.output_lines {
                let is_stderr = line.starts_with("[stderr]");
                items.push(ListItem::new(Line::from(Span::styled(
                    format!("   {}", line),
                    Style::default().fg(if is_stderr { theme.accent_error } else { theme.text_primary }),
                ))));
            }

            // Separator between commands
            if i + 1 < self.cmd_states.len() {
                let sep_width = area.width.saturating_sub(6) as usize;
                let separator = "╌".repeat(sep_width);
                items.push(ListItem::new(Line::from(Span::styled(
                    separator,
                    Style::default().fg(theme.text_disabled).add_modifier(Modifier::DIM),
                ))));
            }
        }

        // Summary at bottom if completed
        if self.completed {
            let total_dur = self
                .total_duration_ms
                .map(|d| format_duration(d))
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
        let footer_text = if self.completed {
            if self.continue_from.is_some() {
                " [q] Back to main  [n] Continue from next  [r] Re-execute all"
            } else {
                " [q] Back to main  [r] Re-execute"
            }
        } else {
            " [q] Back to main  [s] Skip  [z] Auto-scroll  [Ctrl+C] Interrupt"
        };

        let body_layout = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]);
        let [list_area, footer_area] = body_layout.areas(body_area);

        let list_block = bordered_block(theme, " Output ", false);
        let list_inner = list_block.inner(list_area);
        frame.render_widget(&list_block, list_area);

        // Split list inner into content + scrollbar
        let (content_area, scrollbar_area) = list_scrollbar_areas(list_inner);

        // Use ListState with offset for auto-scroll
        let mut list_state = ratatui::widgets::ListState::default()
            .with_offset(self.scroll_offset);
        frame.render_stateful_widget(List::new(items), content_area, &mut list_state);

        // Scrollbar tracks current command position
        render_scrollbar(frame, scrollbar_area, theme, self.cmd_states.len(), self.current_index);

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                footer_text,
                theme.dim_style(),
            ))),
            footer_area,
        );
    }

    /// Handle key events.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ExecutionScreenAction {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('q') => ExecutionScreenAction::BackToMain,
            KeyCode::Char('c')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                ExecutionScreenAction::Interrupt
            }
            KeyCode::Char('s') if !self.completed => ExecutionScreenAction::Skip,
            KeyCode::Char('n') if self.completed && self.continue_from.is_some() => ExecutionScreenAction::Continue,
            KeyCode::Char('r') if self.completed => ExecutionScreenAction::Reexecute,
            KeyCode::Char('z') => {
                self.auto_scroll = !self.auto_scroll;
                ExecutionScreenAction::None
            }
            _ => ExecutionScreenAction::None,
        }
    }
}

fn format_duration(d: u128) -> String {
    format!(" ({}.{:02}s)", d / 1000, d % 1000 / 10)
}
