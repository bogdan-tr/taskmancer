use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::project::{Project, DEFAULT_PROJECT_COLOR};
use crate::project_storage;
use crate::project_tree::{self, would_create_cycle};
use crate::recurrence::{anchor_matches_frequency, occurrence_dates_in_range, resolve_due_rule};
use crate::series::{validate_series, DueRule, RecurrenceFrequency, Series};
use crate::series_storage;
use crate::settings::{
    resolve_due_relative_date, resolve_scheduled_relative_date, validate_due_relative_date_code,
    validate_ink_mode, validate_lightness, validate_priority_id,
    validate_scheduled_relative_date_code, validate_settings, validate_status_id, Settings,
    TaskDefaults,
};
use crate::settings_storage;
use crate::storage;
use crate::task::Task;

/// How many days into the future a newly created series generates
/// occurrences immediately, before any scroll-triggered extension —
/// the user's chosen baseline window.
const RECURRENCE_BASELINE_LOOKAHEAD_DAYS: i64 = 60;

/// Shared application state holding the directory where task markdown
/// files are stored, the directory where archived task markdown files are
/// moved (see [`finish_day`] and [`delete_project`]), the file where project
/// metadata is stored, the file where global settings are stored, and the
/// file where recurrence series are stored.
///
/// `projects_lock` serializes the read-modify-write cycles in
/// [`list_projects`] (which may backfill), [`create_project`],
/// [`update_project`], and [`delete_project`] so concurrent commands can't
/// read a stale project list and overwrite each other's changes.
/// `series_lock` does the same for series read-modify-write cycles.
pub struct AppState {
    pub tasks_dir: PathBuf,
    pub archive_dir: PathBuf,
    pub projects_file: PathBuf,
    pub settings_file: PathBuf,
    pub series_file: PathBuf,
    pub projects_lock: Mutex<()>,
    pub series_lock: Mutex<()>,
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
/// requested. Checked in order: each project in `project_chain` (the task's
/// own project, then its ancestors nearest-first — see
/// [`crate::project_tree::self_and_ancestors`])'s `board.default_status` (if
/// it names a currently-defined status), `settings.defaults.status` (if it
/// names a currently-defined status), the status with the lowest `order`
/// (order 1 sorts first), otherwise `"backlog"` if no statuses are defined
/// at all.
fn resolve_default_status(settings: &Settings, project_chain: &[&Project]) -> String {
    for project in project_chain {
        if let Some(default_id) = &project.board.default_status {
            if validate_status_id(settings, default_id).is_ok() {
                return default_id.clone();
            }
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

/// Looks up `project_id` among `projects`, or `None` if no project with
/// that id exists (e.g. it was deleted after the frontend last refreshed
/// its project list).
fn find_project<'a>(projects: &'a [Project], project_id: &str) -> Option<&'a Project> {
    projects.iter().find(|p| p.id == project_id)
}

/// Resolves the project id a task should be saved with: `project_id` if
/// it's `Some`, otherwise `settings.default_project_id`. Ensures a task can
/// never be created or updated without a project id.
fn resolve_project_id(project_id: Option<String>, settings: &Settings) -> String {
    project_id.unwrap_or_else(|| settings.default_project_id.clone())
}

/// Merges `defaults` into `explicit`, appending any default tag not already
/// present so quick-add tags always come first and no tag is duplicated.
fn merge_tags(explicit: Vec<String>, defaults: Vec<String>) -> Vec<String> {
    let mut merged = explicit;
    for tag in defaults {
        if !merged.contains(&tag) {
            merged.push(tag);
        }
    }
    merged
}

/// Returns the default tags that should be merged into a newly-created
/// task's explicit tags: the nearest project in `project_chain` (the task's
/// own project, then its ancestors nearest-first) with a non-empty
/// `defaults.tags`, otherwise the global default tags.
fn effective_default_tags(global: &TaskDefaults, project_chain: &[&Project]) -> Vec<String> {
    project_chain
        .iter()
        .map(|p| &p.defaults.tags)
        .find(|tags| !tags.is_empty())
        .cloned()
        .unwrap_or_else(|| global.tags.clone())
}

/// Resolves the effective `estimated_minutes` default for a new task that
/// doesn't specify its own estimate: the nearest project in `project_chain`
/// (the task's own project, then its ancestors nearest-first) with an
/// override, otherwise the global default, otherwise `None` (no estimate).
fn effective_default_estimated_minutes(
    global: &TaskDefaults,
    project_chain: &[&Project],
) -> Option<u32> {
    project_chain
        .iter()
        .find_map(|p| p.defaults.estimated_minutes)
        .or(global.estimated_minutes)
}

/// Resolves a [`SCHEDULED_RELATIVE_DATE_CODES`](crate::settings::SCHEDULED_RELATIVE_DATE_CODES)
/// `code` (e.g. `"tomorrow"`) to an absolute `YYYY-MM-DD` date string relative
/// to `today`. Returns `None` if `code` is `None` or isn't a recognized
/// scheduled-date code.
fn resolve_default_scheduled_date(
    code: Option<&String>,
    today: chrono::NaiveDate,
) -> Option<String> {
    code.and_then(|c| resolve_scheduled_relative_date(c, today))
        .map(|date| date.format("%Y-%m-%d").to_string())
}

/// Resolves a [`DUE_RELATIVE_DATE_CODES`](crate::settings::DUE_RELATIVE_DATE_CODES)
/// `code` (e.g. `"next_day"`) to an absolute `YYYY-MM-DD` date string relative
/// to `scheduled`. Returns `None` if `code` is `None`, is `"none"` (never
/// due), or isn't a recognized due-date code.
fn resolve_default_due_date(code: Option<&String>, scheduled: chrono::NaiveDate) -> Option<String> {
    code.and_then(|c| resolve_due_relative_date(c, scheduled))
        .map(|date| date.format("%Y-%m-%d").to_string())
}

/// Resolves the tags, due date, and scheduled date for a newly-created task
/// by combining explicit quick-add values with the global and project-level
/// defaults. Explicit `scheduled` always wins; otherwise the project's
/// scheduled-default code is used if set, falling back to the global default,
/// falling back to `today` if neither resolves. Explicit `due` always wins,
/// except for the `"none"` sentinel (meaning "never due"), which resolves to
/// `None`; otherwise the project's due-default code is used if set, falling
/// back to the global default, resolved relative to the resolved `scheduled`
/// date (not `today`). Tags are merged additively: any default tag not
/// already present in the explicit tags is appended.
fn resolve_creation_defaults(
    settings: &Settings,
    project_chain: &[&Project],
    today: chrono::NaiveDate,
    tags: Option<Vec<String>>,
    due: Option<String>,
    scheduled: Option<String>,
) -> (Vec<String>, Option<String>, String) {
    let due_code = project_chain
        .iter()
        .find_map(|p| p.defaults.due.as_ref())
        .or(settings.defaults.due.as_ref());
    let scheduled_code = project_chain
        .iter()
        .find_map(|p| p.defaults.scheduled.as_ref())
        .or(settings.defaults.scheduled.as_ref());

    let resolved_scheduled = scheduled
        .or_else(|| resolve_default_scheduled_date(scheduled_code, today))
        .unwrap_or_else(|| today.format("%Y-%m-%d").to_string());

    let resolved_due = match due.as_deref() {
        Some("none") => None,
        Some(_) => due,
        None => {
            let scheduled_date =
                chrono::NaiveDate::parse_from_str(&resolved_scheduled, "%Y-%m-%d").ok();
            scheduled_date.and_then(|date| resolve_default_due_date(due_code, date))
        }
    };

    let default_tags = effective_default_tags(&settings.defaults, project_chain);
    let final_tags = merge_tags(tags.unwrap_or_default(), default_tags);

    (final_tags, resolved_due, resolved_scheduled)
}

/// Applies optional overrides parsed from quick-add syntax onto a freshly
/// created task. `project`/`tags`/`priority`/`due`/`estimated_minutes` left
/// as `None` keep `Task::new`'s defaults (a `None` due means "never due", a
/// `None` estimate means no estimate). `scheduled` is always set: every task
/// must have a scheduled date.
#[allow(clippy::too_many_arguments)]
fn apply_create_overrides(
    task: &mut Task,
    project_id: Option<String>,
    tags: Option<Vec<String>>,
    priority: Option<String>,
    due: Option<String>,
    scheduled: String,
    estimated_minutes: Option<u32>,
) {
    if let Some(project_id) = project_id {
        task.project_id = Some(project_id);
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
    task.scheduled = Some(scheduled);
    if let Some(estimated_minutes) = estimated_minutes {
        task.estimated_minutes = Some(estimated_minutes);
    }
}

/// Creates and saves a new task. A missing `project_id` falls back to
/// `settings.default_project_id` (see [`resolve_project_id`]), so a task can
/// never be created without a project.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_task(
    state: State<AppState>,
    title: String,
    project_id: Option<String>,
    tags: Option<Vec<String>>,
    priority: Option<String>,
    status: Option<String>,
    due: Option<String>,
    scheduled: Option<String>,
    estimated_minutes: Option<u32>,
) -> Result<Task, String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("task title must not be empty".to_string());
    }
    validate_due_creation_field(&due)?;
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

    let resolved_project_id = resolve_project_id(project_id, &settings);

    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let matched_project = find_project(&projects, &resolved_project_id);
    let project_chain: Vec<&Project> = matched_project
        .map(|p| project_tree::self_and_ancestors(&projects, &p.id))
        .unwrap_or_default();
    let status = match status {
        Some(id) => {
            validate_status_id(&settings, &id)?;
            id
        }
        None => resolve_default_status(&settings, &project_chain),
    };

    // Use the user's local date so relative-date defaults (e.g. "today",
    // "tomorrow") match the day they see on their device's calendar,
    // regardless of the machine's UTC offset.
    let today = chrono::Local::now().date_naive();
    let (final_tags, resolved_due, resolved_scheduled) =
        resolve_creation_defaults(&settings, &project_chain, today, tags, due, scheduled);
    let resolved_estimated_minutes = estimated_minutes
        .or_else(|| effective_default_estimated_minutes(&settings.defaults, &project_chain));

    let mut task = Task::new(title);
    task.status = status;
    apply_create_overrides(
        &mut task,
        Some(resolved_project_id),
        Some(final_tags),
        Some(priority),
        resolved_due,
        resolved_scheduled,
        resolved_estimated_minutes,
    );

    storage::save_task(&state.tasks_dir, &task).map_err(|e| e.to_string())?;
    Ok(task)
}

/// Builds the single occurrence task for `date` from `series`'s template,
/// resolving status the same way any freshly created task does (never
/// copied from the template — see `Series`'s own doc comment for why) and
/// due relative to `date` itself via `series.due_rule`.
fn build_series_occurrence(
    series: &Series,
    settings: &Settings,
    project_chain: &[&Project],
    date: chrono::NaiveDate,
) -> Task {
    let mut task = Task::new(series.title.clone());
    task.status = resolve_default_status(settings, project_chain);
    task.project_id = series.project_id.clone();
    task.tags = series.tags.clone();
    task.priority = series.priority.clone();
    task.estimated_minutes = series.estimated_minutes;
    task.scheduled = Some(date.format("%Y-%m-%d").to_string());
    task.due = resolve_due_rule(&series.due_rule, date).map(|d| d.format("%Y-%m-%d").to_string());
    task.series_id = Some(series.id.clone());
    task
}

/// Generates and saves any new occurrences for `series` in
/// `(series.generated_until, through]` (clamped to the series' own
/// `end_date`, if any), then advances `series.generated_until` in place to
/// the highest date actually processed — the last occurrence generated, or
/// the clamped horizon itself if none were (e.g. the window landed entirely
/// in a gap, or past `end_date`), so a later call never re-examines a
/// range that's already been fully accounted for. Returns the newly
/// created tasks. Does not persist `series` itself — the caller does that
/// alongside whatever else it's saving in the same operation.
fn generate_series_occurrences(
    tasks_dir: &Path,
    settings: &Settings,
    project_chain: &[&Project],
    series: &mut Series,
    through: chrono::NaiveDate,
) -> Result<Vec<Task>, String> {
    let generated_until = chrono::NaiveDate::parse_from_str(&series.generated_until, "%Y-%m-%d")
        .map_err(|_| "series has an invalid generated_until date".to_string())?;

    let dates = occurrence_dates_in_range(series, generated_until, through);
    let mut created = Vec::with_capacity(dates.len());
    for date in &dates {
        let task = build_series_occurrence(series, settings, project_chain, *date);
        storage::save_task(tasks_dir, &task).map_err(|e| e.to_string())?;
        created.push(task);
    }

    let effective_horizon = series
        .end_date
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .map(|end| through.min(end))
        .unwrap_or(through);
    // Never move the watermark backward: a `through` at or before what's
    // already generated (e.g. Week view's near-term initial call, made
    // after a wider baseline/extension already advanced it further) must
    // be a complete no-op, not a regression that would make the next call
    // think the already-generated range still needs generating.
    let new_watermark = dates
        .last()
        .copied()
        .unwrap_or(effective_horizon)
        .max(generated_until);
    series.generated_until = new_watermark.format("%Y-%m-%d").to_string();

    Ok(created)
}

/// Creates a recurring task: a `Series` template + rule, with occurrences
/// immediately generated for the next [`RECURRENCE_BASELINE_LOOKAHEAD_DAYS`]
/// days (see [`ensure_occurrences_until`] for extending further as the user
/// scrolls), plus — *only if the scheduled date itself satisfies the
/// recurrence rule* (see [`anchor_matches_frequency`]) — an occurrence
/// created directly on that date, the same way [`create_task`] creates any
/// task. When the scheduled date doesn't satisfy the rule (e.g. scheduling
/// a Mon/Tue/Wed series for a Saturday), no occurrence is created on it at
/// all — the series' real first occurrence is simply the first one the
/// normal generation step produces, which already correctly searches for
/// the next date matching the rule. Returns every task created, in date
/// order.
///
/// `due_rule`, if given, is the series' due rule exactly as the frontend
/// derived it from whatever due phrase was typed (see `naturalLanguage.ts`)
/// — an explicit override the same way `due`/`priority`/etc. already are,
/// taking precedence over the configured project/global default due code
/// when set. This is what fixes the original inconsistency bug: the first
/// occurrence and every later one now derive their due date from the same
/// rule, instead of the first using whatever `due` resolved to and every
/// other occurrence silently falling back to an unrelated default.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_recurring_task(
    state: State<AppState>,
    title: String,
    project_id: Option<String>,
    tags: Option<Vec<String>>,
    priority: Option<String>,
    status: Option<String>,
    due: Option<String>,
    scheduled: Option<String>,
    estimated_minutes: Option<u32>,
    frequency: RecurrenceFrequency,
    end_date: Option<String>,
    due_rule: Option<DueRule>,
) -> Result<Vec<Task>, String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("task title must not be empty".to_string());
    }
    validate_due_creation_field(&due)?;
    validate_date_field(&scheduled, "scheduled")?;
    validate_date_field(&end_date, "end date")?;

    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    let priority = match priority {
        Some(id) => {
            validate_priority_id(&settings, &id)?;
            id
        }
        None => resolve_default_priority(&settings),
    };

    let resolved_project_id = resolve_project_id(project_id, &settings);

    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let matched_project = find_project(&projects, &resolved_project_id);
    let project_chain: Vec<&Project> = matched_project
        .map(|p| project_tree::self_and_ancestors(&projects, &p.id))
        .unwrap_or_default();
    let status = match status {
        Some(id) => {
            validate_status_id(&settings, &id)?;
            id
        }
        None => resolve_default_status(&settings, &project_chain),
    };

    let today = chrono::Local::now().date_naive();
    // `resolve_creation_defaults`'s own due resolution is discarded here —
    // it only knows about the literal `due` string, not `due_rule`, and
    // would otherwise resolve the first occurrence's due from the
    // configured default code regardless of what `due_rule` says, while
    // every later occurrence correctly uses `series.due_rule` (see
    // `build_series_occurrence`) — the exact inconsistency this whole
    // feature exists to fix, just one occurrence later than before.
    let (final_tags, _due_resolution_unused_for_recurring_tasks, resolved_scheduled) =
        resolve_creation_defaults(&settings, &project_chain, today, tags, due, scheduled);
    let resolved_estimated_minutes = estimated_minutes
        .or_else(|| effective_default_estimated_minutes(&settings.defaults, &project_chain));
    let series_due_rule = due_rule.unwrap_or_else(|| {
        project_chain
            .iter()
            .find_map(|p| p.defaults.due.as_ref())
            .or(settings.defaults.due.as_ref())
            .map(|code| DueRule::DefaultCode { code: code.clone() })
            .unwrap_or(DueRule::Never)
    });

    let anchor = chrono::NaiveDate::parse_from_str(&resolved_scheduled, "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
    let resolved_due =
        resolve_due_rule(&series_due_rule, anchor).map(|d| d.format("%Y-%m-%d").to_string());

    if let Some(end) = &end_date {
        let end_date = chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d")
            .map_err(|_| "end date must be a valid date in YYYY-MM-DD format".to_string())?;
        if end_date < anchor {
            return Err("end date must be on or after the scheduled date".to_string());
        }
    }

    // Checked before `frequency` is moved into `Series::new` below — see
    // `anchor_matches_frequency`'s own doc comment for why a scheduled
    // date that doesn't itself satisfy the rule (e.g. a Mon/Tue/Wed series
    // scheduled for a Saturday) must not get a forced, mismatched
    // occurrence created directly on it.
    let anchor_matches = anchor_matches_frequency(&frequency, anchor);

    let mut series = Series::new(
        frequency,
        resolved_scheduled.clone(),
        end_date,
        series_due_rule,
        title.clone(),
        Some(resolved_project_id.clone()),
        priority.clone(),
        final_tags.clone(),
        resolved_estimated_minutes,
        String::new(),
    );
    validate_series(&series)?;

    let _guard = state
        .series_lock
        .lock()
        .map_err(|_| "series lock poisoned".to_string())?;

    let first_task = if anchor_matches {
        let mut first_task = Task::new(title);
        first_task.status = status.clone();
        apply_create_overrides(
            &mut first_task,
            Some(resolved_project_id),
            Some(final_tags),
            Some(priority),
            resolved_due,
            resolved_scheduled.clone(),
            resolved_estimated_minutes,
        );
        first_task.series_id = Some(series.id.clone());
        storage::save_task(&state.tasks_dir, &first_task).map_err(|e| e.to_string())?;
        Some(first_task)
    } else {
        None
    };

    let horizon = anchor + chrono::Duration::days(RECURRENCE_BASELINE_LOOKAHEAD_DAYS);
    let mut created = generate_series_occurrences(
        &state.tasks_dir,
        &settings,
        &project_chain,
        &mut series,
        horizon,
    )?;

    let mut all_series =
        series_storage::list_series(&state.series_file).map_err(|e| e.to_string())?;
    all_series.push(series);
    series_storage::save_series(&state.series_file, &all_series).map_err(|e| e.to_string())?;

    if let Some(first_task) = first_task {
        created.insert(0, first_task);
    }
    Ok(created)
}

