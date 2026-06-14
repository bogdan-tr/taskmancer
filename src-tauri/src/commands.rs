use std::path::PathBuf;
use std::sync::Mutex;

use tauri::State;

use crate::project::{Project, ProjectBoard, DEFAULT_PROJECT_COLOR};
use crate::project_storage;
use crate::settings::{validate_priority_id, validate_settings, validate_status_id, Settings};
use crate::settings_storage;
use crate::storage;
use crate::task::Task;

/// Shared application state holding the directory where task markdown
/// files are stored, the file where project metadata is stored, and the
/// file where global settings are stored.
///
/// `projects_lock` serializes the read-modify-write cycles in
/// [`list_projects`] (which may backfill), [`create_project`], and
/// [`update_project`] so concurrent commands can't read a stale project
/// list and overwrite each other's changes.
pub struct AppState {
    pub tasks_dir: PathBuf,
    pub projects_file: PathBuf,
    pub settings_file: PathBuf,
    pub projects_lock: Mutex<()>,
}

#[tauri::command]
pub fn list_tasks(state: State<AppState>) -> Result<Vec<Task>, String> {
    storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())
}

/// Resolves the priority a new task should get when none was explicitly
/// requested: `settings.defaults.priority` if it names a currently-defined
/// priority level, otherwise the level with the lowest `rank` (rank 1 sorts
/// first / is the highest priority), otherwise `"medium"` if no priority
/// levels are defined at all.
fn resolve_default_priority(settings: &Settings) -> String {
    if let Some(default_id) = &settings.defaults.priority {
        if validate_priority_id(settings, default_id).is_ok() {
            return default_id.clone();
        }
    }

    settings
        .priorities
        .iter()
        .min_by_key(|level| level.rank)
        .map(|level| level.id.clone())
        .unwrap_or_else(|| "medium".to_string())
}

/// Resolves the status a new task should get when none was explicitly
/// requested. Checked in order: `project_board.default_status` (if it names
/// a currently-defined status), `settings.defaults.status` (if it names a
/// currently-defined status), the status with the lowest `order` (order 1
/// sorts first), otherwise `"backlog"` if no statuses are defined at all.
fn resolve_default_status(settings: &Settings, project_board: Option<&ProjectBoard>) -> String {
    if let Some(default_id) = project_board.and_then(|board| board.default_status.as_ref()) {
        if validate_status_id(settings, default_id).is_ok() {
            return default_id.clone();
        }
    }

    if let Some(default_id) = &settings.defaults.status {
        if validate_status_id(settings, default_id).is_ok() {
            return default_id.clone();
        }
    }

    settings
        .statuses
        .iter()
        .min_by_key(|status| status.order)
        .map(|status| status.id.clone())
        .unwrap_or_else(|| "backlog".to_string())
}

/// Looks up `project_name` case-insensitively among `projects` and returns
/// its board configuration, or `None` if no project matches (e.g. the named
/// project doesn't exist yet and will be backfilled by `list_projects`).
fn find_project_board<'a>(projects: &'a [Project], project_name: &str) -> Option<&'a ProjectBoard> {
    projects
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(project_name))
        .map(|p| &p.board)
}

/// Applies optional overrides parsed from quick-add syntax onto a freshly
/// created task. Fields left as `None` keep `Task::new`'s defaults.
fn apply_create_overrides(
    task: &mut Task,
    project: Option<String>,
    tags: Option<Vec<String>>,
    priority: Option<String>,
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
    priority: Option<String>,
    due: Option<String>,
    scheduled: Option<String>,
) -> Result<Task, String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("task title must not be empty".to_string());
    }
    validate_date_field(&due, "due")?;
    validate_date_field(&scheduled, "scheduled")?;

    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    let priority = match priority {
        Some(id) => {
            validate_priority_id(&settings, &id)?;
            id
        }
        None => resolve_default_priority(&settings),
    };

    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let project_board = project
        .as_deref()
        .and_then(|name| find_project_board(&projects, name));
    let status = resolve_default_status(&settings, project_board);

    let mut task = Task::new(title);
    task.status = status;
    apply_create_overrides(&mut task, project, tags, Some(priority), due, scheduled);

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

    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    validate_priority_id(&settings, &task.priority)?;

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

