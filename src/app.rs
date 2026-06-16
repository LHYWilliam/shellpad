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

/// Consolidated execution lifecycle — replaces separate `exec_screen`,
/// `execution`, and `pending_set` fields. Only one variant is active.
pub(crate) enum ExecutionState {
    /// No execution in progress.
    Idle {
        /// Pending set indices for execution. Set temporarily between
        /// `ConfirmVariables` / `ExecuteSet` and `do_execute()`.
        pending_set: Option<(usize, usize)>,
    },
    /// Background thread is running with an active screen.
    Running {
        screen: Box<ExecutionScreenState>,
        manager: ExecutionManager,
        /// (group_index, set_index) — saved for restart / continue.
        pending_set: (usize, usize),
    },
}

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

    pub(crate) execution_state: ExecutionState,
    pub prev_mode: Option<AppMode>,
    pub variable_screen: VariableScreenState,

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
            data,
            mode: AppMode::Main,
            running: true,
            execution_state: ExecutionState::Idle { pending_set: None },
            prev_mode: None,
            variable_screen: VariableScreenState::new(),
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

            // Drain execution events on each tick
            if let ExecutionState::Running {
                ref mut screen,
                ref manager, ..
            } = self.execution_state
                && let Some(ref rx) = manager.rx
            {
                screen.process_events(rx);
            }
        }
        Ok(())
    }

    fn do_execute(&mut self) {
        let pending = match &mut self.execution_state {
            ExecutionState::Idle { pending_set } => pending_set.take(),
            ExecutionState::Running { .. } => None,
        };
        if let Some((gi, si)) = pending {
            self.do_execute_with(gi, si, 0);
        }
    }

    fn do_execute_with(&mut self, gi: usize, si: usize, start_from: usize) {
        if gi >= self.data.groups.len() || si >= self.data.groups[gi].sets.len() {
            return;
        }
        let set = &self.data.groups[gi].sets[si];
        let shell_cmd = set.shell.resolve_command();

        if start_from == 0 {
            let cmds = set.commands.clone();
            let screen = ExecutionScreenState::new(set.name.clone(), &cmds);
            let mut manager = ExecutionManager::new();
            manager.start(
                cmds,
                set.exec_mode,
                set.variables.clone(),
                shell_cmd,
                0usize,
            );
            self.execution_state = ExecutionState::Running {
                screen: Box::new(screen),
                manager,
                pending_set: (gi, si),
            };
            self.mode = AppMode::Execution;
            return;
        }

        // Continuing from a skip point — screen + manager already exist
        let cmds = set.commands[start_from..].to_vec();
        if let ExecutionState::Running {
            ref mut screen,
            ref mut manager,
            ..
        } = self.execution_state
        {
            screen.reset_from(start_from);
            manager.start(
                cmds,
                set.exec_mode,
                set.variables.clone(),
                shell_cmd,
                start_from,
            );
        }
        self.mode = AppMode::Execution;
    }

    fn teardown_execution(&mut self, keep_screen: bool, mark_skipped: bool) {
        if let ExecutionState::Running {
            ref mut screen,
            ref mut manager,
            ..
        } = self.execution_state
        {
            manager.kill();
            if mark_skipped {
                screen.mark_remaining_as_skipped();
            }
        }
        if !keep_screen {
            self.execution_state = ExecutionState::Idle { pending_set: None };
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let ExecutionState::Running {
            ref mut manager, ..
        } = self.execution_state
        {
            manager.kill();
        }
        let _ = storage::save_app_data(&self.data);
    }
}
