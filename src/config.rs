use directories::ProjectDirs;
use std::path::PathBuf;

/// Minimum terminal dimensions required by the TUI
pub const MIN_TERMINAL_WIDTH: u16 = 80;
pub const MIN_TERMINAL_HEIGHT: u16 = 24;

/// Returns the project data directory (~/.config/launcher/ on Linux, %APPDATA%/launcher on Windows)
pub fn data_dir() -> PathBuf {
    ProjectDirs::from("com", "launcher", "launcher")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| {
            #[cfg(windows)]
            {
                let appdata = std::env::var("APPDATA")
                    .unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Roaming".to_string());
                PathBuf::from(appdata).join("launcher")
            }
            #[cfg(not(windows))]
            {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(home).join(".config/launcher")
            }
        })
}

/// Path to the main data JSON file
pub fn data_file_path() -> PathBuf {
    data_dir().join("sets.json")
}

/// Path used for atomic temporary writes (write to .tmp, then rename)
pub fn temp_file_path() -> PathBuf {
    data_dir().join("sets.json.tmp")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_dir_is_not_empty() {
        let path = data_dir();
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn test_data_file_path_ends_with_sets_json() {
        let path = data_file_path();
        assert!(path.to_string_lossy().ends_with("sets.json"));
    }

    #[test]
    fn test_temp_file_path_ends_with_tmp() {
        let path = temp_file_path();
        assert!(path.to_string_lossy().ends_with(".tmp"));
    }
}
