use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
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

    /// Clamp `selected` after a deletion: if the last item was removed,
    /// move selection to the new last item; otherwise keep it.
    pub fn clamp_selected(&mut self, len: usize) {
        if self.selected >= len {
            self.selected = len.saturating_sub(1);
        }
    }

    /// Return `Some(selected)` if the list is non-empty, else `None`,
    /// with the selected index clamped to `len - 1`.
    pub fn selected_or_none(&self, len: usize) -> Option<usize> {
        if len == 0 {
            None
        } else {
            Some(self.selected.min(len.saturating_sub(1)))
        }
    }
}

// ---------------------------------------------------------------------------
// InlineEdit — generic inline text-edit state for a list of T
// ---------------------------------------------------------------------------

/// Generic inline text-edit state for a list.
#[derive(Clone)]
pub struct InlineEdit {
    pub editing: Option<usize>,   // index of item being edited (or None)
    pub edit_input: TextInput,
    pub insert_at: Option<usize>, // Some(pos) = inserting new item at pos
}

impl InlineEdit {
    pub fn new() -> Self {
        Self {
            editing: None,
            edit_input: TextInput::new(String::new()),
            insert_at: None,
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing.is_some()
    }

    /// Commit the edit, either inserting at `insert_at` position or replacing at `idx`.
    pub fn commit<T>(&mut self, idx: usize, items: &mut Vec<T>, new_item: T, list: &mut ScrollableList) {
        if let Some(insert_pos) = self.insert_at.take() {
            items.insert(insert_pos, new_item);
            list.selected = insert_pos;
        } else {
            items[idx] = new_item;
            list.selected = idx;
        }
    }

    /// Cancel the current edit.
    pub fn cancel(&mut self) {
        self.insert_at = None;
        self.editing = None;
    }

    /// Handle a plain text key event (delegates to handle_text_input).
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        handle_text_input(&mut self.edit_input, key);
    }

    /// Handle a key event with an optional prefix-protection byte position.
    /// If `protect` is Some(pos), Backspace/Delete/Left are blocked when
    /// the cursor is at or before `pos`.
    pub fn handle_key_protected(&mut self, key: crossterm::event::KeyEvent, protect: Option<usize>) {
        use crossterm::event::KeyCode;
        let guard = protect.unwrap_or(0);
        match key.code {
            KeyCode::Backspace => { if self.edit_input.cursor > guard { self.edit_input.delete_before(); } }
            KeyCode::Delete => { if self.edit_input.cursor > guard { self.edit_input.delete_at(); } }
            KeyCode::Left => { if self.edit_input.cursor > guard { self.edit_input.move_cursor_left(); } }
            KeyCode::Right => self.edit_input.move_cursor_right(),
            KeyCode::Home => self.edit_input.move_cursor_to_start(),
            KeyCode::End => self.edit_input.move_cursor_to_end(),
            _ => { handle_text_input(&mut self.edit_input, key); }
        }
    }
}

/// Set the terminal cursor after a text prefix at the given row.
/// `prefix_display_width` is the display column width of the label before the editable content.
/// `content` is the full editable text, `cursor` is the byte offset within it.
pub fn set_cursor_after_prefix(
    frame: &mut Frame,
    content: &str,
    cursor: usize,
    prefix_display_width: u16,
    row: Rect,
) {
    let cursor_display = unicode_width::UnicodeWidthStr::width(
        &content[..cursor.min(content.len())],
    );
    frame.set_cursor_position((
        row.x + prefix_display_width + cursor_display as u16,
        row.y,
    ));
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

// ---------------------------------------------------------------------------
// Shared UI helper functions
// ---------------------------------------------------------------------------

/// Render a default scrollbar at the right side of a list area.
pub fn render_scrollbar(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    content_len: usize,
    position: usize,
) {
    let pos = position.min(content_len.saturating_sub(1));
    let mut state = ScrollbarState::new(content_len).position(pos);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(theme.surface_border)),
        area,
        &mut state,
    );
}

/// Create a bordered Block with a title and optional focus highlighting.
pub fn bordered_block<'a>(theme: &Theme, title: &'a str, focused: bool) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused {
            theme.accent_primary
        } else {
            theme.surface_border
        }))
        .title(title)
}

/// Create a bordered Block with accent_info color for overlay dialogs.
pub fn bordered_block_info<'a>(theme: &Theme, title: &'a str) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent_info))
        .title(title)
}

/// Create a disabled/italic ListItem for empty-state guidance.
pub fn empty_hint<'a>(theme: &Theme, text: &'a str) -> ListItem<'a> {
    ListItem::new(Line::from(Span::styled(
        text,
        Style::default().fg(theme.text_disabled).add_modifier(Modifier::ITALIC),
    )))
}

/// Split a Rect into a main list area (left) and a 1-column scrollbar area (right).
pub fn list_scrollbar_areas(area: Rect) -> (Rect, Rect) {
    let layout = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]);
    let [list, scrollbar] = layout.areas(area);
    (list, scrollbar)
}

/// Compute a centered Rect of the given width/height within the outer area.
pub fn centered_rect(outer: Rect, width: u16, height: u16) -> Rect {
    let x = outer.x + (outer.width.saturating_sub(width)) / 2;
    let y = outer.y + (outer.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(outer.width), height.min(outer.height))
}

/// Render a status bar with a top separator line and dim text.
pub fn render_status_bar(frame: &mut Frame, area: Rect, theme: &Theme, text: &str) {
    let sep = "─".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(sep, Style::default().fg(theme.surface_border)))),
        Rect::new(area.x, area.y, area.width, 1),
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            text,
            Style::default().fg(theme.text_secondary).add_modifier(Modifier::DIM),
        ))),
        Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1)),
    );
}

/// Position the cursor on an inline-editing row within a scrollable list.
/// Does nothing if the item is scrolled out of the visible area.
pub fn render_inline_cursor(
    frame: &mut Frame,
    list_area: Rect,
    list_offset: usize,
    item_index: usize,
    input: &TextInput,
    prefix_display_width: u16,
) {
    let item_y = list_area.y + item_index.saturating_sub(list_offset) as u16;
    if item_index >= list_offset && item_y < list_area.y + list_area.height {
        set_cursor_after_prefix(
            frame,
            &input.content,
            input.cursor,
            prefix_display_width,
            Rect::new(list_area.x, item_y, list_area.width, 1),
        );
    }
}

/// Pad a styled Line with trailing spaces up to `target_width` columns,
/// so that the background highlight extends to the full row width.
/// Uses `fill_style` for the padding spaces (typically the same style as the row).
pub fn fill_row(line: Line<'_>, fill_style: Style, target_width: u16) -> Line<'_> {
    let current: usize = line
        .spans
        .iter()
        .map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref()))
        .sum();
    let need = target_width.saturating_sub(current as u16) as usize;
    if need > 0 {
        let mut spans = line.spans;
        spans.push(Span::styled(" ".repeat(need), fill_style));
        Line::from(spans)
    } else {
        line
    }
}
