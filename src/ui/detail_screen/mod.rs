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
}

pub struct DetailScreenState {
    pub set: CommandSet,
    pub groups: Vec<Group>,
    pub name_input: TextInput,
    pub focus: DetailFocus,
    pub variable_list: ScrollableList,
    pub command_list: ScrollableList,
    pub editing_name: bool,
    pub workdir_editing: bool,
    pub workdir_input: TextInput,
    pub var_edit: InlineEdit,
    pub cmd_edit: InlineEdit,
}

impl DetailScreenState {
    pub fn new(set: CommandSet, groups: Vec<Group>) -> Self {
        let name = set.name.clone();
        Self {
            set,
            groups,
            name_input: TextInput::new(name),
            focus: DetailFocus::Name,
            variable_list: ScrollableList::new(),
            command_list: ScrollableList::new(),
            editing_name: false,
            workdir_editing: false,
            workdir_input: TextInput::new(String::new()),
            var_edit: InlineEdit::new(),
            cmd_edit: InlineEdit::new(),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent_info))
            .title(format!(
                " Edit: {} ",
                if self.editing_name {
                    &self.name_input.content
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
            Constraint::Length(2), // status bar (separator + content)
        ]);
        let [meta_area, var_area, cmd_area, status_area] = layout.areas(inner);

        // Update scroll offsets (approx inner height = area - 2 for borders)
        self.variable_list
            .update_offset(var_area.height.saturating_sub(2) as usize);
        self.command_list
            .update_offset(cmd_area.height.saturating_sub(2) as usize);

        // When an Option is focused, split into Properties (left) + Picker (right)
        let show_picker = matches!(
            self.focus,
            DetailFocus::Group | DetailFocus::Shell | DetailFocus::ExecMode
        );
        if show_picker {
            let split = Layout::horizontal([
                Constraint::Ratio(1, 2),
                Constraint::Ratio(1, 2),
            ]);
            let [props_area, picker_area] = split.areas(meta_area);
            self.render_metadata(frame, props_area, theme);
            self.render_picker(frame, picker_area, theme);
        } else {
            self.render_metadata(frame, meta_area, theme);
        }

        self.render_variables(frame, var_area, theme);
        self.render_commands(frame, cmd_area, theme);
        self.render_status_bar(frame, status_area, theme);
    }

    fn cycle_group(&mut self, delta: isize) {
        let current = self
            .groups
            .iter()
            .position(|g| g.id == self.set.group_id)
            .unwrap_or(0);
        let len = self.groups.len();
        if len == 0 {
            return;
        }
        let next = (current as isize + delta).rem_euclid(len as isize) as usize;
        self.set.group_id = self.groups[next].id;
    }

    fn cycle_shell(&mut self, delta: isize) {
        // Build 6-element cycle: SystemDefault, Bash, Zsh, Fish, PowerShell, Custom(prev path)
        let saved_custom = match &self.set.shell {
            ShellType::Custom(p) => Some(p.clone()),
            _ => None,
        };
        let variants = ShellType::builtin_variants();
        let current = match &self.set.shell {
            ShellType::Custom(_) => 5usize,
            other => variants
                .iter()
                .position(|s| std::mem::discriminant(s) == std::mem::discriminant(other))
                .unwrap_or(0),
        };
        let next = ((current as isize + delta).rem_euclid(6)) as usize;
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
            .unwrap_or(0);
        let next = (pos as isize + delta).rem_euclid(variants.len() as isize) as usize;
        self.set.exec_mode = variants[next];
    }
}