/// Extends a series' generated occurrences up through `through`
/// (`YYYY-MM-DD`) — called as the user scrolls a calendar/week view further
/// into the future than what's already been generated. A no-op (returns no
/// tasks) if the series is no longer active (recurrence was removed from
/// it) or `through` is at or before what's already generated.
#[tauri::command]
pub fn ensure_occurrences_until(
    state: State<AppState>,
    series_id: String,
    through: String,
) -> Result<Vec<Task>, String> {
    let through_date = chrono::NaiveDate::parse_from_str(&through, "%Y-%m-%d")
        .map_err(|_| "through must be a valid date in YYYY-MM-DD format".to_string())?;

    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    let _guard = state
        .series_lock
        .lock()
        .map_err(|_| "series lock poisoned".to_string())?;

    let mut all_series =
        series_storage::list_series(&state.series_file).map_err(|e| e.to_string())?;
    let Some(series) = all_series.iter_mut().find(|s| s.id == series_id) else {
        return Err(format!("series '{series_id}' not found"));
    };
    if !series.active {
        return Ok(Vec::new());
    }

    let project_chain: Vec<&Project> = series
        .project_id
        .as_deref()
        .and_then(|id| find_project(&projects, id))
        .map(|p| project_tree::self_and_ancestors(&projects, &p.id))
        .unwrap_or_default();

    let created = generate_series_occurrences(
        &state.tasks_dir,
        &settings,
        &project_chain,
        series,
        through_date,
    )?;

    series_storage::save_series(&state.series_file, &all_series).map_err(|e| e.to_string())?;
    Ok(created)
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

/// Rejects `Some(value)` values for [`create_task`]'s `due` parameter that
/// aren't `YYYY-MM-DD` or the `"none"` sentinel (meaning "never due" — see
/// [`resolve_creation_defaults`]). `None` is always valid (apply the
/// configured due-date default).
fn validate_due_creation_field(value: &Option<String>) -> Result<(), String> {
    match value.as_deref() {
        None | Some("none") => Ok(()),
        Some(date) if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() => {
            Err("due must be a valid date in YYYY-MM-DD format, or 'none'".to_string())
        }
        _ => Ok(()),
    }
}

/// Applies `update_task`'s editable fields from `task` onto `existing`:
/// `title` (already trimmed by the caller), `project_id` (the already
/// project-fallback-resolved id — see [`resolve_project_id`] — rather
/// than `task.project_id` directly), `tags`, `priority`, `status`, `due`,
/// `scheduled`, `estimated_minutes`, and `notes`. `status` is a normal
/// editable field on this path: `TaskEditDialog.svelte`'s status dropdown
/// (used from the week view's "Edit" button) saves through `update_task`,
/// not just the Kanban board's dedicated `reorder_task` command, so it must
/// take effect here too. `existing.id`, `.created`, `.depends_on`, and
/// `.tracked_minutes` are left untouched — the request payload can never
/// overwrite them or redirect the write to a different task's file;
/// `tracked_minutes` specifically has no user-facing edit control at all
/// (only the future time-tracking infrastructure writes it).
fn apply_task_update(existing: &mut Task, title: String, project_id: String, task: Task) {
    existing.title = title;
    existing.project_id = Some(project_id);
    existing.tags = task.tags;
    existing.priority = task.priority;
    existing.status = task.status;
    existing.due = task.due;
    existing.scheduled = task.scheduled;
    existing.estimated_minutes = task.estimated_minutes;
    existing.notes = task.notes;
}

/// If `container_id` names a project still present in `projects`, returns
/// an updated copy of it with `name`/`parent_id` synced to `task_title`/
/// `task_project_id` — the rename/move-sync a subtask container is
/// supposed to track (see the Subtasks design spec: renaming or moving a
/// task keeps its container's name and tree position matching). Called
/// unconditionally on every `update_task` for a task with a container
/// rather than only when the title/project actually changed — resyncing
/// to the current values is idempotent, and simpler than threading
/// before/after comparisons through `apply_task_update`'s call site.
/// Returns `None` if the container can't be found (already deleted).
fn synced_subtask_container(
    projects: &[Project],
    container_id: &str,
    task_title: &str,
    task_project_id: &str,
) -> Option<Project> {
    let container = projects.iter().find(|p| p.id == container_id)?;
    Some(Project {
        name: task_title.to_string(),
        parent_id: Some(task_project_id.to_string()),
        ..container.clone()
    })
}

/// Updates the editable fields of an existing task (title, project, tags,
/// priority, status, due/scheduled dates, estimated time, notes — see
/// [`apply_task_update`]). The task's `id`, `created`, `depends_on`, and
/// `tracked_minutes` are loaded from disk and preserved, so a stale or
/// malformed `task` payload from the frontend cannot corrupt these fields or
/// redirect the write to a different task's file.
///
/// A missing `task.project_id` falls back to `settings.default_project_id`
/// (see [`resolve_project_id`]), so a task can never be saved without a
/// project.
///
/// If the task owns a subtask container (`subtask_project_id` is `Some`),
/// also syncs that container's name/parent to the task's (possibly just
/// updated) title/project — see [`synced_subtask_container`].
#[tauri::command]
pub fn update_task(state: State<AppState>, task: Task) -> Result<Task, String> {
    let title = task.title.trim().to_string();
    if title.is_empty() {
        return Err("task title must not be empty".to_string());
    }
    validate_date_field(&task.due, "due")?;
    validate_date_field(&task.scheduled, "scheduled")?;
    if task.scheduled.is_none() {
        return Err("scheduled date must not be empty".to_string());
    }

    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    validate_priority_id(&settings, &task.priority)?;
    validate_status_id(&settings, &task.status)?;

    let resolved_project_id = resolve_project_id(task.project_id.clone(), &settings);
    let mut existing = storage::load_task(&state.tasks_dir, &task.id).map_err(|e| e.to_string())?;
    apply_task_update(&mut existing, title, resolved_project_id, task);

    if let Some(container_id) = &existing.subtask_project_id {
        let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
        let mut projects =
            project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
        let project_id = existing.project_id.as_deref().unwrap_or_default();
        if let Some(updated) =
            synced_subtask_container(&projects, container_id, &existing.title, project_id)
        {
            if let Some(index) = projects.iter().position(|p| &p.id == container_id) {
                projects[index] = updated;
                project_storage::save_projects(&state.projects_file, &projects)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    storage::update_task(&state.tasks_dir, &existing).map_err(|e| e.to_string())?;
    Ok(existing)
}

/// Every current subtask of `task_id`'s container, each with `project_id`
/// changed to `target_project_id`, plus that container's own id — what
/// [`delete_subtask_container`] needs to move the subtasks out before
/// removing the now-unused container. Unlike
/// [`subtasks_and_container_to_delete`] (which deletes the subtasks
/// outright for [`delete_task`]'s cascade), this keeps every subtask
/// around as an ordinary task, just relocated. Returns `(vec![], None)`
/// if `task_id` doesn't name a task in `tasks`, or names one with no
/// container.
fn subtasks_to_reassign_and_container_to_remove(
    tasks: &[Task],
    task_id: &str,
    target_project_id: &str,
) -> (Vec<Task>, Option<String>) {
    let Some(target) = tasks.iter().find(|t| t.id == task_id) else {
        return (Vec::new(), None);
    };
    let Some(container_id) = &target.subtask_project_id else {
        return (Vec::new(), None);
    };

    let reassigned: Vec<Task> = tasks
        .iter()
        .filter(|t| t.project_id.as_deref() == Some(container_id.as_str()))
        .map(|t| Task {
            project_id: Some(target_project_id.to_string()),
            ..t.clone()
        })
        .collect();

    (reassigned, Some(container_id.clone()))
}

/// The subtasks and container project that should be deleted alongside
/// `task_id`, if it owns a subtask container — every task whose
/// `project_id` equals `task_id`'s `subtask_project_id`, plus that
/// container project itself. Mirrors `projects_to_delete`'s shape for the
/// analogous project-cascade case (the target task itself is *not*
/// included here — the caller's own single-file delete handles that).
/// Returns `(vec![], None)` if `task_id` doesn't name a task in `tasks`,
/// or names one with no container.
fn subtasks_and_container_to_delete(
    tasks: &[Task],
    projects: &[Project],
    task_id: &str,
) -> (Vec<Task>, Option<Project>) {
    let Some(target) = tasks.iter().find(|t| t.id == task_id) else {
        return (Vec::new(), None);
    };
    let Some(container_id) = &target.subtask_project_id else {
        return (Vec::new(), None);
    };

    let subtasks: Vec<Task> = tasks
        .iter()
        .filter(|t| t.project_id.as_deref() == Some(container_id.as_str()))
        .cloned()
        .collect();
    let container = projects.iter().find(|p| &p.id == container_id).cloned();

    (subtasks, container)
}

/// If `deleted_task_project_id` names a subtask container whose owning
/// task can be found in `tasks`, and no task *other than*
/// `deleted_task_id` still lives in that container, returns the owning
/// task — the empty-container-cleanup case from the Subtasks design spec
/// (deleting the last subtask clears the owner's pointer and removes the
/// now-empty container). Returns `None` if there's no owner at all, or
/// the container still has other subtasks in it.
fn owning_task_if_container_now_empty<'a>(
    tasks: &'a [Task],
    deleted_task_project_id: &str,
    deleted_task_id: &str,
) -> Option<&'a Task> {
    let owner = tasks
        .iter()
        .find(|t| t.subtask_project_id.as_deref() == Some(deleted_task_project_id))?;

    let any_remaining = tasks.iter().any(|t| {
        t.id != deleted_task_id && t.project_id.as_deref() == Some(deleted_task_project_id)
    });

    if any_remaining {
        None
    } else {
        Some(owner)
    }
}

/// Deletes a task. If it owns a subtask container, also deletes every
/// subtask in it and the container itself (see
/// [`subtasks_and_container_to_delete`]) — callers should confirm with the
/// user first, showing the exact subtask count, mirroring
/// `delete_project`'s cascade-delete confirmation. If the task being
/// deleted is itself the last remaining subtask in some *other* task's
/// container, that container is cleaned up too (see
/// [`owning_task_if_container_now_empty`]) — these two cases are
/// independent checks, not mutually exclusive branches, even though a
/// given task can't realistically be both in this app's data model.
#[tauri::command]
pub fn delete_task(state: State<AppState>, id: String) -> Result<(), String> {
    let target = storage::load_task(&state.tasks_dir, &id).map_err(|e| e.to_string())?;

    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    let (subtasks, owned_container) = subtasks_and_container_to_delete(&tasks, &projects, &id);
    for subtask in &subtasks {
        storage::delete_task(&state.tasks_dir, &subtask.id).map_err(|e| e.to_string())?;
    }

    let mut containers_to_remove: Vec<String> = owned_container.into_iter().map(|p| p.id).collect();

    if let Some(project_id) = &target.project_id {
        if let Some(owner) = owning_task_if_container_now_empty(&tasks, project_id, &id) {
            let mut owner = owner.clone();
            owner.subtask_project_id = None;
            storage::update_task(&state.tasks_dir, &owner).map_err(|e| e.to_string())?;
            containers_to_remove.push(project_id.clone());
        }
    }

    if !containers_to_remove.is_empty() {
        projects.retain(|p| !containers_to_remove.contains(&p.id));
        project_storage::save_projects(&state.projects_file, &projects)
            .map_err(|e| e.to_string())?;
    }

    storage::delete_task(&state.tasks_dir, &id).map_err(|e| e.to_string())
}

/// Forces every task in `tasks` to `done_status`, except ones already at
/// `cancelled_status` (left untouched — cancelled is a deliberate "won't
/// do," not an oversight to override). Used by [`delete_subtask_container`]
/// when the parent task is marked done: every outstanding subtask,
/// including every still-pending occurrence of a recurring one, is
/// completed alongside the parent, rather than coming back as an ordinary,
/// still-incomplete task once the container disappears.
fn force_done_except_cancelled(
    tasks: Vec<Task>,
    done_status: &str,
    cancelled_status: Option<&str>,
) -> Vec<Task> {
    tasks
        .into_iter()
        .map(|t| {
            if Some(t.status.as_str()) == cancelled_status {
                t
            } else {
                Task {
                    status: done_status.to_string(),
                    ..t
                }
            }
        })
        .collect()
}

/// Disbands `task_id`'s subtask container: every current subtask is marked
/// done (see [`force_done_except_cancelled`] — a recurring subtask's still-
/// pending occurrences are completed too, not left to come back as
/// ordinary incomplete tasks) and moved into `task_id`'s own project (see
/// [`subtasks_to_reassign_and_container_to_remove`]), and the now-unused
/// container project is removed — but unlike [`delete_task`]'s cascade, the
/// subtasks themselves are kept, not deleted. Used when the parent task is
/// marked done: the design spec wants the temporary "subproject" view gone
/// once its purpose is served, while the work recorded in each subtask
/// stays around as an ordinary, completed task. A no-op (returns the task
/// unchanged) if `task_id` has no container.
#[tauri::command]
pub fn delete_subtask_container(state: State<AppState>, task_id: String) -> Result<Task, String> {
    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let mut task = storage::load_task(&state.tasks_dir, &task_id).map_err(|e| e.to_string())?;

    let Some(container_id) = task.subtask_project_id.clone() else {
        return Ok(task);
    };

    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    let target_project_id = task
        .project_id
        .clone()
        .unwrap_or_else(|| settings.default_project_id.clone());

    let tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
    let (reassigned, _) =
        subtasks_to_reassign_and_container_to_remove(&tasks, &task_id, &target_project_id);
    let completed = force_done_except_cancelled(
        reassigned,
        &settings.done_status,
        settings.cancelled_status.as_deref(),
    );
    for subtask in &completed {
        storage::update_task(&state.tasks_dir, subtask).map_err(|e| e.to_string())?;
    }

    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    projects.retain(|p| p.id != container_id);
    project_storage::save_projects(&state.projects_file, &projects).map_err(|e| e.to_string())?;

    task.subtask_project_id = None;
    storage::update_task(&state.tasks_dir, &task).map_err(|e| e.to_string())?;

    Ok(task)
}

/// Copies the template fields a `Series` shares across every occurrence
/// (title, project_id, priority, tags, estimated time, notes — see `Series`'s
/// own doc comment for why `status`/`due`/`scheduled` are excluded) from
/// `task` onto `series`. Used by [`update_series_occurrence`]'s `"future"`
/// scope to fold an edit back into the template, so occurrences generated
/// *after* this edit also pick it up.
fn apply_series_template_update(series: &mut Series, task: &Task) {
    series.title = task.title.clone();
    series.project_id = task.project_id.clone();
    series.priority = task.priority.clone();
    series.tags = task.tags.clone();
    series.estimated_minutes = task.estimated_minutes;
    series.notes = task.notes.clone();
}

/// Applies the same template fields [`apply_series_template_update`] copies
/// onto a series, directly onto another already-generated occurrence —
/// used to propagate a `"future"`-scoped edit to every other occurrence at
/// or after the edited one's date, without touching that occurrence's own
/// status/due/scheduled.
fn apply_template_to_occurrence(occurrence: &mut Task, task: &Task) {
    occurrence.title = task.title.clone();
    occurrence.project_id = task.project_id.clone();
    occurrence.priority = task.priority.clone();
    occurrence.tags = task.tags.clone();
    occurrence.estimated_minutes = task.estimated_minutes;
    occurrence.notes = task.notes.clone();
}

/// Returns every task in `all_tasks` belonging to `series_id` (other than
/// `exclude_id`, if given) whose `scheduled` date is on or after `cutoff` —
/// the set a `"future"`-scoped edit ([`update_series_occurrence`])
/// propagates shared-field changes onto, or a `"future"`-scoped delete
/// ([`delete_series_occurrence`]) removes. The two callers pass a
/// different `exclude_id`: an edit already saves the occurrence that
/// triggered it separately, so it excludes that id to avoid double-applying
/// the update; a delete has no separate "already handled" task — deleting
/// "this and future" must delete that occurrence too — so it passes `None`.
/// A task with no `scheduled` date is never included, since there's no
/// date to compare against `cutoff`.
fn future_occurrences<'a>(
    all_tasks: &'a [Task],
    series_id: &str,
    exclude_id: Option<&str>,
    cutoff: &str,
) -> Vec<&'a Task> {
    all_tasks
        .iter()
        .filter(|task| exclude_id.is_none_or(|id| task.id != id))
        .filter(|task| task.series_id.as_deref() == Some(series_id))
        .filter(|task| {
            task.scheduled
                .as_deref()
                .is_some_and(|scheduled| scheduled >= cutoff)
        })
        .collect()
}

