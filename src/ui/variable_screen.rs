use crate::models::CommandSet;
use crate::ui::render::{bordered_block_info, centered_rect, fill_row, set_cursor_after_prefix};
use crate::ui::theme::Theme;
use crate::ui::widget::TextInput;
use crate::ui::widget::text_input::handle_text_input;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

pub enum VariableScreenAction {
    Execute { gi: usize, si: usize },
    Cancel,
    None,
}

pub struct VariableScreenState {
    pub active: bool,
    pub inputs: Vec<TextInput>,
    pub names: Vec<String>,
    pub focus: usize,
    /// Copies of the original group/set indices (not owned by pending_set)
    pub gi: usize,
    pub si: usize,
}

impl VariableScreenState {
    pub fn new() -> Self {
        Self {
            active: false,
            inputs: Vec::new(),
            names: Vec::new(),
            focus: 0,
            gi: 0,
            si: 0,
        }
    }

    pub fn activate(&mut self, set: &CommandSet, gi: usize, si: usize) {
        self.active = true;
        self.inputs = set
            .variables
            .iter()
            .map(|v| TextInput::new(v.default_value.clone()))
            .collect();
        self.names = set.variables.iter().map(|v| v.name.clone()).collect();
        self.focus = 0;
        self.gi = gi;
        self.si = si;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> VariableScreenAction {
        match key.code {
            KeyCode::Enter => VariableScreenAction::Execute {
                gi: self.gi,
                si: self.si,
            },
            KeyCode::Esc => VariableScreenAction::Cancel,
            KeyCode::Tab | KeyCode::Down => {
                let n = self.inputs.len();
                if n > 0 {
                    self.focus = (self.focus + 1) % n;
                }
                VariableScreenAction::None
            }
            KeyCode::Up => {
                let n = self.inputs.len();
                if n > 0 {
                    self.focus = (self.focus + n - 1) % n;
                }
                VariableScreenAction::None
            }
            _ => {
                let n = self.inputs.len();
                if n > 0 && self.focus < n {
                    handle_text_input(&mut self.inputs[self.focus], key);
                }
                VariableScreenAction::None
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.active {
            return;
        }
        let count = self.inputs.len();
        if count == 0 {
            return;
        }
        let dialog = centered_rect(area, area.width.min(60).saturating_sub(4), count as u16 + 4);

        frame.render_widget(Clear, dialog);

        let block =
            bordered_block_info(theme, " Set Variables ").style(Style::default().bg(theme.surface));
        frame.render_widget(&block, dialog);

        let inner = block.inner(dialog);

        for i in 0..count {
            let focused = i == self.focus;
            let row_style = if focused {
                theme.selected_style(theme.selection_bg_primary)
            } else {
                theme.normal_style()
            };
            let row = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
            let display = format!(" {} = {}", self.names[i], self.inputs[i].content);
            let var_line = fill_row(
                Line::from(Span::styled(display, row_style)),
                row_style,
                row.width,
            );
            frame.render_widget(Paragraph::new(var_line), row);
            if focused {
                let prefix_w = unicode_width::UnicodeWidthStr::width(" ") // leading space
                    + unicode_width::UnicodeWidthStr::width(self.names[i].as_str())
                    + unicode_width::UnicodeWidthStr::width(" = ");
                set_cursor_after_prefix(
                    frame,
                    &self.inputs[i].content,
                    self.inputs[i].cursor,
                    prefix_w as u16,
                    Rect::new(inner.x, inner.y + i as u16, inner.width, 1),
                );
            }
        }

        let hint_y = inner.y + count as u16;
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " [Enter] Execute  [Esc] Cancel  [Tab/Down] Next  [Up] Prev",
                theme.dim_style(),
            ))),
            Rect::new(inner.x, hint_y, inner.width, 1),
        );
    }
}