/// Sets a task's board position (`order`) and optionally its status and/or
/// priority, used by the frontend after a drag-and-drop reorder, a move
/// between status columns, or a move between priority groups within a
/// column. `order` is an opaque sort key with no validation: it only affects
/// display ordering and cannot corrupt task data, so arbitrary values are
/// accepted. `status` and `priority`, if provided, are validated against the
/// current settings.
#[tauri::command]
pub fn reorder_task(
    state: State<AppState>,
    id: String,
    order: i64,
    status: Option<String>,
    priority: Option<String>,
) -> Result<Task, String> {
    if status.is_some() || priority.is_some() {
        let settings =
            settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
        if let Some(status) = &status {
            validate_status_id(&settings, status)?;
        }
        if let Some(priority) = &priority {
            validate_priority_id(&settings, priority)?;
        }
    }

    let mut task = storage::load_task(&state.tasks_dir, &id).map_err(|e| e.to_string())?;
    task.order = order;
    if let Some(status) = status {
        task.status = status;
    }
    if let Some(priority) = priority {
        task.priority = priority;
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

/// Applies an `update_project` request onto the project at `index` within
/// `projects`, preserving that project's `id` and `created` timestamp from
/// disk. Validates that `update.name` (trimmed) is non-empty and
/// case-insensitively unique among the *other* projects, and that every
/// status/priority id referenced by `update.board` and `update.defaults` is
/// defined in `settings` (mirroring the checks `validate_settings` applies to
/// `Settings.defaults`).
fn apply_project_update(
    projects: &mut [Project],
    index: usize,
    update: Project,
    settings: &Settings,
) -> Result<Project, String> {
    let name = update.name.trim().to_string();
    if name.is_empty() {
        return Err("project name must not be empty".to_string());
    }
    if projects
        .iter()
        .enumerate()
        .any(|(i, p)| i != index && p.name.eq_ignore_ascii_case(&name))
    {
        return Err(format!("a project named '{name}' already exists"));
    }

    for status_id in &update.board.statuses {
        validate_status_id(settings, status_id)?;
    }
    if let Some(status_id) = &update.board.default_status {
        validate_status_id(settings, status_id)?;
    }
    if let Some(status_id) = &update.defaults.status {
        validate_status_id(settings, status_id)?;
    }
    if let Some(priority_id) = &update.defaults.priority {
        validate_priority_id(settings, priority_id)?;
    }

    let existing = &projects[index];
    let updated = Project {
        id: existing.id.clone(),
        name,
        color: update.color,
        order: update.order,
        created: existing.created.clone(),
        board: update.board,
        defaults: update.defaults,
    };
    projects[index] = updated.clone();
    Ok(updated)
}

/// Updates an existing project's editable fields (name, color, order, board
/// configuration, and default-attribute overrides). The project's `id` and
/// `created` timestamp are loaded from disk and preserved. The board's
/// status ids and the default-attribute overrides are validated against the
/// current settings (see [`apply_project_update`]).
#[tauri::command]
pub fn update_project(state: State<AppState>, project: Project) -> Result<Project, String> {
    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;

    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    let index = projects
        .iter()
        .position(|p| p.id == project.id)
        .ok_or_else(|| format!("project '{}' not found", project.id))?;

    let updated = apply_project_update(&mut projects, index, project, &settings)?;

    project_storage::save_projects(&state.projects_file, &projects).map_err(|e| e.to_string())?;
    Ok(updated)
}

/// Returns the global settings (custom priority levels, the global status
/// list, and global default task attributes), seeding and persisting
/// defaults on first use if no settings file exists yet.
#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    if !state.settings_file.exists() {
        settings_storage::save_settings(&state.settings_file, &settings)
            .map_err(|e| e.to_string())?;
    }
    Ok(settings)
}

