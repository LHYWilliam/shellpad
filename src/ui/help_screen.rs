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

    let mut lines = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from("  Global:").style(Style::default().fg(Color::Cyan)));
    lines.push(Line::from("    ? / Ctrl+H    Show this help"));
    lines.push(Line::from("    q             Quit / Go back"));
    lines.push(Line::from(""));
    lines.push(Line::from("  Main Screen:").style(Style::default().fg(Color::Cyan)));
    lines.push(Line::from("    ↑/↓           Navigate list"));
    lines.push(Line::from("    ←/→           Fold / expand groups"));
    lines.push(Line::from("    Enter         Execute selected command set"));
    lines.push(Line::from("    e             Edit selected command set"));
    lines.push(Line::from("    n             New command set"));
    lines.push(Line::from("    d             Delete command set"));
    lines.push(Line::from("    g             New group"));
    lines.push(Line::from("    R             Rename group"));
    lines.push(Line::from("    D             Delete group"));
    lines.push(Line::from("    /             Search"));
    lines.push(Line::from(""));
    lines.push(Line::from("  Detail Screen:").style(Style::default().fg(Color::Cyan)));
    lines.push(Line::from("    Tab/Shift+Tab  Switch focus region"));
    lines.push(Line::from("    Ctrl+S        Save"));
    lines.push(Line::from("    Esc           Cancel"));
    lines.push(Line::from("    a             Add item"));
    lines.push(Line::from("    e / Enter     Edit selected item"));
    lines.push(Line::from("    d             Delete selected item"));
    lines.push(Line::from(""));
    lines.push(Line::from("  Execution Screen:").style(Style::default().fg(Color::Cyan)));
    lines.push(Line::from("    q             Back to main"));
    lines.push(Line::from("    Ctrl+C        Interrupt current command"));
    lines.push(Line::from(""));
    lines.push(Line::from("  Press any key to close."));

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(paragraph, inner);
}
