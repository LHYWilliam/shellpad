//! Unified action enum for screen-to-app communication.
//!
//! All screens return [`AppAction`] variants from their `handle_key()` methods.
//! The `app::handler` module processes these centrally in `App::handle_action()`.

use crate::models::CommandSet;

/// What the user is about to delete — carries enough context
/// for the confirm dialog to render a descriptive prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteKind {
    Set {
        group_index: usize,
        set_index: usize,
        set_name: String,
    },
    Group {
        group_index: usize,
        group_name: String,
        set_count: usize,
    },
    Variable {
        var_index: usize,
        var_name: String,
    },
    Command {
        cmd_index: usize,
        cmd_preview: String,
    },
}

/// What the user wants to reorder — identifies the target item and its position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReorderKind {
    Group(usize),
    Set(usize, usize),
    Variable(usize),
    Command(usize),
}

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

    // === Confirmation ===
    RequestDelete(DeleteKind),

    // === Reordering ===
    Reorder(ReorderKind, isize), // direction: -1 up, +1 down
}
