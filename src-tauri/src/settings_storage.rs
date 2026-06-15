use std::fs;
use std::path::Path;

use crate::settings::Settings;
use crate::storage::StorageError;

/// Reads settings from `file`. Returns [`Settings::default`] (the seeded
/// defaults) if `file` does not exist.
pub fn load_settings(file: &Path) -> Result<Settings, StorageError> {
    if !file.exists() {
        return Ok(Settings::default());
    }

    let content = fs::read_to_string(file)?;
    let settings: Settings = serde_json::from_str(&content)?;
    Ok(settings.normalize())
}

/// Writes `settings` to `file` as JSON, creating parent directories if
/// necessary and overwriting any existing file. Writes to a temporary file
/// in the same directory and renames it into place, so a crash mid-write
/// cannot leave `file` truncated or corrupted.
pub fn save_settings(file: &Path, settings: &Settings) -> Result<(), StorageError> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(settings)?;
    let tmp_file = file.with_extension("json.tmp");
    fs::write(&tmp_file, content)?;
    fs::rename(&tmp_file, file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_settings_returns_seeded_defaults_for_missing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("settings.json");

        let settings = load_settings(&file).unwrap();

        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn save_then_load_round_trips_settings() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let mut settings = Settings::default();
        settings.defaults.tags.push("home".to_string());

        save_settings(&file, &settings).unwrap();
        let loaded = load_settings(&file).unwrap();

        assert_eq!(loaded, settings);
    }

    #[test]
    fn save_settings_overwrites_existing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let mut first = Settings::default();
        first.defaults.due = Some("tomorrow".to_string());
        let mut second = Settings::default();
        second.defaults.due = Some("friday".to_string());

        save_settings(&file, &first).unwrap();
        save_settings(&file, &second).unwrap();
        let loaded = load_settings(&file).unwrap();

        assert_eq!(loaded, second);
    }

    #[test]
    fn save_settings_creates_parent_directory() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("nested").join("settings.json");
        let settings = Settings::default();

        save_settings(&file, &settings).unwrap();
        let loaded = load_settings(&file).unwrap();

        assert_eq!(loaded, settings);
    }

    #[test]
    fn load_settings_normalizes_legacy_priorities_missing_color() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let legacy_json = r#"{
            "priorities": [
                {"id": "high", "label": "High", "rank": 1},
                {"id": "medium", "label": "Medium", "rank": 2},
                {"id": "low", "label": "Low", "rank": 3}
            ],
            "statuses": [],
            "defaults": {}
        }"#;
        fs::write(&file, legacy_json).unwrap();

        let loaded = load_settings(&file).unwrap();

        assert_eq!(loaded.priorities, Settings::default().priorities);
    }

    #[test]
    fn load_settings_migrates_legacy_settings_missing_done_status() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let legacy_json = r#"{
            "priorities": [],
            "statuses": [
                {"id": "backlog", "label": "Backlog", "order": 1},
                {"id": "do", "label": "Do", "order": 2},
                {"id": "done", "label": "Done", "order": 3}
            ],
            "defaults": {}
        }"#;
        fs::write(&file, legacy_json).unwrap();

        let loaded = load_settings(&file).unwrap();

        assert_eq!(loaded.done_status, "done");
        assert_eq!(loaded.cancelled_status, None);
    }

    #[test]
    fn load_settings_returns_error_for_malformed_json() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("settings.json");
        fs::write(&file, "not valid json").unwrap();

        let result = load_settings(&file);

        assert!(matches!(result, Err(StorageError::Json(_))));
    }
}
