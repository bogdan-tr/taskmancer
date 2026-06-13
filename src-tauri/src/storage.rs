use std::fs;
use std::path::Path;

use crate::task::{Task, TaskError};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("task error: {0}")]
    Task(#[from] TaskError),
    #[error("task with id '{0}' not found")]
    NotFound(String),
    #[error("'{0}' is not a valid task id")]
    InvalidId(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Rejects ids that are empty or contain path separators / `..`, since ids
/// are used to build file paths under `dir`.
fn validate_task_id(id: &str) -> Result<(), StorageError> {
    let is_safe = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if is_safe {
        Ok(())
    } else {
        Err(StorageError::InvalidId(id.to_string()))
    }
}

/// Reads every `*.md` task file in `dir` and returns the parsed tasks,
/// sorted by `order` (with creation time as a tiebreaker, e.g. for legacy
/// files that default to `order: 0`). Returns an empty list if `dir` does
/// not exist. Files that fail to parse are skipped, with a warning printed
/// to stderr.
pub fn list_tasks(dir: &Path) -> Result<Vec<Task>, StorageError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut tasks = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let content = fs::read_to_string(&path)?;
        match Task::from_markdown(&content) {
            Ok(task) => tasks.push(task),
            Err(err) => eprintln!("skipping unreadable task file {}: {err}", path.display()),
        }
    }

    tasks.sort_by(|a, b| {
        a.order
            .cmp(&b.order)
            .then_with(|| a.created.cmp(&b.created))
    });
    Ok(tasks)
}

/// Writes a task to `<dir>/<id>.md`, creating `dir` if necessary.
pub fn save_task(dir: &Path, task: &Task) -> Result<(), StorageError> {
    validate_task_id(&task.id)?;
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.md", task.id));
    let content = task.to_markdown()?;
    fs::write(path, content)?;
    Ok(())
}

/// Loads a single task by id from `<dir>/<id>.md`. The requested `id` is
/// written back onto the returned task so that a mismatched `id:` field in
/// the file's frontmatter cannot cause subsequent saves to target a
/// different file.
pub fn load_task(dir: &Path, id: &str) -> Result<Task, StorageError> {
    validate_task_id(id)?;
    let path = dir.join(format!("{id}.md"));
    if !path.exists() {
        return Err(StorageError::NotFound(id.to_string()));
    }
    let content = fs::read_to_string(path)?;
    let mut task = Task::from_markdown(&content)?;
    task.id = id.to_string();
    Ok(task)
}

/// Overwrites an existing task at `<dir>/<id>.md` with new field values.
/// Returns `NotFound` if no task with `task.id` currently exists, so that
/// updates cannot accidentally create new task files.
pub fn update_task(dir: &Path, task: &Task) -> Result<(), StorageError> {
    validate_task_id(&task.id)?;
    let path = dir.join(format!("{}.md", task.id));
    if !path.exists() {
        return Err(StorageError::NotFound(task.id.clone()));
    }
    let content = task.to_markdown()?;
    fs::write(path, content)?;
    Ok(())
}

