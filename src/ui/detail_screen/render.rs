use crate::models::{ExecMode, ShellType};
use crate::ui::render::bordered_block_zone;
use crate::ui::render::{
    empty_hint, fill_row, list_item_style, list_scrollbar_areas, render_inline_cursor,
    render_scrollbar, render_status_bar, set_cursor_after_prefix, styled_list_item,
};
use crate::ui::theme::Theme;
use crate::ui::widget::{InlineEdit, ScrollableList, TextInput};
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
            DetailFocus::Name
                | DetailFocus::Group
                | DetailFocus::Shell
                | DetailFocus::ExecMode
                | DetailFocus::WorkDir
        );
        let inner = bordered_block_zone(frame, area, theme, " Properties ", props_focused);

        // Text Fields (inline edit) + Options (one per row)
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let [name_row, workdir_row, sep_row, group_row, shell_row, mode_row] = rows.areas(inner);

        // Name
        self.render_editable_field(
            frame, name_row, theme, "Name",
            self.focus == DetailFocus::Name,
            self.editing_name,
            &self.name_input,
            &self.set.name,
            false,
        );

        // WorkDir
        self.render_editable_field(
            frame, workdir_row, theme, "WorkDir",
            self.focus == DetailFocus::WorkDir,
            self.workdir_editing,
            &self.workdir_input,
            self.set.working_dir.as_deref().unwrap_or("(default — shellpad CWD)"),
            self.set.working_dir.is_none(),
        );

        // Separator — full width
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" ── Options {} ", "─".repeat(sep_row.width.saturating_sub(12) as usize)),
                Style::default().fg(theme.text_disabled).add_modifier(Modifier::DIM),
            ))),
            sep_row,
        );

        // Group
        let group_name = self
            .groups
            .iter()
            .find(|g| g.id == self.set.group_id)
            .map(|g| g.name.as_str())
            .unwrap_or("(unknown)");
        let group_style = if self.focus == DetailFocus::Group {
            theme.selected_style()
        } else {
            theme.normal_style()
        };
        let group_label = if self.focus == DetailFocus::Group {
            format!(" ◄ Group: {} ►", group_name)
        } else {
            format!(" Group: {}", group_name)
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(group_label, group_style))),
            group_row,
        );

        // Shell
        let shell_style = if self.focus == DetailFocus::Shell {
            theme.selected_style()
        } else {
            theme.normal_style()
        };
        let shell_label = if self.focus == DetailFocus::Shell {
            format!(" ◄ Shell: {} ►", self.set.shell.label())
        } else {
            format!(" Shell: {}", self.set.shell.label())
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(shell_label, shell_style))),
            shell_row,
        );

        // Exec mode
        let mode_style = if self.focus == DetailFocus::ExecMode {
            theme.selected_style()
        } else {
            theme.normal_style()
        };
        let mode_label = if self.focus == DetailFocus::ExecMode {
            format!(" ◄ Mode: {} ►", self.set.exec_mode.label())
        } else {
            format!(" Mode: {}", self.set.exec_mode.label())
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(mode_label, mode_style))),
            mode_row,
        );
    }

    fn render_editable_field(
        &self,
        frame: &mut Frame,
        row: Rect,
        theme: &Theme,
        label: &str,
        focused: bool,
        editing: bool,
        input: &TextInput,
        display: &str,
        dim: bool,
    ) {
        let style = if editing {
            theme.editing_style()
        } else if focused {
            theme.selected_style()
        } else {
            theme.normal_style()
        };

        let display_style = if dim && !focused && !editing {
            Style::default()
                .fg(theme.text_disabled)
                .add_modifier(Modifier::DIM)
        } else {
            style
        };

        let text = if editing {
            format!(" {}: {}", label, input.content)
        } else {
            format!(" {}: {}", label, display)
        };

        let line = fill_row(
            Line::from(Span::styled(text, display_style)),
            display_style,
            row.width,
        );
        frame.render_widget(Paragraph::new(line), row);

        if editing {
            let prefix_width = unicode_width::UnicodeWidthStr::width(format!(" {}: ", label).as_str());
            set_cursor_after_prefix(
                frame,
                &input.content,
                input.cursor,
                prefix_width as u16,
                row,
            );
        }
    }

    fn render_edit_cursor(
        &self,
        frame: &mut Frame,
        list_area: Rect,
        edit: &InlineEdit,
        list: &ScrollableList,
        prefix: &str,
    ) {
        if let Some(idx) = edit.editing {
            let pos = edit.insert_at.unwrap_or(idx);
            render_inline_cursor(
                frame,
                list_area,
                list.offset,
                pos,
                &edit.edit_input,
                unicode_width::UnicodeWidthStr::width(prefix) as u16,
            );
        }
    }

    pub(crate) fn render_picker(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let (names, selected_idx, title): (Vec<String>, Option<usize>, &str) = match self.focus {
            DetailFocus::Group => {
                let idx = self.groups.iter().position(|g| g.id == self.set.group_id);
                let names = self.groups.iter().map(|g| g.name.clone()).collect();
                (names, idx, " Groups ")
            }
            DetailFocus::Shell => {
                let variants = ShellType::builtin_variants();
                let saved_custom = match &self.set.shell {
                    ShellType::Custom(p) => Some(p.clone()),
                    _ => None,
                };
                let mut names = Vec::new();
                let mut selected_idx = None;
                for (i, v) in variants.iter().enumerate() {
                    let selected = std::mem::discriminant(&self.set.shell)
                        == std::mem::discriminant(v);
                    if selected { selected_idx = Some(i); }
                    names.push(match v {
                        ShellType::SystemDefault => "System Default".to_string(),
                        ShellType::Custom(_) => unreachable!(),
                        _ => v.label(),
                    });
                }
                if let Some(ref path) = saved_custom {
                    if matches!(&self.set.shell, ShellType::Custom(_)) {
                        selected_idx = Some(names.len());
                    }
                    names.push(format!("Custom: {}", path));
                } else {
                    names.push("Custom".to_string());
                }
                (names, selected_idx, " Shells ")
            }
            DetailFocus::ExecMode => {
                let modes = ["Stop on Error", "Continue on Error"];
                let idx = if self.set.exec_mode == ExecMode::StopOnError {
                    Some(0)
                } else {
                    Some(1)
                };
                let names = modes.iter().map(|s| s.to_string()).collect();
                (names, idx, " Exec Mode ")
            }
            _ => return,
        };

        let total = names.len();
        let sel = selected_idx.unwrap_or(0);
        let inner = crate::ui::render::bordered_block_info_zone(frame, area, theme, title);

        // 7 rows: row 3 (0-indexed position 2) always holds sel
        const VISIBLE: usize = 7;
        const SEL_ROW: isize = 3;
        let sel_isize = sel as isize;
        let total_isize = total as isize;

        let center_label = |name: &str, style: Style, width: u16| -> ListItem<'static> {
            let raw = format!(" {}", name);
            let raw_w = unicode_width::UnicodeWidthStr::width(raw.as_str()) as u16;
            let left_pad = (width.saturating_sub(raw_w)) / 2;
            let label = format!("{}{}", " ".repeat(left_pad as usize), raw);
            styled_list_item(label, style, width)
        };

        let mut items: Vec<ListItem<'_>> = Vec::new();
        let mut sel_visual = None;
        for visual_row in 0..VISIBLE {
            let offset = visual_row as isize - SEL_ROW;
            let idx = sel_isize + offset;
            let in_bounds = idx >= 0 && idx < total_isize;
            if in_bounds {
                let i = idx as usize;
                let is_selected = offset == 0;
                let is_peek = offset == -SEL_ROW || offset == SEL_ROW;
                let style = if is_selected {
                    Style::default().fg(theme.accent_primary)
                } else if is_peek {
                    Style::default()
                        .fg(theme.text_disabled)
                        .add_modifier(Modifier::DIM)
                } else {
                    theme.normal_style()
                };
                if is_selected {
                    sel_visual = Some(items.len());
                }
                items.push(center_label(&names[i], style, inner.width));
            } else {
                items.push(styled_list_item(
                    String::new(), theme.normal_style(), inner.width,
                ));
                if offset == 0 {
                    sel_visual = Some(items.len() - 1);
                }
            }
        }

        let mut list_state = ratatui::widgets::ListState::default();
        if total > 0 {
            list_state.select(sel_visual);
        }
        frame.render_stateful_widget(
            List::new(items).highlight_style(
                Style::default().bg(theme.surface_border),
            ),
            inner,
            &mut list_state,
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
            let style = theme.editing_style();
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

        self.render_edit_cursor(frame, list_area, &self.var_edit, &self.variable_list, "  ▶ ");
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
            self.render_edit_cursor(frame, list_area, &self.cmd_edit, &self.command_list,
                &format!("  #{}▶ ", pos));
        }
    }

    pub(crate) fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_editing = self.var_edit.is_editing() || self.cmd_edit.is_editing();
        let text = match (is_editing, self.focus) {
            (true, _) => "[Enter] Confirm  [Esc] Cancel",
            (false, DetailFocus::Name) => "[Enter] Edit  [↑/↓] Navigate  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::Group) => "[←/→] Change  [↑/↓] Navigate  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::Shell) => "[←/→] Change  [↑/↓] Navigate  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::ExecMode) => "[←/→] Change  [↑/↓] Navigate  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::WorkDir) => {
                "[Enter] Edit  [↑/↓] Navigate  [Tab] Next  |  [Ctrl+S] Save"
            }
            (false, DetailFocus::Variables) => {
                "[a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Ctrl+↑/↓] Move  [Tab] Next  |  [Ctrl+S] Save"
            }
            (false, DetailFocus::Commands) => {
                "[a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Ctrl+↑/↓] Move  [Tab] Next  |  [Ctrl+S] Save"
            }
        };
        render_status_bar(frame, area, theme, text);
    }
}
