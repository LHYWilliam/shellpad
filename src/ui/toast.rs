use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToastSeverity {
    Success,
    Error,
    Info,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub severity: ToastSeverity,
    pub created_at: Instant,
}

impl Toast {
    pub fn new(message: impl Into<String>, severity: ToastSeverity) -> Self {
        Self {
            message: message.into(),
            severity,
            created_at: Instant::now(),
        }
    }
}
