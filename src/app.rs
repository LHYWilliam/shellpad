//! Application state machine and event loop.
//!
//! [`App`] holds all mutable state (`data`, `mode`, screen states) and runs the
//! main event loop. Sub-modules handle rendering (`app::render`), action
//! dispatch (`app::handler`), toast notifications (`app::toast`), and execution
//! lifecycle (`app::execution`).

use crate::app::execution::ExecutionManager;
use crate::app::toast::ToastManager;
use crate::mode::AppMode;
use crate::models::AppData;
use crate::storage;
use crate::tui::TuiTerminal;
use crate::ui::detail_screen::DetailScreenState;
use crate::ui::execution_screen::ExecutionScreenState;
use crate::ui::main_screen::MainScreenState;
use crate::ui::theme::Theme;
use crate::ui::variable_screen::VariableScreenState;
use crossterm::event::{self, Event, KeyEventKind};
use std::io;
use std::time::Duration;

/// Event loop tick interval (milliseconds).
const TICK_RATE_MS: u64 = 100;

pub(crate) mod execution;
pub(crate) mod handler;
pub(crate) mod render;
pub(crate) mod toast;

pub struct App {
    pub data: AppData,
    pub mode: AppMode,
    pub running: bool,

    pub main_screen: MainScreenState,
    pub detail_screen: Option<DetailScreenState>,
    pub exec_screen: Option<ExecutionScreenState>,

    pub execution: ExecutionManager,
    pub variable_screen: VariableScreenState,
    pub pending_set: Option<(usize, usize)>,

    pub theme: Theme,
    pub toasts: ToastManager,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let data = storage::load_app_data().unwrap_or_else(|e| {
            eprintln!("{e}");
            AppData::empty()
        });
        Self {
            main_screen: MainScreenState::new(),
            detail_screen: None,
            exec_screen: None,
            data,
            mode: AppMode::Main,
            running: true,
            execution: ExecutionManager::new(),
            variable_screen: VariableScreenState::new(),
            pending_set: None,
            theme: Theme::default_dark(),
            toasts: ToastManager::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut TuiTerminal) -> io::Result<()> {
        let tick_rate = Duration::from_millis(TICK_RATE_MS);

        while self.running {
            self.toasts.clean_expired();
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
                && let Some(ref rx) = self.execution.rx
                && let Some(ref mut es) = self.exec_screen
            {
                es.process_events(rx);
            }
        }
        Ok(())
    }

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
            let cmds = set.commands.clone();
            self.exec_screen = Some(ExecutionScreenState::new(set.name.clone(), &cmds));
            self.pending_set = Some((gi, si));
            (cmds, 0usize)
        } else {
            let cmds = set.commands[start_from..].to_vec();
            if let Some(ref mut es) = self.exec_screen {
                es.reset_from(start_from);
            }
            (cmds, start_from)
        };

        self.execution.start(
            commands,
            set.exec_mode,
            set.variables.clone(),
            shell_cmd,
            index_offset,
        );
        self.mode = AppMode::Execution;
    }

    fn teardown_execution(&mut self, keep_screen: bool, mark_skipped: bool) {
        self.execution.kill();
        if mark_skipped && let Some(ref mut es) = self.exec_screen {
            es.mark_remaining_as_skipped();
        }
        if !keep_screen {
            self.exec_screen = None;
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.execution.kill();
        let _ = storage::save_app_data(&self.data);
    }
}
