use crate::action::DeleteKind;
use crate::ui::render::{bordered_block_error_zone, centered_rect};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

/// Render the delete confirmation overlay dialog.
pub fn draw_confirm_dialog(frame: &mut Frame, area: Rect, theme: &Theme, kind: &DeleteKind) {
    let prompt = match kind {
        DeleteKind::Set { set_name, .. } => {
            format!("Delete set \"{}\"?", set_name)
        }
        DeleteKind::Group {
            group_name, set_count, ..
        } => {
            if *set_count > 0 {
                format!(
                    "Delete group \"{}\" and all {} sets in it?",
                    group_name, set_count
                )
            } else {
                format!("Delete empty group \"{}\"?", group_name)
            }
        }
        DeleteKind::Variable { var_name, .. } => {
            format!("Delete variable \"{}\"?", var_name)
        }
        DeleteKind::Command {
            cmd_index, cmd_preview, ..
        } => {
            let preview = if cmd_preview.len() > 40 {
                format!("{}...", &cmd_preview[..37])
            } else {
                cmd_preview.clone()
            };
            format!("Delete command #{} \"{}\"?", cmd_index, preview)
        }
    };

    let hint = " y — confirm    n / Esc — cancel ";

    let dialog_width = area.width.saturating_sub(8).min(50);
    let dialog_height = 7;
    let dialog_area = centered_rect(area, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let inner = bordered_block_error_zone(frame, dialog_area, theme, " Delete ");

    // Vertical layout: empty, prompt, empty, hint, empty
    let inner_center_y = inner.y + 1;
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &prompt,
            Style::default().fg(theme.text_primary),
        )))
        .alignment(Alignment::Center),
        Rect::new(inner.x, inner_center_y, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hint,
            Style::default()
                .fg(theme.text_disabled)
                .add_modifier(Modifier::DIM),
        )))
        .alignment(Alignment::Center),
        Rect::new(inner.x, inner_center_y + 2, inner.width, 1),
    );
}
