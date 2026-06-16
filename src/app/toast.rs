use crate::ui::toast::{Toast, ToastSeverity};
use std::time::Duration;

const TOAST_DURATION: Duration = Duration::from_secs(3);

/// Manages toast notifications. Data-only — rendering happens in `app/render.rs`.
pub struct ToastManager {
    pub toasts: Vec<Toast>,
}

impl ToastManager {
    pub fn new() -> Self {
        Self { toasts: Vec::new() }
    }

    pub fn add(&mut self, message: impl Into<String>, severity: ToastSeverity) {
        self.toasts.push(Toast::new(message, severity));
    }

    /// Remove expired toasts.
    pub fn clean_expired(&mut self) {
        self.toasts
            .retain(|t| t.created_at.elapsed() < TOAST_DURATION);
    }
}
