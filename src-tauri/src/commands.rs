use std::path::PathBuf;
use std::sync::Mutex;

use tauri::State;

use crate::project::{Project, DEFAULT_PROJECT_COLOR};
use crate::project_storage;
use crate::storage;
use crate::task::{Priority, Task, TaskStatus};

/// Shared application state holding the directory where task markdown
/// files are stored and the file where project metadata is stored.
///
/// `projects_lock` serializes the read-modify-write cycles in
/// [`list_projects`] (which may backfill) and [`create_project`] so
/// concurrent commands can't read a stale project list and overwrite each
/// other's changes.
pub struct AppState {
    pub tasks_dir: PathBuf,
    pub projects_file: PathBuf,
    pub projects_lock: Mutex<()>,
}

#[tauri::command]
pub fn list_tasks(state: State<AppState>) -> Result<Vec<Task>, String> {
    storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())
}

/// Applies optional overrides parsed from quick-add syntax onto a freshly
/// created task. Fields left as `None` keep `Task::new`'s defaults.
fn apply_create_overrides(
    task: &mut Task,
    project: Option<String>,
    tags: Option<Vec<String>>,
    priority: Option<Priority>,
    due: Option<String>,
    scheduled: Option<String>,
) {
    if let Some(project) = project {
        task.project = Some(project);
    }
    if let Some(tags) = tags {
        task.tags = tags;
    }
    if let Some(priority) = priority {
        task.priority = priority;
    }
    if let Some(due) = due {
        task.due = Some(due);
    }
    if let Some(scheduled) = scheduled {
        task.scheduled = Some(scheduled);
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_task(
    state: State<AppState>,
    title: String,
    project: Option<String>,
    tags: Option<Vec<String>>,
    priority: Option<Priority>,
    due: Option<String>,
    scheduled: Option<String>,
) -> Result<Task, String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("task title must not be empty".to_string());
    }
    validate_date_field(&due, "due")?;
    validate_date_field(&scheduled, "scheduled")?;

    let mut task = Task::new(title);
    apply_create_overrides(&mut task, project, tags, priority, due, scheduled);

    storage::save_task(&state.tasks_dir, &task).map_err(|e| e.to_string())?;
    Ok(task)
}

#[tauri::command]
pub fn update_task_status(
    state: State<AppState>,
    id: String,
    status: TaskStatus,
) -> Result<Task, String> {
    let mut task = storage::load_task(&state.tasks_dir, &id).map_err(|e| e.to_string())?;
    task.status = status;
    storage::save_task(&state.tasks_dir, &task).map_err(|e| e.to_string())?;
    Ok(task)
}

/// Rejects `Some(date)` values that aren't `YYYY-MM-DD`. `None` is always valid.
fn validate_date_field(value: &Option<String>, field: &str) -> Result<(), String> {
    match value {
        Some(date) if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() => {
            Err(format!("{field} must be a valid date in YYYY-MM-DD format"))
        }
        _ => Ok(()),
    }
}

/// Updates the editable fields of an existing task (title, project, tags,
/// priority, due/scheduled dates, notes). The task's `id`, `created`,
/// `status`, and `depends_on` are loaded from disk and preserved, so a
/// stale or malformed `task` payload from the frontend cannot corrupt
/// these fields or redirect the write to a different task's file.
#[tauri::command]
pub fn update_task(state: State<AppState>, task: Task) -> Result<Task, String> {
    let title = task.title.trim().to_string();
    if title.is_empty() {
        return Err("task title must not be empty".to_string());
    }
    validate_date_field(&task.due, "due")?;
    validate_date_field(&task.scheduled, "scheduled")?;

    let mut existing = storage::load_task(&state.tasks_dir, &task.id).map_err(|e| e.to_string())?;
    existing.title = title;
    existing.project = task.project;
    existing.tags = task.tags;
    existing.priority = task.priority;
    existing.due = task.due;
    existing.scheduled = task.scheduled;
    existing.notes = task.notes;

    storage::update_task(&state.tasks_dir, &existing).map_err(|e| e.to_string())?;
    Ok(existing)
}

#[tauri::command]
pub fn delete_task(state: State<AppState>, id: String) -> Result<(), String> {
    storage::delete_task(&state.tasks_dir, &id).map_err(|e| e.to_string())
}