/// Updates an occurrence of a recurring task, with `scope` (`"this"` or
/// `"future"`) deciding how far the edit reaches:
/// - `"this"`: behaves exactly like [`update_task`], plus severs
///   `series_id` — the edited task becomes a fully standalone task from
///   this point on.
/// - `"future"`: saves the edit on this occurrence (keeping its
///   `series_id`), folds the same shared-field values into the series
///   template (so future-generated occurrences inherit them too — see
///   [`apply_series_template_update`]), and propagates them onto every
///   other already-generated occurrence whose `scheduled` date is on or
///   after this one's — never touching any of their own
///   status/due/scheduled, or any occurrence *before* this date (past
///   occurrences are left as an untouched historical record).
///
/// Returns every task that was changed (just the one, for `"this"`; the
/// edited occurrence plus every future one updated, for `"future"`).
#[tauri::command]
pub fn update_series_occurrence(
    state: State<AppState>,
    task: Task,
    scope: String,
) -> Result<Vec<Task>, String> {
    let title = task.title.trim().to_string();
    if title.is_empty() {
        return Err("task title must not be empty".to_string());
    }
    validate_date_field(&task.due, "due")?;
    validate_date_field(&task.scheduled, "scheduled")?;
    if task.scheduled.is_none() {
        return Err("scheduled date must not be empty".to_string());
    }
    if scope != "this" && scope != "future" {
        return Err("scope must be 'this' or 'future'".to_string());
    }

    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    validate_priority_id(&settings, &task.priority)?;
    validate_status_id(&settings, &task.status)?;

    let resolved_project_id = resolve_project_id(task.project_id.clone(), &settings);
    let mut existing = storage::load_task(&state.tasks_dir, &task.id).map_err(|e| e.to_string())?;
    let series_id = existing.series_id.clone();
    apply_task_update(&mut existing, title, resolved_project_id, task);

    if scope == "this" {
        existing.series_id = None;
        storage::update_task(&state.tasks_dir, &existing).map_err(|e| e.to_string())?;
        return Ok(vec![existing]);
    }

    let Some(series_id) = series_id else {
        return Err("task is not part of a recurring series".to_string());
    };
    let scheduled = existing.scheduled.clone().unwrap_or_default();

    storage::update_task(&state.tasks_dir, &existing).map_err(|e| e.to_string())?;
    let mut updated = vec![existing.clone()];

    let _guard = state
        .series_lock
        .lock()
        .map_err(|_| "series lock poisoned".to_string())?;

    let mut all_series =
        series_storage::list_series(&state.series_file).map_err(|e| e.to_string())?;
    let Some(series) = all_series.iter_mut().find(|s| s.id == series_id) else {
        return Err(format!("series '{series_id}' not found"));
    };
    apply_series_template_update(series, &existing);
    series_storage::save_series(&state.series_file, &all_series).map_err(|e| e.to_string())?;

    let all_tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
    for other in future_occurrences(&all_tasks, &series_id, Some(&existing.id), &scheduled) {
        let mut other = other.clone();
        apply_template_to_occurrence(&mut other, &existing);
        storage::update_task(&state.tasks_dir, &other).map_err(|e| e.to_string())?;
        updated.push(other);
    }

    Ok(updated)
}

/// Deletes an occurrence of a recurring task, with `scope` (`"this"` or
/// `"future"`) deciding how far the deletion reaches:
/// - `"this"`: behaves exactly like [`delete_task`] — only this occurrence
///   is removed, the series and every other occurrence are untouched.
/// - `"future"`: deletes this occurrence and every other already-generated
///   occurrence in the same series whose `scheduled` date is on or after
///   this one's, then caps the series' `end_date` to the day before this
///   date so generation never recreates them. Past occurrences (before
///   this date) are left untouched.
#[tauri::command]
pub fn delete_series_occurrence(
    state: State<AppState>,
    task_id: String,
    scope: String,
) -> Result<(), String> {
    if scope != "this" && scope != "future" {
        return Err("scope must be 'this' or 'future'".to_string());
    }

    let task = storage::load_task(&state.tasks_dir, &task_id).map_err(|e| e.to_string())?;

    if scope == "this" {
        return storage::delete_task(&state.tasks_dir, &task_id).map_err(|e| e.to_string());
    }

    let (Some(series_id), Some(scheduled)) = (task.series_id.clone(), task.scheduled.clone())
    else {
        return storage::delete_task(&state.tasks_dir, &task_id).map_err(|e| e.to_string());
    };

    let _guard = state
        .series_lock
        .lock()
        .map_err(|_| "series lock poisoned".to_string())?;

    let all_tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
    for other in future_occurrences(&all_tasks, &series_id, None, &scheduled) {
        storage::delete_task(&state.tasks_dir, &other.id).map_err(|e| e.to_string())?;
    }

    let mut all_series =
        series_storage::list_series(&state.series_file).map_err(|e| e.to_string())?;
    if let Some(series) = all_series.iter_mut().find(|s| s.id == series_id) {
        if let Some(cutoff) = chrono::NaiveDate::parse_from_str(&scheduled, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.checked_sub_days(chrono::Days::new(1)))
        {
            series.end_date = Some(cutoff.format("%Y-%m-%d").to_string());
        }
    }
    series_storage::save_series(&state.series_file, &all_series).map_err(|e| e.to_string())?;

    Ok(())
}

/// Every series id belonging to a subtask of any occurrence of
/// `parent_series_id` — for cascading a parent's recurrence edit or
/// removal to its subtasks' own series, since a subtask's pattern is
/// locked to match its parent's (see the Subtasks design spec's
/// recurrence-inheritance decision). Walks: every task sharing
/// `parent_series_id` that also owns a subtask container, to that
/// container's subtasks, to each one's own `series_id` if it has one.
/// Independent occurrences of the same recurring parent can each have
/// their *own* container (subtask containers are per-task-row, not
/// per-series — see `get_or_create_subtask_container`), so this checks
/// every occurrence, not just one.
fn linked_subtask_series_ids(tasks: &[Task], parent_series_id: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for occurrence in tasks
        .iter()
        .filter(|t| t.series_id.as_deref() == Some(parent_series_id))
    {
        let Some(container_id) = &occurrence.subtask_project_id else {
            continue;
        };
        for subtask in tasks
            .iter()
            .filter(|t| t.project_id.as_deref() == Some(container_id.as_str()))
        {
            if let Some(series_id) = &subtask.series_id {
                if !ids.contains(series_id) {
                    ids.push(series_id.clone());
                }
            }
        }
    }
    ids
}

/// Stops a recurring task's series from generating any further occurrences,
/// and cascades the same to every subtask series linked to it (see
/// [`linked_subtask_series_ids`]) — a subtask's recurrence can't outlive
/// its parent's. Existing occurrences (past and future, parent's and
/// subtasks') keep their `series_id` rather than having it severed — the
/// user's explicit choice, so a future series-level report could still
/// group them.
#[tauri::command]
pub fn remove_recurrence(state: State<AppState>, task_id: String) -> Result<(), String> {
    let task = storage::load_task(&state.tasks_dir, &task_id).map_err(|e| e.to_string())?;
    let Some(series_id) = task.series_id else {
        return Err("task is not part of a recurring series".to_string());
    };

    let _guard = state
        .series_lock
        .lock()
        .map_err(|_| "series lock poisoned".to_string())?;

    let all_tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
    let mut series_ids_to_stop = vec![series_id.clone()];
    series_ids_to_stop.extend(linked_subtask_series_ids(&all_tasks, &series_id));

    let mut all_series =
        series_storage::list_series(&state.series_file).map_err(|e| e.to_string())?;
    let Some(series) = all_series.iter_mut().find(|s| s.id == series_id) else {
        return Err(format!("series '{series_id}' not found"));
    };
    series.active = false;

    for linked_id in &series_ids_to_stop[1..] {
        if let Some(linked) = all_series.iter_mut().find(|s| &s.id == linked_id) {
            linked.active = false;
        }
    }

    series_storage::save_series(&state.series_file, &all_series).map_err(|e| e.to_string())?;
    Ok(())
}

/// Returns the `Series` identified by `series_id`, for the recurrence
/// builder to pre-fill when editing an existing recurring task's
/// frequency/due rule — the frontend otherwise has no way to see a
/// series' recurrence configuration at all, only its occurrences' own
/// task fields.
#[tauri::command]
pub fn get_series(state: State<AppState>, series_id: String) -> Result<Series, String> {
    let all_series = series_storage::list_series(&state.series_file).map_err(|e| e.to_string())?;
    all_series
        .into_iter()
        .find(|s| s.id == series_id)
        .ok_or_else(|| format!("series '{series_id}' not found"))
}

/// The actual work behind [`update_series_recurrence`] for one series:
/// updates `frequency`/`due_rule`/`end_date`, deletes every already-
/// generated occurrence on or after `cutoff_date` (see
/// [`future_occurrences`]), recreates the cutoff date's own occurrence
/// directly when it's the series' anchor date and satisfies the new
/// pattern (mirrors [`create_recurring_task`]'s own
/// [`anchor_matches_frequency`] check — see that command's doc comment for
/// why), and regenerates a fresh [`RECURRENCE_BASELINE_LOOKAHEAD_DAYS`]-day
/// baseline from there. Mutates `series` in place and returns every newly
/// created task. Called once directly for the series being edited, and
/// once more per linked subtask series when cascading (see
/// [`linked_subtask_series_ids`]) — a subtask's pattern is locked to match
/// its parent's, so every linked series gets the identical new
/// frequency/due_rule/end_date and the same absolute `cutoff_date`.
#[allow(clippy::too_many_arguments)]
fn apply_recurrence_change(
    tasks_dir: &Path,
    settings: &Settings,
    projects: &[Project],
    series: &mut Series,
    cutoff_date: chrono::NaiveDate,
    frequency: RecurrenceFrequency,
    due_rule: DueRule,
    end_date: Option<String>,
) -> Result<Vec<Task>, String> {
    series.frequency = frequency;
    series.due_rule = due_rule;
    series.end_date = end_date;
    validate_series(series)?;

    let project_chain: Vec<&Project> = series
        .project_id
        .as_deref()
        .and_then(|id| find_project(projects, id))
        .map(|p| project_tree::self_and_ancestors(projects, &p.id))
        .unwrap_or_default();

    let cutoff = cutoff_date.format("%Y-%m-%d").to_string();
    let all_tasks = storage::list_tasks(tasks_dir).map_err(|e| e.to_string())?;
    for stale in future_occurrences(&all_tasks, &series.id, None, &cutoff) {
        storage::delete_task(tasks_dir, &stale.id).map_err(|e| e.to_string())?;
    }

    let mut created = Vec::new();

    // Mirrors create_recurring_task's own anchor_matches_frequency check:
    // occurrence_dates_in_range only ever returns dates strictly *after*
    // its `after` parameter, so if `cutoff` is the series' anchor date
    // itself (i.e. this edit was made from the series' very first
    // occurrence, which was just deleted above), the generation step
    // below can never reconsider that date — it would be lost forever,
    // even when it satisfies the new pattern, without this direct check.
    let anchor_date = chrono::NaiveDate::parse_from_str(&series.anchor_date, "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
    if cutoff_date == anchor_date && anchor_matches_frequency(&series.frequency, anchor_date) {
        let occurrence = build_series_occurrence(series, settings, &project_chain, anchor_date);
        storage::save_task(tasks_dir, &occurrence).map_err(|e| e.to_string())?;
        created.push(occurrence);
    }

    let new_watermark = cutoff_date
        .checked_sub_days(chrono::Days::new(1))
        .ok_or_else(|| "cutoff date is out of range".to_string())?;
    series.generated_until = new_watermark.format("%Y-%m-%d").to_string();

    let horizon = cutoff_date + chrono::Duration::days(RECURRENCE_BASELINE_LOOKAHEAD_DAYS);
    let generated =
        generate_series_occurrences(tasks_dir, settings, &project_chain, series, horizon)?;
    created.extend(generated);

    Ok(created)
}

/// Updates an existing recurring task's frequency, due rule, and/or end
/// date — always a whole-series change, never scoped to "just this
/// occurrence" (unlike [`update_series_occurrence`]'s shared-field edits):
/// there's no meaningful "just this one occurrence follows a different
/// recurrence pattern" the way there is for title/priority/tags. Also
/// cascades the identical change to every subtask series linked to this
/// one (see [`linked_subtask_series_ids`]) — a subtask's recurrence can't
/// independently differ from its parent's.
///
/// See [`apply_recurrence_change`] for exactly what "update" means per
/// series — deleting/regenerating occurrences from `cutoff` forward. Past
/// occurrences (before `cutoff`) are never touched, for the edited series
/// or any cascaded one. The series' `anchor_date` is never changed by this
/// — only by removing and recreating the series entirely — so a
/// `Weekly`/`MonthlyByDay` pattern with `interval_weeks`/month-skip
/// behavior still counts from the same original reference point.
///
/// Rejects the edit if the series is no longer active (recurrence already
/// removed via [`remove_recurrence`]) — otherwise this would silently
/// resume generating occurrences for a series the user explicitly stopped.
/// A linked subtask series found inactive is silently skipped rather than
/// rejecting the whole edit, since the user only asked to edit the parent.
/// Returns every newly generated task, across the edited series and any
/// cascaded subtask series.
#[tauri::command]
pub fn update_series_recurrence(
    state: State<AppState>,
    series_id: String,
    cutoff: String,
    frequency: RecurrenceFrequency,
    due_rule: DueRule,
    end_date: Option<String>,
) -> Result<Vec<Task>, String> {
    let cutoff_date = chrono::NaiveDate::parse_from_str(&cutoff, "%Y-%m-%d")
        .map_err(|_| "cutoff must be a valid date in YYYY-MM-DD format".to_string())?;
    validate_date_field(&end_date, "end date")?;
    if let Some(end) = &end_date {
        let end_date_parsed =
            chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d").map_err(|e| e.to_string())?;
        if end_date_parsed < cutoff_date {
            return Err("end date must be on or after the cutoff date".to_string());
        }
    }

    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    let _guard = state
        .series_lock
        .lock()
        .map_err(|_| "series lock poisoned".to_string())?;

    let all_tasks_before = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
    let linked_series_ids = linked_subtask_series_ids(&all_tasks_before, &series_id);

    let mut all_series =
        series_storage::list_series(&state.series_file).map_err(|e| e.to_string())?;
    let series_index = all_series
        .iter()
        .position(|s| s.id == series_id)
        .ok_or_else(|| format!("series '{series_id}' not found"))?;
    if !all_series[series_index].active {
        return Err(
            "this series' recurrence has been removed and can no longer be edited".to_string(),
        );
    }

    let mut created = apply_recurrence_change(
        &state.tasks_dir,
        &settings,
        &projects,
        &mut all_series[series_index],
        cutoff_date,
        frequency.clone(),
        due_rule.clone(),
        end_date.clone(),
    )?;

    for linked_id in &linked_series_ids {
        let Some(linked_index) = all_series.iter().position(|s| &s.id == linked_id) else {
            continue;
        };
        if !all_series[linked_index].active {
            continue;
        }
        created.extend(apply_recurrence_change(
            &state.tasks_dir,
            &settings,
            &projects,
            &mut all_series[linked_index],
            cutoff_date,
            frequency.clone(),
            due_rule.clone(),
            end_date.clone(),
        )?);
    }

    series_storage::save_series(&state.series_file, &all_series).map_err(|e| e.to_string())?;

    Ok(created)
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

/// Ensures `settings.default_project_id` references a project that actually
/// exists, creating a new top-level "General" project (using
/// [`DEFAULT_PROJECT_COLOR`] and the next available `order`, via the same
/// [`next_order`] every other project-creation path uses) if it doesn't.
/// Covers both a brand-new install (`default_project_id` seeds as an empty
/// string — see `Settings::default`) and an upgrade from before this field
/// existed, where the id is just stale or never resolved. Returns
/// `Some((updated_projects, updated_settings))` if a project was created
/// (so the caller knows both files need saving), or `None` if
/// `default_project_id` already pointed at a real project.
pub fn ensure_default_project(
    projects: Vec<Project>,
    settings: Settings,
) -> Option<(Vec<Project>, Settings)> {
    if projects.iter().any(|p| p.id == settings.default_project_id) {
        return None;
    }

    let mut projects = projects;
    let mut settings = settings;
    let order = next_order(&projects);
    let project = Project::new(
        "General".to_string(),
        DEFAULT_PROJECT_COLOR.to_string(),
        order,
    );
    settings.default_project_id = project.id.clone();
    projects.push(project);
    Some((projects, settings))
}

/// Returns all projects, sorted by `order`.
#[tauri::command]
pub fn list_projects(state: State<AppState>) -> Result<Vec<Project>, String> {
    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    projects.sort_by_key(|p| p.order);
    Ok(projects)
}

/// Returns `Ok(())` if `color` is a 6-digit hex color (`#RRGGBB`,
/// case-insensitive), and an error otherwise. `Project.color` has always
/// been hex by convention (see [`DEFAULT_PROJECT_COLOR`]) and is now
/// exclusively produced by the `ColorPicker` UI, unlike
/// `PriorityLevel.color`/`StatusDefinition.color`, which still accept any
/// valid CSS color from their free-form text inputs.
fn validate_hex_color(color: &str) -> Result<(), String> {
    let digits = color.strip_prefix('#');
    let is_valid =
        matches!(digits, Some(d) if d.len() == 6 && d.chars().all(|c| c.is_ascii_hexdigit()));
    if is_valid {
        Ok(())
    } else {
        Err(format!(
            "'{color}' is not a valid hex color (expected #RRGGBB)"
        ))
    }
}

/// Resolves the color for a newly-created project: validates `color` as a
/// 6-digit hex color (see [`validate_hex_color`]) if provided, or falls back
/// to [`DEFAULT_PROJECT_COLOR`] when `color` is `None`.
fn resolve_project_color(color: Option<String>) -> Result<String, String> {
    match color {
        Some(c) => {
            validate_hex_color(&c)?;
            Ok(c)
        }
        None => Ok(DEFAULT_PROJECT_COLOR.to_string()),
    }
}

/// Returns an error if `parent_id` (when set) doesn't reference an existing
/// project in `projects`, or — when `moving_id` is also set (an existing
/// project being re-parented, as opposed to a brand-new project that can
/// never have descendants yet) — would create a cycle (see
/// [`would_create_cycle`]).
fn validate_parent_id(
    projects: &[Project],
    parent_id: Option<&str>,
    moving_id: Option<&str>,
) -> Result<(), String> {
    let Some(parent_id) = parent_id else {
        return Ok(());
    };

    if !projects.iter().any(|p| p.id == parent_id) {
        return Err(format!("parent project '{parent_id}' not found"));
    }

    if let Some(moving_id) = moving_id {
        if would_create_cycle(projects, moving_id, parent_id) {
            return Err(
                "cannot move a project under itself or one of its own subprojects".to_string(),
            );
        }
    }

    Ok(())
}

/// Creates a new project with a trimmed, non-empty, case-insensitively
/// unique name. `order` is set to one past the current maximum so new
/// projects sort after existing ones. Falls back to
/// [`DEFAULT_PROJECT_COLOR`] when `color` is `None`; otherwise `color` must
/// be a 6-digit hex color (see [`validate_hex_color`]). `parent_id`, when
/// set, must reference an existing project (see [`validate_parent_id`]) —
/// nesting depth is otherwise unrestricted.
#[tauri::command]
pub fn create_project(
    state: State<AppState>,
    name: String,
    color: Option<String>,
    parent_id: Option<String>,
) -> Result<Project, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("project name must not be empty".to_string());
    }

    let color = resolve_project_color(color)?;

    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    if projects.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
        return Err(format!("a project named '{name}' already exists"));
    }
    validate_parent_id(&projects, parent_id.as_deref(), None)?;

    let order = next_order(&projects);
    let mut project = Project::new(name, color, order);
    project.parent_id = parent_id;
    projects.push(project.clone());
    project_storage::save_projects(&state.projects_file, &projects).map_err(|e| e.to_string())?;

    Ok(project)
}

