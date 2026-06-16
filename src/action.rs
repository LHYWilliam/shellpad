//! Unified action enum for screen-to-app communication.
//!
//! All screens return [`AppAction`] variants from their `handle_key()` methods.
//! The `app::handler` module processes these centrally in `App::handle_action()`.

use crate::models::CommandSet;

/// Unified action enum returned by all screens.
/// The `app/handler.rs` handles all variants centrally.
pub enum AppAction {
    None,
    Quit,
    Help,

    // === Main screen ===
    ExecuteSet(usize, usize), // (group_index, set_index)
    EditSet(usize, usize),    // (group_index, set_index) — handler resolves data
    NewSet(usize),            // group_index
    DeleteSet(usize, usize),  // (group_index, set_index)
    NewGroup,
    RenameGroup(usize, String), // (group_index, new_name)
    DeleteGroup(usize),

    // === Detail screen ===
    SaveSet(CommandSet),
    CancelEdit,
    DeleteVariable(usize),
    DeleteCommand(usize),

    // === Execution screen ===
    SkipCurrent,
    ContinueFrom(usize),
    ReExec,
    BackToMain,

    // === Variable overlay ===
    ConfirmVariables, // handler reads from variable_screen.inputs
    CancelVariables,
}
