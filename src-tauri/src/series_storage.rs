use std::fs;
use std::path::Path;

use crate::series::Series;
use crate::storage::StorageError;

/// Reads the series list from `file`. Returns an empty list if `file` does
/// not exist.
pub fn list_series(file: &Path) -> Result<Vec<Series>, StorageError> {
    if !file.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(file)?;
    let series = serde_json::from_str(&content)?;
    Ok(series)
}

/// Writes `series` to `file` as JSON, creating parent directories if
/// necessary and overwriting any existing file. Writes to a temporary file
/// in the same directory and renames it into place, so a crash mid-write
/// cannot leave `file` truncated or corrupted.
pub fn save_series(file: &Path, series: &[Series]) -> Result<(), StorageError> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(series)?;
    let tmp_file = file.with_extension("json.tmp");
    fs::write(&tmp_file, content)?;
    fs::rename(&tmp_file, file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::series::RecurrenceFrequency;
    use tempfile::tempdir;

    fn new_series(title: &str) -> Series {
        Series::new(
            RecurrenceFrequency::EveryNDays { interval: 1 },
            "2026-06-15".to_string(),
            None,
            None,
            title.to_string(),
            None,
            "medium".to_string(),
            vec![],
            None,
            String::new(),
        )
    }

    #[test]
    fn list_series_returns_empty_for_missing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("series.json");

        let series = list_series(&file).unwrap();

        assert!(series.is_empty());
    }

    #[test]
    fn save_then_list_round_trips_series() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("series.json");
        let series = vec![new_series("Water the plants"), new_series("Take out trash")];

        save_series(&file, &series).unwrap();
        let loaded = list_series(&file).unwrap();

        assert_eq!(loaded, series);
    }

    #[test]
    fn save_series_overwrites_existing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("series.json");
        let first = vec![new_series("Water the plants")];
        let second = vec![new_series("Take out trash")];

        save_series(&file, &first).unwrap();
        save_series(&file, &second).unwrap();
        let loaded = list_series(&file).unwrap();

        assert_eq!(loaded, second);
    }

    #[test]
    fn save_series_creates_parent_directory() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("nested").join("series.json");
        let series = vec![new_series("Water the plants")];

        save_series(&file, &series).unwrap();
        let loaded = list_series(&file).unwrap();

        assert_eq!(loaded, series);
    }

    #[test]
    fn list_series_returns_error_for_malformed_json() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("series.json");
        fs::write(&file, "not valid json").unwrap();

        let result = list_series(&file);

        assert!(matches!(result, Err(StorageError::Json(_))));
    }
}
