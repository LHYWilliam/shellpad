use super::{CmdStatus, ExecutionScreenState, MAX_OUTPUT_LINES};
use crate::executor::ExecutionEvent;
use std::sync::mpsc;

impl ExecutionScreenState {
    /// Calculate the flat items Vec index for a given command index.
    pub(crate) fn items_offset_for_command(&self, cmd_idx: usize) -> usize {
        let mut offset = 0;
        let count = cmd_idx.min(self.cmd_states.len());
        for i in 0..count {
            offset += 1; // command header line
            if self.cmd_states[i].truncated {
                offset += 1; // truncation marker
            }
            offset += self.cmd_states[i].output_lines.len(); // output lines
            if i + 1 < self.cmd_states.len() {
                offset += 1; // separator (not after last command)
            }
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
                        state.output_lines.push_back(format!("[stderr] {}", line));
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

    #[test]
    fn test_output_truncation_drops_oldest_lines() {
        const LIMIT: usize = super::super::MAX_OUTPUT_LINES;
        let mut state = make_state(&["flood"]);
        let (tx, rx) = mpsc::channel();

        // Send exactly LIMIT lines
        for i in 0..LIMIT {
            let _ = tx.send(ExecutionEvent::StdoutLine {
                index: 0,
                line: format!("line_{}", i),
            });
        }
        state.process_events(&rx);
        assert_eq!(state.cmd_states[0].output_lines.len(), LIMIT);
        assert!(!state.cmd_states[0].truncated);
        assert!(!state.output_truncated);

        // Send 1 more — oldest line dropped
        let _ = tx.send(ExecutionEvent::StdoutLine {
            index: 0,
            line: "overflow".to_string(),
        });
        state.process_events(&rx);
        assert_eq!(state.cmd_states[0].output_lines.len(), LIMIT);
        // "line_0" was the oldest, should be gone
        let front = state.cmd_states[0].output_lines.front().unwrap();
        assert_eq!(front, "line_1");
        assert!(state.cmd_states[0].truncated);
        assert!(state.output_truncated);
    }

    #[test]
    fn test_output_truncation_output_truncated_flag_resets() {
        let mut state = make_state(&["flood"]);
        let (tx, rx) = mpsc::channel();

        // Trigger truncation once
        let _ = tx.send(ExecutionEvent::StdoutLine {
            index: 0,
            line: "first".to_string(),
        });
        state.process_events(&rx);
        // Clear the flag (simulates app.rs consuming it)
        state.output_truncated = false;

        // Send enough to cross limit — flag should be set again
        const LIMIT: usize = super::super::MAX_OUTPUT_LINES;
        for i in 0..LIMIT {
            let _ = tx.send(ExecutionEvent::StdoutLine {
                index: 0,
                line: format!("l{}", i),
            });
        }
        state.process_events(&rx);
        assert!(state.cmd_states[0].truncated);
        assert!(state.output_truncated);
    }

    #[test]
    fn test_items_offset_works_with_vecdeque_output_lines() {
        let mut state = make_state(&["a", "b"]);
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(ExecutionEvent::StdoutLine {
            index: 0,
            line: "hello".to_string(),
        });
        let _ = tx.send(ExecutionEvent::StdoutLine {
            index: 0,
            line: "world".to_string(),
        });
        let _ = tx.send(ExecutionEvent::StdoutLine {
            index: 1,
            line: "foo".to_string(),
        });
        state.process_events(&rx);

        // cmd 0: 1 header + 2 outputs + 1 separator = 4
        assert_eq!(state.items_offset_for_command(1), 4);
        assert_eq!(state.items_offset_for_command(0), 0);
    }

    #[test]
    fn test_items_offset_includes_truncation_marker() {
        let mut state = make_state(&["a", "b"]);
        state.cmd_states[0].truncated = true;
        assert_eq!(state.items_offset_for_command(1), 3);
    }

    #[test]
    fn test_items_offset_no_trailing_separator_for_last_command() {
        let state = make_state(&["a", "b"]);
        assert_eq!(state.items_offset_for_command(2), 3);
        assert_eq!(state.items_offset_for_command(0), 0);
        assert_eq!(state.items_offset_for_command(1), 2);
    }
}
