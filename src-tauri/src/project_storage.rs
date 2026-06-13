use std::fs;
use std::path::Path;

use crate::project::Project;
use crate::storage::StorageError;

/// Reads the project list from `file`. Returns an empty list if `file` does
/// not exist.
pub fn list_projects(file: &Path) -> Result<Vec<Project>, StorageError> {
    if !file.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(file)?;
    let projects = serde_json::from_str(&content)?;
    Ok(projects)
}

/// Writes `projects` to `file` as JSON, creating parent directories if
/// necessary and overwriting any existing file. Writes to a temporary file
/// in the same directory and renames it into place, so a crash mid-write
/// cannot leave `file` truncated or corrupted.
pub fn save_projects(file: &Path, projects: &[Project]) -> Result<(), StorageError> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(projects)?;
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
    fn list_projects_returns_empty_for_missing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("projects.json");

        let projects = list_projects(&file).unwrap();

        assert!(projects.is_empty());
    }

    #[test]
    fn save_then_list_round_trips_projects() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("projects.json");
        let projects = vec![
            Project::new("Inbox".to_string(), "#6366f1".to_string(), 1),
            Project::new("Homework".to_string(), "#ef4444".to_string(), 2),
        ];

        save_projects(&file, &projects).unwrap();
        let loaded = list_projects(&file).unwrap();

        assert_eq!(loaded, projects);
    }

    #[test]
    fn save_projects_overwrites_existing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("projects.json");
        let first = vec![Project::new("Inbox".to_string(), "#6366f1".to_string(), 1)];
        let second = vec![Project::new(
            "Homework".to_string(),
            "#ef4444".to_string(),
            1,
        )];

        save_projects(&file, &first).unwrap();
        save_projects(&file, &second).unwrap();
        let loaded = list_projects(&file).unwrap();

        assert_eq!(loaded, second);
    }

    #[test]
    fn save_projects_creates_parent_directory() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("nested").join("projects.json");
        let projects = vec![Project::new("Inbox".to_string(), "#6366f1".to_string(), 1)];

        save_projects(&file, &projects).unwrap();
        let loaded = list_projects(&file).unwrap();

        assert_eq!(loaded, projects);
    }

    #[test]
    fn list_projects_returns_error_for_malformed_json() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("projects.json");
        fs::write(&file, "not valid json").unwrap();

        let result = list_projects(&file);

        assert!(matches!(result, Err(StorageError::Json(_))));
    }
}