/// Sets a task's board position (`order`) and optionally its status, used by
/// the frontend after a drag-and-drop reorder or move between columns.
/// `order` is an opaque sort key with no validation: it only affects display
/// ordering and cannot corrupt task data, so arbitrary values are accepted.
#[tauri::command]
pub fn reorder_task(
    state: State<AppState>,
    id: String,
    order: i64,
    status: Option<TaskStatus>,
) -> Result<Task, String> {
    let mut task = storage::load_task(&state.tasks_dir, &id).map_err(|e| e.to_string())?;
    task.order = order;
    if let Some(status) = status {
        task.status = status;
    }
    storage::update_task(&state.tasks_dir, &task).map_err(|e| e.to_string())?;
    Ok(task)
}

/// Returns the next `order` value for a new project: one greater than the
/// current maximum order, or `1` if there are no projects yet.
fn next_order(projects: &[Project]) -> i64 {
    projects
        .iter()
        .map(|p| p.order)
        .max()
        .map_or(1, |max| max + 1)
}

/// Ensures every distinct, non-empty `task.project` name has a corresponding
/// `Project` entry (matched case-insensitively against existing names).
/// Returns the (possibly extended) project list and whether any entries were
/// added, so callers can avoid rewriting the projects file when nothing
/// changed. New entries get fresh ids, [`DEFAULT_PROJECT_COLOR`], and
/// ascending `order` values starting just past the current maximum.
fn backfill_projects(existing: Vec<Project>, tasks: &[Task]) -> (Vec<Project>, bool) {
    let mut projects = existing;
    let mut next = next_order(&projects);
    let mut changed = false;
    let mut added_names: Vec<String> = Vec::new();

    for task in tasks {
        let Some(name) = task
            .project
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
        else {
            continue;
        };

        let already_known = projects.iter().any(|p| p.name.eq_ignore_ascii_case(name))
            || added_names
                .iter()
                .any(|added| added.eq_ignore_ascii_case(name));
        if already_known {
            continue;
        }

        added_names.push(name.to_string());
        projects.push(Project::new(
            name.to_string(),
            DEFAULT_PROJECT_COLOR.to_string(),
            next,
        ));
        next += 1;
        changed = true;
    }

    (projects, changed)
}

/// Returns all projects, sorted by `order`. Lazily backfills a `Project`
/// entry for any project name referenced by a task but not yet present in
/// the projects file, persisting the updated list only when something
/// changed.
#[tauri::command]
pub fn list_projects(state: State<AppState>) -> Result<Vec<Project>, String> {
    let tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;

    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let existing =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    let (mut projects, changed) = backfill_projects(existing, &tasks);
    if changed {
        project_storage::save_projects(&state.projects_file, &projects)
            .map_err(|e| e.to_string())?;
    }

    projects.sort_by_key(|p| p.order);
    Ok(projects)
}

