use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn draw_help(frame: &mut Frame, area: Rect) {
    let width = area.width.saturating_sub(8).min(60);
    let height = 28;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    let help_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, help_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Help ")
        .style(Style::default().bg(Color::DarkGray));

    let inner = block.inner(help_area);
    frame.render_widget(&block, help_area);

    let cyan = Color::Cyan;
    let lines = vec![
        Line::from(""),
        Line::from("  Global:").style(Style::default().fg(cyan)),
        Line::from("    ? / Ctrl+H    Show this help"),
        Line::from("    q             Quit / Go back"),
        Line::from(""),
        Line::from("  Main Screen:").style(Style::default().fg(cyan)),
        Line::from("    ↑/↓           Navigate list"),
        Line::from("    ←/→           Switch between panels"),
        Line::from("    Enter         Execute selected command set"),
        Line::from("    e             Edit selected command set"),
        Line::from("    n             New command set"),
        Line::from("    d             Delete command set"),
        Line::from("    g             New group"),
        Line::from("    R             Rename group"),
        Line::from("    D             Delete group"),
        Line::from("    /             Search"),
        Line::from(""),
        Line::from("  Detail Screen:").style(Style::default().fg(cyan)),
        Line::from("    Tab/Shift+Tab  Switch focus region"),
        Line::from("    Ctrl+S        Save"),
        Line::from("    Esc           Cancel"),
        Line::from("    a             Add item"),
        Line::from("    e / Enter     Edit selected item"),
        Line::from("    d             Delete selected item"),
        Line::from(""),
        Line::from("  Execution Screen:").style(Style::default().fg(cyan)),
        Line::from("    q             Back to main"),
        Line::from("    Ctrl+C        Interrupt current command"),
        Line::from(""),
        Line::from("  Press any key to close."),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(paragraph, inner);
}
