use crate::action::{ConfirmChoice, DeleteKind};
use crate::ui::render::{bordered_block_error, centered_rect};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

/// Render the delete confirmation overlay dialog.
pub fn draw_confirm_dialog(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    kind: &DeleteKind,
    selected: ConfirmChoice,
) {
    let prompt = match kind {
        DeleteKind::Set { set_name, .. } => {
            format!("Delete set \"{}\"?", set_name)
        }
        DeleteKind::Group {
            group_name,
            set_count,
            ..
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
            cmd_index,
            cmd_preview,
            ..
        } => {
            let preview = if cmd_preview.len() > 40 {
                format!("{}...", &cmd_preview[..37])
            } else {
                cmd_preview.clone()
            };
            format!("Delete command #{} \"{}\"?", cmd_index, preview)
        }
    };

    let dialog_width = area.width.saturating_sub(8).min(50);
    let dialog_height = 7;
    let dialog_area = centered_rect(area, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let block = bordered_block_error(theme, " Delete ");
    let inner = block.inner(dialog_area);
    frame.render_widget(&block, dialog_area);

    // Prompt
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &prompt,
            Style::default().fg(theme.text_primary),
        )))
        .alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    // Button row
    let confirm_style = if matches!(selected, ConfirmChoice::Confirm) {
        theme.selected_style()
    } else {
        theme.normal_style()
    };
    let cancel_style = if matches!(selected, ConfirmChoice::Cancel) {
        theme.selected_style()
    } else {
        theme.normal_style()
    };

    let buttons = Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled(
            if matches!(selected, ConfirmChoice::Confirm) {
                "[Confirm]"
            } else {
                " Confirm "
            },
            confirm_style,
        ),
        Span::styled("      ", Style::default()),
        Span::styled(
            if matches!(selected, ConfirmChoice::Cancel) {
                "[Cancel]"
            } else {
                " Cancel "
            },
            cancel_style,
        ),
    ]);
    frame.render_widget(
        Paragraph::new(buttons).alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
    );

    // Hint
    let hint = " ←/→ Select      Enter — Confirm ";
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hint,
            Style::default()
                .fg(theme.text_disabled)
                .add_modifier(Modifier::DIM),
        )))
        .alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + 5, inner.width, 1),
    );
}
