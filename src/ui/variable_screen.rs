use crate::action::AppAction;
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

/// Active variable-editing overlay — present only when prompting for variables.
pub struct VariableOverlay {
    pub inputs: Vec<TextInput>,
    pub names: Vec<String>,
    pub focus: usize,
    pub gi: usize,
    pub si: usize,
}

pub struct VariableScreenState {
    pub overlay: Option<VariableOverlay>,
}

impl Default for VariableScreenState {
    fn default() -> Self {
        Self::new()
    }
}

impl VariableScreenState {
    pub fn new() -> Self {
        Self { overlay: None }
    }

    pub fn activate(&mut self, set: &CommandSet, gi: usize, si: usize) {
        self.overlay = Some(VariableOverlay {
            inputs: set
                .variables
                .iter()
                .map(|v| TextInput::new(v.default_value.clone()))
                .collect(),
            names: set.variables.iter().map(|v| v.name.clone()).collect(),
            focus: 0,
            gi,
            si,
        });
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AppAction {
        let overlay = match &mut self.overlay {
            Some(o) => o,
            None => return AppAction::None,
        };
        match key.code {
            KeyCode::Enter => AppAction::ConfirmVariables,
            KeyCode::Esc => AppAction::CancelVariables,
            KeyCode::Tab | KeyCode::Down => {
                let n = overlay.inputs.len();
                if n > 0 {
                    overlay.focus = (overlay.focus + 1) % n;
                }
                AppAction::None
            }
            KeyCode::Up => {
                let n = overlay.inputs.len();
                if n > 0 {
                    overlay.focus = (overlay.focus + n - 1) % n;
                }
                AppAction::None
            }
            _ => {
                let n = overlay.inputs.len();
                if n > 0 && overlay.focus < n {
                    handle_text_input(&mut overlay.inputs[overlay.focus], key);
                }
                AppAction::None
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let overlay = match &self.overlay {
            Some(o) => o,
            None => return,
        };
        let count = overlay.inputs.len();
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
            let focused = i == overlay.focus;
            let row_style = if focused {
                theme.selected_style()
            } else {
                theme.normal_style()
            };
            let row = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
            let display = format!(" {} = {}", overlay.names[i], overlay.inputs[i].content);
            let var_line = fill_row(
                Line::from(Span::styled(display, row_style)),
                row_style,
                row.width,
            );
            frame.render_widget(Paragraph::new(var_line), row);
            if focused {
                let prefix_w = unicode_width::UnicodeWidthStr::width(" ")
                    + unicode_width::UnicodeWidthStr::width(overlay.names[i].as_str())
                    + unicode_width::UnicodeWidthStr::width(" = ");
                set_cursor_after_prefix(
                    frame,
                    &overlay.inputs[i].content,
                    overlay.inputs[i].cursor,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::AppAction;
    use crate::models::CommandSet;
    use crate::test_utils::make_key;
    use crossterm::event::KeyCode;

    #[test]
    fn test_tab_advances_focus() {
        let mut state = VariableScreenState::new();
        let set = CommandSet::new("test".to_string(), uuid::Uuid::new_v4());
        state.activate(&set, 0, 0);
        let _ = state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.overlay.as_ref().unwrap().focus, 0);
    }

    #[test]
    fn test_enter_with_variables_returns_confirm() {
        let mut state = VariableScreenState::new();
        state.overlay = Some(VariableOverlay {
            inputs: vec![TextInput::new("val".to_string())],
            names: vec!["x".to_string()],
            focus: 0,
            gi: 0,
            si: 0,
        });
        let action = state.handle_key(make_key(KeyCode::Enter));
        assert!(matches!(action, AppAction::ConfirmVariables));
    }

    #[test]
    fn test_esc_returns_cancel_variables() {
        let mut state = VariableScreenState::new();
        state.overlay = Some(VariableOverlay {
            inputs: vec![],
            names: vec![],
            focus: 0,
            gi: 0,
            si: 0,
        });
        let action = state.handle_key(make_key(KeyCode::Esc));
        assert!(matches!(action, AppAction::CancelVariables));
    }
}
