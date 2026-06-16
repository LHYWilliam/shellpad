/// Application screen modes — only one screen is active at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    /// Main list screen: groups on the left, command sets on the right
    Main,
    /// Detail / edit screen: full-screen form for editing a command set
    Detail,
    /// Full-screen execution view: real-time command output
    Execution,
    /// Help overlay: keyboard shortcuts reference
    Help,
    /// Confirmation overlay for destructive actions (delete Set/Group/Variable/Command)
    ConfirmDelete {
        /// What is being deleted (for prompt text)
        kind: crate::action::DeleteKind,
        /// Mode to restore after confirm/cancel
        prev: Box<AppMode>,
    },
}
