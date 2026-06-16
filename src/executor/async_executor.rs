use crate::executor::events::ExecutionEvent;
use crate::models::{Command, CommandSet, ExecMode, ShellCommand, Variable};
use std::io::{BufRead, BufReader, Read};
use std::time::Duration;
use std::process::{Child, Command as StdCommand, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;

/// Polling interval for child process status (milliseconds).
const POLL_MS: u64 = 50;

/// Substitute `{{var}}` placeholders in `template` with values from the command set.
pub fn substitute_variables(template: &str, set: &CommandSet) -> String {
    let vars = set.variables.iter().map(|v| (v.name.as_str(), v.default_value.as_str()));
    crate::executor::substitute_variables_core(template, vars)
}

/// Spawn a shell command and return the child process.
fn spawn_shell_command(
    shell_cmd: &ShellCommand,
    command: &str,
    working_dir: Option<&str>,
) -> std::io::Result<Child> {
    let mut cmd = StdCommand::new(&shell_cmd.program);
    cmd.arg(&shell_cmd.flag).arg(command);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    cmd.spawn()
}

/// Read all lines from a reader and send them through the channel.
fn pipe_reader<R: Read + Send + 'static>(
    reader: R,
    index: usize,
    tx: mpsc::Sender<ExecutionEvent>,
    is_stderr: bool,
) {
    let reader = BufReader::new(reader);
    for line in reader.lines().map_while(Result::ok) {
        let event = if is_stderr {
            ExecutionEvent::StderrLine { index, line }
        } else {
            ExecutionEvent::StdoutLine { index, line }
        };
        if tx.send(event).is_err() {
            break;
        }
    }
}

/// Execute commands on a background thread.
///
/// Events are sent through the `mpsc::Receiver` for the TUI to poll.
/// `index_offset` is added to event indices (used when continuing from a skip).
pub fn execute_set(
    commands: Vec<Command>,
    exec_mode: ExecMode,
    variables: Vec<Variable>,
    shell_cmd: ShellCommand,
    tx: mpsc::Sender<ExecutionEvent>,
    kill_signal: Arc<AtomicBool>,
    index_offset: usize,
    working_dir: Option<String>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let start = std::time::Instant::now();
        let mut succeeded = 0usize;
        let mut failed = 0usize;
        let total = commands.len();

        for (actual_index, cmd) in commands.iter().enumerate() {
            let actual_index = actual_index + index_offset;

            if kill_signal.load(Ordering::Relaxed) {
                return;
            }

            let resolved = {
                let vars = variables.iter().map(|v| (v.name.as_str(), v.default_value.as_str()));
                crate::executor::substitute_variables_core(&cmd.command, vars)
            };

            if tx
                .send(ExecutionEvent::Starting {
                    index: actual_index,
                    command: resolved.clone(),
                })
                .is_err()
            {
                return;
            }

            let cmd_start = std::time::Instant::now();

            let mut child = match spawn_shell_command(&shell_cmd, &resolved, working_dir.as_deref()) {
                Ok(c) => c,
                Err(e) => {
                    if tx.send(ExecutionEvent::StderrLine {
                        index: actual_index,
                        line: format!("Failed to spawn command: {}", e),
                    }).is_err() { return; }
                    if tx.send(ExecutionEvent::Finished {
                        index: actual_index,
                        success: false,
                        duration_ms: cmd_start.elapsed().as_millis(),
                    }).is_err() { return; }
                    failed += 1;
                    if matches!(exec_mode, ExecMode::StopOnError) {
                        break;
                    }
                    continue;
                }
            };

            if let Some(stdout) = child.stdout.take() {
                let tx_out = tx.clone();
                thread::spawn(move || pipe_reader(stdout, actual_index, tx_out, false));
            }
            if let Some(stderr) = child.stderr.take() {
                let tx_err = tx.clone();
                thread::spawn(move || pipe_reader(stderr, actual_index, tx_err, true));
            }

            let success = loop {
                if kill_signal.load(Ordering::Relaxed) {
                    let _ = child.kill();
                    child.wait().ok();
                    break false;
                }
                match child.try_wait() {
                    Ok(Some(status)) => break status.success(),
                    Ok(None) => thread::sleep(Duration::from_millis(POLL_MS)),
                    Err(_) => break false,
                }
            };

            let duration = cmd_start.elapsed().as_millis();

            if tx
                .send(ExecutionEvent::Finished {
                    index: actual_index,
                    success,
                    duration_ms: duration,
                })
                .is_err()
            {
                return;
            }

            if success {
                succeeded += 1;
            } else {
                failed += 1;
                if matches!(exec_mode, ExecMode::StopOnError) {
                    break;
                }
            }
        }

        if tx.send(ExecutionEvent::CompletedAll {
            total,
            succeeded,
            failed,
            total_duration_ms: start.elapsed().as_millis(),
        }).is_err() { return; }
    })
}
