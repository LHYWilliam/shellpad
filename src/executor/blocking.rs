use crate::models::{CommandSet, ExecMode};
use std::collections::HashMap;
use std::process::{Command, Stdio};

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
            .map_err(|e| ExecuteError::CommandFailed(idx, e.raw_os_error()))?;

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
