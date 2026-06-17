use super::{CmdStatus, ExecutionScreenState, MAX_OUTPUT_LINES};
use crate::executor::ExecutionEvent;
use std::sync::mpsc;

impl ExecutionScreenState {
    /// Calculate the flat items Vec index for a given command index.
    pub(crate) fn items_offset_for_command(&self, cmd_idx: usize) -> usize {
        let mut offset = 0;
        for i in 0..cmd_idx.min(self.cmd_states.len()) {
            offset += 1; // command header line
            offset += self.cmd_states[i].output_lines.len(); // output lines
            offset += 1; // separator line
        }
        offset
    }

    /// Mark all remaining Pending commands as Skipped.
    /// Called after the execution thread is stopped (Skip or Interrupt).
    pub(crate) fn mark_remaining_as_skipped(&mut self) {
        self.completed = true;
        for (i, state) in self.cmd_states.iter_mut().enumerate() {
            if state.status == CmdStatus::Pending {
                state.status = CmdStatus::Skipped;
                self.skipped += 1;
                if self.continue_from.is_none() {
                    self.continue_from = Some(i);
                }
            }
        }
    }

    /// Reset the screen for continuing execution from a skip point.
    pub(crate) fn reset_from(&mut self, start_from: usize) {
        self.auto_scroll = true;
        self.scroll_offset = 0;
        for state in self.cmd_states[start_from..].iter_mut() {
            if state.status == CmdStatus::Skipped {
                state.status = CmdStatus::Pending;
            }
        }
        self.completed = false;
        self.continue_from = None;
    }

    /// Process events from the execution channel.
    pub(crate) fn process_events(&mut self, rx: &mpsc::Receiver<ExecutionEvent>) {
        while let Ok(event) = rx.try_recv() {
            match event {
                ExecutionEvent::Starting { index, command } => {
                    if index < self.cmd_states.len() {
                        self.cmd_states[index].status = CmdStatus::Running;
                        self.cmd_states[index].command = command;
                        self.current_index = index;
                        if self.auto_scroll {
                            self.scroll_offset = self.items_offset_for_command(index);
                            self.focus_index = None;
                        }
                    }
                }
                ExecutionEvent::StdoutLine { index, line } => {
                    if index < self.cmd_states.len() {
                        let state = &mut self.cmd_states[index];
                        state.output_lines.push_back(line);
                        if state.output_lines.len() > MAX_OUTPUT_LINES {
                            state.output_lines.pop_front();
                            if !state.truncated {
                                state.truncated = true;
                                self.output_truncated = true;
                            }
                        }
                    }
                }
                ExecutionEvent::StderrLine { index, line } => {
                    if index < self.cmd_states.len() {
                        let state = &mut self.cmd_states[index];
                        state
                            .output_lines
                            .push_back(format!("[stderr] {}", line));
                        if state.output_lines.len() > MAX_OUTPUT_LINES {
                            state.output_lines.pop_front();
                            if !state.truncated {
                                state.truncated = true;
                                self.output_truncated = true;
                            }
                        }
                    }
                }
                ExecutionEvent::Finished {
                    index,
                    success,
                    duration_ms,
                } => {
                    if index < self.cmd_states.len() {
                        self.cmd_states[index].status = if success {
                            self.succeeded += 1;
                            CmdStatus::Success
                        } else {
                            self.failed += 1;
                            CmdStatus::Failure
                        };
                        self.cmd_states[index].duration_ms = Some(duration_ms);
                    }
                }
                ExecutionEvent::CompletedAll {
                    total: _,
                    succeeded: _,
                    failed: _,
                    total_duration_ms,
                } => {
                    self.completed = true;
                    self.total_duration_ms = Some(total_duration_ms);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionEvent;
    use std::sync::mpsc;

    fn make_state(commands: &[&str]) -> ExecutionScreenState {
        let cmds: Vec<_> = commands
            .iter()
            .map(|c| crate::models::Command {
                position: 0,
                command: c.to_string(),
            })
            .collect();
        ExecutionScreenState::new("test".to_string(), &cmds)
    }

    #[test]
    fn test_process_starting() {
        let mut state = make_state(&["echo hello"]);
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(ExecutionEvent::Starting {
            index: 0,
            command: "echo hello".to_string(),
        });
        state.process_events(&rx);
        assert_eq!(state.cmd_states[0].status, CmdStatus::Running);
    }

    #[test]
    fn test_process_stdout_line() {
        let mut state = make_state(&["echo hi"]);
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(ExecutionEvent::StdoutLine {
            index: 0,
            line: "hi".to_string(),
        });
        state.process_events(&rx);
        let lines: Vec<&str> = state.cmd_states[0]
            .output_lines
            .iter()
            .map(|s| s.as_str())
            .collect();
        assert_eq!(lines, vec!["hi"]);
    }

    #[test]
    fn test_process_stderr_line() {
        let mut state = make_state(&["error"]);
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(ExecutionEvent::StderrLine {
            index: 0,
            line: "err".to_string(),
        });
        state.process_events(&rx);
        let lines: Vec<&str> = state.cmd_states[0]
            .output_lines
            .iter()
            .map(|s| s.as_str())
            .collect();
        assert_eq!(lines, vec!["[stderr] err"]);
    }

    #[test]
    fn test_process_finished_success() {
        let mut state = make_state(&["ok"]);
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(ExecutionEvent::Starting {
            index: 0,
            command: "ok".to_string(),
        });
        let _ = tx.send(ExecutionEvent::Finished {
            index: 0,
            success: true,
            duration_ms: 100,
        });
        state.process_events(&rx);
        assert_eq!(state.cmd_states[0].status, CmdStatus::Success);
        assert_eq!(state.succeeded, 1);
    }

    #[test]
    fn test_process_finished_failure() {
        let mut state = make_state(&["fail"]);
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(ExecutionEvent::Starting {
            index: 0,
            command: "fail".to_string(),
        });
        let _ = tx.send(ExecutionEvent::Finished {
            index: 0,
            success: false,
            duration_ms: 50,
        });
        state.process_events(&rx);
        assert_eq!(state.cmd_states[0].status, CmdStatus::Failure);
        assert_eq!(state.failed, 1);
    }

    #[test]
    fn test_process_completed_all() {
        let mut state = make_state(&["a", "b"]);
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(ExecutionEvent::CompletedAll {
            total: 2,
            succeeded: 1,
            failed: 1,
            total_duration_ms: 500,
        });
        state.process_events(&rx);
        assert!(state.completed);
        assert_eq!(state.total_duration_ms, Some(500));
    }

    #[test]
    fn test_mark_remaining_as_skipped() {
        let mut state = make_state(&["a", "b", "c"]);
        state.mark_remaining_as_skipped();
        assert!(state.completed);
        for (i, cmd) in state.cmd_states.iter().enumerate() {
            assert_eq!(cmd.status, CmdStatus::Skipped, "cmd {i} should be skipped");
        }
        assert_eq!(state.skipped, 3);
        assert_eq!(state.continue_from, Some(0));
    }
}
