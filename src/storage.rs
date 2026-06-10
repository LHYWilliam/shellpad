use crate::config::{data_file_path, temp_file_path};
use crate::models::AppData;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Load application data from the default config file.
pub fn load_app_data() -> AppData {
    let path = data_file_path();
    load_app_data_from(&path)
}

/// Save application data atomically to the default config file.
pub fn save_app_data(data: &AppData) -> io::Result<()> {
    let path = data_file_path();
    let tmp = temp_file_path();
    save_app_data_to(data, &path, &tmp)
}

// ---- Internal path-accepting functions (used by tests too) ----

fn load_app_data_from(path: &Path) -> AppData {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        return AppData::empty();
    }

    match fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(data) => data,
            Err(e) => {
                eprintln!(
                    "Corrupted data file at `{}`, backing up to `.bak`: {}",
                    path.display(),
                    e
                );
                let bak = path.with_extension("json.bak");
                let _ = fs::rename(path, &bak);
                AppData::empty()
            }
        },
        Err(e) => {
            eprintln!("Failed to read `{}`: {}", path.display(), e);
            AppData::empty()
        }
    }
}

fn save_app_data_to(data: &AppData, path: &Path, tmp: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(data)
        .map_err(io::Error::other)?;

    // Write to temp file and fsync (ensure data reaches disk before rename)
    let mut file = fs::File::create(tmp)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;
    drop(file);

    // Atomic rename. Fall back to copy+remove on cross-filesystem.
    if let Err(e) = fs::rename(tmp, path) {
        if e.kind() == io::ErrorKind::CrossesDevices {
            fs::copy(tmp, path)?;
            let _ = fs::remove_file(tmp);
        } else {
            return Err(e);
        }
    }

    // Sync parent directory metadata so the rename is durable
    if let Some(parent) = path.parent() {
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CommandSet, Group};
    use std::fs;
    use uuid::Uuid;

    /// Helper: create a temporary directory and run the test closure.
    /// Returns the directory path so tests can use it.
    fn with_temp_dir(f: impl FnOnce(&Path)) {
        let tmp_dir = std::env::temp_dir().join(format!("launcher_test_{}", Uuid::new_v4()));
        let _ = fs::create_dir_all(&tmp_dir);
        f(&tmp_dir);
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_load_empty_on_first_run() {
        with_temp_dir(|dir| {
            let path = dir.join("sets.json");
            let data = load_app_data_from(&path);
            assert!(data.groups.is_empty());
        });
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        with_temp_dir(|dir| {
            let path = dir.join("sets.json");
            let tmp = dir.join("sets.json.tmp");

            let mut group = Group::new("Test".to_string());
            let set = CommandSet::new("Test Set".to_string(), group.id);
            group.sets.push(set);
            let data = AppData {
                groups: vec![group],
            };

            save_app_data_to(&data, &path, &tmp).expect("save failed");
            let loaded = load_app_data_from(&path);
            assert_eq!(data, loaded);
        });
    }

    #[test]
    fn test_atomic_write_does_not_leave_tmp() {
        with_temp_dir(|dir| {
            let path = dir.join("sets.json");
            let tmp = dir.join("sets.json.tmp");

            let data = AppData::empty();
            save_app_data_to(&data, &path, &tmp).expect("save failed");

            let content = fs::read_to_string(&path).expect("read");
            assert!(content.contains("\"groups\""));
            assert!(!tmp.exists());
        });
    }

    #[test]
    fn test_corrupted_file_backs_up() {
        with_temp_dir(|dir| {
            let path = dir.join("sets.json");
            fs::write(&path, "this is not json").unwrap();

            let data = load_app_data_from(&path);
            assert!(data.groups.is_empty());

            let bak = path.with_extension("json.bak");
            assert!(bak.exists());
        });
    }

    #[test]
    fn test_multiple_saves() {
        with_temp_dir(|dir| {
            let path = dir.join("sets.json");
            let tmp = dir.join("sets.json.tmp");

            let data1 = AppData::empty();
            save_app_data_to(&data1, &path, &tmp).unwrap();

            let mut group = Group::new("G".to_string());
            group.sets.push(CommandSet::new("S".to_string(), group.id));
            let data2 = AppData { groups: vec![group] };
            save_app_data_to(&data2, &path, &tmp).unwrap();

            let loaded = load_app_data_from(&path);
            assert_eq!(loaded.groups.len(), 1);
            assert_eq!(loaded.groups[0].name, "G");
        });
    }
}
