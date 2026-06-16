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
    KillExec,
    SkipCurrent,
    ContinueFrom(usize),
    ReExec,
    #[allow(dead_code)]
    ToggleAutoScroll,
    BackToMain,

    // === Variable overlay ===
    ConfirmVariables, // handler reads from variable_screen.inputs
    CancelVariables,
}
