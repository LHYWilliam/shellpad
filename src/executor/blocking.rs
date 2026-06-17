use crate::error::ExecuteError;
use crate::models::{CommandSet, ExecMode, ShellCommand};
use std::collections::HashMap;
use std::process::{Command, Stdio};

/// Substitute variables from a pre-resolved HashMap.
pub fn substitute_variables_from_map(template: &str, vars: &HashMap<String, String>) -> String {
    crate::executor::substitute_variables_core(
        template,
        vars.iter().map(|(k, v)| (k.as_str(), v.as_str())),
    )
}

/// Result of a blocking command set execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteResult {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
}

/// Execute a command set synchronously, piping output directly to stdout/stderr.
pub fn execute_set_blocking(
    set: &CommandSet,
    shell_cmd: &ShellCommand,
    vars: &HashMap<String, String>,
    working_dir: Option<&str>,
) -> Result<ExecuteResult, ExecuteError> {
    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let total = set.commands.len();

    for (idx, cmd) in set.commands.iter().enumerate() {
        let resolved = substitute_variables_from_map(&cmd.command, vars);

        eprintln!("[{}/{}] $ {}", idx + 1, total, resolved);

        let mut cmd_builder = Command::new(&shell_cmd.program);
        cmd_builder
            .arg(&shell_cmd.flag)
            .arg(&resolved)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        if let Some(dir) = working_dir {
            cmd_builder.current_dir(dir);
        }
        let mut child = cmd_builder.spawn().map_err(|e| ExecuteError::SpawnFailed {
            idx: idx + 1,
            detail: e.to_string(),
        })?;

        let status = child.wait().map_err(|e| ExecuteError::CommandFailed {
            idx: idx + 1,
            code: e.raw_os_error(),
        })?;

        if status.success() {
            succeeded += 1;
        } else {
            failed += 1;
            eprintln!("Command {} exited with code {:?}", idx + 1, status.code());
            if matches!(set.exec_mode, ExecMode::StopOnError) {
                return Err(ExecuteError::CommandFailed {
                    idx: idx + 1,
                    code: status.code(),
                });
            }
        }
    }

    Ok(ExecuteResult {
        total,
        succeeded,
        failed,
    })
}
