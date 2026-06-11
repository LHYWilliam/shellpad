mod async_executor;
mod blocking;
pub mod events;

pub use async_executor::{execute_set, substitute_variables};
pub use blocking::{execute_set_blocking, ExecuteError, ExecuteResult, substitute_variables_from_map};
pub use events::ExecutionEvent;

#[cfg(test)]
mod tests;