/// Returns an error if any `project` still references a status or priority
/// id that's missing from `settings` — in its board's status list or default
/// status, or in its default-attribute overrides. Called before persisting
/// `settings` so a status/priority can't be removed while a project's board
/// configuration still depends on it, leaving a dangling reference that
/// `apply_project_update` would otherwise carry forward indefinitely.
fn validate_settings_against_projects(
    settings: &Settings,
    projects: &[Project],
) -> Result<(), String> {
    for project in projects {
        for status_id in &project.board.statuses {
            validate_status_id(settings, status_id).map_err(|_| {
                format!(
                    "cannot remove status '{status_id}': still used on project '{}' board",
                    project.name
                )
            })?;
        }
        if let Some(status_id) = &project.board.default_status {
            validate_status_id(settings, status_id).map_err(|_| {
                format!(
                    "cannot remove status '{status_id}': still used as the default status for project '{}'",
                    project.name
                )
            })?;
        }
        if let Some(status_id) = &project.defaults.status {
            validate_status_id(settings, status_id).map_err(|_| {
                format!(
                    "cannot remove status '{status_id}': still used as a default-status override for project '{}'",
                    project.name
                )
            })?;
        }
        if let Some(priority_id) = &project.defaults.priority {
            validate_priority_id(settings, priority_id).map_err(|_| {
                format!(
                    "cannot remove priority '{priority_id}': still used as a default-priority override for project '{}'",
                    project.name
                )
            })?;
        }
    }
    Ok(())
}

/// Persists `settings` as the new global settings, overwriting any
/// previous values. Rejects settings that would leave `priorities` or
/// `statuses` empty, containing duplicate ids, or pointing
/// `defaults.priority`/`defaults.status` at an undefined id — any of which
/// would break `validate_priority_id`/`validate_status_id` for every later
/// task write. Also rejects removing a status or priority that's still
/// referenced by any project's board configuration or default-attribute
/// overrides (see [`validate_settings_against_projects`]).
#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> Result<Settings, String> {
    validate_settings(&settings)?;

    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    validate_settings_against_projects(&settings, &projects)?;

    settings_storage::save_settings(&state.settings_file, &settings).map_err(|e| e.to_string())?;
    Ok(settings)
}

/// Counts `tasks` by their `priority` id. Used by the settings UI to warn
/// before deleting a priority level that's still referenced by tasks.
fn tally_priorities(tasks: &[Task]) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    for task in tasks {
        *counts.entry(task.priority.clone()).or_insert(0) += 1;
    }
    counts
}

/// Returns the number of tasks currently using each priority id.
#[tauri::command]
pub fn count_tasks_by_priority(
    state: State<AppState>,
) -> Result<std::collections::HashMap<String, usize>, String> {
    let tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
    Ok(tally_priorities(&tasks))
}

/// Counts `tasks` by their `status` id. Used by the settings UI to warn
/// before deleting a status that's still referenced by tasks.
fn tally_statuses(tasks: &[Task]) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    for task in tasks {
        *counts.entry(task.status.clone()).or_insert(0) += 1;
    }
    counts
}

