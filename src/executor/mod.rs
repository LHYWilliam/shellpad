//! Command execution engine with two entry points.
//!
//! - [`execute_set`] runs commands asynchronously on a background thread,
//!   streaming output via `mpsc` channel (used by the TUI).
//! - [`execute_set_blocking`] runs commands synchronously with inherited
//!   stdio (used by CLI mode).

mod async_executor;
mod blocking;
pub mod events;

pub use crate::error::ExecuteError;
pub use async_executor::{execute_set, substitute_variables};
pub use blocking::{ExecuteResult, execute_set_blocking, substitute_variables_from_map};
pub use events::ExecutionEvent;

/// Core variable substitution: replace `{{name}}` placeholders with values.
///
/// `vars` accepts any iterator of `(key, value)` pairs where both are `AsRef<str>`.
/// Placeholders that don't have a corresponding variable are left as-is.
pub(crate) fn substitute_variables_core(
    template: &str,
    vars: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
) -> String {
    let mut result = template.to_string();
    for (name, value) in vars {
        let pattern = format!("{{{{{}}}}}", name.as_ref());
        result = result.replace(&pattern, value.as_ref());
    }
    result
}

#[cfg(test)]
mod tests;
