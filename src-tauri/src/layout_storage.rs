use std::fs;
use std::path::Path;

use crate::layout::StatLayout;
use crate::storage::StorageError;

/// Reads the layout list from `file`. Returns an empty list if `file` does
/// not exist.
///
/// Unlike `settings_storage::load_settings` (which calls `.normalize()` on
/// every load to repair legacy/missing data), this does no equivalent pass —
/// there's no legacy shape to migrate yet, since `StatLayout` didn't exist
/// before this milestone. Revisit if `KNOWN_STATUS_LINE_STAT_IDS` ever drops
/// an id a persisted layout still references, or once Milestone 2's command
/// layer can actually produce a layout that needs repairing on load.
pub fn list_layouts(file: &Path) -> Result<Vec<StatLayout>, StorageError> {
    if !file.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(file)?;
    let layouts = serde_json::from_str(&content)?;
    Ok(layouts)
}

/// Writes `layouts` to `file` as JSON, creating parent directories if
/// necessary and overwriting any existing file. Writes to a temporary file
/// in the same directory and renames it into place, so a crash mid-write
/// cannot leave `file` truncated or corrupted.
pub fn save_layouts(file: &Path, layouts: &[StatLayout]) -> Result<(), StorageError> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(layouts)?;
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
    fn list_layouts_returns_empty_for_missing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("layouts.json");

        let layouts = list_layouts(&file).unwrap();

        assert!(layouts.is_empty());
    }

    #[test]
    fn save_then_list_round_trips_layouts() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("layouts.json");
        let layouts = vec![
            StatLayout::new_status_line(
                "Default".to_string(),
                vec!["status_badge".to_string(), "completion_pct".to_string()],
            ),
            StatLayout::new_status_line("Minimal".to_string(), vec!["status_badge".to_string()]),
        ];

        save_layouts(&file, &layouts).unwrap();
        let loaded = list_layouts(&file).unwrap();

        assert_eq!(loaded, layouts);
    }

    #[test]
    fn save_layouts_overwrites_existing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("layouts.json");
        let first = vec![StatLayout::new_status_line(
            "First".to_string(),
            vec!["status_badge".to_string()],
        )];
        let second = vec![StatLayout::new_status_line(
            "Second".to_string(),
            vec!["completion_pct".to_string()],
        )];

        save_layouts(&file, &first).unwrap();
        save_layouts(&file, &second).unwrap();
        let loaded = list_layouts(&file).unwrap();

        assert_eq!(loaded, second);
    }

    #[test]
    fn save_layouts_creates_parent_directory() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("nested").join("layouts.json");
        let layouts = vec![StatLayout::new_status_line(
            "Default".to_string(),
            vec!["status_badge".to_string()],
        )];

        save_layouts(&file, &layouts).unwrap();
        let loaded = list_layouts(&file).unwrap();

        assert_eq!(loaded, layouts);
    }

    #[test]
    fn list_layouts_returns_error_for_malformed_json() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("layouts.json");
        fs::write(&file, "not valid json").unwrap();

        let result = list_layouts(&file);

        assert!(matches!(result, Err(StorageError::Json(_))));
    }

    #[test]
    fn save_layouts_can_persist_an_empty_list() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("layouts.json");

        save_layouts(&file, &[]).unwrap();
        let loaded = list_layouts(&file).unwrap();

        assert!(loaded.is_empty());
    }
}
