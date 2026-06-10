use crate::config::{MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH};
use crate::executor::{execute_set, ExecutionEvent};
use crate::mode::AppMode;
use crate::models::{AppData, CommandSet, ShellType};
use crate::storage;
use crate::ui::components::{handle_text_input, TextInput};
use crate::ui::detail_screen::{DetailScreenAction, DetailScreenState};
use crate::ui::execution_screen::{ExecutionScreenAction, ExecutionScreenState};
use crate::ui::help_screen::draw_help;
use crate::ui::main_screen::{MainScreenAction, MainScreenState, Panel};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
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

    // Variable input overlay (shown before execution)
    variable_input_mode: bool,
    variable_inputs: Vec<TextInput>,
    variable_names: Vec<String>,
    pending_set: Option<(usize, usize)>, // (group_index, set_index)
}

impl App {
    pub fn new() -> Self {
        let data = storage::load_app_data();
        Self {
            main_screen: MainScreenState::new(),
            detail_screen: None,
            exec_screen: None,
            data,
            mode: AppMode::Main,
            running: true,
            execution_rx: None,
            execution_handle: None,
            variable_input_mode: false,
            variable_inputs: Vec::new(),
            variable_names: Vec::new(),
            pending_set: None,
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
                self.main_screen.render(frame, area, &self.data);
            }
            AppMode::Detail => {
                if let Some(ref mut ds) = self.detail_screen {
                    ds.render(frame, area);
                }
            }
            AppMode::Execution => {
                if let Some(ref es) = self.exec_screen {
                    es.render(frame, area);
                }
            }
            AppMode::Help => {
                self.main_screen.render(frame, area, &self.data);
                draw_help(frame, area);
            }
        }

        if self.variable_input_mode {
            self.render_variable_input(frame, area);
        }
    }

    fn render_variable_input(&self, frame: &mut Frame, area: Rect) {
        let width = area.width.min(60).saturating_sub(4);
        let count = self.variable_inputs.len().max(1) as u16;
        let height = count + 6;
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let dialog = Rect::new(x, y, width, height);

        frame.render_widget(Clear, dialog);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Set Variables ")
            .style(Style::default().bg(Color::DarkGray));
        frame.render_widget(&block, dialog);

        let inner = block.inner(dialog);
        for (i, input) in self.variable_inputs.iter().enumerate() {
            let label = format!(" {} =", self.variable_names[i]);
            frame.render_widget(
                Paragraph::new(Line::from(label)).style(Style::default().fg(Color::White)),
                Rect::new(inner.x, inner.y + i as u16, inner.width / 2, 1),
            );
            let val_area = Rect::new(
                inner.x + inner.width / 2,
                inner.y + i as u16,
                inner.width.saturating_sub(inner.width / 2),
                1,
            );
            input.render(frame, val_area, true, "");
        }
        let hint = format!(
            " [Enter] Execute  [Esc] Cancel  [Tab] Next  [{}] Edit",
            if self.variable_inputs.is_empty() {
                "—"
            } else {
                "↑↓"
            }
        );
        frame.render_widget(
            Paragraph::new(Line::from(hint)).style(Style::default().fg(Color::DarkGray)),
            Rect::new(inner.x, inner.y + count + 1, inner.width, 1),
        );
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        if self.variable_input_mode {
            self.handle_variable_key(key);
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

    fn handle_variable_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                // Copy variable values from inputs back to the set
                if let Some((gi, si)) = self.pending_set
                    && gi < self.data.groups.len()
                    && si < self.data.groups[gi].sets.len()
                {
                    let set = &mut self.data.groups[gi].sets[si];
                    for (i, input) in self.variable_inputs.iter().enumerate() {
                        if i < set.variables.len() {
                            set.variables[i].default_value = input.content.clone();
                        }
                    }
                }
                self.variable_input_mode = false;
                self.do_execute();
            }
            KeyCode::Esc => {
                self.variable_input_mode = false;
                self.variable_inputs.clear();
                self.variable_names.clear();
                self.pending_set = None;
            }
            KeyCode::Tab => {
                // Cycle to next input
                if let Some(first) = self.variable_inputs.first().cloned() {
                    self.variable_inputs.push(first);
                    self.variable_inputs.remove(0);
                }
            }
            _ => {
                if let Some(input) = self.variable_inputs.last_mut() {
                    handle_text_input(input, key);
                }
            }
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
                    self.variable_input_mode = true;
                    self.variable_inputs = set
                        .variables
                        .iter()
                        .map(|v| TextInput::new(v.default_value.clone()))
                        .collect();
                    self.variable_names =
                        set.variables.iter().map(|v| v.name.clone()).collect();
                    self.pending_set = Some((gi, si));
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
                }
            }
        }
    }

    // ---- Execution screen actions ----

    fn on_exec_action(&mut self, action: ExecutionScreenAction) {
        match action {
            ExecutionScreenAction::BackToMain => {
                self.exec_screen = None;
                self.execution_rx = None;
                self.execution_handle = None;
                self.mode = AppMode::Main;
            }
            ExecutionScreenAction::Interrupt => {
                self.exec_screen = None;
                self.execution_rx = None;
                self.execution_handle = None;
                self.mode = AppMode::Main;
            }
            ExecutionScreenAction::Reexecute => {
                self.exec_screen = None;
                self.execution_rx = None;
                self.execution_handle = None;
                // Re-trigger execution without variable input
                if let Some((gi, si)) = self.pending_set {
                    self.do_execute_with(gi, si);
                }
            }
            ExecutionScreenAction::Skip => {}
            ExecutionScreenAction::None => {}
        }
    }

    // ---- Execution ----

    fn do_execute(&mut self) {
        if let Some((gi, si)) = self.pending_set.take() {
            self.do_execute_with(gi, si);
        }
    }

    fn do_execute_with(&mut self, gi: usize, si: usize) {
        if gi >= self.data.groups.len() || si >= self.data.groups[gi].sets.len() {
            return;
        }
        let set = &self.data.groups[gi].sets[si];
        let shell = resolve_shell(&set.shell);
        let commands = set.commands.clone();
        let set_name = set.name.clone();
        let set_clone = set.clone();

        let (tx, rx) = mpsc::channel();
        let handle = execute_set(&set_clone, &shell, tx);

        self.exec_screen = Some(ExecutionScreenState::new(set_name, &commands));
        self.execution_rx = Some(rx);
        self.execution_handle = Some(handle);
        self.pending_set = Some((gi, si));
        self.mode = AppMode::Execution;
    }

    fn auto_save(&self) {
        if let Err(e) = storage::save_app_data(&self.data) {
            eprintln!("Auto-save failed: {}", e);
        }
    }
}

fn resolve_shell(shell: &ShellType) -> String {
    match shell {
        ShellType::SystemDefault => std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string()),
        ShellType::Bash => "bash".to_string(),
        ShellType::Zsh => "zsh".to_string(),
        ShellType::Fish => "fish".to_string(),
        ShellType::Custom(path) => path.clone(),
    }
}
