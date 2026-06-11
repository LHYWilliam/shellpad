use crate::models::{CommandSet, ExecMode};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

/// Events emitted by the executor during command set execution.
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    /// A command is about to start.
    Starting {
        index: usize,
        command: String,
    },
    /// A line of stdout from the currently running command.
    StdoutLine {
        index: usize,
        line: String,
    },
    /// A line of stderr from the currently running command.
    StderrLine {
        index: usize,
        line: String,
    },
    /// The current command has finished.
    Finished {
        index: usize,
        success: bool,
        duration_ms: u128,
    },
    /// All commands in the set have been executed.
    CompletedAll {
        total: usize,
        succeeded: usize,
        failed: usize,
        total_duration_ms: u128,
    },
    /// Execution was interrupted by user (results are partial).
    Interrupted {
        last_index: usize,
    },
}

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
fn spawn_shell_command(shell: &str, command: &str) -> std::io::Result<Child> {
    let mut cmd = Command::new(shell);
    cmd.arg("-c").arg(command);
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

/// Execute a command set on a background thread.
///
/// Events are sent through the `mpsc::Receiver` for the TUI to poll.
pub fn execute_set(
    set: &CommandSet,
    shell: &str,
    tx: mpsc::Sender<ExecutionEvent>,
    kill_signal: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    let commands = set.commands.clone();
    let exec_mode = set.exec_mode;
    let shell = shell.to_string();
    let variables = set.variables.clone();

    thread::spawn(move || {
        let start = std::time::Instant::now();
        let mut succeeded = 0usize;
        let mut failed = 0usize;
        let total = commands.len();

        for (index, cmd) in commands.iter().enumerate() {
            // Check kill signal before starting each command
            if kill_signal.load(Ordering::Relaxed) {
                return;
            }

            // Substitute variables
            let resolved = substitute_variables_inner(&cmd.command, &variables);

            // Signal starting
            if tx.send(ExecutionEvent::Starting {
                index,
                command: resolved.clone(),
            })
            .is_err()
            {
                return;
            }

            let cmd_start = std::time::Instant::now();

            // Spawn the process
            let mut child = match spawn_shell_command(&shell, &resolved) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ExecutionEvent::StderrLine {
                        index,
                        line: format!("Failed to spawn command: {}", e),
                    });
                    let _ = tx.send(ExecutionEvent::Finished {
                        index,
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

            // Pipe stdout and stderr on separate threads
            if let Some(stdout) = child.stdout.take() {
                let tx_out = tx.clone();
                thread::spawn(move || pipe_reader(stdout, index, tx_out, false));
            }
            if let Some(stderr) = child.stderr.take() {
                let tx_err = tx.clone();
                thread::spawn(move || pipe_reader(stderr, index, tx_err, true));
            }

            // Poll for completion, checking kill signal periodically
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
                    index,
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

/// Inline variable substitution without requiring a full CommandSet reference.
fn substitute_variables_inner(template: &str, variables: &[crate::models::Variable]) -> String {
    let mut result = template.to_string();
    for var in variables {
        let pattern = format!("{{{{{}}}}}", var.name);
        result = result.replace(&pattern, &var.default_value);
    }
    result
}

/// Substitute variables from a pre-resolved HashMap.
pub fn substitute_variables_from_map(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (name, value) in vars {
        let pattern = format!("{{{{{}}}}}", name);
        result = result.replace(&pattern, value);
    }
    result
}

/// Result of a blocking command set execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteResult {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
}

/// Error during blocking execution.
#[derive(Debug)]
pub enum ExecuteError {
    SpawnFailed(usize, String),
    CommandFailed(usize, Option<i32>),
}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecuteError::SpawnFailed(idx, msg) => {
                write!(f, "Command {} failed to spawn: {}", idx + 1, msg)
            }
            ExecuteError::CommandFailed(idx, code) => {
                write!(f, "Command {} failed with exit code {:?}", idx + 1, code)
            }
        }
    }
}

impl std::error::Error for ExecuteError {}

/// Execute a command set synchronously, piping output directly to stdout/stderr.
/// Returns a summary of results, or the first error encountered.
pub fn execute_set_blocking(
    set: &CommandSet,
    shell: &str,
    vars: &HashMap<String, String>,
) -> Result<ExecuteResult, ExecuteError> {
    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let total = set.commands.len();

    for (idx, cmd) in set.commands.iter().enumerate() {
        let resolved = substitute_variables_from_map(&cmd.command, vars);

        eprintln!("[{}/{}] $ {}", idx + 1, total, resolved);

        let mut child = Command::new(shell)
            .arg("-c")
            .arg(&resolved)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| ExecuteError::SpawnFailed(idx, e.to_string()))?;

        let status = child
            .wait()
            .map_err(|e| ExecuteError::CommandFailed(idx, Some(e.raw_os_error().unwrap_or(-1))))?;

        if status.success() {
            succeeded += 1;
        } else {
            failed += 1;
            eprintln!("Command {} exited with code {:?}", idx + 1, status.code());
            if matches!(set.exec_mode, ExecMode::StopOnError) {
                return Err(ExecuteError::CommandFailed(idx, status.code()));
            }
        }
    }

    Ok(ExecuteResult {
        total,
        succeeded,
        failed,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Command, Variable};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use uuid::Uuid;

    #[test]
    fn test_substitute_single_variable() {
        let mut set = CommandSet::new("test".to_string(), Uuid::new_v4());
        set.variables.push(Variable {
            name: "server".to_string(),
            default_value: "192.168.1.1".to_string(),
        });
        let result = substitute_variables("ssh {{server}}", &set);
        assert_eq!(result, "ssh 192.168.1.1");
    }

    #[test]
    fn test_substitute_multiple_variables() {
        let mut set = CommandSet::new("test".to_string(), Uuid::new_v4());
        set.variables.push(Variable {
            name: "user".to_string(),
            default_value: "admin".to_string(),
        });
        set.variables.push(Variable {
            name: "host".to_string(),
            default_value: "example.com".to_string(),
        });
        let result = substitute_variables("ssh {{user}}@{{host}}", &set);
        assert_eq!(result, "ssh admin@example.com");
    }

    #[test]
    fn test_substitute_no_variables() {
        let set = CommandSet::new("test".to_string(), Uuid::new_v4());
        let result = substitute_variables("echo hello", &set);
        assert_eq!(result, "echo hello");
    }

    #[test]
    fn test_substitute_missing_variable_leaves_placeholder() {
        let set = CommandSet::new("test".to_string(), Uuid::new_v4());
        let result = substitute_variables("ssh {{missing}}", &set);
        assert_eq!(result, "ssh {{missing}}");
    }

    #[test]
    fn test_substitute_empty_value() {
        let mut set = CommandSet::new("test".to_string(), Uuid::new_v4());
        set.variables.push(Variable {
            name: "x".to_string(),
            default_value: "".to_string(),
        });
        let result = substitute_variables("a{{x}}b", &set);
        assert_eq!(result, "ab");
    }

    #[test]
    fn test_substitute_multiple_occurrences() {
        let mut set = CommandSet::new("test".to_string(), Uuid::new_v4());
        set.variables.push(Variable {
            name: "tag".to_string(),
            default_value: "v1.0".to_string(),
        });
        let result = substitute_variables("git tag {{tag}} && git push origin {{tag}}", &set);
        assert_eq!(result, "git tag v1.0 && git push origin v1.0");
    }

    #[test]
    fn test_execute_echo() {
        let (tx, rx) = mpsc::channel();

        let mut set = CommandSet::new("echo test".to_string(), Uuid::new_v4());
        set.commands.push(Command {
            position: 0,
            command: "echo hello_world".to_string(),
        });
        set.exec_mode = ExecMode::StopOnError;

        let handle = execute_set(&set, "sh", tx.clone(), Arc::new(AtomicBool::new(false)));
        handle.join().unwrap();
        drop(tx);

        let events: Vec<ExecutionEvent> = rx.iter().collect();

        // Should have Starting, StdoutLine, Finished, CompletedAll
        assert!(events.iter().any(|e| matches!(e, ExecutionEvent::Starting { .. })));
        assert!(events.iter().any(|e| matches!(e, ExecutionEvent::StdoutLine { line, .. } if line == "hello_world")));
        assert!(events.iter().any(|e| matches!(e, ExecutionEvent::Finished { .. })));
        assert!(events.iter().any(|e| matches!(e, ExecutionEvent::CompletedAll { .. })));
    }

    #[test]
    fn test_execute_failure_continue_on_error() {
        let (tx, rx) = mpsc::channel();

        let mut set = CommandSet::new("test".to_string(), Uuid::new_v4());
        set.commands.push(Command {
            position: 0,
            command: "false".to_string(), // fails
        });
        set.commands.push(Command {
            position: 1,
            command: "echo still_running".to_string(),
        });
        set.exec_mode = ExecMode::ContinueOnError;

        let handle = execute_set(&set, "sh", tx.clone(), Arc::new(AtomicBool::new(false)));
        handle.join().unwrap();
        drop(tx);

        let events: Vec<ExecutionEvent> = rx.iter().collect();

        let completed = events
            .iter()
            .find_map(|e| {
                if let ExecutionEvent::CompletedAll { succeeded, failed, .. } = e {
                    Some((*succeeded, *failed))
                } else {
                    None
                }
            })
            .expect("CompletedAll event");

        assert_eq!(completed, (1, 1));
    }

    #[test]
    fn test_execute_failure_stop_on_error() {
        let (tx, rx) = mpsc::channel();

        let mut set = CommandSet::new("test".to_string(), Uuid::new_v4());
        set.commands.push(Command {
            position: 0,
            command: "false".to_string(),
        });
        set.commands.push(Command {
            position: 1,
            command: "echo should_not_run".to_string(),
        });
        set.exec_mode = ExecMode::StopOnError;

        let handle = execute_set(&set, "sh", tx.clone(), Arc::new(AtomicBool::new(false)));
        handle.join().unwrap();
        drop(tx);

        let events: Vec<ExecutionEvent> = rx.iter().collect();
        let finished: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let ExecutionEvent::Finished { index, .. } = e {
                    Some(index)
                } else {
                    None
                }
            })
            .collect();

        // Only the first command should have run
        assert_eq!(finished.len(), 1);
        assert_eq!(*finished[0], 0);
    }
}
