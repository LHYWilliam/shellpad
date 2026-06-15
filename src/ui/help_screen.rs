use crate::ui::components::{bordered_block_info, centered_rect};
use crate::ui::theme::Theme;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

pub fn draw_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    let help_area = centered_rect(area, area.width.saturating_sub(8).min(60), 28);

    frame.render_widget(Clear, help_area);

    let block = bordered_block_info(theme, " Help ")
        .style(Style::default().bg(theme.surface));

    let inner = block.inner(help_area);
    frame.render_widget(&block, help_area);

    let section_color = theme.accent_info;
    let lines = vec![
        Line::from(""),
        Line::from("  Global:").style(Style::default().fg(section_color)),
        Line::from("    ? / Ctrl+H    Show this help"),
        Line::from("    q             Quit / Go back"),
        Line::from(""),
        Line::from("  Main Screen:").style(Style::default().fg(section_color)),
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
        Line::from("  Detail Screen:").style(Style::default().fg(section_color)),
        Line::from("    Tab/Shift+Tab  Switch focus region"),
        Line::from("    Ctrl+S        Save"),
        Line::from("    Esc           Cancel"),
        Line::from("    a             Add item"),
        Line::from("    e / Enter     Edit selected item"),
        Line::from("    d             Delete selected item"),
        Line::from(""),
        Line::from("  Execution Screen:").style(Style::default().fg(section_color)),
        Line::from("    q             Back to main"),
        Line::from("    Ctrl+C        Interrupt current command"),
        Line::from(""),
        Line::from("  Press any key to close."),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(paragraph, inner);
}