/// Returns the number of tasks currently using each status id.
#[tauri::command]
pub fn count_tasks_by_status(
    state: State<AppState>,
) -> Result<std::collections::HashMap<String, usize>, String> {
    let tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
    Ok(tally_statuses(&tasks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectBoard;
    use crate::settings::{PriorityLevel, StatusDefinition, TaskDefaults};

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
            Some("high".to_string()),
            Some("2026-07-01".to_string()),
            Some("2026-06-25".to_string()),
        );

        assert_eq!(task.project, Some("Vacation".to_string()));
        assert_eq!(task.tags, vec!["travel".to_string()]);
        assert_eq!(task.priority, "high");
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

    #[test]
    fn apply_project_update_preserves_id_and_created_from_disk() {
        let existing = Project::new("Inbox".to_string(), "#abcdef".to_string(), 1);
        let existing_id = existing.id.clone();
        let existing_created = existing.created.clone();
        let mut projects = vec![existing];

        let mut update = projects[0].clone();
        update.id = "ignored-id".to_string();
        update.created = "ignored-created".to_string();
        update.name = "Renamed".to_string();
        update.color = "#123456".to_string();

        let updated = apply_project_update(&mut projects, 0, update, &Settings::default()).unwrap();

        assert_eq!(updated.id, existing_id);
        assert_eq!(updated.created, existing_created);
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.color, "#123456");
        assert_eq!(projects[0], updated);
    }

    #[test]
    fn apply_project_update_trims_the_name() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.name = "  Renamed  ".to_string();

        let updated = apply_project_update(&mut projects, 0, update, &Settings::default()).unwrap();

        assert_eq!(updated.name, "Renamed");
    }

    #[test]
    fn apply_project_update_rejects_empty_name() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.name = "   ".to_string();

        let result = apply_project_update(&mut projects, 0, update, &Settings::default());

        assert!(result.is_err());
    }

    #[test]
    fn apply_project_update_rejects_name_colliding_with_another_project() {
        let mut projects = vec![
            Project::new("Inbox".to_string(), "#abcdef".to_string(), 1),
            Project::new("Homework".to_string(), "#fedcba".to_string(), 2),
        ];
        let mut update = projects[1].clone();
        update.name = "inbox".to_string();

        let result = apply_project_update(&mut projects, 1, update, &Settings::default());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn apply_project_update_allows_keeping_its_own_name() {
        let mut projects = vec![
            Project::new("Inbox".to_string(), "#abcdef".to_string(), 1),
            Project::new("Homework".to_string(), "#fedcba".to_string(), 2),
        ];
        let mut update = projects[1].clone();
        update.color = "#000000".to_string();

        let updated = apply_project_update(&mut projects, 1, update, &Settings::default()).unwrap();

        assert_eq!(updated.name, "Homework");
        assert_eq!(updated.color, "#000000");
    }

    #[test]
    fn apply_project_update_persists_board_and_defaults() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.board = ProjectBoard {
            statuses: vec!["backlog".to_string(), "done".to_string()],
            default_status: Some("backlog".to_string()),
        };
        update.defaults = TaskDefaults {
            tags: vec!["home".to_string()],
            priority: Some("high".to_string()),
            status: None,
            due: Some("tomorrow".to_string()),
        };

        let updated =
            apply_project_update(&mut projects, 0, update.clone(), &Settings::default()).unwrap();

        assert_eq!(updated.board, update.board);
        assert_eq!(updated.defaults, update.defaults);
    }

    #[test]
    fn apply_project_update_rejects_unknown_status_in_board_statuses() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.board = ProjectBoard {
            statuses: vec!["backlog".to_string(), "on-hold".to_string()],
            default_status: None,
        };

        let result = apply_project_update(&mut projects, 0, update, &Settings::default());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("on-hold"));
    }

    #[test]
    fn apply_project_update_rejects_unknown_board_default_status() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.board = ProjectBoard {
            statuses: vec!["backlog".to_string()],
            default_status: Some("on-hold".to_string()),
        };

        let result = apply_project_update(&mut projects, 0, update, &Settings::default());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("on-hold"));
    }

    #[test]
    fn apply_project_update_rejects_unknown_defaults_status() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.defaults.status = Some("on-hold".to_string());

        let result = apply_project_update(&mut projects, 0, update, &Settings::default());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("on-hold"));
    }

    #[test]
    fn apply_project_update_rejects_unknown_defaults_priority() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.defaults.priority = Some("urgent".to_string());

        let result = apply_project_update(&mut projects, 0, update, &Settings::default());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("urgent"));
    }

    #[test]
    fn resolve_default_priority_uses_settings_defaults_when_valid() {
        let mut settings = Settings::default();
        settings.defaults.priority = Some("low".to_string());

        assert_eq!(resolve_default_priority(&settings), "low");
    }

    #[test]
    fn resolve_default_priority_falls_back_to_rank_one_when_default_is_invalid() {
        let mut settings = Settings::default();
        settings.defaults.priority = Some("urgent".to_string());

        // Settings::default()'s "high" level has rank 1.
        assert_eq!(resolve_default_priority(&settings), "high");
    }

    #[test]
    fn resolve_default_priority_falls_back_to_rank_one_when_default_is_none() {
        let mut settings = Settings::default();
        settings.defaults.priority = None;

        assert_eq!(resolve_default_priority(&settings), "high");
    }

    #[test]
    fn resolve_default_priority_falls_back_to_medium_when_no_priorities_defined() {
        let settings = Settings {
            priorities: Vec::new(),
            statuses: Vec::new(),
            defaults: TaskDefaults::default(),
        };

        assert_eq!(resolve_default_priority(&settings), "medium");
    }

    #[test]
    fn resolve_default_priority_picks_the_lowest_rank_among_custom_levels() {
        let settings = Settings {
            priorities: vec![
                PriorityLevel {
                    id: "later".to_string(),
                    label: "Later".to_string(),
                    color: "oklch(58% 0.012 60)".to_string(),
                    rank: 2,
                },
                PriorityLevel {
                    id: "now".to_string(),
                    label: "Now".to_string(),
                    color: "oklch(58% 0.012 60)".to_string(),
                    rank: 1,
                },
            ],
            statuses: Vec::new(),
            defaults: TaskDefaults::default(),
        };

        assert_eq!(resolve_default_priority(&settings), "now");
    }

    #[test]
    fn tally_priorities_counts_tasks_by_priority_id() {
        let mut high_task = Task::new("Urgent".to_string());
        high_task.priority = "high".to_string();
        let mut low_task = Task::new("Someday".to_string());
        low_task.priority = "low".to_string();
        let mut other_high_task = Task::new("Also urgent".to_string());
        other_high_task.priority = "high".to_string();

        let counts = tally_priorities(&[high_task, low_task, other_high_task]);

        assert_eq!(counts.get("high"), Some(&2));
        assert_eq!(counts.get("low"), Some(&1));
        assert_eq!(counts.get("medium"), None);
    }

    #[test]
    fn tally_priorities_returns_empty_map_for_no_tasks() {
        let counts = tally_priorities(&[]);

        assert!(counts.is_empty());
    }

    #[test]
    fn resolve_default_status_uses_project_board_default_when_valid() {
        let settings = Settings::default();
        let board = ProjectBoard {
            statuses: vec!["backlog".to_string(), "done".to_string()],
            default_status: Some("done".to_string()),
        };

        assert_eq!(resolve_default_status(&settings, Some(&board)), "done");
    }

    #[test]
    fn resolve_default_status_prefers_project_board_default_over_settings_defaults() {
        let mut settings = Settings::default();
        settings.defaults.status = Some("do".to_string());
        let board = ProjectBoard {
            statuses: vec!["done".to_string()],
            default_status: Some("done".to_string()),
        };

        assert_eq!(resolve_default_status(&settings, Some(&board)), "done");
    }

    #[test]
    fn resolve_default_status_falls_back_to_settings_defaults_when_board_default_is_invalid() {
        let mut settings = Settings::default();
        settings.defaults.status = Some("do".to_string());
        let board = ProjectBoard {
            statuses: vec!["backlog".to_string()],
            default_status: Some("nonexistent".to_string()),
        };

        assert_eq!(resolve_default_status(&settings, Some(&board)), "do");
    }

    #[test]
    fn resolve_default_status_falls_back_to_lowest_order_when_settings_default_is_invalid() {
        let mut settings = Settings::default();
        settings.defaults.status = Some("nonexistent".to_string());

        // Settings::default()'s "backlog" status has order 1.
        assert_eq!(resolve_default_status(&settings, None), "backlog");
    }

    #[test]
    fn resolve_default_status_falls_back_to_backlog_when_no_statuses_defined() {
        let settings = Settings {
            priorities: Vec::new(),
            statuses: Vec::new(),
            defaults: TaskDefaults::default(),
        };

        assert_eq!(resolve_default_status(&settings, None), "backlog");
    }

    #[test]
    fn resolve_default_status_picks_lowest_order_among_custom_statuses() {
        let settings = Settings {
            priorities: Vec::new(),
            statuses: vec![
                StatusDefinition {
                    id: "later".to_string(),
                    label: "Later".to_string(),
                    order: 2,
                    color: "oklch(58% 0.012 60)".to_string(),
                },
                StatusDefinition {
                    id: "now".to_string(),
                    label: "Now".to_string(),
                    order: 1,
                    color: "oklch(58% 0.012 60)".to_string(),
                },
            ],
            defaults: TaskDefaults::default(),
        };

        assert_eq!(resolve_default_status(&settings, None), "now");
    }

    #[test]
    fn find_project_board_matches_case_insensitively() {
        let mut project =
            Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        project.board.default_status = Some("done".to_string());
        let projects = vec![project];

        let board = find_project_board(&projects, "HOMEWORK").expect("should find a match");

        assert_eq!(board.default_status, Some("done".to_string()));
    }

    #[test]
    fn find_project_board_returns_none_when_no_project_matches() {
        let projects = vec![Project::new(
            "Homework".to_string(),
            DEFAULT_PROJECT_COLOR.to_string(),
            1,
        )];

        assert!(find_project_board(&projects, "Inbox").is_none());
    }

    #[test]
    fn tally_statuses_counts_tasks_by_status_id() {
        let mut backlog_task = Task::new("Plan".to_string());
        backlog_task.status = "backlog".to_string();
        let mut done_task = Task::new("Done thing".to_string());
        done_task.status = "done".to_string();
        let mut other_backlog_task = Task::new("Also plan".to_string());
        other_backlog_task.status = "backlog".to_string();

        let counts = tally_statuses(&[backlog_task, done_task, other_backlog_task]);

        assert_eq!(counts.get("backlog"), Some(&2));
        assert_eq!(counts.get("done"), Some(&1));
        assert_eq!(counts.get("do"), None);
    }

    #[test]
    fn tally_statuses_returns_empty_map_for_no_tasks() {
        let counts = tally_statuses(&[]);

        assert!(counts.is_empty());
    }

    #[test]
    fn validate_settings_against_projects_accepts_empty_projects() {
        assert!(validate_settings_against_projects(&Settings::default(), &[]).is_ok());
    }

    #[test]
    fn validate_settings_against_projects_accepts_a_project_with_an_uncustomized_board() {
        let projects = vec![Project::new(
            "Inbox".to_string(),
            DEFAULT_PROJECT_COLOR.to_string(),
            1,
        )];

        assert!(validate_settings_against_projects(&Settings::default(), &projects).is_ok());
    }

    #[test]
    fn validate_settings_against_projects_rejects_removing_a_status_in_board_statuses() {
        let mut project =
            Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        project.board.statuses = vec!["backlog".to_string(), "blocked".to_string()];
        let projects = vec![project];

        let mut settings = Settings::default();
        settings.statuses.retain(|status| status.id != "blocked");

        let err = validate_settings_against_projects(&settings, &projects).unwrap_err();
        assert!(err.contains("blocked"));
        assert!(err.contains("Homework"));
    }

    #[test]
    fn validate_settings_against_projects_rejects_removing_a_boards_default_status() {
        let mut project =
            Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        project.board.default_status = Some("blocked".to_string());
        let projects = vec![project];

        let mut settings = Settings::default();
        settings.statuses.retain(|status| status.id != "blocked");

        let err = validate_settings_against_projects(&settings, &projects).unwrap_err();
        assert!(err.contains("blocked"));
    }

    #[test]
    fn validate_settings_against_projects_rejects_removing_a_defaults_status_override() {
        let mut project =
            Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        project.defaults.status = Some("blocked".to_string());
        let projects = vec![project];

        let mut settings = Settings::default();
        settings.statuses.retain(|status| status.id != "blocked");

        let err = validate_settings_against_projects(&settings, &projects).unwrap_err();
        assert!(err.contains("blocked"));
    }

    #[test]
    fn validate_settings_against_projects_rejects_removing_a_defaults_priority_override() {
        let mut project =
            Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        project.defaults.priority = Some("low".to_string());
        let projects = vec![project];

        let mut settings = Settings::default();
        settings.priorities.retain(|priority| priority.id != "low");

        let err = validate_settings_against_projects(&settings, &projects).unwrap_err();
        assert!(err.contains("low"));
    }
}
