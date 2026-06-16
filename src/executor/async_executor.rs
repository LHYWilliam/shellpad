use crate::executor::events::ExecutionEvent;
use crate::models::{Command, CommandSet, ExecMode, ShellCommand, Variable};
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command as StdCommand, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;

/// Substitute `{{var}}` placeholders in `template` with values from the command set.
pub fn substitute_variables(template: &str, set: &CommandSet) -> String {
    let mut result = template.to_string();
    for var in &set.variables {
        let pattern = format!("{{{{{}}}}}", var.name);
        result = result.replace(&pattern, &var.default_value);
    }
    result
}

/// Spawn a shell command and return the child process.
fn spawn_shell_command(shell_cmd: &ShellCommand, command: &str) -> std::io::Result<Child> {
    let mut cmd = StdCommand::new(&shell_cmd.program);
    cmd.arg(&shell_cmd.flag).arg(command);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
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

/// Inline variable substitution using a slice of Variables (for use inside thread closures).
fn substitute_variables_inner(template: &str, variables: &[crate::models::Variable]) -> String {
    let mut result = template.to_string();
    for var in variables {
        let pattern = format!("{{{{{}}}}}", var.name);
        result = result.replace(&pattern, &var.default_value);
    }
    result
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

            let resolved = substitute_variables_inner(&cmd.command, &variables);

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

            let mut child = match spawn_shell_command(&shell_cmd, &resolved) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ExecutionEvent::StderrLine {
                        index: actual_index,
                        line: format!("Failed to spawn command: {}", e),
                    });
                    let _ = tx.send(ExecutionEvent::Finished {
                        index: actual_index,
                        success: false,
                        duration_ms: cmd_start.elapsed().as_millis(),
                    });
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
                    Ok(None) => thread::sleep(std::time::Duration::from_millis(50)),
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

        let _ = tx.send(ExecutionEvent::CompletedAll {
            total,
            succeeded,
            failed,
            total_duration_ms: start.elapsed().as_millis(),
        });
    })
}
