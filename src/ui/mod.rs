//! Terminal UI components and screen implementations.
//!
//! - [`render`] — Pure rendering helpers (blocks, scrollbars, status bars)
//! - [`widget`] — Reusable widgets (TextInput, ScrollableList, InlineEdit)
//! - `*_screen` — Full-screen state machines with render + key handling
//! - [`theme`] — Color palettes and style helpers
//! - [`notification`] — Toast notification types

pub mod detail_editor;
pub mod detail_screen;
pub mod execution_screen;
pub mod help_screen;
pub mod main_screen;
pub mod notification;
pub mod render;
pub mod theme;
pub mod variable_screen;
pub mod widget;