/// Looks up or lazily creates `task`'s "subtask container" project — the
/// auto-generated subproject that holds its subtasks (see the Subtasks
/// design spec). If `task.subtask_project_id` already names a project
/// still present in `projects`, returns a clone of it unchanged. Otherwise
/// creates a new project named after `task`'s current title, parented
/// under `task.project_id` (always `Some` for any real task — see
/// `Task::project_id`'s own doc comment), colored to match that same
/// parent project (falling back to [`DEFAULT_PROJECT_COLOR`] if it can't
/// be found — shouldn't happen for any real task, but a hidden container
/// still needs *some* color), pushes it onto `projects`, and points
/// `task.subtask_project_id` at it.
///
/// Deliberately skips the case-insensitive name-uniqueness check
/// `create_project` enforces for user-facing creation — this container is
/// never browsed by name (it's hidden from the sidebar entirely), so a
/// coincidental name collision with some unrelated project is harmless.
fn get_or_create_subtask_container(task: &mut Task, projects: &mut Vec<Project>) -> Project {
    if let Some(container_id) = &task.subtask_project_id {
        if let Some(existing) = projects.iter().find(|p| &p.id == container_id) {
            return existing.clone();
        }
    }

    let color = task
        .project_id
        .as_deref()
        .and_then(|id| find_project(projects, id))
        .map(|p| p.color.clone())
        .unwrap_or_else(|| DEFAULT_PROJECT_COLOR.to_string());

    let order = next_order(projects);
    let mut container = Project::new(task.title.clone(), color, order);
    container.parent_id = task.project_id.clone();
    projects.push(container.clone());
    task.subtask_project_id = Some(container.id.clone());

    container
}

/// Returns `parent_task_id`'s subtask container, creating it on first call
/// (see [`get_or_create_subtask_container`]). Only writes to disk when
/// something actually changed — `task.subtask_project_id`'s value before
/// vs. after the call — covering both "no container existed yet" and the
/// defensive case of a stale pointer to an already-deleted project, which
/// also needs a fresh container and a corrected pointer.
#[tauri::command]
pub fn ensure_subtask_container(
    state: State<AppState>,
    parent_task_id: String,
) -> Result<Project, String> {
    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let mut task =
        storage::load_task(&state.tasks_dir, &parent_task_id).map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    let before = task.subtask_project_id.clone();
    let container = get_or_create_subtask_container(&mut task, &mut projects);

    if task.subtask_project_id != before {
        storage::update_task(&state.tasks_dir, &task).map_err(|e| e.to_string())?;
        project_storage::save_projects(&state.projects_file, &projects)
            .map_err(|e| e.to_string())?;
    }

    Ok(container)
}

/// Applies an `update_project` request onto the project at `index` within
/// `projects`, preserving that project's `id` and `created` timestamp from
/// disk. Validates that `update.name` (trimmed) is non-empty and
/// case-insensitively unique among the *other* projects, that every
/// status/priority id referenced by `update.board` and `update.defaults` is
/// defined in `settings`, that `update.defaults.due`/`.scheduled`, if set,
/// are recognized relative-date codes (mirroring the checks
/// `validate_settings` applies to `Settings.defaults`), and that
/// `update.board.card_lightness`/`.bar_lightness`, if set, are valid OKLCH
/// lightness values (see [`validate_lightness`]), and that
/// `update.board.ink_mode`, if set, is a recognized ink mode (see
/// [`validate_ink_mode`]). If `update.color` differs from the project's
/// current color, it must be a 6-digit hex color (see [`validate_hex_color`]);
/// an unchanged color is left as-is even if it predates that requirement.
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
    if let Some(due_code) = &update.defaults.due {
        validate_due_relative_date_code(due_code)?;
    }
    if let Some(scheduled_code) = &update.defaults.scheduled {
        validate_scheduled_relative_date_code(scheduled_code)?;
    }
    if let Some(card_lightness) = update.board.card_lightness {
        validate_lightness(card_lightness)?;
    }
    if let Some(bar_lightness) = update.board.bar_lightness {
        validate_lightness(bar_lightness)?;
    }
    if let Some(ink_mode) = &update.board.ink_mode {
        validate_ink_mode(ink_mode)?;
    }

    let existing_color = projects[index].color.clone();
    let existing_created = projects[index].created.clone();
    if update.color != existing_color {
        validate_hex_color(&update.color)?;
    }

    let existing_id = projects[index].id.clone();
    validate_parent_id(projects, update.parent_id.as_deref(), Some(&existing_id))?;

    let updated = Project {
        id: existing_id,
        name,
        color: update.color,
        parent_id: update.parent_id,
        order: update.order,
        created: existing_created,
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

/// Returns an error if `project` is the configured default project: every
/// task without an explicit project falls back to it (see
/// [`resolve_project_id`]), so it can never be deleted.
fn ensure_not_default_project(project: &Project, settings: &Settings) -> Result<(), String> {
    if project.id == settings.default_project_id {
        Err(format!(
            "'{}' is the default project and cannot be deleted",
            project.name
        ))
    } else {
        Ok(())
    }
}

/// Returns the tasks currently filed under any id in `project_ids` - i.e.
/// those that need to be reassigned, archived, or deleted before the
/// corresponding projects can themselves be deleted.
fn tasks_for_projects<'a>(tasks: &'a [Task], project_ids: &[String]) -> Vec<&'a Task> {
    tasks
        .iter()
        .filter(|t| {
            t.project_id
                .as_deref()
                .is_some_and(|id| project_ids.iter().any(|target| target == id))
        })
        .collect()
}

/// What to do with a project's existing tasks when the project is deleted
/// (see [`delete_project`]).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProjectTaskStrategy {
    /// Move each task to the project identified by `target_project_id`.
    Reassign { target_project_id: String },
    /// Move each task's markdown file to the archive directory, like
    /// [`finish_day`].
    Archive,
    /// Permanently delete each task's markdown file, like [`delete_task`].
    Delete,
}

