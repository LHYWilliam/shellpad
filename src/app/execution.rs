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
    ) {
        let (tx, rx) = mpsc::channel();
        let handle = execute_set(
            commands,
            exec_mode,
            variables,
            shell_cmd,
            tx,
            Arc::clone(&self.kill_signal),
            index_offset,
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
        self.kill_signal.store(false, Ordering::Relaxed);
    }
}
