use crate::executor::events::ExecutionEvent;
use crate::models::{Command, CommandSet, ExecMode, ShellCommand, Variable};
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command as StdCommand, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

/// Polling interval for child process status (milliseconds).
const POLL_MS: u64 = 50;

/// Substitute `{{var}}` placeholders in `template` with values from the command set.
pub fn substitute_variables(template: &str, set: &CommandSet) -> String {
    let vars = set
        .variables
        .iter()
        .map(|v| (v.name.as_str(), v.default_value.as_str()));
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
#[allow(clippy::too_many_arguments, clippy::needless_return)]
pub fn execute_set(
    commands: Vec<Command>,
    defer_commands: Vec<Command>,
    exec_mode: ExecMode,
    variables: Vec<Variable>,
    shell_cmd: ShellCommand,
    tx: mpsc::Sender<ExecutionEvent>,
    kill_signal: Arc<AtomicBool>,
    skip_signal: Arc<AtomicBool>,
    index_offset: usize,
    working_dir: Option<String>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let start = std::time::Instant::now();
        let mut succeeded = 0usize;
        let mut failed = 0usize;

        fn send_finished(
            tx: &mpsc::Sender<ExecutionEvent>,
            index: usize,
            success: bool,
            duration_ms: u128,
            exit_code: Option<i32>,
            skipped: bool,
        ) {
            let _ = tx.send(ExecutionEvent::Finished {
                index,
                success,
                duration_ms,
                exit_code,
                skipped,
            });
        }

        enum PhaseResult {
            Completed,
            Aborted,
        }

        let run_phase = |cmds: &[Command],
                         index_base: usize,
                         stop_on_error: bool,
                         check_signals: bool,
                         succeeded: &mut usize,
                         failed: &mut usize|
         -> PhaseResult {
            for (ci, cmd) in cmds.iter().enumerate() {
                let actual_index = ci + index_base;

                let resolved = {
                    let vars = variables
                        .iter()
                        .map(|v| (v.name.as_str(), v.default_value.as_str()));
                    crate::executor::substitute_variables_core(&cmd.command, vars)
                };

                if tx
                    .send(ExecutionEvent::Starting {
                        index: actual_index,
                        command: resolved.clone(),
                    })
                    .is_err()
                {
                    return PhaseResult::Completed;
                }

                let cmd_start = std::time::Instant::now();

                let mut child =
                    match spawn_shell_command(&shell_cmd, &resolved, working_dir.as_deref()) {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = tx.send(ExecutionEvent::StderrLine {
                                index: actual_index,
                                line: format!("Failed to spawn command: {}", e),
                            });
                            send_finished(
                                &tx, actual_index, false,
                                cmd_start.elapsed().as_millis(), None, false,
                            );
                            *failed += 1;
                            if stop_on_error {
                                break;
                            } else {
                                continue;
                            }
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

                let (success, exit_code, skipped) = if check_signals {
                    let (succ, code, act) = loop {
                        if kill_signal.load(Ordering::Relaxed) {
                            let _ = child.kill();
                            child.wait().ok();
                            break (false, None, "abort");
                        }
                        if skip_signal.load(Ordering::Relaxed) {
                            let _ = child.kill();
                            child.wait().ok();
                            break (false, None, "skip");
                        }
                        match child.try_wait() {
                            Ok(Some(s)) => break (s.success(), s.code(), "done"),
                            Ok(None) => thread::sleep(Duration::from_millis(POLL_MS)),
                            Err(_) => break (false, None, "error"),
                        }
                    };

                    if act == "abort" {
                        send_finished(
                            &tx, actual_index, false,
                            cmd_start.elapsed().as_millis(), None, true,
                        );
                        return PhaseResult::Aborted;
                    }
                    if act == "skip" {
                        send_finished(
                            &tx, actual_index, false,
                            cmd_start.elapsed().as_millis(), None, true,
                        );
                        loop {
                            thread::sleep(Duration::from_millis(100));
                            if !skip_signal.load(Ordering::Relaxed) {
                                break;
                            }
                            if kill_signal.load(Ordering::Relaxed) {
                                return PhaseResult::Aborted;
                            }
                        }
                        continue;
                    }
                    (succ, code, false)
                } else {
                    let (succ, code) = loop {
                        match child.try_wait() {
                            Ok(Some(s)) => break (s.success(), s.code()),
                            Ok(None) => thread::sleep(Duration::from_millis(POLL_MS)),
                            Err(_) => break (false, None),
                        }
                    };
                    (succ, code, false)
                };

                if tx
                    .send(ExecutionEvent::Finished {
                        index: actual_index,
                        success,
                        duration_ms: cmd_start.elapsed().as_millis(),
                        exit_code,
                        skipped,
                    })
                    .is_err()
                {
                    return PhaseResult::Completed;
                }

                if success {
                    *succeeded += 1;
                } else {
                    *failed += 1;
                    if stop_on_error {
                        break;
                    }
                }
            }
            PhaseResult::Completed
        };

        // Phase 1: normal commands (signal-aware)
        run_phase(
            &commands,
            index_offset,
            matches!(exec_mode, ExecMode::StopOnError),
            true, // check_signals
            &mut succeeded,
            &mut failed,
        );

        // Phase 2: defer commands (signal-proof)
        if !defer_commands.is_empty() {
            kill_signal.store(false, Ordering::Relaxed);
            skip_signal.store(false, Ordering::Relaxed);
            run_phase(
                &defer_commands,
                commands.len() + index_offset,
                false, // never stop on error
                false, // check_signals = false
                &mut succeeded,
                &mut failed,
            );
        }

        let total = commands.len() + defer_commands.len();
        if tx
            .send(ExecutionEvent::CompletedAll {
                total,
                succeeded,
                failed,
                total_duration_ms: start.elapsed().as_millis(),
            })
            .is_err()
        {
            return;
        }
    })
}
