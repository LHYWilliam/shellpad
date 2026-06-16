use crate::ui::render::bordered_block_zone;
use crate::ui::render::{
    empty_hint, fill_row, list_item_style, list_scrollbar_areas, render_inline_cursor,
    render_scrollbar, render_status_bar, set_cursor_after_prefix, styled_list_item,
};
use crate::ui::theme::Theme;
use crate::ui::widget::ScrollableList;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};
use super::{DetailFocus, DetailScreenState};

/// Editor context bundle for `render_items_list`.
pub(crate) struct ItemListEditCtx<'a> {
    editing_item: Option<usize>,
    insert_at: Option<usize>,
    preview_label: Option<String>,
    empty_text: &'a str,
}

impl DetailScreenState {
    pub(crate) fn render_metadata(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let props_focused = matches!(
            self.focus,
            DetailFocus::Name | DetailFocus::Group | DetailFocus::Shell | DetailFocus::ExecMode
        );
        let inner = bordered_block_zone(frame, area, theme, " Properties ", props_focused);

        // Name, Group+Shell, ExecMode in rows inside the block
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let [name_row, gs_row, mode_row] = rows.areas(inner);

        // Name
        let is_name_focused = self.focus == DetailFocus::Name;
        let name_style = if is_name_focused {
            if self.editing_name {
                Style::default()
                    .fg(theme.text_on_selected)
                    .bg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.accent_primary)
            }
        } else {
            theme.normal_style()
        };
        let display_name = if self.editing_name {
            self.name_input.content.as_str()
        } else {
            self.set.name.as_str()
        };
        let name_text = format!(" Name: {}", display_name);
        let name_line = fill_row(
            Line::from(Span::styled(name_text, name_style)),
            name_style,
            name_row.width,
        );
        frame.render_widget(Paragraph::new(name_line), name_row);

        // Cursor for name editing
        if self.editing_name {
            let prefix_width = unicode_width::UnicodeWidthStr::width(" Name: ");
            set_cursor_after_prefix(
                frame,
                &self.name_input.content,
                self.name_input.cursor,
                prefix_width as u16,
                name_row,
            );
        }

        // Group and Shell on the same row (side by side)
        let group_name = self
            .groups
            .iter()
            .find(|g| g.id == self.set.group_id)
            .map(|g| g.name.as_str())
            .unwrap_or("(unknown)");
        let group_style = if self.focus == DetailFocus::Group {
            Style::default().fg(theme.accent_primary)
        } else {
            theme.normal_style()
        };

        let shell_style = if self.focus == DetailFocus::Shell {
            Style::default().fg(theme.accent_primary)
        } else {
            theme.normal_style()
        };

