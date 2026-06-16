//! Shared test helpers for unit and integration tests.
//!
//! This module is `#[cfg(test)]` — only compiled during `cargo test`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Shorthand for creating a key event with no modifiers.
pub(crate) fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}
