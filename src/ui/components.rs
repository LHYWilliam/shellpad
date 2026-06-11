use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

// ---------------------------------------------------------------------------
// TextInput — a single-line text input with cursor
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct TextInput {
    pub content: String,
    pub cursor: usize,
}

impl TextInput {
    pub fn new(content: String) -> Self {
        let cursor = content.len();
        Self { content, cursor }
    }

    /// Handle a character input.
    pub fn insert_char(&mut self, c: char) {
        let pos = self.content.floor_char_boundary(self.cursor);
        self.content.insert(pos, c);
        self.cursor = pos + c.len_utf8();
    }

    /// Delete the character before the cursor (backspace).
    pub fn delete_before(&mut self) {
        let pos = self.content.floor_char_boundary(self.cursor);
        if pos > 0 {
            let prev = self.content[..pos - 1].floor_char_boundary(pos - 1);
            self.content.remove(prev);
            self.cursor = prev;
        }
    }

    /// Delete the character at the cursor (delete).
    pub fn delete_at(&mut self) {
        let pos = self.content.floor_char_boundary(self.cursor);
        if pos < self.content.len() {
            self.content.remove(pos);
            self.cursor = pos;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            // floor_char_boundary on the prefix (known valid slice) finds prev char start
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

    /// Render the text input inside a given area.
    pub fn render(&self, frame: &mut Frame, area: Rect, focused: bool, title: &str) {
        let border_style = if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
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
            // Set cursor position (display width, not byte offset)
            let col = unicode_width::UnicodeWidthStr::width(&self.content[..self.cursor.min(self.content.len())]);
            let cursor_x = inner.x + col as u16;
            frame.set_cursor_position((cursor_x, inner.y));
        }
    }
}

// ---------------------------------------------------------------------------
// ScrollableList — a generic scrollable list of items
// ---------------------------------------------------------------------------

pub struct ScrollableList {
    pub selected: usize,
    pub offset: usize,
}

impl ScrollableList {
    pub fn new() -> Self {
        Self {
            selected: 0,
            offset: 0,
        }
    }

    pub fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn select_next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        if self.selected + 1 < len {
            self.selected += 1;
        }
    }

    /// Ensure selected item is visible, adjust offset if needed.
    pub fn update_offset(&mut self, visible_height: usize) {
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + visible_height {
            self.offset = self.selected.saturating_add(1).saturating_sub(visible_height);
        }
    }

    pub fn reset(&mut self) {
        self.selected = 0;
        self.offset = 0;
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
