use crate::ui::render::{bordered_block_info, centered_rect};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Clear, Paragraph};

pub fn draw_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    let help_area = centered_rect(area, area.width.saturating_sub(8).min(60), 28);

    frame.render_widget(Clear, help_area);

    let block = bordered_block_info(theme, " Help ").style(Style::default().bg(theme.surface));
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
        Line::from("    up/down / j/k Navigate list"),
        Line::from("    left/right    Switch between panels"),
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
        Line::from("    up/down        Navigate list (variables/commands)"),
        Line::from("    left/right     Cycle option (group/shell/mode)"),
        Line::from("    Enter / e      Edit selected item"),
        Line::from("    a              Add new item"),
        Line::from("    d              Delete selected item"),
        Line::from("    Ctrl+S         Save"),
        Line::from("    Esc            Cancel / Back"),
        Line::from(""),
        Line::from("  Execution Screen:").style(Style::default().fg(section_color)),
        Line::from("    left/right     Browse command output"),
        Line::from("    z              Toggle auto-scroll / Follow current"),
        Line::from("    s              Skip current command (running)"),
        Line::from("    Ctrl+C         Interrupt (running)"),
        Line::from("    n              Continue from next (after skip)"),
        Line::from("    r              Re-execute all (completed)"),
        Line::from("    q              Back to main"),
        Line::from(""),
        Line::from("  Press any key to close."),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(paragraph, inner);
}