/// Applies `strategy` to every task in `tasks` (all belonging to one of the
/// projects identified by `deleted_project_ids`, which are about to be
/// deleted), using `tasks_dir`/`archive_dir` for file operations and
/// `projects` to resolve a `Reassign` target's name. Returns the number of
/// tasks affected.
fn apply_task_strategy(
    tasks_dir: &Path,
    archive_dir: &Path,
    projects: &[Project],
    deleted_project_ids: &[String],
    tasks: &[&Task],
    strategy: &ProjectTaskStrategy,
) -> Result<usize, String> {
    match strategy {
        ProjectTaskStrategy::Reassign { target_project_id } => {
            if deleted_project_ids.contains(target_project_id) {
                return Err("cannot reassign tasks to a project being deleted".to_string());
            }
            if !projects.iter().any(|p| &p.id == target_project_id) {
                return Err(format!("target project '{target_project_id}' not found"));
            }
            for task in tasks {
                let mut updated = (*task).clone();
                updated.project_id = Some(target_project_id.clone());
                storage::update_task(tasks_dir, &updated).map_err(|e| e.to_string())?;
            }
        }
        ProjectTaskStrategy::Archive => {
            for task in tasks {
                storage::archive_task(tasks_dir, archive_dir, &task.id)
                    .map_err(|e| e.to_string())?;
            }
        }
        ProjectTaskStrategy::Delete => {
            for task in tasks {
                storage::delete_task(tasks_dir, &task.id).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(tasks.len())
}

/// Result of [`delete_project`]: how many of the deleted project's tasks were
/// reassigned, archived, or deleted as part of removing it.
#[derive(Debug, Serialize)]
pub struct DeleteProjectResult {
    pub affected_tasks: usize,
    /// How many descendant subprojects were deleted along with the
    /// requested project (0 if it had none).
    pub deleted_subprojects: usize,
}

/// Returns the project identified by `project_id` together with all of its
/// descendants (see [`crate::project_tree::descendants_of`]) — the full set
/// of projects [`delete_project`] removes for a cascading delete. Empty if
/// `project_id` doesn't exist in `projects`.
fn projects_to_delete(projects: &[Project], project_id: &str) -> Vec<Project> {
    let Some(target) = projects.iter().find(|p| p.id == project_id) else {
        return Vec::new();
    };
    std::iter::once(target.clone())
        .chain(
            project_tree::descendants_of(projects, project_id)
                .into_iter()
                .cloned(),
        )
        .collect()
}

/// Deletes the project identified by `project_id` together with every
/// descendant subproject (see [`projects_to_delete`]) — a cascading delete.
/// The configured default project can never be deleted, nor can any project
/// whose subtree contains it (see [`ensure_not_default_project`]). If the
/// project or any descendant still has tasks (matched by name,
/// case-insensitively - see [`tasks_for_projects`]), `task_strategy` is
/// required and is applied to all of them together (see
/// [`apply_task_strategy`]) before every project in the subtree is removed
/// from the projects file. Tasks already moved to the archive don't count
/// toward this check and never block deletion.
#[tauri::command]
pub fn delete_project(
    state: State<AppState>,
    project_id: String,
    task_strategy: Option<ProjectTaskStrategy>,
) -> Result<DeleteProjectResult, String> {
    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;

    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    let doomed = projects_to_delete(&projects, &project_id);
    if doomed.is_empty() {
        return Err(format!("project '{project_id}' not found"));
    }
    for project in &doomed {
        ensure_not_default_project(project, &settings)?;
    }

    let doomed_ids: Vec<String> = doomed.iter().map(|p| p.id.clone()).collect();
    let doomed_names: Vec<String> = doomed.iter().map(|p| p.name.clone()).collect();
    let deleted_subprojects = doomed.len() - 1;

    let tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
    let matching = tasks_for_projects(&tasks, &doomed_names);

    let affected_tasks = if matching.is_empty() {
        0
    } else {
        let strategy = task_strategy.ok_or_else(|| {
            "this project still has tasks; choose how to handle them before deleting it".to_string()
        })?;
        apply_task_strategy(
            &state.tasks_dir,
            &state.archive_dir,
            &projects,
            &doomed_ids,
            &matching,
            &strategy,
        )?
    };

    projects.retain(|p| !doomed_ids.contains(&p.id));
    project_storage::save_projects(&state.projects_file, &projects).map_err(|e| e.to_string())?;

    Ok(DeleteProjectResult {
        affected_tasks,
        deleted_subprojects,
    })
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
    if !projects.iter().any(|p| p.id == settings.default_project_id) {
        return Err(format!(
            "default project '{}' does not exist",
            settings.default_project_id
        ));
    }

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

/// Returns `true` if `task.status` matches `settings.done_status`, or
/// `settings.cancelled_status` when one is configured - i.e. this task is
/// "finished" and should be archived by [`finish_day`].
fn is_finished(task: &Task, settings: &Settings) -> bool {
    task.status == settings.done_status
        || settings
            .cancelled_status
            .as_deref()
            .is_some_and(|cancelled| task.status == cancelled)
}

/// Result of [`finish_day`]: how many tasks were archived.
#[derive(Debug, Serialize)]
pub struct FinishDayResult {
    pub archived_count: usize,
}

/// Every task id [`finish_day`] should archive, given the directly-finished
/// tasks (status matches done/cancelled, see [`is_finished`]) plus, for
/// each of those that owns a subtask container, every one of its subtasks
/// too — regardless of *their* own individual status. A finished parent
/// task's subtasks have no remaining reason to linger as a barely-
/// discoverable orphaned subproject, mirroring why the existing manual
/// cascade-delete (`delete_task`) also removes every subtask unconditionally
/// rather than only the already-finished ones. Also returns every
/// now-emptied container project's id, for the caller to remove from
/// `projects.json`. A task that's independently finished but isn't itself
/// a subtask-container owner is included exactly once, with no cascade.
fn tasks_and_containers_to_finish(
    tasks: &[Task],
    settings: &Settings,
) -> (Vec<String>, Vec<String>) {
    let mut task_ids: Vec<String> = Vec::new();
    let mut container_ids = Vec::new();

    for task in tasks {
        if !is_finished(task, settings) {
            continue;
        }
        if !task_ids.contains(&task.id) {
            task_ids.push(task.id.clone());
        }
        let Some(container_id) = &task.subtask_project_id else {
            continue;
        };
        container_ids.push(container_id.clone());
        for subtask in tasks
            .iter()
            .filter(|t| t.project_id.as_deref() == Some(container_id.as_str()))
        {
            if !task_ids.contains(&subtask.id) {
                task_ids.push(subtask.id.clone());
            }
        }
    }

    (task_ids, container_ids)
}

/// Archives every task across all projects whose status is the configured
/// done or cancelled status (see [`is_finished`]), by moving its markdown
/// file from `tasks_dir` to `archive_dir` (see [`storage::archive_task`]).
/// If a finished task owns a subtask container, cascades to archive every
/// one of its subtasks too and removes the now-empty container project —
/// see [`tasks_and_containers_to_finish`].
#[tauri::command]
pub fn finish_day(state: State<AppState>) -> Result<FinishDayResult, String> {
    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    let tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;

    let (task_ids, container_ids) = tasks_and_containers_to_finish(&tasks, &settings);

    for task_id in &task_ids {
        storage::archive_task(&state.tasks_dir, &state.archive_dir, task_id)
            .map_err(|e| e.to_string())?;
    }

    if !container_ids.is_empty() {
        let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
        let mut projects =
            project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
        projects.retain(|p| !container_ids.contains(&p.id));
        project_storage::save_projects(&state.projects_file, &projects)
            .map_err(|e| e.to_string())?;
    }

    Ok(FinishDayResult {
        archived_count: task_ids.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectBoard;
    use crate::settings::{PriorityLevel, StatusDefinition, TaskDefaults};
    use tempfile::tempdir;

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
    fn validate_due_creation_field_accepts_none() {
        assert!(validate_due_creation_field(&None).is_ok());
    }

    #[test]
    fn validate_due_creation_field_accepts_the_none_sentinel() {
        assert!(validate_due_creation_field(&Some("none".to_string())).is_ok());
    }

    #[test]
    fn validate_due_creation_field_accepts_a_valid_iso_date() {
        assert!(validate_due_creation_field(&Some("2026-07-01".to_string())).is_ok());
    }

    #[test]
    fn validate_due_creation_field_rejects_a_malformed_date() {
        let result = validate_due_creation_field(&Some("07/01/2026".to_string()));

        assert!(result.is_err());
    }

    #[test]
    fn apply_create_overrides_keeps_optional_defaults_when_all_none() {
        let mut task = Task::new("Buy milk".to_string());
        let defaults = task.clone();

        apply_create_overrides(
            &mut task,
            None,
            None,
            None,
            None,
            "2026-06-14".to_string(),
            None,
        );

        assert_eq!(task.project_id, defaults.project_id);
        assert_eq!(task.tags, defaults.tags);
        assert_eq!(task.priority, defaults.priority);
        assert_eq!(task.due, defaults.due);
        assert_eq!(task.scheduled, Some("2026-06-14".to_string()));
        assert_eq!(task.estimated_minutes, defaults.estimated_minutes);
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
            "2026-06-25".to_string(),
            Some(90),
        );

        assert_eq!(task.project_id, Some("Vacation".to_string()));
        assert_eq!(task.tags, vec!["travel".to_string()]);
        assert_eq!(task.priority, "high");
        assert_eq!(task.due, Some("2026-07-01".to_string()));
        assert_eq!(task.scheduled, Some("2026-06-25".to_string()));
        assert_eq!(task.estimated_minutes, Some(90));
    }

    #[test]
    fn apply_task_update_applies_all_editable_fields_including_status() {
        let mut existing = Task::new("Original title".to_string());
        existing.status = "backlog".to_string();

        let mut incoming = Task::new("New title".to_string());
        incoming.status = "done".to_string();
        incoming.tags = vec!["urgent".to_string()];
        incoming.priority = "high".to_string();
        incoming.due = Some("2026-07-01".to_string());
        incoming.scheduled = Some("2026-06-20".to_string());
        incoming.estimated_minutes = Some(45);
        incoming.notes = "updated notes".to_string();

        apply_task_update(
            &mut existing,
            "New title".to_string(),
            "Work".to_string(),
            incoming,
        );

        assert_eq!(existing.title, "New title");
        assert_eq!(existing.project_id, Some("Work".to_string()));
        assert_eq!(existing.tags, vec!["urgent".to_string()]);
        assert_eq!(existing.priority, "high");
        assert_eq!(existing.status, "done");
        assert_eq!(existing.due, Some("2026-07-01".to_string()));
        assert_eq!(existing.scheduled, Some("2026-06-20".to_string()));
        assert_eq!(existing.estimated_minutes, Some(45));
        assert_eq!(existing.notes, "updated notes".to_string());
    }

    #[test]
    fn apply_task_update_preserves_id_created_depends_on_and_tracked_minutes() {
        let mut existing = Task::new("Original title".to_string());
        existing.depends_on = vec!["blocker-id".to_string()];
        existing.tracked_minutes = 120;
        let original_id = existing.id.clone();
        let original_created = existing.created.clone();

        let mut incoming = Task::new("New title".to_string());
        incoming.id = "a-different-id".to_string();
        incoming.created = "2000-01-01T00:00:00Z".to_string();
        incoming.depends_on = vec!["should-not-apply".to_string()];
        incoming.tracked_minutes = 9999;

        apply_task_update(
            &mut existing,
            "New title".to_string(),
            "Work".to_string(),
            incoming,
        );

        assert_eq!(existing.id, original_id);
        assert_eq!(existing.created, original_created);
        assert_eq!(existing.depends_on, vec!["blocker-id".to_string()]);
        assert_eq!(existing.tracked_minutes, 120);
    }

    #[test]
    fn synced_subtask_container_renames_to_match_the_task_title() {
        let mut container = Project::new("Old title".to_string(), "#3b82f6".to_string(), 1);
        container.parent_id = Some("work".to_string());
        let container_id = container.id.clone();
        let projects = vec![container];

        let updated =
            synced_subtask_container(&projects, &container_id, "New title", "work").unwrap();

        assert_eq!(updated.name, "New title");
        assert_eq!(updated.parent_id, Some("work".to_string()));
        assert_eq!(updated.id, container_id);
    }

    #[test]
    fn synced_subtask_container_reparents_to_match_the_task_project() {
        let mut container = Project::new("Fix the bug".to_string(), "#3b82f6".to_string(), 1);
        container.parent_id = Some("work".to_string());
        let container_id = container.id.clone();
        let projects = vec![container];

        let updated =
            synced_subtask_container(&projects, &container_id, "Fix the bug", "personal").unwrap();

        assert_eq!(updated.parent_id, Some("personal".to_string()));
    }

    #[test]
    fn synced_subtask_container_is_a_no_op_when_nothing_changed() {
        let mut container = Project::new("Fix the bug".to_string(), "#3b82f6".to_string(), 1);
        container.parent_id = Some("work".to_string());
        let container_id = container.id.clone();
        let projects = vec![container.clone()];

        let updated =
            synced_subtask_container(&projects, &container_id, "Fix the bug", "work").unwrap();

        assert_eq!(updated, container);
    }

    #[test]
    fn synced_subtask_container_returns_none_when_the_container_is_missing() {
        let projects: Vec<Project> = vec![];

        let result =
            synced_subtask_container(&projects, "deleted-container", "Fix the bug", "work");

        assert!(result.is_none());
    }

    #[test]
    fn merge_tags_appends_default_tags_not_already_present() {
        let merged = merge_tags(
            vec!["urgent".to_string()],
            vec!["home".to_string(), "urgent".to_string()],
        );

        assert_eq!(merged, vec!["urgent".to_string(), "home".to_string()]);
    }

    #[test]
    fn merge_tags_with_empty_explicit_returns_defaults() {
        let merged = merge_tags(Vec::new(), vec!["home".to_string()]);

        assert_eq!(merged, vec!["home".to_string()]);
    }

    #[test]
    fn merge_tags_with_empty_defaults_returns_explicit() {
        let merged = merge_tags(vec!["urgent".to_string()], Vec::new());

        assert_eq!(merged, vec!["urgent".to_string()]);
    }

    #[test]
    fn effective_default_tags_uses_project_tags_when_non_empty() {
        let global = TaskDefaults {
            tags: vec!["global".to_string()],
            ..Default::default()
        };
        let mut project = Project::new("Test".to_string(), "#111111".to_string(), 1);
        project.defaults = TaskDefaults {
            tags: vec!["project".to_string()],
            ..Default::default()
        };
        let chain = vec![&project];

        assert_eq!(
            effective_default_tags(&global, &chain),
            vec!["project".to_string()]
        );
    }

    #[test]
    fn effective_default_tags_falls_back_to_global_when_project_tags_empty() {
        let global = TaskDefaults {
            tags: vec!["global".to_string()],
            ..Default::default()
        };
        let project = Project::new("Test".to_string(), "#111111".to_string(), 1);
        let chain = vec![&project];

        assert_eq!(
            effective_default_tags(&global, &chain),
            vec!["global".to_string()]
        );
    }

    #[test]
    fn effective_default_estimated_minutes_uses_project_override_when_set() {
        let global = TaskDefaults {
            estimated_minutes: Some(60),
            ..Default::default()
        };
        let mut project = Project::new("Test".to_string(), "#111111".to_string(), 1);
        project.defaults = TaskDefaults {
            estimated_minutes: Some(15),
            ..Default::default()
        };
        let chain = vec![&project];

        assert_eq!(
            effective_default_estimated_minutes(&global, &chain),
            Some(15)
        );
    }

    #[test]
    fn effective_default_estimated_minutes_falls_back_to_global_when_project_unset() {
        let global = TaskDefaults {
            estimated_minutes: Some(60),
            ..Default::default()
        };
        let project = Project::new("Test".to_string(), "#111111".to_string(), 1);
        let chain = vec![&project];

        assert_eq!(
            effective_default_estimated_minutes(&global, &chain),
            Some(60)
        );
    }

    #[test]
    fn effective_default_estimated_minutes_is_none_when_neither_set() {
        let global = TaskDefaults::default();

        assert_eq!(effective_default_estimated_minutes(&global, &[]), None);
    }

    #[test]
    fn effective_default_tags_falls_back_to_global_when_no_project() {
        let global = TaskDefaults {
            tags: vec!["global".to_string()],
            ..Default::default()
        };

        assert_eq!(
            effective_default_tags(&global, &[]),
            vec!["global".to_string()]
        );
    }

    #[test]
    fn resolve_default_scheduled_date_resolves_a_known_code() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let code = "tomorrow".to_string();

        assert_eq!(
            resolve_default_scheduled_date(Some(&code), today),
            Some("2026-06-15".to_string())
        );
    }

    #[test]
    fn resolve_default_scheduled_date_returns_none_when_code_is_none() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(resolve_default_scheduled_date(None, today), None);
    }

    #[test]
    fn resolve_default_due_date_resolves_a_known_code() {
        let scheduled = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let code = "next_day".to_string();

        assert_eq!(
            resolve_default_due_date(Some(&code), scheduled),
            Some("2026-06-15".to_string())
        );
    }

    #[test]
    fn resolve_default_due_date_returns_none_for_the_none_sentinel() {
        let scheduled = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let code = "none".to_string();

        assert_eq!(resolve_default_due_date(Some(&code), scheduled), None);
    }

    #[test]
    fn resolve_default_due_date_returns_none_when_code_is_none() {
        let scheduled = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(resolve_default_due_date(None, scheduled), None);
    }

    #[test]
    fn resolve_creation_defaults_falls_back_fully_to_global_when_no_project() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let settings = Settings {
            defaults: TaskDefaults {
                tags: vec!["global".to_string()],
                due: Some("next_day".to_string()),
                scheduled: Some("in_1_week".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let (tags, due, scheduled) =
            resolve_creation_defaults(&settings, &[], today, None, None, None);

        assert_eq!(tags, vec!["global".to_string()]);
        assert_eq!(scheduled, "2026-06-21".to_string());
        // due is relative to the resolved scheduled date (2026-06-21), not "today".
        assert_eq!(due, Some("2026-06-22".to_string()));
    }

    #[test]
    fn resolve_creation_defaults_falls_back_to_today_when_scheduled_is_unset_everywhere() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let settings = Settings {
            defaults: TaskDefaults::default(),
            ..Default::default()
        };

        let (_tags, _due, scheduled) =
            resolve_creation_defaults(&settings, &[], today, None, None, None);

        assert_eq!(scheduled, "2026-06-14".to_string());
    }

    #[test]
    fn resolve_creation_defaults_due_none_sentinel_overrides_default() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let settings = Settings {
            defaults: TaskDefaults {
                due: Some("next_day".to_string()),
                scheduled: Some("today".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let (_tags, due, _scheduled) =
            resolve_creation_defaults(&settings, &[], today, None, Some("none".to_string()), None);

        assert_eq!(due, None);
    }

    #[test]
    fn resolve_creation_defaults_due_default_resolves_relative_to_explicit_scheduled() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let settings = Settings {
            defaults: TaskDefaults {
                due: Some("same_day".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let (_tags, due, scheduled) = resolve_creation_defaults(
            &settings,
            &[],
            today,
            None,
            None,
            Some("2026-07-01".to_string()),
        );

        assert_eq!(scheduled, "2026-07-01".to_string());
        assert_eq!(due, Some("2026-07-01".to_string()));
    }

    #[test]
    fn resolve_creation_defaults_project_tags_override_replaces_global_tags() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let settings = Settings {
            defaults: TaskDefaults {
                tags: vec!["global".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let mut project = Project::new("Test".to_string(), "#111111".to_string(), 1);
        project.defaults = TaskDefaults {
            tags: vec!["project".to_string()],
            ..Default::default()
        };
        let chain = vec![&project];

        let (tags, _due, _scheduled) = resolve_creation_defaults(
            &settings,
            &chain,
            today,
            Some(vec!["urgent".to_string()]),
            None,
            None,
        );

        assert_eq!(tags, vec!["urgent".to_string(), "project".to_string()]);
    }

    #[test]
    fn resolve_creation_defaults_project_due_and_scheduled_override_wins_over_global() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let settings = Settings {
            defaults: TaskDefaults {
                due: Some("same_day".to_string()),
                scheduled: Some("tomorrow".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut project = Project::new("Test".to_string(), "#111111".to_string(), 1);
        project.defaults = TaskDefaults {
            due: Some("in_1_week".to_string()),
            scheduled: Some("in_1_month".to_string()),
            ..Default::default()
        };
        let chain = vec![&project];

        let (_tags, due, scheduled) =
            resolve_creation_defaults(&settings, &chain, today, None, None, None);

        // scheduled = in_1_month from 2026-06-14 = 2026-07-14.
        assert_eq!(scheduled, "2026-07-14".to_string());
        // due = in_1_week relative to the resolved scheduled date (2026-07-14).
        assert_eq!(due, Some("2026-07-21".to_string()));
    }

    #[test]
    fn resolve_creation_defaults_explicit_values_short_circuit_defaults() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let settings = Settings {
            defaults: TaskDefaults {
                tags: vec!["global".to_string()],
                due: Some("next_day".to_string()),
                scheduled: Some("in_1_week".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let (tags, due, scheduled) = resolve_creation_defaults(
            &settings,
            &[],
            today,
            Some(vec!["explicit".to_string()]),
            Some("2026-08-01".to_string()),
            Some("2026-08-02".to_string()),
        );

        assert_eq!(tags, vec!["explicit".to_string(), "global".to_string()]);
        assert_eq!(due, Some("2026-08-01".to_string()));
        assert_eq!(scheduled, "2026-08-02".to_string());
    }

    #[test]
    fn resolve_default_status_falls_through_to_grandparent_when_parent_has_no_override() {
        let settings = Settings::default();
        let mut grandparent = Project::new("Grandparent".to_string(), "#111111".to_string(), 1);
        grandparent.board.default_status = Some("do".to_string());
        let parent = Project::new("Parent".to_string(), "#222222".to_string(), 2);
        let child = Project::new("Child".to_string(), "#333333".to_string(), 3);
        let chain = vec![&child, &parent, &grandparent];

        let result = resolve_default_status(&settings, &chain);

        assert_eq!(result, "do");
    }

    #[test]
    fn resolve_default_status_prefers_nearest_override_over_a_further_one() {
        let settings = Settings::default();
        let mut grandparent = Project::new("Grandparent".to_string(), "#111111".to_string(), 1);
        grandparent.board.default_status = Some("do".to_string());
        let mut parent = Project::new("Parent".to_string(), "#222222".to_string(), 2);
        parent.board.default_status = Some("blocked".to_string());
        let child = Project::new("Child".to_string(), "#333333".to_string(), 3);
        let chain = vec![&child, &parent, &grandparent];

        let result = resolve_default_status(&settings, &chain);

        assert_eq!(result, "blocked");
    }

    #[test]
    fn effective_default_tags_falls_through_to_grandparent() {
        let global = TaskDefaults::default();
        let mut grandparent = Project::new("Grandparent".to_string(), "#111111".to_string(), 1);
        grandparent.defaults.tags = vec!["inherited".to_string()];
        let parent = Project::new("Parent".to_string(), "#222222".to_string(), 2);
        let child = Project::new("Child".to_string(), "#333333".to_string(), 3);
        let chain = vec![&child, &parent, &grandparent];

        let result = effective_default_tags(&global, &chain);

        assert_eq!(result, vec!["inherited".to_string()]);
    }

    #[test]
    fn effective_default_estimated_minutes_falls_through_to_grandparent() {
        let global = TaskDefaults::default();
        let mut grandparent = Project::new("Grandparent".to_string(), "#111111".to_string(), 1);
        grandparent.defaults.estimated_minutes = Some(90);
        let parent = Project::new("Parent".to_string(), "#222222".to_string(), 2);
        let child = Project::new("Child".to_string(), "#333333".to_string(), 3);
        let chain = vec![&child, &parent, &grandparent];

        let result = effective_default_estimated_minutes(&global, &chain);

        assert_eq!(result, Some(90));
    }

    #[test]
    fn resolve_creation_defaults_due_code_falls_through_to_grandparent() {
        let settings = Settings::default();
        let mut grandparent = Project::new("Grandparent".to_string(), "#111111".to_string(), 1);
        grandparent.defaults.due = Some("next_day".to_string());
        let parent = Project::new("Parent".to_string(), "#222222".to_string(), 2);
        let child = Project::new("Child".to_string(), "#333333".to_string(), 3);
        let chain = vec![&child, &parent, &grandparent];
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();

        let (_, resolved_due, resolved_scheduled) =
            resolve_creation_defaults(&settings, &chain, today, None, None, None);

        assert_eq!(resolved_scheduled, "2026-06-20");
        assert_eq!(resolved_due, Some("2026-06-21".to_string()));
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
    fn ensure_default_project_creates_one_when_none_exists() {
        let projects: Vec<Project> = Vec::new();
        let settings = Settings::default();

        let result = ensure_default_project(projects, settings);

        let (projects, settings) = result.expect("should have created a default project");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "General");
        assert_eq!(settings.default_project_id, projects[0].id);
    }

    #[test]
    fn ensure_default_project_does_nothing_when_default_already_exists() {
        let project = Project::new("Inbox".to_string(), "#111111".to_string(), 1);
        let mut settings = Settings::default();
        settings.default_project_id = project.id.clone();
        let projects = vec![project];

        let result = ensure_default_project(projects, settings);

        assert!(result.is_none());
    }

    #[test]
    fn ensure_default_project_creates_one_when_configured_id_is_stale() {
        let project = Project::new("Inbox".to_string(), "#111111".to_string(), 1);
        let mut settings = Settings::default();
        settings.default_project_id = "a-deleted-project-id".to_string();
        let projects = vec![project];

        let result = ensure_default_project(projects, settings);

        let (projects, settings) = result.expect("should have created a default project");
        assert_eq!(projects.len(), 2);
        assert!(projects.iter().any(|p| p.id == settings.default_project_id));
    }

    #[test]
    fn validate_hex_color_accepts_a_six_digit_hex_color() {
        assert!(validate_hex_color("#3b82f6").is_ok());
    }

    #[test]
    fn validate_hex_color_accepts_uppercase_hex_digits() {
        assert!(validate_hex_color("#3B82F6").is_ok());
    }

    #[test]
    fn validate_hex_color_rejects_a_color_missing_the_hash() {
        assert!(validate_hex_color("3b82f6").is_err());
    }

    #[test]
    fn validate_hex_color_rejects_a_three_digit_hex_color() {
        assert!(validate_hex_color("#fff").is_err());
    }

    #[test]
    fn validate_hex_color_rejects_a_css_color_keyword() {
        assert!(validate_hex_color("blue").is_err());
    }

    #[test]
    fn validate_hex_color_rejects_an_oklch_color() {
        assert!(validate_hex_color("oklch(54% 0.2 350)").is_err());
    }

    #[test]
    fn resolve_project_color_falls_back_to_default_when_none() {
        assert_eq!(resolve_project_color(None).unwrap(), DEFAULT_PROJECT_COLOR);
    }

    #[test]
    fn resolve_project_color_accepts_a_valid_hex_color() {
        assert_eq!(
            resolve_project_color(Some("#abcdef".to_string())).unwrap(),
            "#abcdef"
        );
    }

    #[test]
    fn resolve_project_color_rejects_a_non_hex_color() {
        let result = resolve_project_color(Some("not-a-color".to_string()));

        assert!(result.is_err());
    }

    #[test]
    fn get_or_create_subtask_container_creates_one_on_first_call() {
        let mut task = Task::new("Fix the bug".to_string());
        task.project_id = Some("work".to_string());
        let mut projects = vec![Project::new("Work".to_string(), "#3b82f6".to_string(), 1)];

        let container = get_or_create_subtask_container(&mut task, &mut projects);

        assert_eq!(container.name, "Fix the bug");
        assert_eq!(container.parent_id, Some("work".to_string()));
        assert_eq!(task.subtask_project_id, Some(container.id.clone()));
        assert_eq!(projects.len(), 2);
        assert!(projects.iter().any(|p| p.id == container.id));
    }

    #[test]
    fn get_or_create_subtask_container_matches_the_parent_tasks_project_color() {
        let work = Project::new("Work".to_string(), "#bc267f".to_string(), 1);
        let mut task = Task::new("Fix the bug".to_string());
        task.project_id = Some(work.id.clone());
        let mut projects = vec![work];

        let container = get_or_create_subtask_container(&mut task, &mut projects);

        assert_eq!(container.color, "#bc267f");
    }

    #[test]
    fn get_or_create_subtask_container_falls_back_to_default_color_when_parent_project_is_missing()
    {
        let mut task = Task::new("Fix the bug".to_string());
        task.project_id = Some("does-not-exist".to_string());
        let mut projects = vec![];

        let container = get_or_create_subtask_container(&mut task, &mut projects);

        assert_eq!(container.color, DEFAULT_PROJECT_COLOR);
    }

    #[test]
    fn get_or_create_subtask_container_returns_the_existing_one_on_a_second_call() {
        let mut task = Task::new("Fix the bug".to_string());
        task.project_id = Some("work".to_string());
        let mut projects = vec![Project::new("Work".to_string(), "#3b82f6".to_string(), 1)];

        let first = get_or_create_subtask_container(&mut task, &mut projects);
        let second = get_or_create_subtask_container(&mut task, &mut projects);

        assert_eq!(first.id, second.id);
        assert_eq!(projects.len(), 2);
    }

    #[test]
    fn get_or_create_subtask_container_recreates_one_for_a_stale_pointer() {
        let mut task = Task::new("Fix the bug".to_string());
        task.project_id = Some("work".to_string());
        task.subtask_project_id = Some("already-deleted-container".to_string());
        let mut projects = vec![Project::new("Work".to_string(), "#3b82f6".to_string(), 1)];

        let container = get_or_create_subtask_container(&mut task, &mut projects);

        assert_ne!(container.id, "already-deleted-container");
        assert_eq!(task.subtask_project_id, Some(container.id));
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
    fn apply_project_update_rejects_changing_color_to_a_non_hex_value() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.color = "oklch(54% 0.2 350)".to_string();

        let result = apply_project_update(&mut projects, 0, update, &Settings::default());

        assert!(result.is_err());
    }

    #[test]
    fn apply_project_update_allows_an_unchanged_legacy_color_when_other_fields_change() {
        let mut projects = vec![Project::new(
            "Inbox".to_string(),
            "oklch(54% 0.2 350)".to_string(),
            1,
        )];
        let mut update = projects[0].clone();
        update.name = "Renamed".to_string();

        let updated = apply_project_update(&mut projects, 0, update, &Settings::default()).unwrap();

        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.color, "oklch(54% 0.2 350)");
    }

    #[test]
    fn apply_project_update_persists_board_and_defaults() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.board = ProjectBoard {
            statuses: vec!["backlog".to_string(), "done".to_string()],
            default_status: Some("backlog".to_string()),
            ..Default::default()
        };
        update.defaults = TaskDefaults {
            tags: vec!["home".to_string()],
            priority: Some("high".to_string()),
            status: None,
            due: Some("next_day".to_string()),
            scheduled: Some("in_1_week".to_string()),
            estimated_minutes: Some(20),
        };

        let updated =
            apply_project_update(&mut projects, 0, update.clone(), &Settings::default()).unwrap();

        assert_eq!(updated.board, update.board);
        assert_eq!(updated.defaults, update.defaults);
    }

    #[test]
    fn apply_project_update_persists_valid_card_and_bar_lightness_overrides() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.board = ProjectBoard {
            card_lightness: Some(0.65),
            bar_lightness: Some(0.2),
            ..Default::default()
        };

        let updated =
            apply_project_update(&mut projects, 0, update.clone(), &Settings::default()).unwrap();

        assert_eq!(updated.board.card_lightness, Some(0.65));
        assert_eq!(updated.board.bar_lightness, Some(0.2));
    }

    #[test]
    fn apply_project_update_rejects_an_out_of_range_card_lightness_override() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.board = ProjectBoard {
            card_lightness: Some(1.5),
            ..Default::default()
        };

        let result = apply_project_update(&mut projects, 0, update, &Settings::default());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("lightness"));
    }

    #[test]
    fn apply_project_update_rejects_an_out_of_range_bar_lightness_override() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.board = ProjectBoard {
            bar_lightness: Some(-0.1),
            ..Default::default()
        };

        let result = apply_project_update(&mut projects, 0, update, &Settings::default());

        assert!(result.is_err());
    }

    #[test]
    fn apply_project_update_persists_a_valid_ink_mode_override() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.board = ProjectBoard {
            ink_mode: Some("white".to_string()),
            ..Default::default()
        };

        let updated =
            apply_project_update(&mut projects, 0, update.clone(), &Settings::default()).unwrap();

        assert_eq!(updated.board.ink_mode, Some("white".to_string()));
    }

    #[test]
    fn apply_project_update_rejects_an_unknown_ink_mode_override() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.board = ProjectBoard {
            ink_mode: Some("sepia".to_string()),
            ..Default::default()
        };

        let result = apply_project_update(&mut projects, 0, update, &Settings::default());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ink mode"));
    }

    #[test]
    fn apply_project_update_allows_unset_ink_mode_to_inherit_the_global_default() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let update = projects[0].clone();

        let updated = apply_project_update(&mut projects, 0, update, &Settings::default()).unwrap();

        assert_eq!(updated.board.ink_mode, None);
    }

    #[test]
    fn apply_project_update_allows_unset_card_and_bar_lightness_to_inherit_the_global_default() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let update = projects[0].clone();

        let updated = apply_project_update(&mut projects, 0, update, &Settings::default()).unwrap();

        assert_eq!(updated.board.card_lightness, None);
        assert_eq!(updated.board.bar_lightness, None);
    }

    #[test]
    fn apply_project_update_rejects_unknown_status_in_board_statuses() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.board = ProjectBoard {
            statuses: vec!["backlog".to_string(), "on-hold".to_string()],
            default_status: None,
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
        let mut project = Project::new("Test".to_string(), "#111111".to_string(), 1);
        project.board = ProjectBoard {
            statuses: vec!["backlog".to_string(), "done".to_string()],
            default_status: Some("done".to_string()),
            ..Default::default()
        };
        let chain = vec![&project];

        assert_eq!(resolve_default_status(&settings, &chain), "done");
    }

    #[test]
    fn resolve_default_status_prefers_project_board_default_over_settings_defaults() {
        let mut settings = Settings::default();
        settings.defaults.status = Some("do".to_string());
        let mut project = Project::new("Test".to_string(), "#111111".to_string(), 1);
        project.board = ProjectBoard {
            statuses: vec!["done".to_string()],
            default_status: Some("done".to_string()),
            ..Default::default()
        };
        let chain = vec![&project];

        assert_eq!(resolve_default_status(&settings, &chain), "done");
    }

    #[test]
    fn resolve_default_status_falls_back_to_settings_defaults_when_board_default_is_invalid() {
        let mut settings = Settings::default();
        settings.defaults.status = Some("do".to_string());
        let mut project = Project::new("Test".to_string(), "#111111".to_string(), 1);
        project.board = ProjectBoard {
            statuses: vec!["backlog".to_string()],
            default_status: Some("nonexistent".to_string()),
            ..Default::default()
        };
        let chain = vec![&project];

        assert_eq!(resolve_default_status(&settings, &chain), "do");
    }

    #[test]
    fn resolve_default_status_falls_back_to_lowest_order_when_settings_default_is_invalid() {
        let mut settings = Settings::default();
        settings.defaults.status = Some("nonexistent".to_string());

        // Settings::default()'s "backlog" status has order 1.
        assert_eq!(resolve_default_status(&settings, &[]), "backlog");
    }

    #[test]
    fn resolve_default_status_falls_back_to_backlog_when_no_statuses_defined() {
        let settings = Settings {
            priorities: Vec::new(),
            statuses: Vec::new(),
            defaults: TaskDefaults::default(),
            ..Default::default()
        };

        assert_eq!(resolve_default_status(&settings, &[]), "backlog");
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
            ..Default::default()
        };

        assert_eq!(resolve_default_status(&settings, &[]), "now");
    }

    #[test]
    fn find_project_matches_by_id() {
        let mut project =
            Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        project.board.default_status = Some("done".to_string());
        let id = project.id.clone();
        let projects = vec![project];

        let found = find_project(&projects, &id).expect("should find a match");

        assert_eq!(found.board.default_status, Some("done".to_string()));
    }

    #[test]
    fn find_project_returns_none_when_no_project_matches() {
        let projects = vec![Project::new(
            "Homework".to_string(),
            DEFAULT_PROJECT_COLOR.to_string(),
            1,
        )];

        assert!(find_project(&projects, "does-not-exist").is_none());
    }

    #[test]
    fn resolve_project_id_returns_the_explicit_project_id() {
        let settings = Settings::default();

        let resolved = resolve_project_id(Some("homework-id".to_string()), &settings);

        assert_eq!(resolved, "homework-id");
    }

    #[test]
    fn resolve_project_id_falls_back_to_default_project_id_when_none() {
        let mut settings = Settings::default();
        settings.default_project_id = "inbox-id".to_string();

        let resolved = resolve_project_id(None, &settings);

        assert_eq!(resolved, "inbox-id");
    }

    /// `create_task` resolves the project id (falling back to
    /// `settings.default_project_id`) *before* looking it up with
    /// `find_project`, so a task created with no `+Project` token still picks
    /// up the default project's `TaskDefaults` and `ProjectBoard`.
    #[test]
    fn default_project_fallback_resolves_to_project_with_matching_defaults() {
        let mut project =
            Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        project.defaults.tags = vec!["school".to_string()];
        let settings = Settings {
            default_project_id: project.id.clone(),
            ..Default::default()
        };
        let projects = vec![project];

        let resolved_project_id = resolve_project_id(None, &settings);
        let matched = find_project(&projects, &resolved_project_id);

        assert_eq!(
            matched.map(|p| p.defaults.tags.clone()),
            Some(vec!["school".to_string()])
        );
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
    fn validate_settings_against_projects_rejects_a_default_project_that_does_not_exist() {
        let mut settings = Settings::default();
        settings.default_project_id = "does-not-exist".to_string();
        let projects = vec![Project::new("Inbox".to_string(), "#111111".to_string(), 1)];

        let result = validate_settings_against_projects(&settings, &projects);

        assert!(result.is_err());
    }

    #[test]
    fn validate_settings_against_projects_accepts_a_project_with_an_uncustomized_board() {
        let project = Project::new("Inbox".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        let mut settings = Settings::default();
        settings.default_project_id = project.id.clone();
        let projects = vec![project];

        assert!(validate_settings_against_projects(&settings, &projects).is_ok());
    }

    #[test]
    fn validate_settings_against_projects_rejects_removing_a_status_in_board_statuses() {
        let mut project =
            Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        project.board.statuses = vec!["backlog".to_string(), "blocked".to_string()];
        let mut settings = Settings::default();
        settings.default_project_id = project.id.clone();
        let projects = vec![project];

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
        let mut settings = Settings::default();
        settings.default_project_id = project.id.clone();
        let projects = vec![project];

        settings.statuses.retain(|status| status.id != "blocked");

        let err = validate_settings_against_projects(&settings, &projects).unwrap_err();
        assert!(err.contains("blocked"));
    }

    #[test]
    fn validate_settings_against_projects_rejects_removing_a_defaults_status_override() {
        let mut project =
            Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        project.defaults.status = Some("blocked".to_string());
        let mut settings = Settings::default();
        settings.default_project_id = project.id.clone();
        let projects = vec![project];

        settings.statuses.retain(|status| status.id != "blocked");

        let err = validate_settings_against_projects(&settings, &projects).unwrap_err();
        assert!(err.contains("blocked"));
    }

    #[test]
    fn validate_settings_against_projects_rejects_removing_a_defaults_priority_override() {
        let mut project =
            Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        project.defaults.priority = Some("low".to_string());
        let mut settings = Settings::default();
        settings.default_project_id = project.id.clone();
        let projects = vec![project];

        settings.priorities.retain(|priority| priority.id != "low");

        let err = validate_settings_against_projects(&settings, &projects).unwrap_err();
        assert!(err.contains("low"));
    }

    #[test]
    fn projects_to_delete_with_no_descendants_returns_just_the_target() {
        let project = Project::new("Solo".to_string(), "#111111".to_string(), 1);
        let id = project.id.clone();
        let projects = vec![project];

        let doomed = projects_to_delete(&projects, &id);

        assert_eq!(doomed.len(), 1);
        assert_eq!(doomed[0].id, id);
    }

    #[test]
    fn projects_to_delete_includes_every_descendant() {
        let mut parent = Project::new("Parent".to_string(), "#111111".to_string(), 1);
        parent.id = "parent".to_string();
        let mut child = Project::new("Child".to_string(), "#222222".to_string(), 2);
        child.id = "child".to_string();
        child.parent_id = Some("parent".to_string());
        let mut grandchild = Project::new("Grandchild".to_string(), "#333333".to_string(), 3);
        grandchild.id = "grandchild".to_string();
        grandchild.parent_id = Some("child".to_string());
        let mut unrelated = Project::new("Unrelated".to_string(), "#444444".to_string(), 4);
        unrelated.id = "unrelated".to_string();
        let projects = vec![parent, child, grandchild, unrelated];

        let doomed = projects_to_delete(&projects, "parent");

        let mut ids: Vec<&str> = doomed.iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["child", "grandchild", "parent"]);
    }

    #[test]
    fn projects_to_delete_for_a_missing_id_is_empty() {
        let projects = vec![Project::new("Solo".to_string(), "#111111".to_string(), 1)];

        let doomed = projects_to_delete(&projects, "does-not-exist");

        assert!(doomed.is_empty());
    }

    #[test]
    fn subtasks_to_reassign_and_container_to_remove_moves_every_subtask_to_the_target_project() {
        let mut parent = Task::new("Fix the bug".to_string());
        parent.id = "parent".to_string();
        parent.subtask_project_id = Some("container".to_string());
        let mut sub1 = Task::new("Reproduce it".to_string());
        sub1.project_id = Some("container".to_string());
        let mut sub2 = Task::new("Write the fix".to_string());
        sub2.project_id = Some("container".to_string());
        let mut unrelated = Task::new("Unrelated".to_string());
        unrelated.project_id = Some("somewhere-else".to_string());
        let tasks = vec![parent, sub1, sub2, unrelated];

        let (reassigned, container_id) =
            subtasks_to_reassign_and_container_to_remove(&tasks, "parent", "work");

        assert_eq!(reassigned.len(), 2);
        assert!(reassigned
            .iter()
            .all(|t| t.project_id == Some("work".to_string())));
        assert_eq!(container_id, Some("container".to_string()));
    }

    #[test]
    fn subtasks_to_reassign_and_container_to_remove_is_empty_for_a_task_with_no_container() {
        let task = Task::new("Plain task".to_string());
        let task_id = task.id.clone();
        let tasks = vec![task];

        let (reassigned, container_id) =
            subtasks_to_reassign_and_container_to_remove(&tasks, &task_id, "work");

        assert!(reassigned.is_empty());
        assert!(container_id.is_none());
    }

    #[test]
    fn subtasks_to_reassign_and_container_to_remove_for_a_missing_id_is_empty() {
        let (reassigned, container_id) =
            subtasks_to_reassign_and_container_to_remove(&[], "does-not-exist", "work");

        assert!(reassigned.is_empty());
        assert!(container_id.is_none());
    }

    #[test]
    fn force_done_except_cancelled_completes_every_outstanding_subtask() {
        let mut backlog = Task::new("Reproduce it".to_string());
        backlog.status = "backlog".to_string();
        let mut in_progress = Task::new("Write the fix".to_string());
        in_progress.status = "in_progress".to_string();

        let result =
            force_done_except_cancelled(vec![backlog, in_progress], "done", Some("cancelled"));

        assert!(result.iter().all(|t| t.status == "done"));
    }

    #[test]
    fn force_done_except_cancelled_leaves_cancelled_subtasks_untouched() {
        let mut cancelled = Task::new("Abandoned".to_string());
        cancelled.status = "cancelled".to_string();

        let result = force_done_except_cancelled(vec![cancelled], "done", Some("cancelled"));

        assert_eq!(result[0].status, "cancelled");
    }

    #[test]
    fn force_done_except_cancelled_completes_every_pending_occurrence_of_a_recurring_subtask() {
        let mut today = Task::new("Hi".to_string());
        today.series_id = Some("hi-series".to_string());
        today.status = "backlog".to_string();
        let mut future = Task::new("Hi".to_string());
        future.series_id = Some("hi-series".to_string());
        future.status = "backlog".to_string();

        let result = force_done_except_cancelled(vec![today, future], "done", Some("cancelled"));

        assert!(result.iter().all(|t| t.status == "done"));
    }

    #[test]
    fn force_done_except_cancelled_works_with_no_configured_cancelled_status() {
        let mut backlog = Task::new("Reproduce it".to_string());
        backlog.status = "backlog".to_string();

        let result = force_done_except_cancelled(vec![backlog], "done", None);

        assert_eq!(result[0].status, "done");
    }

    #[test]
    fn subtasks_and_container_to_delete_finds_every_subtask_and_the_container() {
        let mut container = Project::new("Fix the bug".to_string(), "#3b82f6".to_string(), 1);
        container.id = "container".to_string();
        let mut parent = Task::new("Fix the bug".to_string());
        parent.id = "parent".to_string();
        parent.subtask_project_id = Some("container".to_string());
        let mut sub1 = Task::new("Reproduce it".to_string());
        sub1.project_id = Some("container".to_string());
        let mut sub2 = Task::new("Write the fix".to_string());
        sub2.project_id = Some("container".to_string());
        let mut unrelated = Task::new("Unrelated".to_string());
        unrelated.project_id = Some("somewhere-else".to_string());
        let tasks = vec![parent, sub1, sub2, unrelated];
        let projects = vec![container];

        let (subtasks, container) = subtasks_and_container_to_delete(&tasks, &projects, "parent");

        assert_eq!(subtasks.len(), 2);
        assert!(container.is_some());
        assert_eq!(container.unwrap().id, "container");
    }

    #[test]
    fn subtasks_and_container_to_delete_is_empty_for_a_task_with_no_container() {
        let task = Task::new("Plain task".to_string());
        let task_id = task.id.clone();
        let tasks = vec![task];

        let (subtasks, container) = subtasks_and_container_to_delete(&tasks, &[], &task_id);

        assert!(subtasks.is_empty());
        assert!(container.is_none());
    }

    #[test]
    fn subtasks_and_container_to_delete_for_a_missing_id_is_empty() {
        let (subtasks, container) = subtasks_and_container_to_delete(&[], &[], "does-not-exist");

        assert!(subtasks.is_empty());
        assert!(container.is_none());
    }

    #[test]
    fn owning_task_if_container_now_empty_finds_the_owner_when_no_siblings_remain() {
        let mut owner = Task::new("Fix the bug".to_string());
        owner.id = "owner".to_string();
        owner.subtask_project_id = Some("container".to_string());
        let mut deleted = Task::new("Last subtask".to_string());
        deleted.id = "deleted".to_string();
        deleted.project_id = Some("container".to_string());
        let tasks = vec![owner, deleted];

        let result = owning_task_if_container_now_empty(&tasks, "container", "deleted");

        assert_eq!(result.map(|t| t.id.as_str()), Some("owner"));
    }

    #[test]
    fn owning_task_if_container_now_empty_is_none_when_siblings_remain() {
        let mut owner = Task::new("Fix the bug".to_string());
        owner.id = "owner".to_string();
        owner.subtask_project_id = Some("container".to_string());
        let mut deleted = Task::new("One of two".to_string());
        deleted.id = "deleted".to_string();
        deleted.project_id = Some("container".to_string());
        let mut sibling = Task::new("Still here".to_string());
        sibling.id = "sibling".to_string();
        sibling.project_id = Some("container".to_string());
        let tasks = vec![owner, deleted, sibling];

        let result = owning_task_if_container_now_empty(&tasks, "container", "deleted");

        assert!(result.is_none());
    }

    #[test]
    fn owning_task_if_container_now_empty_is_none_for_a_normal_task() {
        let mut deleted = Task::new("Just a task".to_string());
        deleted.id = "deleted".to_string();
        deleted.project_id = Some("some-project".to_string());
        let tasks = vec![deleted];

        let result = owning_task_if_container_now_empty(&tasks, "some-project", "deleted");

        assert!(result.is_none());
    }

    #[test]
    fn tasks_for_projects_matches_tasks_filed_under_the_given_id() {
        let mut homework = Task::new("Algebra".to_string());
        homework.project_id = Some("homework-id".to_string());
        let mut other = Task::new("Groceries".to_string());
        other.project_id = Some("errands-id".to_string());
        let unfiled = Task::new("No project".to_string());

        let tasks = vec![homework, other, unfiled];
        let matching = tasks_for_projects(&tasks, &["homework-id".to_string()]);

        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].title, "Algebra");
    }

    #[test]
    fn tasks_for_projects_matches_any_of_several_ids() {
        let mut homework = Task::new("Read chapter 1".to_string());
        homework.project_id = Some("homework-id".to_string());
        let mut chores = Task::new("Clean room".to_string());
        chores.project_id = Some("chores-id".to_string());
        let mut other = Task::new("Plan trip".to_string());
        other.project_id = Some("vacation-id".to_string());
        let tasks = vec![homework, chores, other];

        let matching = tasks_for_projects(
            &tasks,
            &["homework-id".to_string(), "chores-id".to_string()],
        );

        assert_eq!(matching.len(), 2);
    }

    #[test]
    fn ensure_not_default_project_rejects_the_default_project() {
        let project = Project::new("General".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        let settings = Settings {
            default_project_id: project.id.clone(),
            ..Default::default()
        };

        let err = ensure_not_default_project(&project, &settings).unwrap_err();

        assert!(err.contains("General"));
    }

    #[test]
    fn ensure_not_default_project_allows_other_projects() {
        let settings = Settings {
            default_project_id: "some-other-id".to_string(),
            ..Default::default()
        };
        let project = Project::new("Work".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);

        assert!(ensure_not_default_project(&project, &settings).is_ok());
    }

    #[test]
    fn apply_task_strategy_reassign_updates_task_project() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Algebra".to_string());
        task.project_id = Some("homework-id".to_string());
        storage::save_task(dir.path(), &task).unwrap();

        let source = Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        let target = Project::new("School".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 2);
        let target_id = target.id.clone();
        let source_id = source.id.clone();
        let projects = vec![source, target];

        let archive_dir = dir.path().join("archive");
        let affected = apply_task_strategy(
            dir.path(),
            &archive_dir,
            &projects,
            &[source_id.clone()],
            &[&task],
            &ProjectTaskStrategy::Reassign {
                target_project_id: target_id.clone(),
            },
        )
        .unwrap();

        assert_eq!(affected, 1);
        let loaded = storage::load_task(dir.path(), &task.id).unwrap();
        assert_eq!(loaded.project_id, Some(target_id));
    }

    #[test]
    fn apply_task_strategy_reassign_rejects_self_target() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Algebra".to_string());
        task.project_id = Some("homework-id".to_string());
        storage::save_task(dir.path(), &task).unwrap();

        let source = Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        let source_id = source.id.clone();
        let projects = vec![source];

        let archive_dir = dir.path().join("archive");
        let err = apply_task_strategy(
            dir.path(),
            &archive_dir,
            &projects,
            &[source_id.clone()],
            &[&task],
            &ProjectTaskStrategy::Reassign {
                target_project_id: source_id.clone(),
            },
        )
        .unwrap_err();

        assert!(err.contains("being deleted"));
    }

    #[test]
    fn apply_task_strategy_reassign_rejects_unknown_target() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Algebra".to_string());
        task.project_id = Some("homework-id".to_string());
        storage::save_task(dir.path(), &task).unwrap();

        let source = Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        let source_id = source.id.clone();
        let projects = vec![source];

        let archive_dir = dir.path().join("archive");
        let err = apply_task_strategy(
            dir.path(),
            &archive_dir,
            &projects,
            &[source_id.clone()],
            &[&task],
            &ProjectTaskStrategy::Reassign {
                target_project_id: "missing-id".to_string(),
            },
        )
        .unwrap_err();

        assert!(err.contains("not found"));
    }

    #[test]
    fn apply_task_strategy_archive_moves_task_files() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Algebra".to_string());
        task.project_id = Some("homework-id".to_string());
        storage::save_task(dir.path(), &task).unwrap();

        let source = Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        let source_id = source.id.clone();
        let projects = vec![source];

        let archive_dir = dir.path().join("archive");
        let affected = apply_task_strategy(
            dir.path(),
            &archive_dir,
            &projects,
            &[source_id.clone()],
            &[&task],
            &ProjectTaskStrategy::Archive,
        )
        .unwrap();

        assert_eq!(affected, 1);
        assert!(matches!(
            storage::load_task(dir.path(), &task.id),
            Err(storage::StorageError::NotFound(_))
        ));
        let archived = storage::load_task(&archive_dir, &task.id).unwrap();
        assert_eq!(archived.id, task.id);
    }

    #[test]
    fn apply_task_strategy_delete_removes_task_files() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Algebra".to_string());
        task.project_id = Some("homework-id".to_string());
        storage::save_task(dir.path(), &task).unwrap();

        let source = Project::new("Homework".to_string(), DEFAULT_PROJECT_COLOR.to_string(), 1);
        let source_id = source.id.clone();
        let projects = vec![source];

        let archive_dir = dir.path().join("archive");
        let affected = apply_task_strategy(
            dir.path(),
            &archive_dir,
            &projects,
            &[source_id.clone()],
            &[&task],
            &ProjectTaskStrategy::Delete,
        )
        .unwrap();

        assert_eq!(affected, 1);
        assert!(matches!(
            storage::load_task(dir.path(), &task.id),
            Err(storage::StorageError::NotFound(_))
        ));
    }

    #[test]
    fn apply_task_strategy_rejects_reassigning_to_any_doomed_descendant() {
        let dir = tempdir().unwrap();
        let archive_dir = dir.path().join("archive");
        let target = Project::new("Descendant".to_string(), "#111111".to_string(), 1);
        let target_id = target.id.clone();
        let projects = vec![target];
        let task = Task::new("Demo".to_string());
        let tasks: Vec<&Task> = vec![&task];

        let result = apply_task_strategy(
            dir.path(),
            &archive_dir,
            &projects,
            &["parent-id".to_string(), target_id.clone()],
            &tasks,
            &ProjectTaskStrategy::Reassign {
                target_project_id: target_id,
            },
        );

        assert!(result.is_err());
    }

    #[test]
    fn is_finished_matches_done_status() {
        let settings = Settings::default();
        let mut task = Task::new("Wrap up".to_string());
        task.status = "done".to_string();

        assert!(is_finished(&task, &settings));
    }

    #[test]
    fn is_finished_matches_cancelled_status_when_set() {
        let settings = Settings {
            cancelled_status: Some("cancelled".to_string()),
            ..Default::default()
        };
        let mut task = Task::new("Drop it".to_string());
        task.status = "cancelled".to_string();

        assert!(is_finished(&task, &settings));
    }

    #[test]
    fn is_finished_false_for_other_statuses() {
        let settings = Settings::default();
        let mut task = Task::new("Still going".to_string());
        task.status = "in-progress".to_string();

        assert!(!is_finished(&task, &settings));
    }

    #[test]
    fn is_finished_false_for_cancelled_status_string_when_none_configured() {
        let settings = Settings {
            cancelled_status: None,
            ..Default::default()
        };
        let mut task = Task::new("Edge case".to_string());
        task.status = "cancelled".to_string();

        assert!(!is_finished(&task, &settings));
    }

    #[test]
    fn tasks_and_containers_to_finish_cascades_a_finished_parents_subtasks_regardless_of_their_own_status(
    ) {
        let settings = Settings::default();
        let mut parent = Task::new("Fix the bug".to_string());
        parent.status = "done".to_string();
        parent.subtask_project_id = Some("container".to_string());
        let mut subtask_done = Task::new("Write the test".to_string());
        subtask_done.project_id = Some("container".to_string());
        subtask_done.status = "done".to_string();
        let mut subtask_not_done = Task::new("Deploy the fix".to_string());
        subtask_not_done.project_id = Some("container".to_string());
        subtask_not_done.status = "backlog".to_string();
        let tasks = vec![
            parent.clone(),
            subtask_done.clone(),
            subtask_not_done.clone(),
        ];

        let (task_ids, container_ids) = tasks_and_containers_to_finish(&tasks, &settings);

        assert_eq!(
            task_ids
                .into_iter()
                .collect::<std::collections::HashSet<_>>(),
            [parent.id, subtask_done.id, subtask_not_done.id]
                .into_iter()
                .collect(),
        );
        assert_eq!(container_ids, vec!["container".to_string()]);
    }

    #[test]
    fn tasks_and_containers_to_finish_does_not_cascade_for_a_task_with_no_container() {
        let settings = Settings::default();
        let mut task = Task::new("Plain finished task".to_string());
        task.status = "done".to_string();
        let tasks = vec![task.clone()];

        let (task_ids, container_ids) = tasks_and_containers_to_finish(&tasks, &settings);

        assert_eq!(task_ids, vec![task.id]);
        assert!(container_ids.is_empty());
    }

    #[test]
    fn tasks_and_containers_to_finish_leaves_an_unfinished_parents_subtasks_alone() {
        let settings = Settings::default();
        let mut parent = Task::new("Still in progress".to_string());
        parent.status = "backlog".to_string();
        parent.subtask_project_id = Some("container".to_string());
        let mut subtask = Task::new("A subtask".to_string());
        subtask.project_id = Some("container".to_string());
        subtask.status = "backlog".to_string();
        let tasks = vec![parent, subtask];

        let (task_ids, container_ids) = tasks_and_containers_to_finish(&tasks, &settings);

        assert!(task_ids.is_empty());
        assert!(container_ids.is_empty());
    }

    #[test]
    fn tasks_and_containers_to_finish_does_not_duplicate_a_subtask_thats_independently_finished() {
        let settings = Settings::default();
        let mut parent = Task::new("Fix the bug".to_string());
        parent.status = "done".to_string();
        parent.subtask_project_id = Some("container".to_string());
        let mut subtask = Task::new("Write the test".to_string());
        subtask.project_id = Some("container".to_string());
        subtask.status = "done".to_string();
        let tasks = vec![parent.clone(), subtask.clone()];

        let (task_ids, _) = tasks_and_containers_to_finish(&tasks, &settings);

        assert_eq!(task_ids.len(), 2);
        assert!(task_ids.contains(&parent.id));
        assert!(task_ids.contains(&subtask.id));
    }

    #[test]
    fn linked_subtask_series_ids_finds_a_subtasks_series_under_a_recurring_parent() {
        let mut parent = Task::new("Weekly report".to_string());
        parent.series_id = Some("parent-series".to_string());
        parent.subtask_project_id = Some("container".to_string());
        let mut subtask = Task::new("Draft section".to_string());
        subtask.project_id = Some("container".to_string());
        subtask.series_id = Some("subtask-series".to_string());
        let tasks = vec![parent, subtask];

        let ids = linked_subtask_series_ids(&tasks, "parent-series");

        assert_eq!(ids, vec!["subtask-series".to_string()]);
    }

    #[test]
    fn linked_subtask_series_ids_checks_every_occurrence_of_the_parent_series() {
        let mut occurrence_a = Task::new("Weekly report".to_string());
        occurrence_a.series_id = Some("parent-series".to_string());
        occurrence_a.subtask_project_id = Some("container-a".to_string());
        let mut subtask_a = Task::new("Draft section".to_string());
        subtask_a.project_id = Some("container-a".to_string());
        subtask_a.series_id = Some("series-a".to_string());

        let mut occurrence_b = Task::new("Weekly report".to_string());
        occurrence_b.series_id = Some("parent-series".to_string());
        occurrence_b.subtask_project_id = Some("container-b".to_string());
        let mut subtask_b = Task::new("Review".to_string());
        subtask_b.project_id = Some("container-b".to_string());
        subtask_b.series_id = Some("series-b".to_string());

        let tasks = vec![occurrence_a, subtask_a, occurrence_b, subtask_b];

        let ids = linked_subtask_series_ids(&tasks, "parent-series");

        assert_eq!(
            ids.into_iter().collect::<std::collections::HashSet<_>>(),
            ["series-a".to_string(), "series-b".to_string()]
                .into_iter()
                .collect(),
        );
    }

    #[test]
    fn linked_subtask_series_ids_ignores_a_non_recurring_subtask() {
        let mut parent = Task::new("Weekly report".to_string());
        parent.series_id = Some("parent-series".to_string());
        parent.subtask_project_id = Some("container".to_string());
        let mut subtask = Task::new("Draft section".to_string());
        subtask.project_id = Some("container".to_string());
        // No series_id — an ordinary, non-recurring subtask.
        let tasks = vec![parent, subtask];

        let ids = linked_subtask_series_ids(&tasks, "parent-series");

        assert!(ids.is_empty());
    }

    #[test]
    fn linked_subtask_series_ids_is_empty_for_a_parent_with_no_container() {
        let mut parent = Task::new("Weekly report".to_string());
        parent.series_id = Some("parent-series".to_string());
        let tasks = vec![parent];

        let ids = linked_subtask_series_ids(&tasks, "parent-series");

        assert!(ids.is_empty());
    }

    #[test]
    fn linked_subtask_series_ids_deduplicates_when_multiple_subtasks_share_a_series_id() {
        let mut parent = Task::new("Weekly report".to_string());
        parent.series_id = Some("parent-series".to_string());
        parent.subtask_project_id = Some("container".to_string());
        let mut subtask_one = Task::new("Draft section".to_string());
        subtask_one.project_id = Some("container".to_string());
        subtask_one.series_id = Some("shared-series".to_string());
        let mut subtask_two = Task::new("Draft section".to_string());
        subtask_two.project_id = Some("container".to_string());
        subtask_two.series_id = Some("shared-series".to_string());
        let tasks = vec![parent, subtask_one, subtask_two];

        let ids = linked_subtask_series_ids(&tasks, "parent-series");

        assert_eq!(ids, vec!["shared-series".to_string()]);
    }

    fn new_test_series(frequency: crate::series::RecurrenceFrequency, anchor_date: &str) -> Series {
        let mut series = Series::new(
            frequency,
            anchor_date.to_string(),
            None,
            DueRule::DefaultCode {
                code: "next_day".to_string(),
            },
            "Water the plants".to_string(),
            Some("Home".to_string()),
            "medium".to_string(),
            vec!["chore".to_string()],
            Some(15),
            String::new(),
        );
        series.generated_until = anchor_date.to_string();
        series
    }

    fn edited_occurrence_task() -> Task {
        let mut task = Task::new("Water the ferns".to_string());
        task.project_id = Some("Garden".to_string());
        task.priority = "high".to_string();
        task.status = "in-progress".to_string();
        task.tags = vec!["urgent".to_string()];
        task.estimated_minutes = Some(20);
        task.notes = "Use the green watering can".to_string();
        task.scheduled = Some("2026-06-20".to_string());
        task.due = Some("2026-06-21".to_string());
        task.series_id = Some("series-abc".to_string());
        task
    }

    #[test]
    fn apply_series_template_update_copies_shared_fields_only() {
        let mut series = new_test_series(
            crate::series::RecurrenceFrequency::EveryNDays { interval: 1 },
            "2026-06-15",
        );
        let edited = edited_occurrence_task();

        apply_series_template_update(&mut series, &edited);

        assert_eq!(series.title, "Water the ferns");
        assert_eq!(series.project_id, Some("Garden".to_string()));
        assert_eq!(series.priority, "high");
        assert_eq!(series.tags, vec!["urgent".to_string()]);
        assert_eq!(series.estimated_minutes, Some(20));
        assert_eq!(series.notes, "Use the green watering can");
        // Unaffected: the series' own date/rule fields aren't touched by a
        // template update, since status/due/scheduled are per-occurrence.
        assert_eq!(series.anchor_date, "2026-06-15");
    }

    #[test]
    fn apply_template_to_occurrence_copies_shared_fields_but_not_status_or_dates() {
        let mut occurrence = Task::new("Old title".to_string());
        occurrence.status = "done".to_string();
        occurrence.scheduled = Some("2026-07-01".to_string());
        occurrence.due = Some("2026-07-02".to_string());
        let edited = edited_occurrence_task();

        apply_template_to_occurrence(&mut occurrence, &edited);

        assert_eq!(occurrence.title, "Water the ferns");
        assert_eq!(occurrence.project_id, Some("Garden".to_string()));
        assert_eq!(occurrence.priority, "high");
        assert_eq!(occurrence.tags, vec!["urgent".to_string()]);
        assert_eq!(occurrence.estimated_minutes, Some(20));
        assert_eq!(occurrence.notes, "Use the green watering can");
        // Untouched: status/scheduled/due stay this occurrence's own.
        assert_eq!(occurrence.status, "done");
        assert_eq!(occurrence.scheduled, Some("2026-07-01".to_string()));
        assert_eq!(occurrence.due, Some("2026-07-02".to_string()));
    }

    fn series_occurrence(id: &str, series_id: &str, scheduled: &str) -> Task {
        let mut task = Task::new("Water the plants".to_string());
        task.id = id.to_string();
        task.series_id = Some(series_id.to_string());
        task.scheduled = Some(scheduled.to_string());
        task
    }

    #[test]
    fn future_occurrences_includes_tasks_on_or_after_the_cutoff() {
        let tasks = vec![
            series_occurrence("a", "series-1", "2026-06-19"),
            series_occurrence("b", "series-1", "2026-06-20"),
            series_occurrence("c", "series-1", "2026-06-21"),
        ];

        let result = future_occurrences(&tasks, "series-1", None, "2026-06-20");

        let ids: Vec<&str> = result.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "c"]);
    }

    #[test]
    fn future_occurrences_excludes_tasks_before_the_cutoff() {
        let tasks = vec![series_occurrence("a", "series-1", "2026-06-19")];

        let result = future_occurrences(&tasks, "series-1", None, "2026-06-20");

        assert!(result.is_empty());
    }

    #[test]
    fn future_occurrences_excludes_tasks_from_a_different_series() {
        let tasks = vec![series_occurrence("a", "series-2", "2026-06-25")];

        let result = future_occurrences(&tasks, "series-1", None, "2026-06-20");

        assert!(result.is_empty());
    }

    #[test]
    fn future_occurrences_excludes_tasks_with_no_series_id() {
        let mut task = Task::new("Unrelated task".to_string());
        task.scheduled = Some("2026-06-25".to_string());
        let tasks = vec![task];

        let result = future_occurrences(&tasks, "series-1", None, "2026-06-20");

        assert!(result.is_empty());
    }

    #[test]
    fn future_occurrences_excludes_tasks_with_no_scheduled_date() {
        let mut task = Task::new("No date".to_string());
        task.series_id = Some("series-1".to_string());
        let tasks = vec![task];

        let result = future_occurrences(&tasks, "series-1", None, "2026-06-20");

        assert!(result.is_empty());
    }

    #[test]
    fn future_occurrences_excludes_the_given_exclude_id_even_if_otherwise_matching() {
        let tasks = vec![series_occurrence("a", "series-1", "2026-06-25")];

        let result = future_occurrences(&tasks, "series-1", Some("a"), "2026-06-20");

        assert!(result.is_empty());
    }

    #[test]
    fn future_occurrences_includes_a_task_exactly_at_the_cutoff_when_not_excluded() {
        // Mirrors `delete_series_occurrence`'s use: the task that originated the
        // "this and future" request has `scheduled == cutoff` and must still be
        // included, since deleting "this and future" deletes that occurrence too.
        let tasks = vec![series_occurrence("a", "series-1", "2026-06-20")];

        let result = future_occurrences(&tasks, "series-1", None, "2026-06-20");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "a");
    }

    #[test]
    fn build_series_occurrence_copies_template_fields() {
        let series = new_test_series(
            crate::series::RecurrenceFrequency::EveryNDays { interval: 1 },
            "2026-06-15",
        );
        let settings = Settings::default();
        let date = chrono::NaiveDate::from_ymd_opt(2026, 6, 16).unwrap();

        let task = build_series_occurrence(&series, &settings, &[], date);

        assert_eq!(task.title, "Water the plants");
        assert_eq!(task.project_id, Some("Home".to_string()));
        assert_eq!(task.priority, "medium");
        assert_eq!(task.tags, vec!["chore".to_string()]);
        assert_eq!(task.estimated_minutes, Some(15));
        assert_eq!(task.scheduled, Some("2026-06-16".to_string()));
        assert_eq!(task.series_id, Some(series.id.clone()));
    }

    #[test]
    fn build_series_occurrence_resolves_due_relative_to_its_own_date_not_the_anchor() {
        let series = new_test_series(
            crate::series::RecurrenceFrequency::EveryNDays { interval: 1 },
            "2026-06-15",
        );
        let settings = Settings::default();
        let date = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();

        let task = build_series_occurrence(&series, &settings, &[], date);

        // due_rule is DefaultCode("next_day"), so due should be one day after *this*
        // occurrence's own scheduled date (06-21), not one day after the series'
        // anchor (06-16).
        assert_eq!(task.due, Some("2026-06-21".to_string()));
    }

    #[test]
    fn build_series_occurrence_has_no_due_when_the_series_due_rule_is_never() {
        let mut series = new_test_series(
            crate::series::RecurrenceFrequency::EveryNDays { interval: 1 },
            "2026-06-15",
        );
        series.due_rule = DueRule::Never;
        let settings = Settings::default();
        let date = chrono::NaiveDate::from_ymd_opt(2026, 6, 16).unwrap();

        let task = build_series_occurrence(&series, &settings, &[], date);

        assert_eq!(task.due, None);
    }

    #[test]
    fn build_series_occurrence_uses_default_status_resolution_not_a_template_status() {
        let series = new_test_series(
            crate::series::RecurrenceFrequency::EveryNDays { interval: 1 },
            "2026-06-15",
        );
        let settings = Settings::default();
        let date = chrono::NaiveDate::from_ymd_opt(2026, 6, 16).unwrap();

        let task = build_series_occurrence(&series, &settings, &[], date);

        assert_eq!(task.status, resolve_default_status(&settings, &[]));
    }

    #[test]
    fn generate_series_occurrences_creates_and_saves_each_occurrence() {
        let dir = tempdir().unwrap();
        let settings = Settings::default();
        let mut series = new_test_series(
            crate::series::RecurrenceFrequency::EveryNDays { interval: 1 },
            "2026-06-15",
        );
        let through = chrono::NaiveDate::from_ymd_opt(2026, 6, 18).unwrap();

        let created =
            generate_series_occurrences(dir.path(), &settings, &[], &mut series, through).unwrap();

        assert_eq!(created.len(), 3);
        let saved = storage::list_tasks(dir.path()).unwrap();
        assert_eq!(saved.len(), 3);
        assert_eq!(series.generated_until, "2026-06-18");
    }

    #[test]
    fn generate_series_occurrences_resumes_from_the_existing_watermark() {
        let dir = tempdir().unwrap();
        let settings = Settings::default();
        let mut series = new_test_series(
            crate::series::RecurrenceFrequency::EveryNDays { interval: 1 },
            "2026-06-15",
        );
        series.generated_until = "2026-06-17".to_string();
        let through = chrono::NaiveDate::from_ymd_opt(2026, 6, 19).unwrap();

        let created =
            generate_series_occurrences(dir.path(), &settings, &[], &mut series, through).unwrap();

        assert_eq!(created.len(), 2);
        assert_eq!(created[0].scheduled, Some("2026-06-18".to_string()));
        assert_eq!(created[1].scheduled, Some("2026-06-19".to_string()));
    }

    #[test]
    fn generate_series_occurrences_advances_generated_until_to_the_horizon_when_nothing_is_generated(
    ) {
        let dir = tempdir().unwrap();
        let settings = Settings::default();
        let mut series = new_test_series(
            crate::series::RecurrenceFrequency::EveryNDays { interval: 10 },
            "2026-06-15",
        );
        // Nothing falls in (06-15, 06-17] for an every-10-days rule.
        let through = chrono::NaiveDate::from_ymd_opt(2026, 6, 17).unwrap();

        let created =
            generate_series_occurrences(dir.path(), &settings, &[], &mut series, through).unwrap();

        assert!(created.is_empty());
        assert_eq!(series.generated_until, "2026-06-17");
    }

    #[test]
    fn generate_series_occurrences_clamps_generated_until_to_the_series_end_date() {
        let dir = tempdir().unwrap();
        let settings = Settings::default();
        let mut series = new_test_series(
            crate::series::RecurrenceFrequency::EveryNDays { interval: 1 },
            "2026-06-15",
        );
        series.end_date = Some("2026-06-16".to_string());
        let through = chrono::NaiveDate::from_ymd_opt(2026, 6, 30).unwrap();

        let created =
            generate_series_occurrences(dir.path(), &settings, &[], &mut series, through).unwrap();

        assert_eq!(created.len(), 1);
        assert_eq!(series.generated_until, "2026-06-16");
    }

    #[test]
    fn generate_series_occurrences_rejects_an_invalid_generated_until() {
        let dir = tempdir().unwrap();
        let settings = Settings::default();
        let mut series = new_test_series(
            crate::series::RecurrenceFrequency::EveryNDays { interval: 1 },
            "2026-06-15",
        );
        series.generated_until = "not-a-date".to_string();
        let through = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();

        let result = generate_series_occurrences(dir.path(), &settings, &[], &mut series, through);

        assert!(result.is_err());
    }

    #[test]
    fn generate_series_occurrences_never_moves_the_watermark_backward_when_through_is_already_covered(
    ) {
        // Regression test: a real user-reported bug where switching between Week and
        // Calendar view (each asking to "ensure occurrences through" a different,
        // often much nearer-term date than what a prior call had already generated)
        // rewound `generated_until` backward, causing the *next* call to think the
        // already-generated range still needed generating — producing duplicate
        // occurrences that multiplied every time a view was switched.
        let dir = tempdir().unwrap();
        let settings = Settings::default();
        let mut series = new_test_series(
            crate::series::RecurrenceFrequency::EveryNDays { interval: 1 },
            "2026-06-15",
        );

        // First call: a wide baseline window, as `create_recurring_task` does.
        let baseline_horizon = chrono::NaiveDate::from_ymd_opt(2026, 8, 14).unwrap();
        let baseline_created =
            generate_series_occurrences(dir.path(), &settings, &[], &mut series, baseline_horizon)
                .unwrap();
        assert_eq!(baseline_created.len(), 60);
        assert_eq!(series.generated_until, "2026-08-14");

        // Second call: a much nearer-term "through", as Week view's initial mount does.
        let near_term_through = chrono::NaiveDate::from_ymd_opt(2026, 6, 22).unwrap();
        let second_call_created =
            generate_series_occurrences(dir.path(), &settings, &[], &mut series, near_term_through)
                .unwrap();

        assert!(second_call_created.is_empty());
        assert_eq!(series.generated_until, "2026-08-14");

        // Third call: back to (or past) the original wide horizon — must still be a
        // no-op, not a re-generation of the range the buggy version would have
        // forgotten about after the second call rewound the watermark.
        let third_call_created =
            generate_series_occurrences(dir.path(), &settings, &[], &mut series, baseline_horizon)
                .unwrap();

        assert!(third_call_created.is_empty());
        let saved = storage::list_tasks(dir.path()).unwrap();
        assert_eq!(saved.len(), 60);
    }

    #[test]
    fn validate_parent_id_accepts_none() {
        let projects = vec![Project::new("Inbox".to_string(), "#111111".to_string(), 1)];

        assert!(validate_parent_id(&projects, None, None).is_ok());
    }

    #[test]
    fn validate_parent_id_accepts_an_existing_project() {
        let parent = Project::new("Inbox".to_string(), "#111111".to_string(), 1);
        let parent_id = parent.id.clone();
        let projects = vec![parent];

        assert!(validate_parent_id(&projects, Some(&parent_id), None).is_ok());
    }

    #[test]
    fn validate_parent_id_rejects_a_missing_project() {
        let projects = vec![Project::new("Inbox".to_string(), "#111111".to_string(), 1)];

        let result = validate_parent_id(&projects, Some("does-not-exist"), None);

        assert!(result.is_err());
    }

    #[test]
    fn validate_parent_id_rejects_a_self_cycle_on_update() {
        let project = Project::new("Inbox".to_string(), "#111111".to_string(), 1);
        let id = project.id.clone();
        let projects = vec![project];

        let result = validate_parent_id(&projects, Some(&id), Some(&id));

        assert!(result.is_err());
    }

    #[test]
    fn validate_parent_id_rejects_moving_under_own_descendant() {
        let mut parent = Project::new("Parent".to_string(), "#111111".to_string(), 1);
        parent.id = "parent".to_string();
        let mut child = Project::new("Child".to_string(), "#222222".to_string(), 2);
        child.id = "child".to_string();
        child.parent_id = Some("parent".to_string());
        let projects = vec![parent, child];

        let result = validate_parent_id(&projects, Some("child"), Some("parent"));

        assert!(result.is_err());
    }

    #[test]
    fn validate_parent_id_allows_an_unrelated_new_parent_on_update() {
        let mut a = Project::new("A".to_string(), "#111111".to_string(), 1);
        a.id = "a".to_string();
        let mut b = Project::new("B".to_string(), "#222222".to_string(), 2);
        b.id = "b".to_string();
        let projects = vec![a, b];

        assert!(validate_parent_id(&projects, Some("b"), Some("a")).is_ok());
    }
}