/// Creates a new project with a trimmed, non-empty, case-insensitively
/// unique name. `order` is set to one past the current maximum so new
/// projects sort after existing ones. Falls back to
/// [`DEFAULT_PROJECT_COLOR`] when `color` is `None`.
#[tauri::command]
pub fn create_project(
    state: State<AppState>,
    name: String,
    color: Option<String>,
) -> Result<Project, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("project name must not be empty".to_string());
    }

    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    if projects.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
        return Err(format!("a project named '{name}' already exists"));
    }

    let order = next_order(&projects);
    let color = color.unwrap_or_else(|| DEFAULT_PROJECT_COLOR.to_string());
    let project = Project::new(name, color, order);
    projects.push(project.clone());
    project_storage::save_projects(&state.projects_file, &projects).map_err(|e| e.to_string())?;

    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_date_field_accepts_none() {
        assert!(validate_date_field(&None, "due").is_ok());
    }

    #[test]
    fn validate_date_field_accepts_valid_iso_date() {
        assert!(validate_date_field(&Some("2026-07-01".to_string()), "due").is_ok());
    }

    #[test]
    fn validate_date_field_rejects_malformed_date() {
        let result = validate_date_field(&Some("07/01/2026".to_string()), "due");

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("due"));
    }

    #[test]
    fn validate_date_field_rejects_invalid_calendar_date() {
        let result = validate_date_field(&Some("2026-02-30".to_string()), "scheduled");

        assert!(result.is_err());
    }

    #[test]
    fn apply_create_overrides_keeps_defaults_when_all_none() {
        let mut task = Task::new("Buy milk".to_string());
        let defaults = task.clone();

        apply_create_overrides(&mut task, None, None, None, None, None);

        assert_eq!(task.project, defaults.project);
        assert_eq!(task.tags, defaults.tags);
        assert_eq!(task.priority, defaults.priority);
        assert_eq!(task.due, defaults.due);
        assert_eq!(task.scheduled, defaults.scheduled);
    }

    #[test]
    fn apply_create_overrides_sets_provided_fields() {
        let mut task = Task::new("Plan trip".to_string());

        apply_create_overrides(
            &mut task,
            Some("Vacation".to_string()),
            Some(vec!["travel".to_string()]),
            Some(Priority::High),
            Some("2026-07-01".to_string()),
            Some("2026-06-25".to_string()),
        );

        assert_eq!(task.project, Some("Vacation".to_string()));
        assert_eq!(task.tags, vec!["travel".to_string()]);
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.due, Some("2026-07-01".to_string()));
        assert_eq!(task.scheduled, Some("2026-06-25".to_string()));
    }

    #[test]
    fn next_order_returns_one_for_empty_list() {
        assert_eq!(next_order(&[]), 1);
    }

    #[test]
    fn next_order_returns_one_past_the_current_maximum() {
        let projects = vec![
            Project::new("A".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 3),
            Project::new("B".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 7),
        ];

        assert_eq!(next_order(&projects), 8);
    }

    #[test]
    fn backfill_projects_adds_missing_project_names_from_tasks() {
        let mut task = Task::new("Do homework".to_string());
        task.project = Some("Homework".to_string());

        let (projects, changed) = backfill_projects(Vec::new(), &[task]);

        assert!(changed);
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "Homework");
        assert_eq!(projects[0].order, 1);
        assert_eq!(projects[0].color, DEFAULT_PROJECT_COLOR);
    }

    #[test]
    fn backfill_projects_is_case_insensitive_against_existing_projects() {
        let existing = vec![Project::new(
            "Homework".to_string(),
            "#abcdef".to_string(),
            1,
        )];
        let mut task = Task::new("Do homework".to_string());
        task.project = Some("HOMEWORK".to_string());

        let (projects, changed) = backfill_projects(existing, &[task]);

        assert!(!changed);
        assert_eq!(projects.len(), 1);
    }

    #[test]
    fn backfill_projects_dedupes_multiple_tasks_with_the_same_new_project_name() {
        let mut task_a = Task::new("First".to_string());
        task_a.project = Some("Side Project".to_string());
        let mut task_b = Task::new("Second".to_string());
        task_b.project = Some("side project".to_string());

        let (projects, changed) = backfill_projects(Vec::new(), &[task_a, task_b]);

        assert!(changed);
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "Side Project");
    }

    #[test]
    fn backfill_projects_ignores_tasks_without_a_project() {
        let task = Task::new("No project".to_string());

        let (projects, changed) = backfill_projects(Vec::new(), &[task]);

        assert!(!changed);
        assert!(projects.is_empty());
    }

    #[test]
    fn backfill_projects_preserves_existing_ids_and_colors() {
        let existing = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let existing_id = existing[0].id.clone();
        let mut task = Task::new("Triage".to_string());
        task.project = Some("Inbox".to_string());

        let (projects, changed) = backfill_projects(existing, &[task]);

        assert!(!changed);
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, existing_id);
        assert_eq!(projects[0].color, "#abcdef");
    }

    #[test]
    fn backfill_projects_assigns_ascending_order_to_new_entries() {
        let mut task_a = Task::new("First".to_string());
        task_a.project = Some("Alpha".to_string());
        let mut task_b = Task::new("Second".to_string());
        task_b.project = Some("Beta".to_string());

        let (projects, _) = backfill_projects(Vec::new(), &[task_a, task_b]);

        assert_eq!(projects[0].order, 1);
        assert_eq!(projects[1].order, 2);
    }
}
