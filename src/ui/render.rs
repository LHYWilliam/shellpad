use crate::ui::theme::Theme;
use crate::ui::widget::text_input::TextInput;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

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
    let cursor_display =
        unicode_width::UnicodeWidthStr::width(&content[..cursor.min(content.len())]);
    frame.set_cursor_position((row.x + prefix_display_width + cursor_display as u16, row.y));
}

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
        Style::default()
            .fg(theme.text_disabled)
            .add_modifier(Modifier::ITALIC),
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
        Paragraph::new(Line::from(Span::styled(
            sep,
            Style::default().fg(theme.surface_border),
        ))),
        Rect::new(area.x, area.y, area.width, 1),
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            text,
            Style::default()
                .fg(theme.text_secondary)
                .add_modifier(Modifier::DIM),
        ))),
        Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        ),
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

/// Determine the style for a list item based on its editing/selection state.
pub fn list_item_style(is_editing: bool, is_selected: bool, theme: &Theme) -> Style {
    if is_editing {
        Style::default()
            .fg(theme.text_on_selected)
            .bg(theme.accent_primary)
            .add_modifier(Modifier::BOLD)
    } else if is_selected {
        theme.selected_style(theme.selection_bg_secondary)
    } else {
        theme.normal_style()
    }
}

/// Render a bordered block onto the frame, then return the inner Rect.
pub fn bordered_block_zone(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &str,
    focused: bool,
) -> Rect {
    let block = bordered_block(theme, title, focused);
    let inner = block.inner(area);
    frame.render_widget(&block, area);
    inner
}

/// Render a bordered info block onto the frame, then return the inner Rect.
pub fn bordered_block_info_zone(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &str,
) -> Rect {
    let block = bordered_block_info(theme, title);
    let inner = block.inner(area);
    frame.render_widget(&block, area);
    inner
}

/// Create a styled ListItem with full-row background fill.
pub fn styled_list_item(label: String, style: Style, width: u16) -> ListItem<'static> {
    ListItem::new(fill_row(
        Line::from(Span::styled(label, style)),
        style,
        width,
    ))
}
