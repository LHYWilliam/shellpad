//! Command execution engine with two entry points.
//!
//! - [`execute_set`] runs commands asynchronously on a background thread,
//!   streaming output via `mpsc` channel (used by the TUI).
//! - [`execute_set_blocking`] runs commands synchronously with inherited
//!   stdio (used by CLI mode).

mod async_executor;
mod blocking;
pub mod events;

pub use async_executor::{execute_set, substitute_variables};
pub use blocking::{
    ExecuteResult, execute_set_blocking, substitute_variables_from_map,
};
pub use crate::error::ExecuteError;
pub use events::ExecutionEvent;

#[cfg(test)]
mod tests;
