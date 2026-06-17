/// Events emitted by the executor during command set execution.
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    /// A command is about to start.
    Starting { index: usize, command: String },
    /// A line of stdout from the currently running command.
    StdoutLine { index: usize, line: String },
    /// A line of stderr from the currently running command.
    StderrLine { index: usize, line: String },
    /// The current command has finished.
    Finished {
        index: usize,
        success: bool,
        duration_ms: u128,
        exit_code: Option<i32>,
    },
    /// All commands in the set have been executed.
    CompletedAll {
        total: usize,
        succeeded: usize,
        failed: usize,
        total_duration_ms: u128,
    },
}
