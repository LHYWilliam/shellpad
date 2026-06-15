use crate::config::{MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH};
use crate::executor::{execute_set, ExecutionEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::mode::AppMode;
use crate::models::{AppData, CommandSet};
use crate::storage;
use crate::ui::detail_screen::{DetailFocus, DetailScreenAction, DetailScreenState};
use crate::ui::notification::{Toast, ToastSeverity};
use crate::ui::theme::Theme;
use crate::ui::variable_screen::{VariableScreenAction, VariableScreenState};
use crate::ui::execution_screen::{ExecutionScreenAction, ExecutionScreenState};
use crate::ui::help_screen::draw_help;
use crate::ui::main_screen::{MainScreenAction, MainScreenState, Panel};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub struct App {
    data: AppData,
    mode: AppMode,
    running: bool,

    main_screen: MainScreenState,
    detail_screen: Option<DetailScreenState>,
    exec_screen: Option<ExecutionScreenState>,

    execution_rx: Option<mpsc::Receiver<ExecutionEvent>>,
    execution_handle: Option<thread::JoinHandle<()>>,

    // Kill signal for the execution thread (set true to abort running commands)
    kill_signal: Arc<AtomicBool>,

    // Variable input overlay (shown before execution — extracted into VariableScreenState)
    variable_screen: VariableScreenState,
    pending_set: Option<(usize, usize)>, // (group_index, set_index)

    // -- UI theme
    theme: Theme,

    // -- Toast notifications
    toasts: Vec<Toast>,
}

impl App {
    pub fn new() -> Self {
        let data = storage::load_app_data().unwrap_or_else(|e| {
            eprintln!("{}", e);
            AppData::empty()
        });
        Self {
            main_screen: MainScreenState::new(),
            detail_screen: None,
            exec_screen: None,
            data,
            mode: AppMode::Main,
            running: true,
            execution_rx: None,
            execution_handle: None,
            kill_signal: Arc::new(AtomicBool::new(false)),
            variable_screen: VariableScreenState::new(),
            pending_set: None,
            theme: Theme::default_dark(),
            toasts: Vec::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut crate::tui::TuiTerminal) -> io::Result<()> {
        let tick_rate = Duration::from_millis(100);

        while self.running {
            self.clean_toasts();
            terminal.draw(|f| self.render(f))?;

            let timeout = tick_rate;
            if event::poll(timeout)?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                self.handle_key(key);
            }

            // Collect execution events on each tick
            if self.mode == AppMode::Execution
                && let Some(ref rx) = self.execution_rx
                && let Some(ref mut es) = self.exec_screen
            {
                es.process_events(rx);
            }
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
            let warning = Paragraph::new(Line::from(format!(
                "Terminal too small: {}x{} (min: {}x{})",
                area.width, area.height, MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT
            )))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Red));
            frame.render_widget(warning, area);
            return;
        }

        // Split off title bar
        let layout = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]);
        let [title_area, content_area] = layout.areas(area);

        // Render title bar
        let mode_str = match self.mode {
            AppMode::Main => "Main",
            AppMode::Detail => "Edit",
            AppMode::Execution => "Run",
            AppMode::Help => "Help",
        };
        let group_count = self.data.groups.len();
        let set_count: usize = self.data.groups.iter().map(|g| g.sets.len()).sum();
        let title_text = format!(
            " Launcher  |  {}  |  {} groups, {} sets  |  ? Help  q Quit",
            mode_str, group_count, set_count,
        );
        let title_paragraph = Paragraph::new(Line::from(Span::styled(
            title_text,
            Style::default()
                .fg(self.theme.text_secondary)
                .add_modifier(Modifier::DIM),
        )));
        frame.render_widget(title_paragraph, title_area);

        match self.mode {
            AppMode::Main => {
                self.main_screen.render(frame, content_area, &self.data, &self.theme);
            }
            AppMode::Detail => {
                if let Some(ref mut ds) = self.detail_screen {
                    ds.render(frame, content_area, &self.theme);
                }
            }
            AppMode::Execution => {
                if let Some(ref es) = self.exec_screen {
                    es.render(frame, content_area, &self.theme);
                }
            }
            AppMode::Help => {
                self.main_screen.render(frame, content_area, &self.data, &self.theme);
                draw_help(frame, content_area, &self.theme);
            }
        }

        self.variable_screen.render(frame, content_area, &self.theme);

        // Render toast notification (centered on title bar)
        if let Some(toast) = self.toasts.last() {
            let (toast_fg, toast_label) = match toast.severity {
                ToastSeverity::Success => (self.theme.accent_success, " ✓ "),
                ToastSeverity::Error => (self.theme.accent_error, " ✗ "),
                ToastSeverity::Info => (self.theme.accent_info, " ● "),
            };
            let toast_msg = format!("{}{}", toast_label, toast.message);
            let toast_display_width = unicode_width::UnicodeWidthStr::width(toast_msg.as_str());
            let toast_width = (toast_display_width as u16 + 2).min(area.width.saturating_sub(4));
            let x = (area.width.saturating_sub(toast_width)) / 2;
            let toast_area = Rect::new(x, title_area.y, toast_width, 1);
            frame.render_widget(Clear, toast_area);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    toast_msg,
                    Style::default().fg(toast_fg).add_modifier(Modifier::BOLD),
                ))),
                toast_area,
            );
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        if self.variable_screen.active {
            let action = self.variable_screen.handle_key(key);
            self.handle_variable_action(action);
            return;
        }
        match self.mode {
            AppMode::Main => {
                let action = self.main_screen.handle_key(key, &self.data);
                self.on_main_action(action);
            }
            AppMode::Detail => {
                if let Some(ref mut ds) = self.detail_screen {
                    let action = ds.handle_key(key);
                    self.on_detail_action(action);
                }
            }
            AppMode::Execution => {
                if let Some(ref mut es) = self.exec_screen {
                    let action = es.handle_key(key);
                    self.on_exec_action(action);
                }
            }
            AppMode::Help => self.mode = AppMode::Main,
        }
    }

    // ---- Variable input ----

    fn handle_variable_action(&mut self, action: VariableScreenAction) {
        match action {
            VariableScreenAction::Execute { gi, si } => {
                // Copy variable values from inputs back to the set
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    let set = &mut self.data.groups[gi].sets[si];
                    for (i, input) in self.variable_screen.inputs.iter().enumerate() {
                        if i < set.variables.len() {
                            set.variables[i].default_value = input.content.clone();
                        }
                    }
                }
                self.variable_screen = VariableScreenState::new();
                self.auto_save();
                self.pending_set = Some((gi, si));
                self.do_execute();
            }
            VariableScreenAction::Cancel => {
                self.variable_screen = VariableScreenState::new();
                self.pending_set = None;
            }
            VariableScreenAction::None => {}
        }
    }

    // ---- Main screen actions ----

    fn on_main_action(&mut self, action: MainScreenAction) {
        match action {
            MainScreenAction::None => {}
            MainScreenAction::Quit => self.running = false,
            MainScreenAction::Help => self.mode = AppMode::Help,
            MainScreenAction::ExecuteSet(gi, si) => {
                let set = &self.data.groups[gi].sets[si];
                if !set.variables.is_empty() {
                    self.variable_screen.activate(set, gi, si);
                } else {
                    self.pending_set = Some((gi, si));
                    self.do_execute();
                }
            }
            MainScreenAction::EditSet(gi, si) => {
                let set = self.data.groups[gi].sets[si].clone();
                let groups = self.data.groups.clone();
                self.detail_screen = Some(DetailScreenState::new(set, groups));
                self.mode = AppMode::Detail;
            }
            MainScreenAction::NewSet(gi) => {
                if gi < self.data.groups.len() {
                    let gid = self.data.groups[gi].id;
                    let set = CommandSet::new("New Command Set".to_string(), gid);
                    let si = (self.main_screen.set_list.selected + 1).min(self.data.groups[gi].sets.len());
                    self.data.groups[gi].sets.insert(si, set.clone());
                    self.auto_save();
                    self.push_toast("Set created", ToastSeverity::Info);
                    let groups = self.data.groups.clone();
                    self.detail_screen = Some(DetailScreenState::new(set, groups));
                    self.mode = AppMode::Detail;
                }
            }
            MainScreenAction::DeleteSet(gi, si) => {
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    self.data.groups[gi].sets.remove(si);
                    self.main_screen.set_list.clamp_selected(self.data.groups[gi].sets.len());
                    if self.data.groups[gi].sets.is_empty() {
                        self.main_screen.active_panel = Panel::Groups;
                    }
                    self.auto_save();
                    self.push_toast("Set deleted", ToastSeverity::Info);
                }
            }
            MainScreenAction::NewGroup => {
                let gi = (self.main_screen.group_list.selected + 1).min(self.data.groups.len());
                let n = self.data.groups.len() + 1;
                self.data
                    .groups
                    .insert(gi, crate::models::Group::new(format!("Group {}", n)));
                self.main_screen.group_list.selected = gi;
                self.main_screen.set_list.reset();
                self.auto_save();
                self.push_toast("Group created", ToastSeverity::Info);
            }
            MainScreenAction::RenameGroup(gi, new_name) => {
                if gi < self.data.groups.len() {
                    self.data.groups[gi].name = new_name;
                    self.auto_save();
                    self.push_toast("Group renamed", ToastSeverity::Info);
                }
            }
            MainScreenAction::DeleteGroup(gi) => {
                if gi < self.data.groups.len() {
                    self.data.groups.remove(gi);
                    self.main_screen.group_list.clamp_selected(self.data.groups.len());
                    self.main_screen.set_list.reset();
                    if self.data.groups.is_empty() {
                        self.main_screen.group_list.reset();
                        self.main_screen.active_panel = Panel::Groups;
                    }
                    self.auto_save();
                    self.push_toast("Group deleted", ToastSeverity::Info);
                }
            }
        }
    }

    // ---- Detail screen actions ----

    fn on_detail_action(&mut self, action: DetailScreenAction) {
        match action {
            DetailScreenAction::None => {}
            DetailScreenAction::Save(set) => {
                let sid = set.id;
                // Find and update the set in data
                for group in &mut self.data.groups {
                    if let Some(existing) = group.sets.iter_mut().find(|s| s.id == sid) {
                        *existing = set;
                        existing.updated_at = chrono::Utc::now();
                        break;
                    }
                }
                self.detail_screen = None;
                self.mode = AppMode::Main;
                self.auto_save();
                self.push_toast("Command set saved", ToastSeverity::Success);
            }
            DetailScreenAction::Cancel => {
                self.detail_screen = None;
                self.mode = AppMode::Main;
            }
            DetailScreenAction::DeleteVariable(idx) => {
                if let Some(ref mut ds) = self.detail_screen
                    && idx < ds.set.variables.len()
                {
                    ds.set.variables.remove(idx);
                    ds.variable_list.clamp_selected(ds.set.variables.len());
                    if ds.set.variables.is_empty() {
                        ds.focus = DetailFocus::Name;
                    }
                    self.push_toast("Variable deleted", ToastSeverity::Info);
                }
            }
            DetailScreenAction::DeleteCommand(idx) => {
                if let Some(ref mut ds) = self.detail_screen
                    && idx < ds.set.commands.len()
                {
                    ds.set.commands.remove(idx);
                    for (i, c) in ds.set.commands.iter_mut().enumerate() {
                        c.position = i;
                    }
                    ds.command_list.clamp_selected(ds.set.commands.len());
                    if ds.set.commands.is_empty() {
                        ds.focus = DetailFocus::Name;
                    }
                    self.push_toast("Command deleted", ToastSeverity::Info);
                }
            }
        }
    }

    /// Tear down the execution thread.
    /// `keep_screen=true` preserves the exec screen (for Skip/Interrupt),
    /// `mark_skipped=true` marks remaining commands as skipped.
    fn teardown_execution(&mut self, keep_screen: bool, mark_skipped: bool) {
        kill_execution(&mut self.kill_signal, &mut self.execution_rx, &mut self.execution_handle);
        if mark_skipped {
            if let Some(ref mut es) = self.exec_screen {
                es.mark_remaining_as_skipped();
            }
        }
        if !keep_screen {
            self.exec_screen = None;
        }
    }

    // ---- Execution screen actions ----

    fn on_exec_action(&mut self, action: ExecutionScreenAction) {
        match action {
            ExecutionScreenAction::BackToMain => {
                if let Some(ref es) = self.exec_screen
                    && es.completed {
                    let summary = format!(
                        "Done: {}/{}",
                        es.succeeded + es.failed + es.skipped,
                        es.total,
                    );
                    let severity = if es.failed > 0 {
                        ToastSeverity::Error
                    } else if es.skipped > 0 {
                        ToastSeverity::Info
                    } else {
                        ToastSeverity::Success
                    };
                    self.push_toast(summary, severity);
                }
                self.teardown_execution(false, false);
                self.mode = AppMode::Main;
            }
            ExecutionScreenAction::Interrupt | ExecutionScreenAction::Skip => {
                self.teardown_execution(true, true);
                self.mode = AppMode::Execution;
            }
            ExecutionScreenAction::Continue => {
                let start_from = self.exec_screen.as_ref()
                    .and_then(|es| es.continue_from)
                    .unwrap_or(0);
                if let Some((gi, si)) = self.pending_set {
                    self.do_execute_with(gi, si, start_from);
                }
            }
            ExecutionScreenAction::Reexecute => {
                self.teardown_execution(false, false);
                if let Some((gi, si)) = self.pending_set {
                    self.do_execute_with(gi, si, 0);
                }
            }
            ExecutionScreenAction::None => {}
        }
    }

    // ---- Execution ----

    fn do_execute(&mut self) {
        if let Some((gi, si)) = self.pending_set.take() {
            self.do_execute_with(gi, si, 0);
        }
    }

    fn do_execute_with(&mut self, gi: usize, si: usize, start_from: usize) {
        if gi >= self.data.groups.len() || si >= self.data.groups[gi].sets.len() {
            return;
        }
        let set = &self.data.groups[gi].sets[si];
        let shell_cmd = set.shell.resolve_command();

        let (commands, index_offset) = if start_from == 0 {
            // Full execution: create fresh screen
            let cmds = set.commands.clone();
            self.exec_screen = Some(ExecutionScreenState::new(set.name.clone(), &cmds));
            self.pending_set = Some((gi, si));
            (cmds, 0usize)
        } else {
            // Continue: reuse existing screen from skip point
            let cmds = set.commands[start_from..].to_vec();
            if let Some(ref mut es) = self.exec_screen {
                es.reset_from(start_from);
            }
            (cmds, start_from)
        };

        let (tx, rx) = mpsc::channel();
        let handle = execute_set(
            commands,
            set.exec_mode,
            set.variables.clone(),
            shell_cmd,
            tx,
            Arc::clone(&self.kill_signal),
            index_offset,
        );

        self.execution_rx = Some(rx);
        self.execution_handle = Some(handle);
        self.mode = AppMode::Execution;
    }

    fn auto_save(&mut self) {
        if let Err(e) = storage::save_app_data(&self.data) {
            self.push_toast(format!("Save failed: {}", e), ToastSeverity::Error);
        }
    }

    fn push_toast(&mut self, message: impl Into<String>, severity: ToastSeverity) {
        self.toasts.push(Toast::new(message, severity));
    }

    fn clean_toasts(&mut self) {
        const TOAST_DURATION: std::time::Duration = std::time::Duration::from_secs(3);
        self.toasts.retain(|t| t.created_at.elapsed() < TOAST_DURATION);
    }
}

impl Drop for App {
    fn drop(&mut self) {
        kill_execution(&mut self.kill_signal, &mut self.execution_rx, &mut self.execution_handle);
        let _ = storage::save_app_data(&self.data);
    }
}

/// Free function that can be called from Drop without full &mut self access.
fn kill_execution(
    kill_signal: &mut Arc<AtomicBool>,
    rx: &mut Option<mpsc::Receiver<ExecutionEvent>>,
    handle: &mut Option<thread::JoinHandle<()>>,
) {
    kill_signal.store(true, Ordering::Relaxed);
    *rx = None;
    if let Some(h) = handle.take() {
        let _ = h.join();
    }
    kill_signal.store(false, Ordering::Relaxed);
}

