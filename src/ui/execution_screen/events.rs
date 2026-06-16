use super::{CmdStatus, ExecutionScreenState};
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
                        }
                    }
                }
                ExecutionEvent::StdoutLine { index, line } => {
                    if index < self.cmd_states.len() {
                        self.cmd_states[index].output_lines.push(line);
                    }
                }
                ExecutionEvent::StderrLine { index, line } => {
                    if index < self.cmd_states.len() {
                        self.cmd_states[index]
                            .output_lines
                            .push(format!("[stderr] {}", line));
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
                ExecutionEvent::Interrupted { last_index: _ } => {
                    self.completed = true;
                }
            }
        }
    }
}
