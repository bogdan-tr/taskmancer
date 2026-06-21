use std::fs;
use std::path::Path;

use crate::project::Project;
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

/// Moves the task file `<tasks_dir>/<id>.md` to `<archive_dir>/<id>.md`,
/// creating `archive_dir` if necessary. Returns `NotFound` if no task with
/// `id` exists in `tasks_dir`.
pub fn archive_task(tasks_dir: &Path, archive_dir: &Path, id: &str) -> Result<(), StorageError> {
    validate_task_id(id)?;
    let src = tasks_dir.join(format!("{id}.md"));
    if !src.exists() {
        return Err(StorageError::NotFound(id.to_string()));
    }
    fs::create_dir_all(archive_dir)?;
    let dest = archive_dir.join(format!("{id}.md"));
    fs::rename(src, dest)?;
    Ok(())
}

/// One-time migration for `*.md` task files saved before every task was
/// required to have a `scheduled` date. For each task missing `scheduled`,
/// backfills it with the local-timezone date of the task's `created`
/// timestamp and rewrites the file. Tasks that already have `scheduled` set
/// are left untouched, so re-running this migration is a no-op. Returns `Ok`
/// without doing anything if `dir` does not exist. Files that fail to parse,
/// or whose `created` timestamp isn't valid RFC3339, are skipped with a
/// warning printed to stderr.
pub fn migrate_scheduled_dates(dir: &Path) -> Result<(), StorageError> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!(
                    "skipping unreadable task file {} during scheduled-date migration: {err}",
                    path.display()
                );
                continue;
            }
        };
        let mut task = match Task::from_markdown(&content) {
            Ok(task) => task,
            Err(err) => {
                eprintln!(
                    "skipping unreadable task file {} during scheduled-date migration: {err}",
                    path.display()
                );
                continue;
            }
        };

        if task.scheduled.is_some() {
            continue;
        }

        let created = match chrono::DateTime::parse_from_rfc3339(&task.created) {
            Ok(created) => created,
            Err(err) => {
                eprintln!(
                    "skipping task {} with unparseable created timestamp during scheduled-date migration: {err}",
                    task.id
                );
                continue;
            }
        };

        let scheduled_date = created.with_timezone(&chrono::Local).date_naive();
        task.scheduled = Some(scheduled_date.format("%Y-%m-%d").to_string());

        let markdown = match task.to_markdown() {
            Ok(markdown) => markdown,
            Err(err) => {
                eprintln!(
                    "skipping task {} due to a serialization error during scheduled-date migration: {err}",
                    task.id
                );
                continue;
            }
        };

        if let Err(err) = fs::write(&path, markdown) {
            eprintln!(
                "failed to write migrated scheduled date for task file {} during scheduled-date migration: {err}",
                path.display()
            );
        }
    }

    Ok(())
}

/// Extracts the legacy `project` frontmatter key (a plain project name
/// string, from before `Task.project_id` replaced it) directly from
/// `content`'s raw YAML frontmatter, without going through
/// `Task::from_markdown` — which can no longer see this key at all, since
/// `Task` itself has no `project` field anymore (only `project_id`).
/// Returns `None` if there's no frontmatter, no `project` key, or the key
/// isn't a plain string.
fn read_legacy_project_name(content: &str) -> Option<String> {
    let after_open = content.strip_prefix("---\n")?;
    let end = after_open.find("\n---")?;
    let frontmatter_yaml = &after_open[..end];
    let value: serde_yaml::Value = serde_yaml::from_str(frontmatter_yaml).ok()?;
    value.get("project")?.as_str().map(str::to_string)
}

