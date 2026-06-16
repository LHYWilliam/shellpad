# Batch 2: 事件处理 + 内联编辑单元测试

> **For agentic workers:** 在 `execution_screen/events.rs` 和 `detail_editor.rs` 末尾追加 `#[cfg(test)] mod tests` 块。不修改生产代码。

**Goal:** 给执行事件处理和内联编辑逻辑添加约 12 个单元测试。

---

### Task 1: `execution_screen/events.rs` 测试（~8 个）

**文件:** `src/ui/execution_screen/events.rs`

需要 import 的 types: 在测试模块中 `use super::*` 获取 `ExecutionScreenState`、`CmdStatus`；`use crate::executor::ExecutionEvent`；`use std::sync::mpsc`。

```rust
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
        tx.send(ExecutionEvent::Starting {
            index: 0,
            command: "echo hello".to_string(),
        })
        .unwrap();
        state.process_events(&rx);
        assert_eq!(state.cmd_states[0].status, CmdStatus::Running);
    }

    #[test]
    fn test_process_stdout_line() {
        let mut state = make_state(&["echo hi"]);
        let (tx, rx) = mpsc::channel();
        tx.send(ExecutionEvent::StdoutLine {
            index: 0,
            line: "hi".to_string(),
        })
        .unwrap();
        state.process_events(&rx);
        assert_eq!(state.cmd_states[0].output_lines, vec!["hi"]);
    }

    #[test]
    fn test_process_stderr_line() {
        let mut state = make_state(&["error"]);
        let (tx, rx) = mpsc::channel();
        tx.send(ExecutionEvent::StderrLine {
            index: 0,
            line: "err".to_string(),
        })
        .unwrap();
        state.process_events(&rx);
        assert_eq!(state.cmd_states[0].output_lines, vec!["[stderr] err"]);
    }

    #[test]
    fn test_process_finished_success() {
        let mut state = make_state(&["ok"]);
        let (tx, rx) = mpsc::channel();
        tx.send(ExecutionEvent::Starting { index: 0, command: "ok".to_string() }).unwrap();
        tx.send(ExecutionEvent::Finished { index: 0, success: true, duration_ms: 100 }).unwrap();
        state.process_events(&rx);
        assert_eq!(state.cmd_states[0].status, CmdStatus::Success);
        assert_eq!(state.succeeded, 1);
    }

    #[test]
    fn test_process_finished_failure() {
        let mut state = make_state(&["fail"]);
        let (tx, rx) = mpsc::channel();
        tx.send(ExecutionEvent::Starting { index: 0, command: "fail".to_string() }).unwrap();
        tx.send(ExecutionEvent::Finished { index: 0, success: false, duration_ms: 50 }).unwrap();
        state.process_events(&rx);
        assert_eq!(state.cmd_states[0].status, CmdStatus::Failure);
        assert_eq!(state.failed, 1);
    }

    #[test]
    fn test_process_completed_all() {
        let mut state = make_state(&["a", "b"]);
        let (tx, rx) = mpsc::channel();
        tx.send(ExecutionEvent::CompletedAll {
            total: 2, succeeded: 1, failed: 1, total_duration_ms: 500,
        }).unwrap();
        state.process_events(&rx);
        assert!(state.completed);
        assert_eq!(state.total_duration_ms, Some(500));
    }

    #[test]
    fn test_process_interrupted() {
        let mut state = make_state(&["a"]);
        let (tx, rx) = mpsc::channel();
        tx.send(ExecutionEvent::Interrupted { last_index: 0 }).unwrap();
        state.process_events(&rx);
        assert!(state.completed);
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
```

### Task 2: `detail_editor.rs` 测试（~4 个）

**文件:** `src/ui/detail_editor.rs`

需要 import 的 types: `use super::*`；`use crate::ui::widget::{InlineEdit, ScrollableList};`；使用 crossterm KeyEvent。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::AppAction;
    use crate::models::Variable;
    use crate::ui::widget::{InlineEdit, ScrollableList};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn test_handle_variable_edit_enter_commits() {
        let mut edit = InlineEdit::new();
        edit.editing = Some(0);
        edit.edit_input = crate::ui::widget::TextInput::new("x=y".to_string());
        let mut vars = vec![
            Variable { name: "old".into(), default_value: "old".into() },
        ];
        let mut list = ScrollableList::new();
        let action = handle_variable_edit(&mut edit, make_key(KeyCode::Enter), 0, &mut vars, &mut list);
        assert!(matches!(action, AppAction::None));
        assert_eq!(vars[0].name, "x");
        assert_eq!(vars[0].default_value, "y");
        assert!(edit.editing.is_none());
    }

    #[test]
    fn test_handle_variable_edit_esc_cancels() {
        let mut edit = InlineEdit::new();
        edit.editing = Some(0);
        edit.edit_input = crate::ui::widget::TextInput::new("a=b".to_string());
        let mut vars = vec![
            Variable { name: "orig".into(), default_value: "orig".into() },
        ];
        let mut list = ScrollableList::new();
        let action = handle_variable_edit(&mut edit, make_key(KeyCode::Esc), 0, &mut vars, &mut list);
        assert!(matches!(action, AppAction::None));
        assert_eq!(vars[0].name, "orig"); // unchanged
        assert!(edit.editing.is_none());
    }

    #[test]
    fn test_handle_variable_edit_text_input() {
        let mut edit = InlineEdit::new();
        edit.editing = Some(0);
        edit.edit_input = crate::ui::widget::TextInput::new(String::new());
        let mut vars = vec![
            Variable { name: "a".into(), default_value: "b".into() },
        ];
        let mut list = ScrollableList::new();
        let action = handle_variable_edit(&mut edit, make_key(KeyCode::Char('x')), 0, &mut vars, &mut list);
        assert!(matches!(action, AppAction::None));
        assert_eq!(edit.edit_input.content, "x");
    }

    #[test]
    fn test_handle_command_edit_enter_commits() {
        let mut edit = InlineEdit::new();
        edit.editing = Some(0);
        edit.edit_input = crate::ui::widget::TextInput::new("echo new".to_string());
        let mut cmds = vec![
            crate::models::Command { position: 0, command: "echo old".to_string() },
        ];
        let mut list = ScrollableList::new();
        let action = handle_command_edit(&mut edit, make_key(KeyCode::Enter), 0, &mut cmds, &mut list);
        assert!(matches!(action, AppAction::None));
        assert_eq!(cmds[0].command, "echo new");
    }
}
```

### Verification

```bash
cargo test      # expect 103+ passed
cargo clippy    # no new warnings
cargo fmt
git add src/ui/execution_screen/events.rs src/ui/detail_editor.rs
git commit -m "test(batch2): add events and detail_editor unit tests"
```
