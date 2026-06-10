use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
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
        self.content.insert(self.cursor, c);
        self.cursor += 1;
    }

    /// Delete the character before the cursor (backspace).
    pub fn delete_before(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.content.remove(self.cursor);
        }
    }

    /// Delete the character at the cursor (delete).
    pub fn delete_at(&mut self) {
        if self.cursor < self.content.len() {
            self.content.remove(self.cursor);
        }
    }

    pub fn move_cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor += 1;
        }
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
            // Set cursor position
            let cursor_x = inner.x + self.cursor as u16;
            let cursor_y = inner.y;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

// ---------------------------------------------------------------------------
// ConfirmDialog — a centered yes/no confirmation dialog
// ---------------------------------------------------------------------------

pub struct ConfirmDialog {
    pub visible: bool,
    pub title: String,
    pub message: String,
    pub selected: bool, // true = Yes, false = No
}

impl ConfirmDialog {
    pub fn new(title: String, message: String) -> Self {
        Self {
            visible: true,
            title,
            message,
            selected: true, // default to Yes
        }
    }

    pub fn toggle(&mut self) {
        self.selected = !self.selected;
    }

    pub fn is_confirmed(&self) -> bool {
        self.selected
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let width = area.width.min(50).saturating_sub(4);
        let height = 7;
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;

        let dialog_area = Rect::new(x, y, width, height);

        // Clear area
        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(self.title.as_str())
            .style(Style::default().bg(Color::DarkGray));

        let inner = block.inner(dialog_area);
        frame.render_widget(&block, dialog_area);

        // Message
        let msg = Paragraph::new(self.message.as_str())
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White));
        frame.render_widget(msg, Rect::new(inner.x, inner.y, inner.width, 1));

        // Yes/No buttons
        let yes_style = if self.selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let no_style = if !self.selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let btn_y = inner.y + 3;
        let btn_width = 8;
        let gap = 4;
        let total_width = btn_width * 2 + gap;
        let start_x = inner.x + (inner.width.saturating_sub(total_width)) / 2;

        let yes_label = Line::from(" Yes ").alignment(Alignment::Center);
        let yes_area = Rect::new(start_x, btn_y, btn_width, 1);
        frame.render_widget(
            Paragraph::new(yes_label).style(yes_style),
            yes_area,
        );

        let no_label = Line::from(" No ").alignment(Alignment::Center);
        let no_area = Rect::new(start_x + btn_width + gap, btn_y, btn_width, 1);
        frame.render_widget(
            Paragraph::new(no_label).style(no_style),
            no_area,
        );
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
