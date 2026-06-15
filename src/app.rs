use crate::config::{MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH};
use crate::executor::{execute_set, ExecutionEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::mode::AppMode;
use crate::models::{AppData, CommandSet};
use crate::storage;
use crate::ui::detail_screen::{DetailScreenAction, DetailScreenState};
use crate::ui::theme::Theme;
use crate::ui::variable_screen::{VariableScreenAction, VariableScreenState};
use crate::ui::execution_screen::{ExecutionScreenAction, ExecutionScreenState};
use crate::ui::help_screen::draw_help;
use crate::ui::main_screen::{MainScreenAction, MainScreenState, Panel};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::layout::Alignment;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
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
        }
    }

    pub fn run(&mut self, terminal: &mut crate::tui::TuiTerminal) -> io::Result<()> {
        let tick_rate = Duration::from_millis(100);

        while self.running {
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

        match self.mode {
            AppMode::Main => {
                self.main_screen.render(frame, area, &self.data, &self.theme);
            }
            AppMode::Detail => {
                if let Some(ref mut ds) = self.detail_screen {
                    ds.render(frame, area, &self.theme);
                }
            }
            AppMode::Execution => {
                if let Some(ref es) = self.exec_screen {
                    es.render(frame, area, &self.theme);
                }
            }
            AppMode::Help => {
                self.main_screen.render(frame, area, &self.data, &self.theme);
                draw_help(frame, area, &self.theme);
            }
        }

        self.variable_screen.render(frame, area, &self.theme);
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
                    self.data.groups[gi].sets.push(set.clone());
                    self.auto_save();
                    let groups = self.data.groups.clone();
                    self.detail_screen = Some(DetailScreenState::new(set, groups));
                    self.mode = AppMode::Detail;
                }
            }
            MainScreenAction::DeleteSet(gi, si) => {
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    self.data.groups[gi].sets.remove(si);
                    self.main_screen.set_list.reset();
                    if self.data.groups[gi].sets.is_empty() {
                        self.main_screen.active_panel = Panel::Groups;
                    }
                    self.auto_save();
                }
            }
            MainScreenAction::NewGroup => {
                let n = self.data.groups.len() + 1;
                self.data
                    .groups
                    .push(crate::models::Group::new(format!("Group {}", n)));
                self.main_screen.group_list.selected =
                    self.data.groups.len().saturating_sub(1);
                self.main_screen.set_list.reset();
                self.auto_save();
            }
            MainScreenAction::RenameGroup(gi, new_name) => {
                if gi < self.data.groups.len() {
                    self.data.groups[gi].name = new_name;
                    self.auto_save();
                }
            }
            MainScreenAction::DeleteGroup(gi) => {
                if gi < self.data.groups.len() {
                    self.data.groups.remove(gi);
                    if self.main_screen.group_list.selected >= self.data.groups.len() {
                        self.main_screen.group_list.selected =
                            self.data.groups.len().saturating_sub(1);
                    }
                    self.main_screen.set_list.reset();
                    if self.data.groups.is_empty() {
                        self.main_screen.group_list.reset();
                        self.main_screen.active_panel = Panel::Groups;
                    }
                    self.auto_save();
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
                    let last = ds.set.variables.len().saturating_sub(1);
                    ds.variable_list.selected = ds.variable_list.selected.min(last);
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
                    let last = ds.set.commands.len().saturating_sub(1);
                    ds.command_list.selected = ds.command_list.selected.min(last);
                }
            }
        }
    }

    /// Signal the execution thread to abort, wait for it, but keep the exec screen alive.
    fn stop_execution(&mut self) {
        kill_execution(&mut self.kill_signal, &mut self.execution_rx, &mut self.execution_handle);
        // Mark remaining pending commands as skipped
        if let Some(ref mut es) = self.exec_screen {
            es.mark_remaining_as_skipped();
        }
    }

    /// Signal the execution thread to abort, destroying the exec screen.
    fn kill_execution(&mut self) {
        kill_execution(&mut self.kill_signal, &mut self.execution_rx, &mut self.execution_handle);
        self.exec_screen = None;
    }

    // ---- Execution screen actions ----

    fn on_exec_action(&mut self, action: ExecutionScreenAction) {
        match action {
            ExecutionScreenAction::BackToMain => {
                self.kill_execution();
                self.mode = AppMode::Main;
            }
            ExecutionScreenAction::Interrupt | ExecutionScreenAction::Skip => {
                self.stop_execution();
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
                self.kill_execution();
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

    fn auto_save(&self) {
        if let Err(e) = storage::save_app_data(&self.data) {
            eprintln!("Auto-save failed: {}", e);
        }
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

