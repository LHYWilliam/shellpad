use crate::models::{CommandSet, ExecMode, Group, ShellType};
use crate::ui::theme::Theme;
use crate::ui::widget::{InlineEdit, ScrollableList, TextInput};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};

pub(crate) mod editor;
pub(crate) mod handler;
pub(crate) mod render;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailFocus {
    Name,
    Group,
    Shell,
    ExecMode,
    WorkDir,
    Variables,
    Commands,
    DeferredCommands,
}

/// Bundles the list and inline-editor for one editable list region.
pub(crate) struct ListEditor {
    pub list: ScrollableList,
    pub edit: InlineEdit,
}

impl ListEditor {
    fn new() -> Self {
        Self {
            list: ScrollableList::new(),
            edit: InlineEdit::new(),
        }
    }
}

/// Exactly one editing operation can be active at a time.
pub(crate) enum EditingState {
    None,
    Name(TextInput),
    WorkDir(TextInput),
    #[allow(dead_code)]
    ListItem,
}

pub struct DetailScreenState {
    pub set: CommandSet,
    pub groups: Vec<Group>,
    pub focus: DetailFocus,
    pub(crate) editing: EditingState,
    pub(crate) var_editor: ListEditor,
    pub(crate) cmd_editor: ListEditor,
    pub(crate) deferred_editor: ListEditor,
}

impl DetailScreenState {
    pub fn new(set: CommandSet, groups: Vec<Group>) -> Self {
        Self {
            set,
            groups,
            focus: DetailFocus::Name,
            editing: EditingState::None,
            var_editor: ListEditor::new(),
            cmd_editor: ListEditor::new(),
            deferred_editor: ListEditor::new(),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent_info))
            .title(format!(
                " Edit: {} ",
                if let EditingState::Name(input) = &self.editing {
                    &input.content
                } else {
                    &self.set.name
                }
            ));

        let inner = block.inner(area);
        frame.render_widget(&block, area);

        // Split into top metadata and bottom command areas
        let layout = Layout::vertical([
            Constraint::Length(9), // Properties block + picker
            Constraint::Min(3),    // variables
            Constraint::Min(3),    // commands
            Constraint::Min(2),    // deferred commands
            Constraint::Length(2), // status bar
        ]);
        let [meta_area, var_area, cmd_area, def_area, status_area] = layout.areas(inner);

        // Update scroll offsets (approx inner height = area - 2 for borders)
        self.var_editor
            .list
            .update_offset(var_area.height.saturating_sub(2) as usize);
        self.cmd_editor
            .list
            .update_offset(cmd_area.height.saturating_sub(2) as usize);
        self.deferred_editor
            .list
            .update_offset(def_area.height.saturating_sub(2) as usize);

        // When an Option is focused, split into Properties (left) + Picker (right)
        let show_picker = matches!(
            self.focus,
            DetailFocus::Group | DetailFocus::Shell | DetailFocus::ExecMode
        );
        if show_picker {
            let split = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]);
            let [props_area, picker_area] = split.areas(meta_area);
            self.render_metadata(frame, props_area, theme);
            self.render_picker(frame, picker_area, theme);
        } else {
            self.render_metadata(frame, meta_area, theme);
        }

        self.render_variables(frame, var_area, theme);
        self.render_commands(frame, cmd_area, theme);
        self.render_deferred_commands(frame, def_area, theme);
        self.render_status_bar(frame, status_area, theme);
    }

    fn cycle_group(&mut self, delta: isize) {
        let current = self
            .groups
            .iter()
            .position(|g| g.id == self.set.group_id)
            .unwrap_or(0) as isize;
        let len = self.groups.len() as isize;
        if len == 0 {
            return;
        }
        let candidate = current + delta;
        if candidate < 0 || candidate >= len {
            return;
        }
        self.set.group_id = self.groups[candidate as usize].id;
    }

    fn cycle_shell(&mut self, delta: isize) {
        let saved_custom = match &self.set.shell {
            ShellType::Custom(p) => Some(p.clone()),
            _ => None,
        };
        let variants = ShellType::builtin_variants();
        let current: isize = match &self.set.shell {
            ShellType::Custom(_) => 5,
            other => variants
                .iter()
                .position(|s| std::mem::discriminant(s) == std::mem::discriminant(other))
                .unwrap_or(0) as isize,
        };
        let candidate = current + delta;
        if !(0..6).contains(&candidate) {
            return;
        }
        let next = candidate as usize;
        self.set.shell = if next == 5 {
            ShellType::Custom(saved_custom.unwrap_or_else(|| "/usr/bin/sh".to_string()))
        } else {
            variants[next].clone()
        };
    }

    fn cycle_exec_mode(&mut self, delta: isize) {
        let variants = &[ExecMode::StopOnError, ExecMode::ContinueOnError];
        let pos = variants
            .iter()
            .position(|v| *v == self.set.exec_mode)
            .unwrap_or(0) as isize;
        let candidate = pos + delta;
        if candidate < 0 || candidate >= variants.len() as isize {
            return;
        }
        self.set.exec_mode = variants[candidate as usize];
    }
}
