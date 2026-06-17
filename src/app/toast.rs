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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::toast::ToastSeverity;

    #[test]
    fn test_toast_add_and_retrieve() {
        let mut mgr = ToastManager::new();
        assert!(mgr.toasts.is_empty());

        mgr.add("Hello", ToastSeverity::Info);
        assert_eq!(mgr.toasts.len(), 1);
        assert_eq!(mgr.toasts[0].message, "Hello");
    }

    #[test]
    fn test_toast_clean_expired_preserves_fresh() {
        let mut mgr = ToastManager::new();
        mgr.add("Fresh", ToastSeverity::Info);
        mgr.clean_expired();
        // Fresh toasts are not removed
        assert_eq!(mgr.toasts.len(), 1);
    }

    #[test]
    fn test_toast_severity_mapping() {
        let mut mgr = ToastManager::new();
        mgr.add("Info", ToastSeverity::Info);
        mgr.add("Success", ToastSeverity::Success);
        mgr.add("Error", ToastSeverity::Error);
        assert_eq!(mgr.toasts[0].severity, ToastSeverity::Info);
        assert_eq!(mgr.toasts[1].severity, ToastSeverity::Success);
        assert_eq!(mgr.toasts[2].severity, ToastSeverity::Error);
    }
}
