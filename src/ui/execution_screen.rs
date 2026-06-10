use crate::executor::ExecutionEvent;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
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
    status: CmdStatus,
    command: String,
    output_lines: Vec<String>,
    duration_ms: Option<u128>,
}

pub enum ExecutionScreenAction {
    BackToMain,
    Interrupt,
    Skip,
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
    pub completed: bool,
    pub total_duration_ms: Option<u128>,
    pub scroll: usize,
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
            completed: false,
            total_duration_ms: None,
            scroll: 0,
        }
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
                        self.scroll = self
                            .cmd_states
                            .len()
                            .saturating_sub(5);
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
                    status,
                    duration_ms,
                } => {
                    if index < self.cmd_states.len() {
                        self.cmd_states[index].status = if status.success() {
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

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]);
        let [header_area, body_area] = vertical.areas(area);

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
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" [{}]", status_text),
                Style::default().fg(if self.completed {
                    Color::Green
                } else {
                    Color::Yellow
                }),
            ),
        ]));
        frame.render_widget(header, header_area);

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
                CmdStatus::Success => Color::Green,
                CmdStatus::Failure => Color::Red,
                CmdStatus::Running => Color::Yellow,
                CmdStatus::Pending => Color::DarkGray,
                CmdStatus::Skipped => Color::DarkGray,
            };

            let duration_str = state
                .duration_ms
                .map(|d| format!(" ({}.{:02}s)", d / 1000, d % 1000 / 10))
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
                    Style::default().fg(if is_stderr { Color::Red } else { Color::White }),
                ))));
            }

            // Separator between commands
            if i + 1 < self.cmd_states.len() {
                items.push(ListItem::new(Line::from("")));
            }
        }

        // Summary at bottom if completed
        if self.completed {
            let total_dur = self
                .total_duration_ms
                .map(|d| format!(" ({}.{:02}s)", d / 1000, d % 1000 / 10))
                .unwrap_or_default();
            items.push(ListItem::new(Line::from("")));
            items.push(ListItem::new(Line::from(Span::styled(
                format!(
                    " {} / {} completed, {} succeeded, {} failed{}",
                    self.succeeded + self.failed,
                    self.total,
                    self.succeeded,
                    self.failed,
                    total_dur
                ),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))));
        }

        // Footer with key hints
        let footer_text = if self.completed {
            " [q] Back to main  [r] Re-execute"
        } else {
            " [q] Back to main  [Ctrl+C] Interrupt"
        };

        let body_layout = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]);
        let [list_area, footer_area] = body_layout.areas(body_area);

        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let list = List::new(items).block(list_block);
        frame.render_widget(list, list_area);

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                footer_text,
                Style::default().fg(Color::DarkGray),
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
            KeyCode::Char('r') if self.completed => ExecutionScreenAction::Reexecute,
            KeyCode::Char('s') if !self.completed => ExecutionScreenAction::Skip,
            KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                ExecutionScreenAction::None
            }
            KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
                ExecutionScreenAction::None
            }
            _ => ExecutionScreenAction::None,
        }
    }
}
