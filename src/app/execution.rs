use crate::executor::{ExecutionEvent, execute_set};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;

/// Bundles the receiver and join handle for a running execution thread.
pub(crate) struct ExecutionThread {
    pub(crate) rx: mpsc::Receiver<ExecutionEvent>,
    pub(crate) handle: thread::JoinHandle<()>,
}

/// Manages the lifecycle of a background execution thread.
pub struct ExecutionManager {
    pub thread: Option<ExecutionThread>,
    pub kill_signal: Arc<AtomicBool>, // Ctrl+C — abort normals
    pub skip_signal: Arc<AtomicBool>, // s/n — skip/pause
}

impl ExecutionManager {
    pub fn new() -> Self {
        Self {
            thread: None,
            kill_signal: Arc::new(AtomicBool::new(false)),
            skip_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start execution of a command set.
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        &mut self,
        commands: Vec<crate::models::Command>,
        defer_commands: Vec<crate::models::Command>,
        exec_mode: crate::models::ExecMode,
        variables: Vec<crate::models::Variable>,
        shell_cmd: crate::models::ShellCommand,
        index_offset: usize,
        working_dir: Option<String>,
    ) {
        self.kill_signal.store(false, Ordering::Relaxed);
        self.skip_signal.store(false, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        let handle = execute_set(
            commands,
            defer_commands,
            exec_mode,
            variables,
            shell_cmd,
            tx,
            Arc::clone(&self.kill_signal),
            Arc::clone(&self.skip_signal),
            index_offset,
            working_dir,
        );
        self.thread = Some(ExecutionThread { rx, handle });
    }

    /// Skip the current command and pause (s key).
    pub fn skip_current(&self) {
        self.skip_signal.store(true, Ordering::Relaxed);
    }

    /// Resume from pause (n key).
    pub fn continue_next(&self) {
        self.skip_signal.store(false, Ordering::Relaxed);
    }

    /// Abort all remaining normal commands, then run defers (Ctrl+C).
    /// Only sets kill_signal — the executor's pause loop will detect it
    /// naturally (within 100ms). Setting skip_signal=false here would cause
    /// the pause loop to break and continue to the next command, producing a
    /// second Finished event that double-counts the skipped command.
    pub fn abort_all(&self) {
        self.kill_signal.store(true, Ordering::Relaxed);
    }

    /// Kill the running execution thread (drop channel, wait for exit).
    pub fn kill(&mut self) {
        self.kill_signal.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.handle.join();
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
            vec![],
            ExecMode::StopOnError,
            vec![],
            test_shell_cmd(),
            0,
            None,
        );

        assert!(mgr.thread.is_some());
        assert!(mgr.thread.is_some());
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
            vec![],
            ExecMode::StopOnError,
            vec![],
            test_shell_cmd(),
            0,
            None,
        );

        mgr.kill();

        assert!(mgr.kill_signal.load(std::sync::atomic::Ordering::Relaxed));
        assert!(mgr.thread.is_none());
    }

    #[test]
    fn test_execution_manager_kill_twice_is_safe() {
        let mut mgr = ExecutionManager::new();
        mgr.start(
            vec![Command {
                position: 0,
                command: "echo ok".to_string(),
            }],
            vec![],
            ExecMode::StopOnError,
            vec![],
            test_shell_cmd(),
            0,
            None,
        );

        mgr.kill();
        mgr.kill(); // should not panic

        assert!(mgr.thread.is_none());
    }
}