/// Deletes the task file `<dir>/<id>.md`. Returns `NotFound` if no such
/// task exists.
pub fn delete_task(dir: &Path, id: &str) -> Result<(), StorageError> {
    validate_task_id(id)?;
    let path = dir.join(format!("{id}.md"));
    if !path.exists() {
        return Err(StorageError::NotFound(id.to_string()));
    }
    fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskStatus;
    use tempfile::tempdir;

    #[test]
    fn list_tasks_returns_empty_for_missing_directory() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");

        let tasks = list_tasks(&missing).unwrap();

        assert!(tasks.is_empty());
    }

    #[test]
    fn save_then_load_round_trips_a_task() {
        let dir = tempdir().unwrap();
        let task = Task::new("Write report".to_string());

        save_task(dir.path(), &task).unwrap();
        let loaded = load_task(dir.path(), &task.id).unwrap();

        assert_eq!(loaded, task);
    }

    #[test]
    fn load_task_returns_not_found_for_missing_id() {
        let dir = tempdir().unwrap();

        let result = load_task(dir.path(), "missing-id");

        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }

    #[test]
    fn list_tasks_ignores_non_markdown_files_and_falls_back_to_created_when_order_ties() {
        let dir = tempdir().unwrap();

        let mut older = Task::new("First".to_string());
        older.order = 0;
        older.created = "2026-01-01T00:00:00+00:00".to_string();
        let mut newer = Task::new("Second".to_string());
        newer.order = 0;
        newer.created = "2026-02-01T00:00:00+00:00".to_string();

        save_task(dir.path(), &newer).unwrap();
        save_task(dir.path(), &older).unwrap();
        fs::write(dir.path().join("notes.txt"), "not a task").unwrap();

        let tasks = list_tasks(dir.path()).unwrap();

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "First");
        assert_eq!(tasks[1].title, "Second");
    }

    #[test]
    fn list_tasks_sorts_by_order_then_created() {
        let dir = tempdir().unwrap();

        let mut higher_order = Task::new("Second by order".to_string());
        higher_order.order = 2000;
        let mut lower_order = Task::new("First by order".to_string());
        lower_order.order = 1000;

        save_task(dir.path(), &higher_order).unwrap();
        save_task(dir.path(), &lower_order).unwrap();

        let tasks = list_tasks(dir.path()).unwrap();

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "First by order");
        assert_eq!(tasks[1].title, "Second by order");
    }

    #[test]
    fn save_task_persists_status_changes() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Do the thing".to_string());
        save_task(dir.path(), &task).unwrap();

        task.status = TaskStatus::Done;
        save_task(dir.path(), &task).unwrap();
        let loaded = load_task(dir.path(), &task.id).unwrap();

        assert_eq!(loaded.status, TaskStatus::Done);
    }

    #[test]
    fn load_task_rejects_ids_with_path_separators() {
        let dir = tempdir().unwrap();

        let result = load_task(dir.path(), "../secrets");

        assert!(matches!(result, Err(StorageError::InvalidId(_))));
    }

    #[test]
    fn load_task_overrides_id_with_requested_id_on_mismatch() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Mismatched id".to_string());
        let original_id = task.id.clone();
        save_task(dir.path(), &task).unwrap();

        // Simulate a file whose frontmatter id no longer matches its filename.
        task.id = "different-id".to_string();
        fs::write(
            dir.path().join(format!("{original_id}.md")),
            task.to_markdown().unwrap(),
        )
        .unwrap();

        let loaded = load_task(dir.path(), &original_id).unwrap();

        assert_eq!(loaded.id, original_id);
    }

    #[test]
    fn update_task_persists_field_changes() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Original title".to_string());
        save_task(dir.path(), &task).unwrap();

        task.title = "Updated title".to_string();
        task.project = Some("Inbox/Personal".to_string());
        task.tags = vec!["urgent".to_string()];
        task.priority = crate::task::Priority::High;
        task.due = Some("2026-07-01".to_string());
        task.notes = "Updated notes".to_string();
        update_task(dir.path(), &task).unwrap();

        let loaded = load_task(dir.path(), &task.id).unwrap();

        assert_eq!(loaded, task);
    }

    #[test]
    fn update_task_round_trips_cleared_optional_fields() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Original title".to_string());
        task.project = Some("Inbox/Personal".to_string());
        task.tags = vec!["urgent".to_string()];
        task.due = Some("2026-07-01".to_string());
        task.scheduled = Some("2026-06-15".to_string());
        save_task(dir.path(), &task).unwrap();

        task.project = None;
        task.tags = Vec::new();
        task.due = None;
        task.scheduled = None;
        update_task(dir.path(), &task).unwrap();

        let loaded = load_task(dir.path(), &task.id).unwrap();

        assert_eq!(loaded.project, None);
        assert!(loaded.tags.is_empty());
        assert_eq!(loaded.due, None);
        assert_eq!(loaded.scheduled, None);
    }

    #[test]
    fn update_task_returns_not_found_for_nonexistent_id() {
        let dir = tempdir().unwrap();
        let task = Task::new("Never saved".to_string());

        let result = update_task(dir.path(), &task);

        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }

    #[test]
    fn update_task_rejects_ids_with_path_separators() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Bad id".to_string());
        task.id = "../escape".to_string();

        let result = update_task(dir.path(), &task);

        assert!(matches!(result, Err(StorageError::InvalidId(_))));
    }

    #[test]
    fn delete_task_removes_existing_file() {
        let dir = tempdir().unwrap();
        let task = Task::new("Throwaway".to_string());
        save_task(dir.path(), &task).unwrap();

        delete_task(dir.path(), &task.id).unwrap();

        assert!(matches!(
            load_task(dir.path(), &task.id),
            Err(StorageError::NotFound(_))
        ));
    }

    #[test]
    fn delete_task_returns_not_found_for_missing_id() {
        let dir = tempdir().unwrap();

        let result = delete_task(dir.path(), "missing-id");

        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }

    #[test]
    fn delete_task_rejects_ids_with_path_separators() {
        let dir = tempdir().unwrap();

        let result = delete_task(dir.path(), "../secrets");

        assert!(matches!(result, Err(StorageError::InvalidId(_))));
    }

    #[test]
    fn delete_task_is_not_repeatable() {
        let dir = tempdir().unwrap();
        let task = Task::new("Delete me once".to_string());
        save_task(dir.path(), &task).unwrap();

        delete_task(dir.path(), &task.id).unwrap();
        let second_delete = delete_task(dir.path(), &task.id);

        assert!(matches!(second_delete, Err(StorageError::NotFound(_))));
    }
}
