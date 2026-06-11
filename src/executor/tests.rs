use super::async_executor::{execute_set, substitute_variables};
use super::blocking::{execute_set_blocking, ExecuteError, ExecuteResult, substitute_variables_from_map};
use super::events::ExecutionEvent;
use crate::models::{Command, CommandSet, ExecMode, Variable};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc};
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

    let handle = execute_set(set.commands.clone(), set.exec_mode, set.variables.clone(), "sh", tx.clone(), Arc::new(AtomicBool::new(false)), 0);
    handle.join().unwrap();
    drop(tx);

    let events: Vec<ExecutionEvent> = rx.iter().collect();

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
        command: "false".to_string(),
    });
    set.commands.push(Command {
        position: 1,
        command: "echo still_running".to_string(),
    });
    set.exec_mode = ExecMode::ContinueOnError;

    let handle = execute_set(set.commands.clone(), set.exec_mode, set.variables.clone(), "sh", tx.clone(), Arc::new(AtomicBool::new(false)), 0);
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

    let handle = execute_set(set.commands.clone(), set.exec_mode, set.variables.clone(), "sh", tx.clone(), Arc::new(AtomicBool::new(false)), 0);
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

    assert_eq!(finished.len(), 1);
    assert_eq!(*finished[0], 0);
}

#[test]
fn test_substitute_variables_from_map() {
    let mut vars = HashMap::new();
    vars.insert("user".to_string(), "admin".to_string());
    assert_eq!(
        substitute_variables_from_map("echo {{user}}", &vars),
        "echo admin"
    );
}

#[test]
fn test_substitute_variables_from_map_empty() {
    let vars = HashMap::new();
    assert_eq!(
        substitute_variables_from_map("echo hello", &vars),
        "echo hello"
    );
}

#[test]
fn test_execute_result_new() {
    let r = ExecuteResult { total: 3, succeeded: 2, failed: 1 };
    assert_eq!(r.total, 3);
    assert_eq!(r.succeeded, 2);
    assert_eq!(r.failed, 1);
}

#[test]
fn test_execute_error_display_spawn_failed() {
    let err = ExecuteError::SpawnFailed(0, "not found".into());
    let msg = err.to_string();
    assert!(msg.contains("Command 1"));
    assert!(msg.contains("failed to spawn"));
}

#[test]
fn test_execute_error_display_command_failed() {
    let err = ExecuteError::CommandFailed(1, Some(127));
    let msg = err.to_string();
    assert!(msg.contains("Command 2"));
    assert!(msg.contains("127"));
}

#[test]
fn test_execute_set_blocking_echo() {
    let mut set = CommandSet::new("test".to_string(), Uuid::new_v4());
    set.commands.push(Command {
        position: 0,
        command: "echo hello".to_string(),
    });
    set.exec_mode = ExecMode::StopOnError;
    let vars = HashMap::new();
    let result = execute_set_blocking(&set, "sh", &vars);
    assert!(result.is_ok());
    let r = result.unwrap();
    assert_eq!(r.succeeded, 1);
}

#[test]
fn test_execute_set_blocking_false_fails() {
    let mut set = CommandSet::new("test".to_string(), Uuid::new_v4());
    set.commands.push(Command {
        position: 0,
        command: "false".to_string(),
    });
    set.exec_mode = ExecMode::StopOnError;
    let vars = HashMap::new();
    let result = execute_set_blocking(&set, "sh", &vars);
    assert!(result.is_err());
}

#[test]
fn test_execute_set_blocking_continue_on_error() {
    let mut set = CommandSet::new("test".to_string(), Uuid::new_v4());
    set.commands.push(Command { position: 0, command: "false".to_string() });
    set.commands.push(Command { position: 1, command: "echo ok".to_string() });
    set.exec_mode = ExecMode::ContinueOnError;
    let vars = HashMap::new();
    let result = execute_set_blocking(&set, "sh", &vars);
    assert!(result.is_ok());
    let r = result.unwrap();
    assert_eq!(r.succeeded, 1);
    assert_eq!(r.failed, 1);
}

#[test]
fn test_execute_set_blocking_stop_on_error() {
    let mut set = CommandSet::new("test".to_string(), Uuid::new_v4());
    set.commands.push(Command { position: 0, command: "false".to_string() });
    set.commands.push(Command { position: 1, command: "echo no".to_string() });
    set.exec_mode = ExecMode::StopOnError;
    let vars = HashMap::new();
    let result = execute_set_blocking(&set, "sh", &vars);
    assert!(result.is_err());
}