/// One-time migration: rewrites every task markdown file in `tasks_dir`
/// that still has the legacy `project` frontmatter key to use `project_id`
/// instead, resolved by case-insensitive name lookup against `projects`
/// (see [`read_legacy_project_name`]). Idempotent — a task with no legacy
/// `project` key (already migrated, or created fresh after this field
/// existed) is left completely untouched, never rewritten. A task whose
/// legacy name doesn't match any project keeps `project_id` unset and gets
/// a logged warning naming the task and the unmatched name, rather than
/// silently losing the link — real user data is at stake here, so an
/// unresolvable reference must be visible, not swallowed.
pub fn migrate_task_project_names_to_ids(
    tasks_dir: &Path,
    projects: &[Project],
) -> Result<(), StorageError> {
    if !tasks_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(tasks_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!(
                    "skipping unreadable task file {} during project-id migration: {err}",
                    path.display()
                );
                continue;
            }
        };

        let Some(legacy_name) = read_legacy_project_name(&content) else {
            continue;
        };

        let mut task = match Task::from_markdown(&content) {
            Ok(task) => task,
            Err(err) => {
                eprintln!(
                    "skipping unreadable task file {} during project-id migration: {err}",
                    path.display()
                );
                continue;
            }
        };

        match projects
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(&legacy_name))
        {
            Some(matched) => task.project_id = Some(matched.id.clone()),
            None => eprintln!(
                "task '{}' ({}) referenced unknown project '{legacy_name}' during project-id migration — leaving its project unset",
                task.title, task.id
            ),
        }

        save_task(tasks_dir, &task)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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

        task.status = "done".to_string();
        save_task(dir.path(), &task).unwrap();
        let loaded = load_task(dir.path(), &task.id).unwrap();

        assert_eq!(loaded.status, "done");
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
        task.project_id = Some("inbox-personal-id".to_string());
        task.tags = vec!["urgent".to_string()];
        task.priority = "high".to_string();
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
        task.project_id = Some("inbox-personal-id".to_string());
        task.tags = vec!["urgent".to_string()];
        task.due = Some("2026-07-01".to_string());
        task.scheduled = Some("2026-06-15".to_string());
        save_task(dir.path(), &task).unwrap();

        task.project_id = None;
        task.tags = Vec::new();
        task.due = None;
        task.scheduled = None;
        update_task(dir.path(), &task).unwrap();

        let loaded = load_task(dir.path(), &task.id).unwrap();

        assert_eq!(loaded.project_id, None);
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

    #[test]
    fn archive_task_moves_file_from_tasks_dir_to_archive_dir() {
        let tasks_dir = tempdir().unwrap();
        let archive_parent = tempdir().unwrap();
        let archive_dir = archive_parent.path().join("archive");
        let task = Task::new("Archive me".to_string());
        save_task(tasks_dir.path(), &task).unwrap();

        archive_task(tasks_dir.path(), &archive_dir, &task.id).unwrap();

        assert!(matches!(
            load_task(tasks_dir.path(), &task.id),
            Err(StorageError::NotFound(_))
        ));
        let archived = load_task(&archive_dir, &task.id).unwrap();
        assert_eq!(archived, task);
    }

    #[test]
    fn archive_task_creates_archive_dir_if_missing() {
        let tasks_dir = tempdir().unwrap();
        let parent = tempdir().unwrap();
        let archive_dir = parent.path().join("nested").join("archive");
        let task = Task::new("Needs new dir".to_string());
        save_task(tasks_dir.path(), &task).unwrap();

        archive_task(tasks_dir.path(), &archive_dir, &task.id).unwrap();

        assert!(archive_dir.join(format!("{}.md", task.id)).exists());
    }

    #[test]
    fn archive_task_returns_not_found_for_missing_source() {
        let tasks_dir = tempdir().unwrap();
        let archive_dir = tasks_dir.path().join("archive");

        let result = archive_task(tasks_dir.path(), &archive_dir, "missing-id");

        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }

    #[test]
    fn archive_task_rejects_ids_with_path_separators() {
        let tasks_dir = tempdir().unwrap();
        let archive_dir = tasks_dir.path().join("archive");

        let result = archive_task(tasks_dir.path(), &archive_dir, "../secrets");

        assert!(matches!(result, Err(StorageError::InvalidId(_))));
    }

    #[test]
    fn archive_task_is_not_repeatable() {
        let tasks_dir = tempdir().unwrap();
        let archive_dir = tasks_dir.path().join("archive");
        let task = Task::new("Once only".to_string());
        save_task(tasks_dir.path(), &task).unwrap();

        archive_task(tasks_dir.path(), &archive_dir, &task.id).unwrap();
        let second = archive_task(tasks_dir.path(), &archive_dir, &task.id);

        assert!(matches!(second, Err(StorageError::NotFound(_))));
    }

    #[test]
    fn migrate_scheduled_dates_returns_ok_for_missing_directory() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");

        let result = migrate_scheduled_dates(&missing);

        assert!(result.is_ok());
    }

    #[test]
    fn migrate_scheduled_dates_backfills_scheduled_from_created_for_tasks_missing_it() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Needs scheduled".to_string());
        task.scheduled = None;
        task.created = "2026-06-10T12:00:00+00:00".to_string();
        save_task(dir.path(), &task).unwrap();

        migrate_scheduled_dates(dir.path()).unwrap();

        let expected = chrono::DateTime::parse_from_rfc3339(&task.created)
            .unwrap()
            .with_timezone(&chrono::Local)
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let loaded = load_task(dir.path(), &task.id).unwrap();
        assert_eq!(loaded.scheduled, Some(expected));
    }

    #[test]
    fn migrate_scheduled_dates_does_not_overwrite_an_existing_scheduled_date() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Already scheduled".to_string());
        task.scheduled = Some("2026-01-01".to_string());
        task.created = "2026-06-10T12:00:00+00:00".to_string();
        save_task(dir.path(), &task).unwrap();

        migrate_scheduled_dates(dir.path()).unwrap();
        migrate_scheduled_dates(dir.path()).unwrap();

        let loaded = load_task(dir.path(), &task.id).unwrap();
        assert_eq!(loaded.scheduled, Some("2026-01-01".to_string()));
    }

    #[test]
    fn migrate_scheduled_dates_skips_unparseable_files_and_continues() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("broken.md"), "not valid frontmatter").unwrap();

        let mut task = Task::new("Valid task".to_string());
        task.scheduled = None;
        save_task(dir.path(), &task).unwrap();

        let result = migrate_scheduled_dates(dir.path());

        assert!(result.is_ok());
        let loaded = load_task(dir.path(), &task.id).unwrap();
        assert!(loaded.scheduled.is_some());
    }

    #[test]
    fn read_legacy_project_name_extracts_the_project_key() {
        let content = "---\nid: abc\ntitle: Demo\nproject: Homework\ncreated: 2026-06-11T10:00:00+00:00\n---\n\nNotes.";

        let name = read_legacy_project_name(content);

        assert_eq!(name, Some("Homework".to_string()));
    }

    #[test]
    fn read_legacy_project_name_returns_none_when_absent() {
        let content =
            "---\nid: abc\ntitle: Demo\ncreated: 2026-06-11T10:00:00+00:00\n---\n\nNotes.";

        let name = read_legacy_project_name(content);

        assert_eq!(name, None);
    }

    #[test]
    fn read_legacy_project_name_returns_none_for_already_migrated_content() {
        let content = "---\nid: abc\ntitle: Demo\nproject_id: some-id\ncreated: 2026-06-11T10:00:00+00:00\n---\n\nNotes.";

        let name = read_legacy_project_name(content);

        assert_eq!(name, None);
    }

    #[test]
    fn read_legacy_project_name_returns_none_for_content_with_no_frontmatter() {
        let name = read_legacy_project_name("Just plain markdown, no frontmatter.");

        assert_eq!(name, None);
    }

    #[test]
    fn migrate_task_project_names_to_ids_resolves_a_matching_project() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Read chapter 1".to_string());
        let task_id = task.id.clone();
        save_task(dir.path(), &task).unwrap();
        // Simulate a legacy file by writing the raw markdown directly,
        // bypassing `Task::to_markdown` (which would write `project_id`,
        // not the legacy `project` key).
        let legacy_content = format!(
            "---\nid: {task_id}\ntitle: Read chapter 1\nproject: Homework\ncreated: {}\n---\n\n",
            task.created
        );
        fs::write(dir.path().join(format!("{task_id}.md")), legacy_content).unwrap();
        task.project_id = None;

        let homework = Project::new("Homework".to_string(), "#111111".to_string(), 1);
        let homework_id = homework.id.clone();
        let projects = vec![homework];

        migrate_task_project_names_to_ids(dir.path(), &projects).unwrap();

        let migrated = load_task(dir.path(), &task_id).unwrap();
        assert_eq!(migrated.project_id, Some(homework_id));
    }

    #[test]
    fn migrate_task_project_names_to_ids_matches_case_insensitively() {
        let dir = tempdir().unwrap();
        let task_id = "task-1".to_string();
        let legacy_content = format!(
            "---\nid: {task_id}\ntitle: Demo\nproject: HOMEWORK\ncreated: 2026-06-11T10:00:00+00:00\n---\n\n"
        );
        fs::write(dir.path().join(format!("{task_id}.md")), legacy_content).unwrap();

        let homework = Project::new("Homework".to_string(), "#111111".to_string(), 1);
        let homework_id = homework.id.clone();
        let projects = vec![homework];

        migrate_task_project_names_to_ids(dir.path(), &projects).unwrap();

        let migrated = load_task(dir.path(), &task_id).unwrap();
        assert_eq!(migrated.project_id, Some(homework_id));
    }

    #[test]
    fn migrate_task_project_names_to_ids_leaves_unmatched_tasks_unset_without_failing() {
        let dir = tempdir().unwrap();
        let task_id = "task-1".to_string();
        let legacy_content = format!(
            "---\nid: {task_id}\ntitle: Demo\nproject: DoesNotExist\ncreated: 2026-06-11T10:00:00+00:00\n---\n\n"
        );
        fs::write(dir.path().join(format!("{task_id}.md")), legacy_content).unwrap();
        let projects: Vec<Project> = Vec::new();

        let result = migrate_task_project_names_to_ids(dir.path(), &projects);

        assert!(result.is_ok());
        let migrated = load_task(dir.path(), &task_id).unwrap();
        assert_eq!(migrated.project_id, None);
    }

    #[test]
    fn migrate_task_project_names_to_ids_is_idempotent() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Already migrated".to_string());
        task.project_id = Some("existing-id".to_string());
        save_task(dir.path(), &task).unwrap();
        let projects = vec![Project::new(
            "Unrelated".to_string(),
            "#222222".to_string(),
            1,
        )];

        migrate_task_project_names_to_ids(dir.path(), &projects).unwrap();

        let unchanged = load_task(dir.path(), &task.id).unwrap();
        assert_eq!(unchanged.project_id, Some("existing-id".to_string()));
    }

    #[test]
    fn migrate_task_project_names_to_ids_returns_ok_for_missing_directory() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");

        let result = migrate_task_project_names_to_ids(&missing, &[]);

        assert!(result.is_ok());
    }
}
