use crate::ui::render::set_cursor_after_prefix;
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

/// Single-line text input with cursor tracking. Supports insert, delete,
/// and cursor movement. Key events are processed by the dedicated
/// `handle_text_input` function in this module.
#[derive(Debug, Clone)]
pub struct TextInput {
    pub content: String,
    pub cursor: usize,
}

impl TextInput {
    pub fn new(content: String) -> Self {
        let cursor = content.len();
        Self { content, cursor }
    }

    pub fn insert_char(&mut self, c: char) {
        let pos = self.content.floor_char_boundary(self.cursor);
        self.content.insert(pos, c);
        self.cursor = pos + c.len_utf8();
    }

    pub fn delete_before(&mut self) {
        let pos = self.content.floor_char_boundary(self.cursor);
        if pos > 0 {
            let prev = self.content[..pos - 1].floor_char_boundary(pos - 1);
            self.content.remove(prev);
            self.cursor = prev;
        }
    }

    pub fn delete_at(&mut self) {
        let pos = self.content.floor_char_boundary(self.cursor);
        if pos < self.content.len() {
            self.content.remove(pos);
            self.cursor = pos;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            let prev = self.content[..self.cursor].floor_char_boundary(self.cursor - 1);
            self.cursor = prev;
        }
    }

    pub fn move_cursor_right(&mut self) {
        let len = self.content.len();
        if self.cursor >= len {
            return;
        }
        let pos = self.content.floor_char_boundary(self.cursor);
        let ch = self.content[pos..].chars().next();
        let char_len = ch.map_or(1, |c| c.len_utf8());
        self.cursor = (pos + char_len).min(len);
    }

    pub fn move_cursor_to_start(&mut self) {
        self.cursor = 0;
    }

    pub fn move_cursor_to_end(&mut self) {
        self.cursor = self.content.len();
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, focused: bool, title: &str, theme: &Theme) {
        let border_style = if focused {
            Style::default().fg(theme.accent_primary)
        } else {
            Style::default().fg(theme.surface_border)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title);

        let inner = block.inner(area);
        frame.render_widget(&block, area);

        let display = if self.content.is_empty() {
            Line::from("")
        } else {
            Line::from(self.content.as_str())
        };

        let paragraph = Paragraph::new(display).style(Style::default());
        frame.render_widget(paragraph, inner);

        if focused {
            set_cursor_after_prefix(frame, &self.content, self.cursor, 0, inner);
        }
    }
}

/// Handle common text input key events.
pub fn handle_text_input(input: &mut TextInput, key: crossterm::event::KeyEvent) {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Char(c) => input.insert_char(c),
        KeyCode::Backspace => input.delete_before(),
        KeyCode::Delete => input.delete_at(),
        KeyCode::Left => input.move_cursor_left(),
        KeyCode::Right => input.move_cursor_right(),
        KeyCode::Home => input.move_cursor_to_start(),
        KeyCode::End => input.move_cursor_to_end(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::TextInput;

    #[test]
    fn test_new_empty() {
        let input = TextInput::new(String::new());
        assert!(input.content.is_empty());
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn test_new_with_content() {
        let input = TextInput::new("hello".to_string());
        assert_eq!(input.content, "hello");
        assert_eq!(input.cursor, 5);
    }

    #[test]
    fn test_insert_char_middle() {
        let mut input = TextInput::new("ab".to_string());
        input.cursor = 1; // between 'a' and 'b'
        input.insert_char('x');
        assert_eq!(input.content, "axb");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_insert_char_at_end() {
        let mut input = TextInput::new("abc".to_string());
        input.insert_char('d');
        assert_eq!(input.content, "abcd");
        assert_eq!(input.cursor, 4);
    }

    #[test]
    fn test_insert_char_empty() {
        let mut input = TextInput::new(String::new());
        input.insert_char('a');
        assert_eq!(input.content, "a");
        assert_eq!(input.cursor, 1);
    }

    #[test]
    fn test_delete_before_middle() {
        let mut input = TextInput::new("abcd".to_string());
        input.cursor = 3; // after 'c', before 'd'
        input.delete_before();
        assert_eq!(input.content, "abd");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_delete_before_at_start() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 0;
        input.delete_before();
        assert_eq!(input.content, "abc"); // unchanged
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn test_delete_at_middle() {
        let mut input = TextInput::new("abcd".to_string());
        input.cursor = 2; // at 'c'
        input.delete_at();
        assert_eq!(input.content, "abd");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_delete_at_end() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 3; // past end
        input.delete_at();
        assert_eq!(input.content, "abc"); // unchanged
        assert_eq!(input.cursor, 3);
    }

    #[test]
    fn test_move_cursor_left() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 2;
        input.move_cursor_left();
        assert_eq!(input.cursor, 1);
    }

    #[test]
    fn test_move_cursor_left_at_start() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 0;
        input.move_cursor_left();
        assert_eq!(input.cursor, 0); // clamped
    }

    #[test]
    fn test_move_cursor_right() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 1;
        input.move_cursor_right();
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_move_cursor_right_at_end() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 3;
        input.move_cursor_right();
        assert_eq!(input.cursor, 3); // clamped
    }

    #[test]
    fn test_move_cursor_to_start() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 2;
        input.move_cursor_to_start();
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn test_move_cursor_to_end() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 0;
        input.move_cursor_to_end();
        assert_eq!(input.cursor, 3);
    }

    #[test]
    fn test_clear() {
        let mut input = TextInput::new("hello".to_string());
        input.cursor = 3;
        input.clear();
        assert!(input.content.is_empty());
        assert_eq!(input.cursor, 0);
    }
}