        let half_layout = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]);
        let [group_col, shell_col] = half_layout.areas(gs_row);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" Group: {}", group_name),
                group_style,
            ))),
            group_col,
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" Shell: {}", self.set.shell.label()),
                shell_style,
            ))),
            shell_col,
        );

        // Exec mode (full width)
        let mode_style = if self.focus == DetailFocus::ExecMode {
            Style::default().fg(theme.accent_primary)
        } else {
            theme.normal_style()
        };
        let mode_text = format!(" Mode: {}", self.set.exec_mode.label());
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(mode_text, mode_style))),
            mode_row,
        );
    }

    /// Shared list renderer for Variables and Commands.
    /// `item_fn(index, is_editing) -> (label, style)` provides per-item content.
    /// Returns `list_area` for cursor positioning.
    pub(crate) fn render_items_list<F>(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        title: &str,
        focused: bool,
        count: usize,
        list: &ScrollableList,
        edit_ctx: ItemListEditCtx,
        item_fn: F,
    ) -> Rect
    where
        F: Fn(usize, bool) -> (String, Style),
    {
        let ItemListEditCtx {
            editing_item,
            insert_at,
            preview_label,
            empty_text,
        } = edit_ctx;

        let inner = bordered_block_zone(frame, area, theme, title, focused);

        let (list_area, scrollbar_area) = list_scrollbar_areas(inner);

        let mut items: Vec<ListItem> = (0..count)
            .map(|i| {
                let is_editing = Some(i) == editing_item;
                let (label, style) = item_fn(i, is_editing);
                styled_list_item(label, style, list_area.width)
            })
            .collect();

        // Preview row for new inserts
        if let Some(idx) = editing_item
            && insert_at.is_some()
            && let Some(label) = &preview_label
        {
            let style = Style::default()
                .fg(theme.text_on_selected)
                .bg(theme.accent_primary)
                .add_modifier(Modifier::BOLD);
            let preview = styled_list_item(label.clone(), style, list_area.width);
            let pos = insert_at.unwrap_or(idx.min(items.len()));
            items.insert(pos, preview);
        }

        if count == 0 {
            items.push(empty_hint(theme, empty_text));
        }

        let mut list_state =
            ratatui::widgets::ListState::default().with_selected(list.selected_or_none(count));
        frame.render_stateful_widget(List::new(items), list_area, &mut list_state);

        render_scrollbar(frame, scrollbar_area, theme, count, list.selected);
        list_area
    }

    pub(crate) fn render_variables(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let count = self.set.variables.len();
        let list_area = self.render_items_list(
            frame,
            area,
            theme,
            &format!(" Variables ({}) ", count),
            self.focus == DetailFocus::Variables,
            count,
            &self.variable_list,
            ItemListEditCtx {
                editing_item: self.var_edit.editing,
                insert_at: self.var_edit.insert_at,
                preview_label: self.var_edit.insert_at.is_some()
                    .then(|| format!("  ▶ {}", self.var_edit.edit_input.content)),
                empty_text: " (empty — press a to add a variable) ",
            },
            |i, is_editing| {
                let label = if is_editing {
                    format!("  ▶ {}", self.var_edit.edit_input.content)
                } else {
                    let v = &self.set.variables[i];
                    format!("  {} = {}", v.name, v.default_value)
                };
                let is_insert = self.var_edit.insert_at.is_some();
                let is_selected = !is_insert
                    && i == self.variable_list.selected
                    && self.focus == DetailFocus::Variables;
                let style = list_item_style(is_editing, is_selected, theme);
                (label, style)
            },
        );

        if let Some(idx) = self.var_edit.editing {
            let pos = self.var_edit.insert_at.unwrap_or(idx);
            render_inline_cursor(
                frame,
                list_area,
                self.variable_list.offset,
                pos,
                &self.var_edit.edit_input,
                unicode_width::UnicodeWidthStr::width("  ▶ ") as u16,
            );
        }
    }

    pub(crate) fn render_commands(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let count = self.set.commands.len();
        let list_area = self.render_items_list(
            frame,
            area,
            theme,
            &format!(" Commands ({}) ", count),
            self.focus == DetailFocus::Commands,
            count,
            &self.command_list,
            ItemListEditCtx {
                editing_item: self.cmd_edit.editing,
                insert_at: self.cmd_edit.insert_at,
                preview_label: self.cmd_edit.insert_at.is_some().then(|| {
                    let pos = self.cmd_edit.insert_at.unwrap_or(0);
                    format!("  #{}▶ {}", pos, self.cmd_edit.edit_input.content)
                }),
                empty_text: " (empty — press a to add a command) ",
            },
            |i, is_editing| {
                let pos = self.set.commands[i].position;
                let is_insert = self.cmd_edit.insert_at.is_some();
                let display_pos = if is_editing {
                    self.cmd_edit.insert_at.unwrap_or(pos)
                } else if is_insert && i >= self.cmd_edit.insert_at.unwrap() {
                    pos + 1
                } else {
                    pos
                };
                let content = if is_editing {
                    self.cmd_edit.edit_input.content.as_str()
                } else {
                    self.set.commands[i].command.as_str()
                };
                let label = format!("  #{}  {}", display_pos, content);
                let is_selected = !is_insert
                    && i == self.command_list.selected
                    && self.focus == DetailFocus::Commands;
                let style = list_item_style(is_editing, is_selected, theme);
                (label, style)
            },
        );

        if let Some(idx) = self.cmd_edit.editing {
            let pos = self.cmd_edit.insert_at.unwrap_or(idx);
            let display_prefix = format!("  #{}▶ ", pos);
            render_inline_cursor(
                frame,
                list_area,
                self.command_list.offset,
                pos,
                &self.cmd_edit.edit_input,
                unicode_width::UnicodeWidthStr::width(display_prefix.as_str()) as u16,
            );
        }
    }

    pub(crate) fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_editing = self.var_edit.is_editing() || self.cmd_edit.is_editing();
        let text = match (is_editing, self.focus) {
            (true, _) => "[Enter] Confirm  [Esc] Cancel",
            (false, DetailFocus::Name) => "[Enter] Edit name  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::Group) => "[←/→] Change group  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::Shell) => "[←/→] Change shell  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::ExecMode) => "[←/→] Change mode  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::Variables) => {
                "[a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Tab] Next  |  [Ctrl+S] Save"
            }
            (false, DetailFocus::Commands) => {
                "[a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Tab] Next  |  [Ctrl+S] Save"
            }
        };
        render_status_bar(frame, area, theme, text);
    }
}
