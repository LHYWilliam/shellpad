use crate::executor::{ExecutionEvent, execute_set};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;

/// Manages the lifecycle of a background execution thread.
pub struct ExecutionManager {
    pub rx: Option<mpsc::Receiver<ExecutionEvent>>,
    pub handle: Option<thread::JoinHandle<()>>,
    pub kill_signal: Arc<AtomicBool>,
}

impl ExecutionManager {
    pub fn new() -> Self {
        Self {
            rx: None,
            handle: None,
            kill_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start execution of a command set.
    pub fn start(
        &mut self,
        commands: Vec<crate::models::Command>,
        exec_mode: crate::models::ExecMode,
        variables: Vec<crate::models::Variable>,
        shell_cmd: crate::models::ShellCommand,
        index_offset: usize,
        working_dir: Option<String>,
    ) {
        self.kill_signal.store(false, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        let handle = execute_set(
            commands,
            exec_mode,
            variables,
            shell_cmd,
            tx,
            Arc::clone(&self.kill_signal),
            index_offset,
            working_dir,
        );
        self.rx = Some(rx);
        self.handle = Some(handle);
    }

    /// Kill the running execution thread.
    pub fn kill(&mut self) {
        self.kill_signal.store(true, Ordering::Relaxed);
        self.rx = None;
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Command, ExecMode, ShellCommand};

    fn test_shell_cmd() -> ShellCommand {
        ShellCommand {
            program: "echo".to_string(),
            flag: "-n".to_string(),
        }
    }

    #[test]
    fn test_execution_manager_start_sets_channel_and_handle() {
        let mut mgr = ExecutionManager::new();
        mgr.start(
            vec![Command {
                position: 0,
                command: "ok".to_string(),
            }],
            ExecMode::StopOnError,
            vec![],
            test_shell_cmd(),
            0,
            None,
        );

        assert!(mgr.rx.is_some());
        assert!(mgr.handle.is_some());
        assert!(!mgr.kill_signal.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn test_execution_manager_kill_flips_signal_and_nulls_rx() {
        let mut mgr = ExecutionManager::new();
        mgr.start(
            vec![Command {
                position: 0,
                command: "echo ok".to_string(),
            }],
            ExecMode::StopOnError,
            vec![],
            test_shell_cmd(),
            0,
            None,
        );

        mgr.kill();

        assert!(mgr.kill_signal.load(std::sync::atomic::Ordering::Relaxed));
        assert!(mgr.rx.is_none());
    }

    #[test]
    fn test_execution_manager_kill_twice_is_safe() {
        let mut mgr = ExecutionManager::new();
        mgr.start(
            vec![Command {
                position: 0,
                command: "echo ok".to_string(),
            }],
            ExecMode::StopOnError,
            vec![],
            test_shell_cmd(),
            0,
            None,
        );

        mgr.kill();
        mgr.kill(); // should not panic

        assert!(mgr.rx.is_none());
    }
}
