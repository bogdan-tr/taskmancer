use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::layout::{self, StatLayout};
use crate::layout_storage;
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
use crate::status_stats;
use crate::status_tier::{self, StatusTier};
use crate::storage;
use crate::task::Task;
use crate::time_storage;
use crate::time_tracking;

/// How many days into the future a newly created series generates
/// occurrences immediately, before any scroll-triggered extension —
/// the user's chosen baseline window.
const RECURRENCE_BASELINE_LOOKAHEAD_DAYS: i64 = 60;

/// Shared application state holding the directory where task markdown
/// files are stored, the directory where archived task markdown files are
/// moved (see [`finish_day`] and [`delete_project`]), the file where project
/// metadata is stored, the file where global settings are stored, the file
/// where recurrence series are stored, and the file where status-line/
/// dashboard `StatLayout`s are stored (see `crate::layout`).
///
/// `projects_lock` serializes the read-modify-write cycles in
/// [`list_projects`] (which may backfill), [`create_project`],
/// [`update_project`], and [`delete_project`] so concurrent commands can't
/// read a stale project list and overwrite each other's changes.
/// `series_lock` does the same for series read-modify-write cycles.
///
/// `time_db` is the SQLite connection (see `crate::time_storage`) backing
/// the time-tracking engine's `time_entries` table — a single connection
/// behind a `Mutex` rather than a pool, mirroring how every other piece of
/// shared state here is a single resource guarded by a lock, since every
/// Tauri command in this codebase runs synchronously and SQLite itself
/// serializes writes per-connection anyway.
pub struct AppState {
    pub tasks_dir: PathBuf,
    pub archive_dir: PathBuf,
    pub projects_file: PathBuf,
    pub settings_file: PathBuf,
    pub series_file: PathBuf,
    pub layouts_file: PathBuf,
    pub projects_lock: Mutex<()>,
    pub series_lock: Mutex<()>,
    pub time_db: Mutex<rusqlite::Connection>,
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
///
/// `pub(crate)`: also reused by [`crate::status_stats::weighted_completion_pct`]
/// for its estimate-fallback chain — see that function's doc comment.
pub(crate) fn effective_default_estimated_minutes(
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

/// Every current subtask of `container_id`, updated to adopt `parent`'s
/// `tags` (always) and `due`/`scheduled` (only when the subtask itself
/// isn't recurring), filtered to just the ones that actually change —
/// what [`update_task`] needs to cascade a parent's edited attributes onto
/// its subtasks immediately, mirroring how editing the parent's recurrence
/// pattern already cascades to linked subtask series (see
/// [`linked_subtask_series_ids`]/[`apply_recurrence_change`]). Subtasks'
/// locked-to-parent attributes (see the Subtasks design spec's attribute-
/// lock rule) are otherwise just a one-time snapshot from creation time —
/// this is what keeps them tracking the parent's *current* values instead.
///
/// A recurring subtask's own `Series` generates a fresh `scheduled`/`due`
/// for every occurrence — the entire point of recurrence, and also why
/// `due`/`scheduled` are deliberately excluded from the "shared" fields a
/// `SeriesEditScope` choice applies across every future occurrence (see
/// `Series`'s own doc comment). Force-overwriting a recurring subtask's
/// current occurrence to the parent's literal date here would corrupt it
/// relative to its own past/future occurrences, so only `tags` (never
/// date-sensitive) cascades to one.
fn subtasks_with_inherited_attributes(
    tasks: &[Task],
    container_id: &str,
    parent: &Task,
) -> Vec<Task> {
    tasks
        .iter()
        .filter(|t| t.project_id.as_deref() == Some(container_id))
        .filter_map(|t| {
            let updated = if t.series_id.is_some() {
                Task {
                    tags: parent.tags.clone(),
                    ..t.clone()
                }
            } else {
                Task {
                    tags: parent.tags.clone(),
                    due: parent.due.clone(),
                    scheduled: parent.scheduled.clone(),
                    ..t.clone()
                }
            };
            (updated != *t).then_some(updated)
        })
        .collect()
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
/// updated) title/project (see [`synced_subtask_container`]), and cascades
/// its tags/due/scheduled onto every current subtask (see
/// [`subtasks_with_inherited_attributes`]).
///
/// If this update moves the task to the done or cancelled status (see
/// [`is_finished`]), also force-stops any active time-tracking session
/// against it first and folds the recomputed `tracked_minutes` into this
/// same write (see [`force_stop_and_recompute`]) — this is the most common
/// real-world "task completed" path, so elapsed time is never silently lost
/// just because the user marked it done rather than archiving/deleting it.
/// Left untouched for every other status change, so most edits never touch
/// the time-tracking database at all.
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

        let tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
        for subtask in subtasks_with_inherited_attributes(&tasks, container_id, &existing) {
            storage::update_task(&state.tasks_dir, &subtask).map_err(|e| e.to_string())?;
        }
    }

    if is_finished(&existing, &settings) {
        existing.tracked_minutes = force_stop_and_recompute(&state, &existing.id)?;
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
///
/// Before any file is removed, force-stops the active time-tracking session
/// (if any) for the target task and for every cascaded subtask (see
/// [`force_stop_and_recompute`]) — the recomputed minutes are discarded
/// here (the markdown file is about to disappear, so there's nothing left
/// to persist them into); the only reason this runs at all is so the
/// SQLite session is actually ended rather than lingering "active" forever
/// against a `task_id` that no longer resolves to any real task, which
/// would otherwise corrupt the orphaned-session-resolution UI's later
/// lookup of that task's title.
#[tauri::command]
pub fn delete_task(state: State<AppState>, id: String) -> Result<(), String> {
    let target = storage::load_task(&state.tasks_dir, &id).map_err(|e| e.to_string())?;

    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    let (subtasks, owned_container) = subtasks_and_container_to_delete(&tasks, &projects, &id);

    force_stop_and_recompute(&state, &id)?;
    for subtask in &subtasks {
        force_stop_and_recompute(&state, &subtask.id)?;
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
///
/// Each subtask being marked done this way also has its active
/// time-tracking session (if any) force-stopped first, with the recomputed
/// `tracked_minutes` folded into the same write that persists its new
/// status (see [`force_stop_and_recompute`]) — these subtasks complete via
/// this cascade rather than going through [`update_task`]'s own
/// force-stop-on-completion handling, so it has to happen here too.
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
    for mut subtask in completed {
        subtask.tracked_minutes = force_stop_and_recompute(&state, &subtask.id)?;
        storage::update_task(&state.tasks_dir, &subtask).map_err(|e| e.to_string())?;
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
/// [`validate_ink_mode`]), and that `update.board.status_tier_rule_overrides`,
/// if set, has exactly 4 entries (see
/// [`status_tier::validate_status_tier_rule_overrides`]). If `update.color`
/// differs from the project's current color, it must be a 6-digit hex color
/// (see [`validate_hex_color`]); an unchanged color is left as-is even if it
/// predates that requirement.
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
    if let Some(overrides) = &update.board.status_tier_rule_overrides {
        status_tier::validate_status_tier_rule_overrides(overrides)?;
    }

    let existing_color = projects[index].color.clone();
    let existing_created = projects[index].created.clone();
    if update.color != existing_color {
        validate_hex_color(&update.color)?;
    }

    let existing_id = projects[index].id.clone();
    validate_parent_id(projects, update.parent_id.as_deref(), Some(&existing_id))?;
    let existing_tracking_task_id = projects[index].tracking_task_id.clone();

    let updated = Project {
        id: existing_id,
        name,
        color: update.color,
        parent_id: update.parent_id,
        order: update.order,
        created: existing_created,
        board: update.board,
        defaults: update.defaults,
        tracking_task_id: existing_tracking_task_id,
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
///
/// Before each task is archived, force-stops its active time-tracking
/// session (if any) and persists the recomputed `tracked_minutes` into its
/// still-in-`tasks_dir` file first (see [`force_stop_and_recompute`]) — the
/// archived copy must carry an accurate final tracked time, and
/// [`storage::archive_task`] simply moves whatever's currently on disk, so
/// the update has to land before the move.
#[tauri::command]
pub fn finish_day(state: State<AppState>) -> Result<FinishDayResult, String> {
    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    let tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;

    let (task_ids, container_ids) = tasks_and_containers_to_finish(&tasks, &settings);

    for task_id in &task_ids {
        let minutes = force_stop_and_recompute(&state, task_id)?;
        let mut task = storage::load_task(&state.tasks_dir, task_id).map_err(|e| e.to_string())?;
        task.tracked_minutes = minutes;
        storage::update_task(&state.tasks_dir, &task).map_err(|e| e.to_string())?;

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

/// The pure logic behind [`force_stop_and_recompute`] — split out so it can
/// be exercised in tests against a real (in-memory or tempfile-backed)
/// `Connection` and `tasks_dir`, without needing a live `State<AppState>`
/// (which, in Tauri 2, has no public constructor outside of an actual
/// running app — see `tauri::State`'s `CommandArg`-only construction). Ends
/// `task_id`'s active time-tracking session, if any, and recomputes its
/// cached `tracked_minutes`. A no-op if no session was active — returns the
/// (possibly unchanged) recomputed minutes either way.
fn force_stop_and_recompute_with(
    conn: &rusqlite::Connection,
    tasks_dir: &Path,
    task_id: &str,
) -> Result<u32, String> {
    time_storage::end_entry(conn, task_id, chrono::Utc::now()).map_err(|e| e.to_string())?;
    time_tracking::recompute_and_persist_tracked_minutes(conn, tasks_dir, task_id)
        .map_err(|e| e.to_string())
}

/// Ends `task_id`'s active time-tracking session, if any, and recomputes its
/// cached `tracked_minutes`. Used at every point a task stops being
/// trackable (becomes done/cancelled, gets archived, or gets deleted) so
/// elapsed time is never silently lost. A no-op if no session was active —
/// returns the (possibly unchanged) recomputed minutes either way.
fn force_stop_and_recompute(state: &State<AppState>, task_id: &str) -> Result<u32, String> {
    let conn = state.time_db.lock().map_err(|e| e.to_string())?;
    force_stop_and_recompute_with(&conn, &state.tasks_dir, task_id)
}

/// Starts a new time-tracking session for `task_id`. A no-op (returns the
/// existing session rather than erroring) if `task_id` already has one
/// active — see [`time_storage::start_entry`]. The created/existing entry
/// itself is discarded: the frontend's own store derives "what's running"
/// UI state from [`get_active_sessions`], not from this command's return
/// value.
///
/// Validates `task_id` names a real task first, so a stale or typo'd id
/// from a desynced frontend store fails fast here with a clear error,
/// rather than succeeding silently and only surfacing as a confusing
/// `NotFound` several steps later, the next time something tries to
/// recompute that (bogus) task's `tracked_minutes`.
#[tauri::command]
pub fn start_tracking(state: State<AppState>, task_id: String) -> Result<(), String> {
    storage::load_task(&state.tasks_dir, &task_id).map_err(|e| e.to_string())?;
    let conn = state.time_db.lock().map_err(|e| e.to_string())?;
    time_storage::start_entry(&conn, &task_id, chrono::Utc::now()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Ends `task_id`'s active time-tracking session, if any, and returns the
/// freshly recomputed `tracked_minutes`. Stopping a task with no active
/// session is a defined no-op (see [`time_storage::end_entry`]), not an
/// error — the recompute still runs unconditionally so the returned value
/// always reflects the current persisted total.
#[tauri::command]
pub fn stop_tracking(state: State<AppState>, task_id: String) -> Result<u32, String> {
    let conn = state.time_db.lock().map_err(|e| e.to_string())?;
    time_storage::end_entry(&conn, &task_id, chrono::Utc::now()).map_err(|e| e.to_string())?;
    time_tracking::recompute_and_persist_tracked_minutes(&conn, &state.tasks_dir, &task_id)
        .map_err(|e| e.to_string())
}

/// Every currently-active time-tracking session across all tasks — used by
/// the frontend to restore "what's running" UI state on launch and to
/// detect orphaned sessions (see [`resolve_orphaned_session`]).
#[tauri::command]
pub fn get_active_sessions(state: State<AppState>) -> Result<Vec<time_storage::TimeEntry>, String> {
    let conn = state.time_db.lock().map_err(|e| e.to_string())?;
    time_storage::list_active_entries(&conn).map_err(|e| e.to_string())
}

/// Updates `last_heartbeat_at` on `task_id`'s active session, if any. Called
/// roughly every 30 seconds by the frontend while any timer is running, so
/// an orphaned session left behind by a force-quit/crash/power-loss can
/// later be told apart from one that's still genuinely active (see
/// [`resolve_orphaned_session`]). A no-op, not an error, if `task_id` has no
/// active session.
#[tauri::command]
pub fn heartbeat(state: State<AppState>, task_id: String) -> Result<(), String> {
    let conn = state.time_db.lock().map_err(|e| e.to_string())?;
    time_storage::update_heartbeat(&conn, &task_id, chrono::Utc::now()).map_err(|e| e.to_string())
}

/// Resolves an orphaned session detected on launch (an active session whose
/// `last_heartbeat_at` is too stale to trust — that staleness check itself
/// is frontend logic, this command just carries out whichever choice the
/// user made). `action` must be `"resume"` (leave the row untouched; the
/// timer keeps counting as if nothing happened) or `"discard"` (ends the
/// session at its `last_heartbeat_at` — the last point tracking was known to
/// genuinely be happening — falling back to `started_at` itself if it never
/// received even one heartbeat, rather than guessing; then recomputes that
/// task's `tracked_minutes`). Any other `action` is rejected.
#[tauri::command]
pub fn resolve_orphaned_session(
    state: State<AppState>,
    entry_id: String,
    action: String,
) -> Result<(), String> {
    validate_orphaned_session_action(&action)?;
    match action.as_str() {
        "resume" => Ok(()),
        "discard" => discard_orphaned_session(&state, &entry_id),
        _ => unreachable!("validate_orphaned_session_action already rejected anything else"),
    }
}

/// Returns `Ok(())` if `action` is `"resume"` or `"discard"` (the only two
/// values [`resolve_orphaned_session`] accepts), or an error naming the
/// invalid value otherwise.
fn validate_orphaned_session_action(action: &str) -> Result<(), String> {
    match action {
        "resume" | "discard" => Ok(()),
        other => Err(format!("invalid action: '{other}'")),
    }
}

/// The pure logic behind [`discard_orphaned_session`] — split out so it can
/// be tested against a real `Connection`/`tasks_dir` without a live
/// `State<AppState>` (see [`force_stop_and_recompute_with`]'s doc comment
/// for why). Ends `entry_id`'s session at its `last_heartbeat_at`, falling
/// back to `started_at` itself if it never received one, then recomputes
/// the owning task's `tracked_minutes`.
fn discard_orphaned_session_with(
    conn: &rusqlite::Connection,
    tasks_dir: &Path,
    entry_id: &str,
) -> Result<(), String> {
    let entry = time_storage::get_entry(conn, entry_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "time entry not found".to_string())?;

    let started_at = parse_rfc3339_field(&entry.started_at, "started_at")?;
    let ended_at = match &entry.last_heartbeat_at {
        Some(heartbeat) => parse_rfc3339_field(heartbeat, "last_heartbeat_at")?,
        None => started_at,
    };

    time_storage::update_entry_times(conn, entry_id, started_at, Some(ended_at))
        .map_err(|e| e.to_string())?;
    time_tracking::recompute_and_persist_tracked_minutes(conn, tasks_dir, &entry.task_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// The `"discard"` branch of [`resolve_orphaned_session`], split out so the
/// command function itself stays a thin dispatcher.
fn discard_orphaned_session(state: &State<AppState>, entry_id: &str) -> Result<(), String> {
    let conn = state.time_db.lock().map_err(|e| e.to_string())?;
    discard_orphaned_session_with(&conn, &state.tasks_dir, entry_id)
}

/// Parses an RFC3339 timestamp read back from `time_entries`, mapping a
/// parse failure to a clear error naming the offending field — used for
/// values that were already validated/written by this same codebase, so a
/// failure here would indicate file/DB corruption rather than bad user
/// input.
fn parse_rfc3339_field(value: &str, field: &str) -> Result<chrono::DateTime<chrono::Utc>, String> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| format!("stored {field} '{value}' is not valid RFC3339: {e}"))
}

/// Parses a client-supplied RFC3339 timestamp for a manual time-entry
/// command (`add_manual_time_entry`/`update_time_entry`), mapping a parse
/// failure to a clear, field-named error.
fn parse_rfc3339_input(value: &str, field: &str) -> Result<chrono::DateTime<chrono::Utc>, String> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| format!("{field} is not a valid RFC3339 timestamp: {e}"))
}

/// Returns `Ok(())` if `ended` is `None` (an open/currently-running entry)
/// or comes strictly after `started`, or an error otherwise — a manual
/// time entry recording zero or negative elapsed time has no meaning.
/// Shared by `add_manual_time_entry` (where `ended` is always `Some`) and
/// `update_time_entry` (where it may be `None`, reopening the entry).
fn validate_time_range(
    started: chrono::DateTime<chrono::Utc>,
    ended: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<(), String> {
    if let Some(ended) = ended {
        if ended <= started {
            return Err("end time must be after start time".to_string());
        }
    }
    Ok(())
}

/// Records a manually-entered, already-completed time-tracking session for
/// `task_id`. Rejects `ended_at` values that don't come strictly after
/// `started_at` (see [`validate_time_range`]). Recomputes `tracked_minutes`
/// afterward.
///
/// Validates `task_id` names a real task *before* inserting the entry —
/// otherwise a bogus id would still commit a real `time_entries` row (since
/// SQLite enforces no foreign key here) only to fail moments later inside
/// the recompute step, leaving an orphaned entry behind despite the overall
/// command reporting an error.
#[tauri::command]
pub fn add_manual_time_entry(
    state: State<AppState>,
    task_id: String,
    started_at: String,
    ended_at: String,
) -> Result<(), String> {
    storage::load_task(&state.tasks_dir, &task_id).map_err(|e| e.to_string())?;
    let started = parse_rfc3339_input(&started_at, "started_at")?;
    let ended = parse_rfc3339_input(&ended_at, "ended_at")?;
    validate_time_range(started, Some(ended))?;

    let conn = state.time_db.lock().map_err(|e| e.to_string())?;
    time_storage::insert_completed_entry(&conn, &task_id, started, ended)
        .map_err(|e| e.to_string())?;
    time_tracking::recompute_and_persist_tracked_minutes(&conn, &state.tasks_dir, &task_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Overwrites an existing time entry's `started_at`/`ended_at` for manual
/// correction. `ended_at` of `None` reopens the entry as currently-running;
/// if given, it must come strictly after `started_at` (see
/// [`validate_time_range`]). Recomputes the owning task's
/// `tracked_minutes` afterward (looked up via the entry itself, since the
/// caller only supplies an entry id).
#[tauri::command]
pub fn update_time_entry(
    state: State<AppState>,
    entry_id: String,
    started_at: String,
    ended_at: Option<String>,
) -> Result<(), String> {
    let started = parse_rfc3339_input(&started_at, "started_at")?;
    let ended = match &ended_at {
        Some(value) => Some(parse_rfc3339_input(value, "ended_at")?),
        None => None,
    };
    validate_time_range(started, ended)?;

    let conn = state.time_db.lock().map_err(|e| e.to_string())?;
    let entry = time_storage::get_entry(&conn, &entry_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "time entry not found".to_string())?;

    time_storage::update_entry_times(&conn, &entry_id, started, ended)
        .map_err(|e| e.to_string())?;
    time_tracking::recompute_and_persist_tracked_minutes(&conn, &state.tasks_dir, &entry.task_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Deletes a single time entry by id, then recomputes its task's
/// `tracked_minutes`. Unlike the force-stop cascade's use of
/// [`time_storage::end_entry`] (a deliberate no-op when there's nothing to
/// end), this is a direct user-initiated delete of one specific, known-to-
/// exist entry from the time-log UI — so a missing `entry_id` is treated as
/// a real error rather than silently doing nothing.
#[tauri::command]
pub fn delete_time_entry(state: State<AppState>, entry_id: String) -> Result<(), String> {
    let conn = state.time_db.lock().map_err(|e| e.to_string())?;
    let entry = time_storage::get_entry(&conn, &entry_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "time entry not found".to_string())?;

    time_storage::delete_entry(&conn, &entry_id).map_err(|e| e.to_string())?;
    time_tracking::recompute_and_persist_tracked_minutes(&conn, &state.tasks_dir, &entry.task_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Every time entry (active or completed) recorded against `task_id` —
/// backs the time-log UI. Date-range filtering is deferred to the Phase
/// 2/3 dashboard work (see the time-tracking-engine spec); not implemented
/// here.
#[tauri::command]
pub fn list_time_entries(
    state: State<AppState>,
    task_id: String,
) -> Result<Vec<time_storage::TimeEntry>, String> {
    let conn = state.time_db.lock().map_err(|e| e.to_string())?;
    time_storage::list_entries_for_task(&conn, &task_id).map_err(|e| e.to_string())
}

/// Looks up `project_id` within `projects` and either returns its existing
/// tracking task unchanged or lazily creates one — the pure read-modify
/// step behind [`get_or_create_tracking_task`], split out so it can be unit
/// tested without a live `State<AppState>` (mirrors how
/// [`get_or_create_subtask_container`] is itself the pure helper behind
/// [`ensure_subtask_container`]).
///
/// `Project.tracking_task_id` (see `Project::tracking_task_id`'s own doc
/// comment) is updated on `projects[index]` in place when a new tracking
/// task is created. If it already names a task that still loads
/// successfully from `tasks_dir`, that id is reused unchanged — including
/// when the file lookup itself fails to parse for reasons other than "the
/// file doesn't exist" (treated the same as a missing file: recreate
/// rather than propagate an unrelated parse error up through a lazy
/// creation path). Returns `Err` only when `project_id` doesn't name any
/// project in `projects`.
fn get_or_create_tracking_task_for_project(
    tasks_dir: &Path,
    projects: &mut [Project],
    project_id: &str,
) -> Result<Task, String> {
    let index = projects
        .iter()
        .position(|p| p.id == project_id)
        .ok_or_else(|| "project not found".to_string())?;

    if let Some(existing_id) = &projects[index].tracking_task_id {
        if let Ok(existing_task) = storage::load_task(tasks_dir, existing_id) {
            return Ok(existing_task);
        }
    }

    let mut tracking_task = Task::new(format!("{} — General time", projects[index].name));
    tracking_task.hidden = true;
    tracking_task.project_id = Some(project_id.to_string());
    projects[index].tracking_task_id = Some(tracking_task.id.clone());

    Ok(tracking_task)
}

/// Looks up or lazily creates `project_id`'s "tracking task" — the hidden
/// `Task` (see `Task::hidden`) used to track a project as a whole, rather
/// than any single task within it (see `Project::tracking_task_id`).
/// Directly mirrors [`ensure_subtask_container`], just inverted: a project
/// growing a hidden task instead of a task growing a hidden project. See
/// [`get_or_create_tracking_task_for_project`] for the pure lookup/creation
/// logic; this wrapper just loads/persists the project list and the new
/// task file (when one was created) around it.
fn get_or_create_tracking_task(
    state: &State<AppState>,
    project_id: &str,
) -> Result<String, String> {
    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    let before = projects
        .iter()
        .find(|p| p.id == project_id)
        .and_then(|p| p.tracking_task_id.clone());

    let tracking_task =
        get_or_create_tracking_task_for_project(&state.tasks_dir, &mut projects, project_id)?;

    if Some(tracking_task.id.clone()) != before {
        storage::save_task(&state.tasks_dir, &tracking_task).map_err(|e| e.to_string())?;
        project_storage::save_projects(&state.projects_file, &projects)
            .map_err(|e| e.to_string())?;
    }

    Ok(tracking_task.id)
}

/// Starts time-tracking for `project_id` as a whole, lazily creating its
/// hidden tracking task on first use (see [`get_or_create_tracking_task`]),
/// then delegating to the same logic as [`start_tracking`].
#[tauri::command]
pub fn start_project_tracking(state: State<AppState>, project_id: String) -> Result<(), String> {
    let task_id = get_or_create_tracking_task(&state, &project_id)?;
    let conn = state.time_db.lock().map_err(|e| e.to_string())?;
    time_storage::start_entry(&conn, &task_id, chrono::Utc::now()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Stops time-tracking for `project_id` as a whole and returns the
/// recomputed `tracked_minutes` for its tracking task. Unlike
/// [`start_project_tracking`], does not lazily create the tracking task —
/// a project that's never been tracked has nothing to stop, which is a
/// real error here (unlike [`stop_tracking`]'s no-active-session no-op),
/// since the caller should never be able to reach "stop" without having
/// reached "start" first.
#[tauri::command]
pub fn stop_project_tracking(state: State<AppState>, project_id: String) -> Result<u32, String> {
    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let project = projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| "project not found".to_string())?;
    let task_id = project
        .tracking_task_id
        .clone()
        .ok_or_else(|| "project has no tracking task".to_string())?;

    let conn = state.time_db.lock().map_err(|e| e.to_string())?;
    time_storage::end_entry(&conn, &task_id, chrono::Utc::now()).map_err(|e| e.to_string())?;
    time_tracking::recompute_and_persist_tracked_minutes(&conn, &state.tasks_dir, &task_id)
        .map_err(|e| e.to_string())
}

/// Resolves whether `project_id`'s board/week/calendar views (and, by
/// extension, the status-line stats that share that same rollup toggle —
/// see `crate::status_stats`) should include descendant subprojects' tasks:
/// the nearest `board.show_subproject_tasks` override walking `project_id`
/// and its ancestors nearest-first (see
/// [`project_tree::self_and_ancestors`]), falling back to
/// `settings.show_subproject_tasks_default` if no project in the chain has
/// set one. Mirrors the frontend's own resolution in `KanbanBoard.svelte`
/// exactly (`projectChain.find(...).board.show_subproject_tasks ?? ...
/// show_subproject_tasks_default`) — this is the one status-line setting
/// that resolves via an ancestor walk rather than single-level, unlike
/// `status_tier_rule_overrides`/`status_line_layout_id` below.
fn resolve_show_subproject_tasks(
    projects: &[Project],
    project_id: &str,
    settings: &Settings,
) -> bool {
    project_tree::self_and_ancestors(projects, project_id)
        .iter()
        .find_map(|p| p.board.show_subproject_tasks)
        .unwrap_or(settings.show_subproject_tasks_default)
}

/// Resolves which `StatLayout` id `project_id`'s status line renders:
/// `project.board.status_line_layout_id` if set, otherwise
/// `settings.default_status_line_layout_id`. Single-level only — no
/// ancestor-chain walk, unlike [`resolve_show_subproject_tasks`] — since a
/// project's status line is its own concern, not something a parent project
/// should be able to impose on it by proxy.
fn resolve_status_line_layout_id(project: &Project, settings: &Settings) -> String {
    project
        .board
        .status_line_layout_id
        .clone()
        .unwrap_or_else(|| settings.default_status_line_layout_id.clone())
}

/// All 6 project-status-line stats for one project — see
/// `docs/features/project-status-line.md`'s "Stat catalog". `status_tier` is
/// the computed health badge, serialized as a stable snake_case string id
/// (`"needs_attention"`, etc. — see [`StatusTier`]'s `Serialize` impl).
/// `estimated_time_left`/`total_time_tracked` are both in minutes;
/// `avg_time_per_week` is in **seconds**, matching
/// `status_stats::avg_time_per_week`'s/`time_storage::tracked_seconds_per_day`'s
/// native unit rather than being converted here — the frontend already has
/// to do minutes/seconds formatting work for other tracked-time displays, so
/// converting in exactly one of the three time stats here while the other
/// two stay in minutes would be a worse inconsistency than documenting the
/// one stat that's natively seconds and leaving all unit conversion to the
/// frontend. `completion_pct`/`weighted_completion_pct` are fractions in
/// `0.0..=1.0` (not pre-multiplied by 100), `None` when there's no
/// meaningful population to divide by — see `status_stats`'s own doc
/// comments for exactly when that happens. `effective_layout_id` is the
/// resolved `StatLayout` id this project's status line should render (see
/// [`resolve_status_line_layout_id`]) — included so the frontend doesn't
/// have to re-derive the same single-level inheritance rule itself.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProjectStatusStats {
    pub status_tier: StatusTier,
    pub estimated_time_left: u32,
    pub total_time_tracked: u32,
    pub avg_time_per_week: f64,
    pub completion_pct: Option<f64>,
    pub weighted_completion_pct: Option<f64>,
    /// `active_completion_pct`: same calculation as `completion_pct` but
    /// restricted to the non-archived (active) task population — "what
    /// fraction of tasks currently on the kanban board are done?" rather than
    /// "what fraction of all tasks ever created for this project are done?".
    pub active_completion_pct: Option<f64>,
    pub effective_layout_id: String,
}

/// The pure computation behind [`get_project_status_stats`]: given the
/// already-loaded settings/projects/merged-task-list/time-tracking
/// connection, resolves every setting this stat catalog depends on and
/// assembles the final [`ProjectStatusStats`]. Split out from the
/// `#[tauri::command]` wrapper so it can be exercised directly in tests
/// without a live `State<AppState>`, mirroring this module's existing
/// `apply_project_update`/`#[tauri::command]` split.
///
/// `estimated_time_left` and the status-tier evaluator's task scan are
/// always computed over `project_id`'s own (never rolled up, regardless of
/// `show_subproject_tasks`) non-hidden, incomplete tasks — per the
/// project-status-line spec's resolved rollup-scope decision, a busy
/// subproject can never flip its parent's badge. `total_time_tracked`,
/// `avg_time_per_week`, `completion_pct`, and `weighted_completion_pct` all
/// respect [`resolve_show_subproject_tasks`].
#[allow(clippy::too_many_arguments)]
fn build_project_status_stats(
    conn: &rusqlite::Connection,
    project_id: &str,
    projects: &[Project],
    active_tasks: &[Task],
    all_tasks: &[Task],
    settings: &Settings,
    week_start: &str,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<ProjectStatusStats, String> {
    let project = projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| format!("project '{project_id}' not found"))?;

    let include_subprojects = resolve_show_subproject_tasks(projects, project_id, settings);

    let own_incomplete_tasks: Vec<&Task> = all_tasks
        .iter()
        .filter(|task| {
            task.project_id.as_deref() == Some(project_id)
                && !task.hidden
                && status_stats::is_incomplete(task, settings)
        })
        .collect();

    let estimated_time_left = status_stats::estimated_time_left(project_id, all_tasks, settings);

    let effective_rules = status_tier::effective_status_tier_rules(
        &settings.default_status_tier_rules,
        project.board.status_tier_rule_overrides.as_deref(),
    );
    let status_tier = status_tier::evaluate_status_tier(
        &effective_rules,
        &own_incomplete_tasks,
        settings,
        estimated_time_left,
        today,
    );

    let total_time_tracked =
        status_stats::total_time_tracked(project_id, projects, all_tasks, include_subprojects);
    let avg_time_per_week = status_stats::avg_time_per_week(
        conn,
        project_id,
        projects,
        all_tasks,
        include_subprojects,
        settings,
        week_start,
        today,
        now,
    )
    .map_err(|e| e.to_string())?;
    let completion_pct = status_stats::completion_pct(
        project_id,
        projects,
        all_tasks,
        include_subprojects,
        settings,
    );
    let weighted_completion_pct = status_stats::weighted_completion_pct(
        project_id,
        projects,
        all_tasks,
        include_subprojects,
        settings,
    );
    let active_completion_pct = status_stats::active_completion_pct(
        project_id,
        projects,
        active_tasks,
        include_subprojects,
        settings,
    );

    let effective_layout_id = resolve_status_line_layout_id(project, settings);

    Ok(ProjectStatusStats {
        status_tier,
        estimated_time_left,
        total_time_tracked,
        avg_time_per_week,
        completion_pct,
        weighted_completion_pct,
        active_completion_pct,
        effective_layout_id,
    })
}

/// Returns every status-line stat (see [`ProjectStatusStats`]) for
/// `project_id`, computed over the combined `tasks_dir` + `archive_dir` task
/// population (so `completion_pct`/`weighted_completion_pct` correctly
/// include already-archived completed work) and the time-tracking
/// database's recorded sessions. `week_starts_on` must be `"monday"` or
/// `"sunday"` (any other value degrades to `"sunday"` — see
/// `status_stats::start_of_week`); the frontend passes its own Week-view
/// display setting, since this backend has no persisted notion of that
/// setting itself. "Today" is computed server-side
/// (`chrono::Local::now().date_naive()`), matching every other date-sensitive
/// command in this codebase. Returns an error if `project_id` doesn't name a
/// real project.
#[tauri::command]
pub fn get_project_status_stats(
    state: State<AppState>,
    project_id: String,
    week_starts_on: String,
) -> Result<ProjectStatusStats, String> {
    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    let active_tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
    let archived_tasks = storage::list_tasks(&state.archive_dir).map_err(|e| e.to_string())?;
    let mut all_tasks = active_tasks.clone();
    all_tasks.extend(archived_tasks);

    let conn = state.time_db.lock().map_err(|e| e.to_string())?;
    let today = chrono::Local::now().date_naive();
    let now = chrono::Utc::now();

    build_project_status_stats(
        &conn,
        &project_id,
        &projects,
        &active_tasks,
        &all_tasks,
        &settings,
        &week_starts_on,
        today,
        now,
    )
}

/// Global stats for the "All tasks" status bar: task counts grouped by status
/// (active non-hidden tasks only, matching what's visible on the kanban board),
/// total project count, and total tracked time today and this week (across all
/// tasks in the time database, not filtered by project). Times are in minutes.
/// `tasks_by_status` is ordered by the global `Settings.statuses` order — only
/// statuses that have at least one visible task are included.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GlobalStatusStats {
    pub tasks_by_status: Vec<(String, usize)>,
    pub total_projects: usize,
    pub time_tracked_today_minutes: u32,
    pub time_tracked_this_week_minutes: u32,
}

/// Returns global stats for the "All tasks" status bar. `week_starts_on` must
/// be `"monday"` or `"sunday"` and is used to determine where the current week
/// begins. Task counts cover non-hidden active (non-archived) tasks, matching
/// the kanban board's visible population.
#[tauri::command]
pub fn get_global_status_stats(
    state: State<AppState>,
    week_starts_on: String,
) -> Result<GlobalStatusStats, String> {
    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let active_tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;

    let today = chrono::Local::now().date_naive();
    let now = chrono::Utc::now();

    // Task counts by status — non-hidden, actively scheduled (scheduled.is_some()) active tasks.
    let scheduled_tasks: Vec<&Task> = active_tasks
        .iter()
        .filter(|t| !t.hidden && t.scheduled.is_some())
        .collect();

    let mut tasks_by_status: Vec<(String, usize)> = settings
        .statuses
        .iter()
        .filter_map(|status_def| {
            let count = scheduled_tasks
                .iter()
                .filter(|t| t.status == status_def.id)
                .count();
            if count > 0 {
                Some((status_def.id.clone(), count))
            } else {
                None
            }
        })
        .collect();
    // Include any tasks whose status isn't in the settings list (shouldn't happen,
    // but degrade gracefully rather than losing counts).
    let known_status_ids: std::collections::HashSet<&str> =
        settings.statuses.iter().map(|s| s.id.as_str()).collect();
    let mut unknown_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for task in &scheduled_tasks {
        if !known_status_ids.contains(task.status.as_str()) {
            *unknown_counts.entry(task.status.clone()).or_insert(0) += 1;
        }
    }
    tasks_by_status.extend(unknown_counts.into_iter());

    // Project count: exclude subtask container projects (those whose id appears
    // as `subtask_project_id` on any active task).
    let subtask_container_ids: std::collections::HashSet<&str> = active_tasks
        .iter()
        .filter_map(|t| t.subtask_project_id.as_deref())
        .collect();
    let total_projects = projects
        .iter()
        .filter(|p| !subtask_container_ids.contains(p.id.as_str()))
        .count();

    let conn = state.time_db.lock().map_err(|e| e.to_string())?;

    let today_start = today
        .and_hms_opt(0, 0, 0)
        .expect("midnight is valid")
        .and_utc();
    let tomorrow_start = today_start + chrono::Duration::days(1);

    let time_tracked_today_seconds = time_storage::total_tracked_seconds_all_tasks_in_range(
        &conn,
        today_start,
        tomorrow_start,
        now,
    )
    .map_err(|e| e.to_string())?;

    // Week start: same logic as status_stats::start_of_week.
    use chrono::Datelike;
    let week_start_day = if week_starts_on == "monday" {
        chrono::Weekday::Mon
    } else {
        chrono::Weekday::Sun
    };
    let days_since_start = (today.weekday().num_days_from_monday() as i64
        - week_start_day.num_days_from_monday() as i64)
        .rem_euclid(7);
    let week_start_date = today - chrono::Duration::days(days_since_start);
    let week_start = week_start_date
        .and_hms_opt(0, 0, 0)
        .expect("midnight is valid")
        .and_utc();

    let time_tracked_this_week_seconds = time_storage::total_tracked_seconds_all_tasks_in_range(
        &conn,
        week_start,
        now,
        now,
    )
    .map_err(|e| e.to_string())?;

    Ok(GlobalStatusStats {
        tasks_by_status,
        total_projects,
        time_tracked_today_minutes: (time_tracked_today_seconds / 60).max(0) as u32,
        time_tracked_this_week_minutes: (time_tracked_this_week_seconds / 60).max(0) as u32,
    })
}

/// Returns every saved `StatLayout`, in storage order (insertion order — no
/// sort is applied, since layouts have no analog of `Project.order`/
/// `Task.order` to sort by).
#[tauri::command]
pub fn list_status_layouts(state: State<AppState>) -> Result<Vec<StatLayout>, String> {
    layout_storage::list_layouts(&state.layouts_file).map_err(|e| e.to_string())
}

/// Creates a new status-line `StatLayout` named `name` with `stat_ids`,
/// validates it (see [`layout::validate_status_layout`]), appends it to the
/// saved list, and returns it.
#[tauri::command]
pub fn create_status_layout(
    state: State<AppState>,
    name: String,
    stat_ids: Vec<String>,
) -> Result<StatLayout, String> {
    let new_layout = StatLayout::new_status_line(name, stat_ids);
    layout::validate_status_layout(&new_layout)?;

    let mut layouts =
        layout_storage::list_layouts(&state.layouts_file).map_err(|e| e.to_string())?;
    layouts.push(new_layout.clone());
    layout_storage::save_layouts(&state.layouts_file, &layouts).map_err(|e| e.to_string())?;

    Ok(new_layout)
}

/// Updates an existing `StatLayout` in place — every project (or the global
/// default) currently pointing at `layout.id` sees the change immediately,
/// per the project-status-line spec's "named presets, shared on edit"
/// decision. Validates `layout` (see [`layout::validate_status_layout`])
/// before saving. Returns an error if `layout.id` doesn't match any saved
/// layout.
#[tauri::command]
pub fn update_status_layout(
    state: State<AppState>,
    layout: StatLayout,
) -> Result<StatLayout, String> {
    layout::validate_status_layout(&layout)?;

    let mut layouts =
        layout_storage::list_layouts(&state.layouts_file).map_err(|e| e.to_string())?;
    let index = layouts
        .iter()
        .position(|l| l.id == layout.id)
        .ok_or_else(|| format!("layout '{}' not found", layout.id))?;

    layouts[index] = layout.clone();
    layout_storage::save_layouts(&state.layouts_file, &layouts).map_err(|e| e.to_string())?;

    Ok(layout)
}

/// Forks `layout_id` into a brand-new `StatLayout` named `new_name`, with a
/// freshly generated id and the same `stat_ids` — the "Duplicate" action the
/// project-status-line spec distinguishes from "Save as new layout" (which
/// commits in-progress edits to a currently-applied layout instead; this
/// command forks immediately, before any edits). Returns an error if
/// `layout_id` doesn't match any saved layout.
#[tauri::command]
pub fn duplicate_status_layout(
    state: State<AppState>,
    layout_id: String,
    new_name: String,
) -> Result<StatLayout, String> {
    let mut layouts =
        layout_storage::list_layouts(&state.layouts_file).map_err(|e| e.to_string())?;
    let source = layouts
        .iter()
        .find(|l| l.id == layout_id)
        .ok_or_else(|| format!("layout '{layout_id}' not found"))?;

    let duplicate = StatLayout::new_status_line(new_name, source.stat_ids.clone());
    layouts.push(duplicate.clone());
    layout_storage::save_layouts(&state.layouts_file, &layouts).map_err(|e| e.to_string())?;

    Ok(duplicate)
}

/// Returns `Ok(())` if no project's `board.status_line_layout_id` and
/// `settings.default_status_line_layout_id` reference `layout_id`, or an
/// error naming how many places still reference it otherwise (the global
/// default counts as exactly one of those places, distinct from any
/// project). Used to refuse deleting a layout still in use, per the
/// project-status-line spec's resolved "nothing is silently reassigned"
/// decision — the user must switch every referencer to a different layout
/// first.
fn ensure_layout_not_in_use(
    layout_id: &str,
    projects: &[Project],
    settings: &Settings,
) -> Result<(), String> {
    let referencing_projects = projects
        .iter()
        .filter(|p| p.board.status_line_layout_id.as_deref() == Some(layout_id))
        .count();
    let is_global_default = settings.default_status_line_layout_id == layout_id;

    if referencing_projects == 0 && !is_global_default {
        return Ok(());
    }

    let global_default_note = if is_global_default {
        " and the global default"
    } else {
        ""
    };
    Err(format!(
        "layout still in use by {referencing_projects} project(s){global_default_note}"
    ))
}

/// Permanently deletes `layout_id`. Refused (see
/// [`ensure_layout_not_in_use`]) while any project or the global default
/// still references it — nothing is silently reassigned. Returns a distinct
/// error if `layout_id` doesn't match any saved layout at all (checked
/// before the in-use check, so a typo'd id is never mistakenly reported as
/// "in use").
#[tauri::command]
pub fn delete_status_layout(state: State<AppState>, layout_id: String) -> Result<(), String> {
    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;
    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let mut layouts =
        layout_storage::list_layouts(&state.layouts_file).map_err(|e| e.to_string())?;

    if !layouts.iter().any(|l| l.id == layout_id) {
        return Err(format!("layout '{layout_id}' not found"));
    }

    ensure_layout_not_in_use(&layout_id, &projects, &settings)?;

    layouts.retain(|l| l.id != layout_id);
    layout_storage::save_layouts(&state.layouts_file, &layouts).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectBoard;
    use crate::settings::{PriorityLevel, StatusDefinition, StatusTierRule, TaskDefaults};
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
    fn subtasks_with_inherited_attributes_cascades_tags_due_and_scheduled_to_a_one_off_subtask() {
        let mut parent = Task::new("Fix the bug".to_string());
        parent.tags = vec!["urgent".to_string()];
        parent.due = Some("2026-07-01".to_string());
        parent.scheduled = Some("2026-06-22".to_string());
        let mut subtask = Task::new("Reproduce it".to_string());
        subtask.project_id = Some("container".to_string());
        subtask.tags = vec!["old-tag".to_string()];
        subtask.due = Some("2026-06-01".to_string());
        subtask.scheduled = Some("2026-06-01".to_string());

        let updated = subtasks_with_inherited_attributes(&[subtask], "container", &parent);

        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].tags, vec!["urgent".to_string()]);
        assert_eq!(updated[0].due, Some("2026-07-01".to_string()));
        assert_eq!(updated[0].scheduled, Some("2026-06-22".to_string()));
    }

    #[test]
    fn subtasks_with_inherited_attributes_cascades_only_tags_to_a_recurring_subtask() {
        let mut parent = Task::new("Hello".to_string());
        parent.tags = vec!["urgent".to_string()];
        parent.due = Some("2026-07-01".to_string());
        parent.scheduled = Some("2026-06-22".to_string());
        let mut subtask = Task::new("Hi".to_string());
        subtask.project_id = Some("container".to_string());
        subtask.series_id = Some("hi-series".to_string());
        subtask.tags = vec!["old-tag".to_string()];
        subtask.due = Some("2026-06-23".to_string());
        subtask.scheduled = Some("2026-06-23".to_string());

        let updated = subtasks_with_inherited_attributes(&[subtask], "container", &parent);

        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].tags, vec!["urgent".to_string()]);
        // Its own occurrence date is left exactly as its Series generated it.
        assert_eq!(updated[0].due, Some("2026-06-23".to_string()));
        assert_eq!(updated[0].scheduled, Some("2026-06-23".to_string()));
    }

    #[test]
    fn subtasks_with_inherited_attributes_omits_a_subtask_that_already_matches() {
        let mut parent = Task::new("Fix the bug".to_string());
        parent.tags = vec!["urgent".to_string()];
        parent.due = Some("2026-07-01".to_string());
        parent.scheduled = Some("2026-06-22".to_string());
        let mut subtask = Task::new("Reproduce it".to_string());
        subtask.project_id = Some("container".to_string());
        subtask.tags = vec!["urgent".to_string()];
        subtask.due = Some("2026-07-01".to_string());
        subtask.scheduled = Some("2026-06-22".to_string());

        let updated = subtasks_with_inherited_attributes(&[subtask], "container", &parent);

        assert!(updated.is_empty());
    }

    #[test]
    fn subtasks_with_inherited_attributes_ignores_tasks_outside_the_container() {
        let parent = Task::new("Fix the bug".to_string());
        let mut unrelated = Task::new("Unrelated".to_string());
        unrelated.project_id = Some("somewhere-else".to_string());

        let updated = subtasks_with_inherited_attributes(&[unrelated], "container", &parent);

        assert!(updated.is_empty());
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
    fn get_or_create_tracking_task_for_project_creates_one_on_first_call() {
        let tasks_dir = tempdir().unwrap();
        let project = Project::new("Acme Launch".to_string(), "#3b82f6".to_string(), 1);
        let project_id = project.id.clone();
        let mut projects = vec![project];

        let tracking_task =
            get_or_create_tracking_task_for_project(tasks_dir.path(), &mut projects, &project_id)
                .unwrap();

        assert_eq!(tracking_task.title, "Acme Launch — General time");
        assert_eq!(tracking_task.project_id, Some(project_id.clone()));
        assert_eq!(projects[0].tracking_task_id, Some(tracking_task.id.clone()));
    }

    #[test]
    fn get_or_create_tracking_task_for_project_created_task_is_hidden() {
        let tasks_dir = tempdir().unwrap();
        let project = Project::new("Acme Launch".to_string(), "#3b82f6".to_string(), 1);
        let project_id = project.id.clone();
        let mut projects = vec![project];

        let tracking_task =
            get_or_create_tracking_task_for_project(tasks_dir.path(), &mut projects, &project_id)
                .unwrap();

        assert!(tracking_task.hidden);
    }

    #[test]
    fn get_or_create_tracking_task_for_project_returns_the_existing_one_on_a_second_call() {
        let tasks_dir = tempdir().unwrap();
        let project = Project::new("Acme Launch".to_string(), "#3b82f6".to_string(), 1);
        let project_id = project.id.clone();
        let mut projects = vec![project];

        let first =
            get_or_create_tracking_task_for_project(tasks_dir.path(), &mut projects, &project_id)
                .unwrap();
        storage::save_task(tasks_dir.path(), &first).unwrap();

        let second =
            get_or_create_tracking_task_for_project(tasks_dir.path(), &mut projects, &project_id)
                .unwrap();

        assert_eq!(first.id, second.id);
    }

    #[test]
    fn get_or_create_tracking_task_for_project_recreates_one_for_a_stale_pointer() {
        let tasks_dir = tempdir().unwrap();
        let mut project = Project::new("Acme Launch".to_string(), "#3b82f6".to_string(), 1);
        project.tracking_task_id = Some("already-deleted-task".to_string());
        let project_id = project.id.clone();
        let mut projects = vec![project];

        let tracking_task =
            get_or_create_tracking_task_for_project(tasks_dir.path(), &mut projects, &project_id)
                .unwrap();

        assert_ne!(tracking_task.id, "already-deleted-task");
        assert_eq!(projects[0].tracking_task_id, Some(tracking_task.id));
    }

    #[test]
    fn get_or_create_tracking_task_for_project_errors_for_an_unknown_project() {
        let tasks_dir = tempdir().unwrap();
        let mut projects: Vec<Project> = vec![];

        let result = get_or_create_tracking_task_for_project(
            tasks_dir.path(),
            &mut projects,
            "does-not-exist",
        );

        assert!(result.is_err());
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
    fn apply_project_update_rejects_a_status_tier_rule_overrides_list_with_the_wrong_length() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.board = ProjectBoard {
            status_tier_rule_overrides: Some(vec![None, None, None]),
            ..Default::default()
        };

        let result = apply_project_update(&mut projects, 0, update, &Settings::default());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("4 entries"));
    }

    #[test]
    fn apply_project_update_accepts_a_valid_four_entry_status_tier_rule_overrides_list() {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let mut update = projects[0].clone();
        update.board = ProjectBoard {
            status_tier_rule_overrides: Some(vec![
                None,
                Some(StatusTierRule {
                    due_within_days: Some(2),
                    ..Default::default()
                }),
                None,
                None,
            ]),
            ..Default::default()
        };

        let updated = apply_project_update(&mut projects, 0, update, &Settings::default()).unwrap();

        assert_eq!(
            updated.board.status_tier_rule_overrides,
            Some(vec![
                None,
                Some(StatusTierRule {
                    due_within_days: Some(2),
                    ..Default::default()
                }),
                None,
                None
            ])
        );
    }

    #[test]
    fn apply_project_update_allows_unset_status_tier_rule_overrides_to_inherit_the_global_default()
    {
        let mut projects = vec![Project::new("Inbox".to_string(), "#abcdef".to_string(), 1)];
        let update = projects[0].clone();

        let updated = apply_project_update(&mut projects, 0, update, &Settings::default()).unwrap();

        assert_eq!(updated.board.status_tier_rule_overrides, None);
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

    /// Integration tests for the force-stop cascade wiring (part C of the
    /// time-tracking-engine Milestone 2 plan): `update_task` completing a
    /// task, `delete_subtask_container` completing every subtask,
    /// `finish_day` archiving a task, and `delete_task` removing a task,
    /// all force-stop any active time-tracking session first.
    ///
    /// These exercise [`force_stop_and_recompute_with`] (the pure helper
    /// behind [`force_stop_and_recompute`]) directly against a real tempdir
    /// + in-memory SQLite connection, combined with each call site's own
    /// real cascade-decision logic (`is_finished`, `force_done_except_cancelled`,
    /// `tasks_and_containers_to_finish`, `subtasks_and_container_to_delete`)
    /// — `#[tauri::command]` functions themselves can't be invoked directly
    /// in a unit test, since `tauri::State` has no public constructor
    /// outside of a running app (see [`force_stop_and_recompute_with`]'s
    /// own doc comment), so this is the closest faithful reproduction of
    /// each wiring point's actual end-to-end behavior.
    mod force_stop_cascade_tests {
        use super::*;
        use rusqlite::Connection;

        fn setup() -> (tempfile::TempDir, Connection) {
            let tasks_dir = tempdir().unwrap();
            let conn = Connection::open_in_memory().unwrap();
            time_storage::init_schema(&conn).unwrap();
            (tasks_dir, conn)
        }

        fn dt(rfc3339: &str) -> chrono::DateTime<chrono::Utc> {
            chrono::DateTime::parse_from_rfc3339(rfc3339)
                .unwrap()
                .with_timezone(&chrono::Utc)
        }

        // --- update_task's "completing a task force-stops it" path ---

        #[test]
        fn completing_a_task_force_stops_its_active_session_and_updates_tracked_minutes() {
            let (tasks_dir, conn) = setup();
            let task = Task::new("Write report".to_string());
            storage::save_task(tasks_dir.path(), &task).unwrap();
            time_storage::start_entry(&conn, &task.id, dt("2026-06-15T09:00:00+00:00")).unwrap();

            let settings = Settings::default();
            let mut existing = task.clone();
            existing.status = settings.done_status.clone();
            assert!(is_finished(&existing, &settings));

            // force_stop_and_recompute_with always ends the session at
            // chrono::Utc::now(), so the exact elapsed minutes depends on
            // wall-clock time at test-run time — assert only that *some*
            // positive amount of time was captured and persisted, not an
            // exact value, to avoid a flaky wall-clock-dependent assertion.
            let minutes =
                force_stop_and_recompute_with(&conn, tasks_dir.path(), &existing.id).unwrap();

            assert!(
                minutes > 0,
                "expected a positive elapsed duration, got {minutes}"
            );
            assert_eq!(
                time_storage::get_active_entry_for_task(&conn, &existing.id).unwrap(),
                None
            );
            let reloaded = storage::load_task(tasks_dir.path(), &existing.id).unwrap();
            assert_eq!(reloaded.tracked_minutes, minutes);
        }

        #[test]
        fn completing_a_task_with_no_active_session_is_a_no_op() {
            let (tasks_dir, conn) = setup();
            let task = Task::new("Write report".to_string());
            storage::save_task(tasks_dir.path(), &task).unwrap();

            let minutes = force_stop_and_recompute_with(&conn, tasks_dir.path(), &task.id).unwrap();

            assert_eq!(minutes, 0);
            assert_eq!(
                time_storage::get_active_entry_for_task(&conn, &task.id).unwrap(),
                None
            );
        }

        #[test]
        fn is_finished_does_not_flag_a_task_left_in_a_non_terminal_status() {
            let settings = Settings::default();
            let mut task = Task::new("Write report".to_string());
            task.status = "in-progress".to_string();

            assert!(!is_finished(&task, &settings));
        }

        // --- delete_subtask_container's per-subtask force-stop ---

        #[test]
        fn delete_subtask_container_cascade_force_stops_every_completed_subtasks_session() {
            let (tasks_dir, conn) = setup();
            let settings = Settings::default();

            let parent_project_id = "parent-project".to_string();
            let subtask_a = Task::new("Subtask A".to_string());
            let subtask_b = Task::new("Subtask B".to_string());
            storage::save_task(tasks_dir.path(), &subtask_a).unwrap();
            storage::save_task(tasks_dir.path(), &subtask_b).unwrap();

            time_storage::start_entry(&conn, &subtask_a.id, dt("2026-06-15T09:00:00+00:00"))
                .unwrap();
            time_storage::start_entry(&conn, &subtask_b.id, dt("2026-06-15T10:00:00+00:00"))
                .unwrap();

            let reassigned = vec![
                Task {
                    project_id: Some(parent_project_id.clone()),
                    ..subtask_a.clone()
                },
                Task {
                    project_id: Some(parent_project_id.clone()),
                    ..subtask_b.clone()
                },
            ];
            let completed = force_done_except_cancelled(
                reassigned,
                &settings.done_status,
                settings.cancelled_status.as_deref(),
            );
            assert_eq!(completed.len(), 2);

            for mut subtask in completed {
                subtask.tracked_minutes =
                    force_stop_and_recompute_with(&conn, tasks_dir.path(), &subtask.id).unwrap();
                storage::update_task(tasks_dir.path(), &subtask).unwrap();
            }

            assert_eq!(
                time_storage::get_active_entry_for_task(&conn, &subtask_a.id).unwrap(),
                None
            );
            assert_eq!(
                time_storage::get_active_entry_for_task(&conn, &subtask_b.id).unwrap(),
                None
            );
            let reloaded_a = storage::load_task(tasks_dir.path(), &subtask_a.id).unwrap();
            assert!(reloaded_a.tracked_minutes > 0);
        }

        #[test]
        fn delete_subtask_container_cascade_is_a_no_op_for_subtasks_with_no_active_session() {
            let (tasks_dir, conn) = setup();
            let settings = Settings::default();

            let subtask = Task::new("Subtask A".to_string());
            storage::save_task(tasks_dir.path(), &subtask).unwrap();

            let completed = force_done_except_cancelled(
                vec![subtask.clone()],
                &settings.done_status,
                settings.cancelled_status.as_deref(),
            );

            for mut subtask in completed {
                subtask.tracked_minutes =
                    force_stop_and_recompute_with(&conn, tasks_dir.path(), &subtask.id).unwrap();
                storage::update_task(tasks_dir.path(), &subtask).unwrap();
            }

            let reloaded = storage::load_task(tasks_dir.path(), &subtask.id).unwrap();
            assert_eq!(reloaded.tracked_minutes, 0);
        }

        // --- finish_day's force-stop-then-archive ordering ---

        #[test]
        fn finish_day_force_stops_and_persists_tracked_minutes_before_archiving() {
            let tasks_dir = tempdir().unwrap();
            let archive_dir = tempdir().unwrap();
            let conn = Connection::open_in_memory().unwrap();
            time_storage::init_schema(&conn).unwrap();
            let settings = Settings::default();

            let mut task = Task::new("Finish this".to_string());
            task.status = settings.done_status.clone();
            storage::save_task(tasks_dir.path(), &task).unwrap();
            time_storage::start_entry(&conn, &task.id, dt("2026-06-15T09:00:00+00:00")).unwrap();

            let (task_ids, _) = tasks_and_containers_to_finish(&[task.clone()], &settings);
            assert_eq!(task_ids, vec![task.id.clone()]);

            for task_id in &task_ids {
                let minutes =
                    force_stop_and_recompute_with(&conn, tasks_dir.path(), task_id).unwrap();
                let mut t = storage::load_task(tasks_dir.path(), task_id).unwrap();
                t.tracked_minutes = minutes;
                storage::update_task(tasks_dir.path(), &t).unwrap();

                storage::archive_task(tasks_dir.path(), archive_dir.path(), task_id).unwrap();
            }

            assert_eq!(
                time_storage::get_active_entry_for_task(&conn, &task.id).unwrap(),
                None
            );
            let archived = storage::load_task(archive_dir.path(), &task.id).unwrap();
            assert!(archived.tracked_minutes > 0);
        }

        #[test]
        fn finish_day_force_stop_is_a_no_op_when_nothing_was_tracking() {
            let tasks_dir = tempdir().unwrap();
            let archive_dir = tempdir().unwrap();
            let conn = Connection::open_in_memory().unwrap();
            time_storage::init_schema(&conn).unwrap();
            let settings = Settings::default();

            let mut task = Task::new("Finish this".to_string());
            task.status = settings.done_status.clone();
            storage::save_task(tasks_dir.path(), &task).unwrap();

            let (task_ids, _) = tasks_and_containers_to_finish(&[task.clone()], &settings);
            for task_id in &task_ids {
                let minutes =
                    force_stop_and_recompute_with(&conn, tasks_dir.path(), task_id).unwrap();
                let mut t = storage::load_task(tasks_dir.path(), task_id).unwrap();
                t.tracked_minutes = minutes;
                storage::update_task(tasks_dir.path(), &t).unwrap();
                storage::archive_task(tasks_dir.path(), archive_dir.path(), task_id).unwrap();
            }

            let archived = storage::load_task(archive_dir.path(), &task.id).unwrap();
            assert_eq!(archived.tracked_minutes, 0);
        }

        // --- delete_task's force-stop of itself and every cascaded subtask ---

        #[test]
        fn delete_task_force_stops_the_targets_active_session_before_deletion() {
            let (tasks_dir, conn) = setup();
            let task = Task::new("Delete me".to_string());
            storage::save_task(tasks_dir.path(), &task).unwrap();
            time_storage::start_entry(&conn, &task.id, dt("2026-06-15T09:00:00+00:00")).unwrap();

            force_stop_and_recompute_with(&conn, tasks_dir.path(), &task.id).unwrap();
            storage::delete_task(tasks_dir.path(), &task.id).unwrap();

            assert_eq!(
                time_storage::get_active_entry_for_task(&conn, &task.id).unwrap(),
                None
            );
        }

        #[test]
        fn delete_task_force_stops_every_cascaded_subtasks_active_session_too() {
            let (tasks_dir, conn) = setup();
            let parent = Task::new("Parent".to_string());
            let subtask = Task::new("Subtask".to_string());
            storage::save_task(tasks_dir.path(), &parent).unwrap();
            storage::save_task(tasks_dir.path(), &subtask).unwrap();
            time_storage::start_entry(&conn, &parent.id, dt("2026-06-15T09:00:00+00:00")).unwrap();
            time_storage::start_entry(&conn, &subtask.id, dt("2026-06-15T09:30:00+00:00")).unwrap();

            force_stop_and_recompute_with(&conn, tasks_dir.path(), &parent.id).unwrap();
            for subtask_id in [&subtask.id] {
                force_stop_and_recompute_with(&conn, tasks_dir.path(), subtask_id).unwrap();
                storage::delete_task(tasks_dir.path(), subtask_id).unwrap();
            }
            storage::delete_task(tasks_dir.path(), &parent.id).unwrap();

            assert_eq!(
                time_storage::get_active_entry_for_task(&conn, &parent.id).unwrap(),
                None
            );
            assert_eq!(
                time_storage::get_active_entry_for_task(&conn, &subtask.id).unwrap(),
                None
            );
        }

        #[test]
        fn delete_task_force_stop_is_a_no_op_when_no_session_was_active() {
            let (tasks_dir, conn) = setup();
            let task = Task::new("Delete me".to_string());
            storage::save_task(tasks_dir.path(), &task).unwrap();

            let result = force_stop_and_recompute_with(&conn, tasks_dir.path(), &task.id);
            storage::delete_task(tasks_dir.path(), &task.id).unwrap();

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        }
    }

    mod time_tracking_command_helper_tests {
        use super::*;
        use rusqlite::Connection;

        fn setup() -> (tempfile::TempDir, Connection) {
            let tasks_dir = tempdir().unwrap();
            let conn = Connection::open_in_memory().unwrap();
            time_storage::init_schema(&conn).unwrap();
            (tasks_dir, conn)
        }

        fn dt(rfc3339: &str) -> chrono::DateTime<chrono::Utc> {
            chrono::DateTime::parse_from_rfc3339(rfc3339)
                .unwrap()
                .with_timezone(&chrono::Utc)
        }

        #[test]
        fn validate_orphaned_session_action_accepts_resume() {
            assert!(validate_orphaned_session_action("resume").is_ok());
        }

        #[test]
        fn validate_orphaned_session_action_accepts_discard() {
            assert!(validate_orphaned_session_action("discard").is_ok());
        }

        #[test]
        fn validate_orphaned_session_action_rejects_an_unknown_action() {
            let err = validate_orphaned_session_action("delete-forever").unwrap_err();
            assert!(err.contains("delete-forever"));
        }

        #[test]
        fn validate_orphaned_session_action_rejects_an_empty_action() {
            assert!(validate_orphaned_session_action("").is_err());
        }

        #[test]
        fn parse_rfc3339_field_accepts_a_valid_timestamp() {
            let result = parse_rfc3339_field("2026-06-15T09:00:00+00:00", "started_at");
            assert!(result.is_ok());
        }

        #[test]
        fn parse_rfc3339_field_rejects_a_malformed_timestamp() {
            let err = parse_rfc3339_field("not-a-date", "started_at").unwrap_err();
            assert!(err.contains("started_at"));
        }

        #[test]
        fn parse_rfc3339_input_accepts_a_valid_timestamp() {
            let result = parse_rfc3339_input("2026-06-15T09:00:00+00:00", "started_at");
            assert!(result.is_ok());
        }

        #[test]
        fn parse_rfc3339_input_rejects_a_malformed_timestamp() {
            let err = parse_rfc3339_input("not-a-date", "ended_at").unwrap_err();
            assert!(err.contains("ended_at"));
        }

        #[test]
        fn discard_orphaned_session_with_ends_at_the_last_heartbeat() {
            let (tasks_dir, conn) = setup();
            let task = Task::new("Tracked task".to_string());
            storage::save_task(tasks_dir.path(), &task).unwrap();
            let entry = time_storage::start_entry(&conn, &task.id, dt("2026-06-15T09:00:00+00:00"))
                .unwrap();
            time_storage::update_heartbeat(&conn, &task.id, dt("2026-06-15T09:20:00+00:00"))
                .unwrap();

            discard_orphaned_session_with(&conn, tasks_dir.path(), &entry.id).unwrap();

            let stored = time_storage::get_entry(&conn, &entry.id).unwrap().unwrap();
            assert_eq!(
                stored.ended_at,
                Some("2026-06-15T09:20:00+00:00".to_string())
            );
            let reloaded = storage::load_task(tasks_dir.path(), &task.id).unwrap();
            assert_eq!(reloaded.tracked_minutes, 20);
        }

        #[test]
        fn discard_orphaned_session_with_falls_back_to_started_at_when_no_heartbeat_was_ever_recorded(
        ) {
            let (tasks_dir, conn) = setup();
            let task = Task::new("Tracked task".to_string());
            storage::save_task(tasks_dir.path(), &task).unwrap();
            let entry = time_storage::start_entry(&conn, &task.id, dt("2026-06-15T09:00:00+00:00"))
                .unwrap();

            discard_orphaned_session_with(&conn, tasks_dir.path(), &entry.id).unwrap();

            let stored = time_storage::get_entry(&conn, &entry.id).unwrap().unwrap();
            assert_eq!(
                stored.ended_at,
                Some("2026-06-15T09:00:00+00:00".to_string())
            );
            let reloaded = storage::load_task(tasks_dir.path(), &task.id).unwrap();
            assert_eq!(reloaded.tracked_minutes, 0);
        }

        #[test]
        fn discard_orphaned_session_with_errors_for_an_unknown_entry_id() {
            let (tasks_dir, conn) = setup();

            let result = discard_orphaned_session_with(&conn, tasks_dir.path(), "does-not-exist");

            assert!(result.is_err());
        }

        #[test]
        fn discard_orphaned_session_with_does_not_disturb_other_tasks_active_sessions() {
            let (tasks_dir, conn) = setup();
            let task_a = Task::new("Task A".to_string());
            let task_b = Task::new("Task B".to_string());
            storage::save_task(tasks_dir.path(), &task_a).unwrap();
            storage::save_task(tasks_dir.path(), &task_b).unwrap();
            let entry_a =
                time_storage::start_entry(&conn, &task_a.id, dt("2026-06-15T09:00:00+00:00"))
                    .unwrap();
            time_storage::start_entry(&conn, &task_b.id, dt("2026-06-15T09:00:00+00:00")).unwrap();

            discard_orphaned_session_with(&conn, tasks_dir.path(), &entry_a.id).unwrap();

            assert_eq!(
                time_storage::get_active_entry_for_task(&conn, &task_a.id).unwrap(),
                None
            );
            assert!(time_storage::get_active_entry_for_task(&conn, &task_b.id)
                .unwrap()
                .is_some());
        }

        #[test]
        fn validate_time_range_accepts_an_end_strictly_after_start() {
            let started = dt("2026-06-15T09:00:00+00:00");
            let ended = dt("2026-06-15T09:30:00+00:00");

            assert!(validate_time_range(started, Some(ended)).is_ok());
        }

        #[test]
        fn validate_time_range_accepts_a_none_end_reopening_the_entry() {
            let started = dt("2026-06-15T09:00:00+00:00");

            assert!(validate_time_range(started, None).is_ok());
        }

        #[test]
        fn validate_time_range_rejects_an_end_equal_to_start() {
            let started = dt("2026-06-15T09:00:00+00:00");

            let err = validate_time_range(started, Some(started)).unwrap_err();
            assert!(err.contains("end time"));
        }

        #[test]
        fn validate_time_range_rejects_an_end_before_start() {
            let started = dt("2026-06-15T09:30:00+00:00");
            let ended = dt("2026-06-15T09:00:00+00:00");

            assert!(validate_time_range(started, Some(ended)).is_err());
        }

        #[test]
        fn manual_entry_then_recompute_updates_tracked_minutes() {
            let (tasks_dir, conn) = setup();
            let task = Task::new("Write report".to_string());
            storage::save_task(tasks_dir.path(), &task).unwrap();

            let started = dt("2026-06-15T09:00:00+00:00");
            let ended = dt("2026-06-15T09:45:00+00:00");
            validate_time_range(started, Some(ended)).unwrap();
            time_storage::insert_completed_entry(&conn, &task.id, started, ended).unwrap();
            time_tracking::recompute_and_persist_tracked_minutes(&conn, tasks_dir.path(), &task.id)
                .unwrap();

            let reloaded = storage::load_task(tasks_dir.path(), &task.id).unwrap();
            assert_eq!(reloaded.tracked_minutes, 45);
        }

        #[test]
        fn update_entry_then_recompute_reflects_the_corrected_duration() {
            let (tasks_dir, conn) = setup();
            let task = Task::new("Write report".to_string());
            storage::save_task(tasks_dir.path(), &task).unwrap();
            let entry = time_storage::insert_completed_entry(
                &conn,
                &task.id,
                dt("2026-06-15T09:00:00+00:00"),
                dt("2026-06-15T09:10:00+00:00"),
            )
            .unwrap();
            time_tracking::recompute_and_persist_tracked_minutes(&conn, tasks_dir.path(), &task.id)
                .unwrap();

            let corrected_started = dt("2026-06-15T09:00:00+00:00");
            let corrected_ended = dt("2026-06-15T10:00:00+00:00");
            validate_time_range(corrected_started, Some(corrected_ended)).unwrap();
            time_storage::update_entry_times(
                &conn,
                &entry.id,
                corrected_started,
                Some(corrected_ended),
            )
            .unwrap();
            time_tracking::recompute_and_persist_tracked_minutes(&conn, tasks_dir.path(), &task.id)
                .unwrap();

            let reloaded = storage::load_task(tasks_dir.path(), &task.id).unwrap();
            assert_eq!(reloaded.tracked_minutes, 60);
        }

        #[test]
        fn delete_entry_then_recompute_drops_its_contribution() {
            let (tasks_dir, conn) = setup();
            let task = Task::new("Write report".to_string());
            storage::save_task(tasks_dir.path(), &task).unwrap();
            let entry = time_storage::insert_completed_entry(
                &conn,
                &task.id,
                dt("2026-06-15T09:00:00+00:00"),
                dt("2026-06-15T09:30:00+00:00"),
            )
            .unwrap();
            time_tracking::recompute_and_persist_tracked_minutes(&conn, tasks_dir.path(), &task.id)
                .unwrap();

            time_storage::delete_entry(&conn, &entry.id).unwrap();
            time_tracking::recompute_and_persist_tracked_minutes(&conn, tasks_dir.path(), &task.id)
                .unwrap();

            let reloaded = storage::load_task(tasks_dir.path(), &task.id).unwrap();
            assert_eq!(reloaded.tracked_minutes, 0);
        }
    }

    mod resolve_show_subproject_tasks_tests {
        use super::*;

        fn project_with_id(id: &str) -> Project {
            let mut p = Project::new(id.to_string(), "#111111".to_string(), 1);
            p.id = id.to_string();
            p
        }

        #[test]
        fn falls_back_to_the_global_default_when_no_project_in_the_chain_has_set_one() {
            let project = project_with_id("p1");
            let settings = Settings {
                show_subproject_tasks_default: true,
                ..Settings::default()
            };

            let result = resolve_show_subproject_tasks(&[project], "p1", &settings);

            assert!(result);
        }

        #[test]
        fn uses_the_projects_own_override_when_set() {
            let mut project = project_with_id("p1");
            project.board.show_subproject_tasks = Some(true);
            let settings = Settings {
                show_subproject_tasks_default: false,
                ..Settings::default()
            };

            let result = resolve_show_subproject_tasks(&[project], "p1", &settings);

            assert!(result);
        }

        #[test]
        fn walks_up_to_the_nearest_ancestor_with_an_override_when_self_has_none() {
            let mut parent = project_with_id("parent");
            parent.board.show_subproject_tasks = Some(true);
            let mut child = project_with_id("child");
            child.parent_id = Some("parent".to_string());
            let settings = Settings {
                show_subproject_tasks_default: false,
                ..Settings::default()
            };

            let result = resolve_show_subproject_tasks(&[parent, child], "child", &settings);

            assert!(result);
        }

        #[test]
        fn nearest_override_wins_over_a_more_distant_ancestors_override() {
            let mut grandparent = project_with_id("grandparent");
            grandparent.board.show_subproject_tasks = Some(true);
            let mut parent = project_with_id("parent");
            parent.parent_id = Some("grandparent".to_string());
            parent.board.show_subproject_tasks = Some(false);
            let mut child = project_with_id("child");
            child.parent_id = Some("parent".to_string());
            let settings = Settings::default();

            let result =
                resolve_show_subproject_tasks(&[grandparent, parent, child], "child", &settings);

            assert!(!result);
        }

        #[test]
        fn missing_project_id_falls_back_to_the_global_default() {
            let settings = Settings {
                show_subproject_tasks_default: true,
                ..Settings::default()
            };

            let result = resolve_show_subproject_tasks(&[], "does-not-exist", &settings);

            assert!(result);
        }
    }

    mod resolve_status_line_layout_id_tests {
        use super::*;

        #[test]
        fn uses_the_projects_own_override_when_set() {
            let mut project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);
            project.board.status_line_layout_id = Some("layout-override".to_string());
            let settings = Settings {
                default_status_line_layout_id: "layout-default".to_string(),
                ..Settings::default()
            };

            let result = resolve_status_line_layout_id(&project, &settings);

            assert_eq!(result, "layout-override");
        }

        #[test]
        fn falls_back_to_the_global_default_when_unset() {
            let project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);
            let settings = Settings {
                default_status_line_layout_id: "layout-default".to_string(),
                ..Settings::default()
            };

            let result = resolve_status_line_layout_id(&project, &settings);

            assert_eq!(result, "layout-default");
        }

        #[test]
        fn does_not_walk_ancestors_unlike_show_subproject_tasks() {
            // A parent's override must never leak onto a child that has its
            // own field unset — status_line_layout_id resolves single-level
            // only, so this only has the global default to fall back to.
            let project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);
            let settings = Settings {
                default_status_line_layout_id: "global-default".to_string(),
                ..Settings::default()
            };

            let result = resolve_status_line_layout_id(&project, &settings);

            assert_eq!(result, "global-default");
        }
    }

    mod build_project_status_stats_tests {
        use super::*;
        use rusqlite::Connection;

        fn setup_conn() -> Connection {
            let conn = Connection::open_in_memory().unwrap();
            time_storage::init_schema(&conn).unwrap();
            conn
        }

        fn project_with_id(id: &str) -> Project {
            let mut p = Project::new(id.to_string(), "#111111".to_string(), 1);
            p.id = id.to_string();
            p
        }

        fn today() -> NaiveDate {
            NaiveDate::from_ymd_opt(2026, 6, 24).unwrap()
        }

        fn now() -> DateTime<Utc> {
            DateTime::parse_from_rfc3339("2026-06-24T21:00:00+00:00")
                .unwrap()
                .with_timezone(&Utc)
        }

        fn settings_with_done(done: &str) -> Settings {
            Settings {
                done_status: done.to_string(),
                ..Settings::default()
            }
        }

        #[test]
        fn errors_when_project_id_does_not_exist() {
            let conn = setup_conn();
            let settings = settings_with_done("done");

            let result = build_project_status_stats(
                &conn,
                "does-not-exist",
                &[],
                &[],
                &[],
                &settings,
                "monday",
                today(),
                now(),
            );

            let err = result.unwrap_err();
            assert!(err.contains("does-not-exist"));
        }

        #[test]
        fn an_empty_project_with_no_tasks_is_great_with_zeroed_stats() {
            let conn = setup_conn();
            let project = project_with_id("p1");
            let settings = settings_with_done("done");

            let result = build_project_status_stats(
                &conn,
                "p1",
                &[project],
                &[],
                &[],
                &settings,
                "monday",
                today(),
                now(),
            )
            .unwrap();

            assert_eq!(result.status_tier, StatusTier::Great);
            assert_eq!(result.estimated_time_left, 0);
            assert_eq!(result.total_time_tracked, 0);
            assert_eq!(result.avg_time_per_week, 0.0);
            assert_eq!(result.completion_pct, None);
            assert_eq!(result.weighted_completion_pct, None);
        }

        #[test]
        fn an_overdue_task_drives_the_status_tier_to_severe() {
            let conn = setup_conn();
            let project = project_with_id("p1");
            let mut overdue_task = Task::new("Overdue".to_string());
            overdue_task.project_id = Some("p1".to_string());
            overdue_task.due = Some("2026-06-20".to_string());
            let settings = settings_with_done("done");

            let tasks = vec![overdue_task];
            let result = build_project_status_stats(
                &conn,
                "p1",
                &[project],
                &tasks,
                &tasks,
                &settings,
                "monday",
                today(),
                now(),
            )
            .unwrap();

            assert_eq!(result.status_tier, StatusTier::Severe);
        }

        #[test]
        fn status_tier_task_scan_never_rolls_up_into_subprojects_even_when_rollup_is_on() {
            let mut parent = project_with_id("parent");
            parent.board.show_subproject_tasks = Some(true);
            let mut child = project_with_id("child");
            child.parent_id = Some("parent".to_string());
            let mut child_overdue = Task::new("Overdue in child".to_string());
            child_overdue.project_id = Some("child".to_string());
            child_overdue.due = Some("2020-01-01".to_string());
            let conn = setup_conn();
            let settings = settings_with_done("done");

            let tasks = vec![child_overdue];
            let result = build_project_status_stats(
                &conn,
                "parent",
                &[parent, child],
                &tasks,
                &tasks,
                &settings,
                "monday",
                today(),
                now(),
            )
            .unwrap();

            // A busy subproject can never flip its parent's badge, per the
            // spec's resolved rollup-scope decision — even with rollup on.
            assert_eq!(result.status_tier, StatusTier::Great);
        }

        #[test]
        fn project_level_status_tier_rule_override_wins_over_the_global_rule() {
            let conn = setup_conn();
            let mut project = project_with_id("p1");
            // Override the severe slot to a much tighter window than the
            // 0-day global default, so a task due in 2 days wouldn't match
            // the global rule but does match the override.
            project.board.status_tier_rule_overrides = Some(vec![
                Some(crate::settings::StatusTierRule {
                    due_within_days: Some(2),
                    min_priority: None,
                    estimated_time_left_exceeds_minutes: None,
                }),
                None,
                None,
                None,
            ]);
            let mut due_soon = Task::new("Due soon".to_string());
            due_soon.project_id = Some("p1".to_string());
            due_soon.due = Some("2026-06-26".to_string());
            let settings = settings_with_done("done");

            let tasks = vec![due_soon];
            let result = build_project_status_stats(
                &conn,
                "p1",
                &[project],
                &tasks,
                &tasks,
                &settings,
                "monday",
                today(),
                now(),
            )
            .unwrap();

            assert_eq!(result.status_tier, StatusTier::Severe);
        }

        #[test]
        fn total_time_tracked_rolls_up_subprojects_when_show_subproject_tasks_is_on() {
            let mut parent = project_with_id("parent");
            parent.board.show_subproject_tasks = Some(true);
            let mut child = project_with_id("child");
            child.parent_id = Some("parent".to_string());
            let mut child_task = Task::new("Child task".to_string());
            child_task.project_id = Some("child".to_string());
            child_task.tracked_minutes = 42;
            let conn = setup_conn();
            let settings = settings_with_done("done");

            let tasks = vec![child_task];
            let result = build_project_status_stats(
                &conn,
                "parent",
                &[parent, child],
                &tasks,
                &tasks,
                &settings,
                "monday",
                today(),
                now(),
            )
            .unwrap();

            assert_eq!(result.total_time_tracked, 42);
        }

        #[test]
        fn total_time_tracked_excludes_subprojects_when_rollup_is_off() {
            let parent = project_with_id("parent");
            let mut child = project_with_id("child");
            child.parent_id = Some("parent".to_string());
            let mut child_task = Task::new("Child task".to_string());
            child_task.project_id = Some("child".to_string());
            child_task.tracked_minutes = 42;
            let conn = setup_conn();
            let settings = settings_with_done("done");

            let tasks = vec![child_task];
            let result = build_project_status_stats(
                &conn,
                "parent",
                &[parent, child],
                &tasks,
                &tasks,
                &settings,
                "monday",
                today(),
                now(),
            )
            .unwrap();

            assert_eq!(result.total_time_tracked, 0);
        }

        #[test]
        fn completion_pct_reflects_done_vs_total_over_the_projects_own_tasks() {
            let conn = setup_conn();
            let project = project_with_id("p1");
            let mut done_task = Task::new("Done".to_string());
            done_task.project_id = Some("p1".to_string());
            done_task.status = "done".to_string();
            let mut not_done_task = Task::new("Not done".to_string());
            not_done_task.project_id = Some("p1".to_string());
            let settings = settings_with_done("done");

            let tasks = vec![done_task, not_done_task];
            let result = build_project_status_stats(
                &conn,
                "p1",
                &[project],
                &tasks,
                &tasks,
                &settings,
                "monday",
                today(),
                now(),
            )
            .unwrap();

            assert_eq!(result.completion_pct, Some(0.5));
        }

        #[test]
        fn weighted_completion_pct_weights_by_estimated_minutes() {
            let conn = setup_conn();
            let project = project_with_id("p1");
            let mut done_task = Task::new("Done".to_string());
            done_task.project_id = Some("p1".to_string());
            done_task.status = "done".to_string();
            done_task.estimated_minutes = Some(30);
            let mut not_done_task = Task::new("Not done".to_string());
            not_done_task.project_id = Some("p1".to_string());
            not_done_task.estimated_minutes = Some(90);
            let settings = settings_with_done("done");

            let tasks = vec![done_task, not_done_task];
            let result = build_project_status_stats(
                &conn,
                "p1",
                &[project],
                &tasks,
                &tasks,
                &settings,
                "monday",
                today(),
                now(),
            )
            .unwrap();

            assert_eq!(result.weighted_completion_pct, Some(30.0 / 120.0));
        }

        #[test]
        fn estimated_time_left_sums_scheduled_incomplete_tasks_estimate_minus_tracked() {
            let conn = setup_conn();
            let project = project_with_id("p1");
            let mut task = Task::new("Has estimate".to_string());
            task.project_id = Some("p1".to_string());
            task.scheduled = Some("2026-06-24".to_string());
            task.estimated_minutes = Some(100);
            task.tracked_minutes = 30;
            let settings = settings_with_done("done");

            let tasks = vec![task];
            let result = build_project_status_stats(
                &conn,
                "p1",
                &[project],
                &tasks,
                &tasks,
                &settings,
                "monday",
                today(),
                now(),
            )
            .unwrap();

            assert_eq!(result.estimated_time_left, 70);
        }

        #[test]
        fn avg_time_per_week_is_in_seconds_not_minutes() {
            let conn = setup_conn();
            let project = project_with_id("p1");
            let mut task = Task::new("Tracked".to_string());
            task.project_id = Some("p1".to_string());
            time_storage::insert_completed_entry(
                &conn,
                &task.id,
                DateTime::parse_from_rfc3339("2026-06-16T08:00:00+00:00")
                    .unwrap()
                    .with_timezone(&Utc),
                DateTime::parse_from_rfc3339("2026-06-16T09:00:00+00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            )
            .unwrap();
            let settings = settings_with_done("done");

            let tasks = vec![task];
            let result = build_project_status_stats(
                &conn,
                "p1",
                &[project],
                &tasks,
                &tasks,
                &settings,
                "monday",
                today(),
                now(),
            )
            .unwrap();

            // 1 tracked hour = 3600 seconds, not 60 minutes — confirms the
            // stat stays in status_stats::avg_time_per_week's native unit.
            assert_eq!(result.avg_time_per_week, 3600.0);
        }

        #[test]
        fn effective_layout_id_uses_the_projects_own_override() {
            let conn = setup_conn();
            let mut project = project_with_id("p1");
            project.board.status_line_layout_id = Some("project-layout".to_string());
            let settings = Settings {
                default_status_line_layout_id: "global-layout".to_string(),
                ..settings_with_done("done")
            };

            let result = build_project_status_stats(
                &conn,
                "p1",
                &[project],
                &[],
                &[],
                &settings,
                "monday",
                today(),
                now(),
            )
            .unwrap();

            assert_eq!(result.effective_layout_id, "project-layout");
        }

        #[test]
        fn effective_layout_id_falls_back_to_the_global_default() {
            let conn = setup_conn();
            let project = project_with_id("p1");
            let settings = Settings {
                default_status_line_layout_id: "global-layout".to_string(),
                ..settings_with_done("done")
            };

            let result = build_project_status_stats(
                &conn,
                "p1",
                &[project],
                &[],
                &[],
                &settings,
                "monday",
                today(),
                now(),
            )
            .unwrap();

            assert_eq!(result.effective_layout_id, "global-layout");
        }

        #[test]
        fn status_tier_serializes_to_a_stable_snake_case_string() {
            assert_eq!(
                serde_json::to_string(&StatusTier::Severe).unwrap(),
                "\"severe\""
            );
            assert_eq!(
                serde_json::to_string(&StatusTier::Critical).unwrap(),
                "\"critical\""
            );
            assert_eq!(
                serde_json::to_string(&StatusTier::NeedsAttention).unwrap(),
                "\"needs_attention\""
            );
            assert_eq!(
                serde_json::to_string(&StatusTier::OnTrack).unwrap(),
                "\"on_track\""
            );
            assert_eq!(
                serde_json::to_string(&StatusTier::Great).unwrap(),
                "\"great\""
            );
        }
    }

    mod layout_command_helper_tests {
        use super::*;

        fn project_with_id(id: &str) -> Project {
            let mut p = Project::new(id.to_string(), "#111111".to_string(), 1);
            p.id = id.to_string();
            p
        }

        // --- list_status_layouts / create_status_layout / update_status_layout
        // / duplicate_status_layout: thin #[tauri::command] wrappers around
        // already-tested layout/layout_storage functions, so the meaningful
        // behavior to test directly is ensure_layout_not_in_use, the one
        // piece of non-trivial logic introduced for delete_status_layout. ---

        #[test]
        fn ensure_layout_not_in_use_allows_deleting_an_unreferenced_layout() {
            let settings = Settings {
                default_status_line_layout_id: "other-layout".to_string(),
                ..Settings::default()
            };

            let result = ensure_layout_not_in_use("layout-1", &[], &settings);

            assert!(result.is_ok());
        }

        #[test]
        fn ensure_layout_not_in_use_refuses_when_it_is_the_global_default() {
            let settings = Settings {
                default_status_line_layout_id: "layout-1".to_string(),
                ..Settings::default()
            };

            let err = ensure_layout_not_in_use("layout-1", &[], &settings).unwrap_err();

            assert!(err.contains("global default"));
        }

        #[test]
        fn ensure_layout_not_in_use_refuses_when_a_project_references_it() {
            let mut project = project_with_id("p1");
            project.board.status_line_layout_id = Some("layout-1".to_string());
            let settings = Settings {
                default_status_line_layout_id: "other-layout".to_string(),
                ..Settings::default()
            };

            let err = ensure_layout_not_in_use("layout-1", &[project], &settings).unwrap_err();

            assert!(err.contains('1'));
            assert!(!err.contains("global default"));
        }

        #[test]
        fn ensure_layout_not_in_use_counts_every_referencing_project() {
            let mut p1 = project_with_id("p1");
            p1.board.status_line_layout_id = Some("layout-1".to_string());
            let mut p2 = project_with_id("p2");
            p2.board.status_line_layout_id = Some("layout-1".to_string());
            let p3 = project_with_id("p3"); // does not reference it
            let settings = Settings {
                default_status_line_layout_id: "other-layout".to_string(),
                ..Settings::default()
            };

            let err = ensure_layout_not_in_use("layout-1", &[p1, p2, p3], &settings).unwrap_err();

            assert!(err.contains('2'));
        }

        #[test]
        fn ensure_layout_not_in_use_reports_both_a_project_and_the_global_default_together() {
            let mut project = project_with_id("p1");
            project.board.status_line_layout_id = Some("layout-1".to_string());
            let settings = Settings {
                default_status_line_layout_id: "layout-1".to_string(),
                ..Settings::default()
            };

            let err = ensure_layout_not_in_use("layout-1", &[project], &settings).unwrap_err();

            assert!(err.contains('1'));
            assert!(err.contains("global default"));
        }

        #[test]
        fn ensure_layout_not_in_use_ignores_a_project_referencing_a_different_layout() {
            let mut project = project_with_id("p1");
            project.board.status_line_layout_id = Some("some-other-layout".to_string());
            let settings = Settings {
                default_status_line_layout_id: "yet-another-layout".to_string(),
                ..Settings::default()
            };

            let result = ensure_layout_not_in_use("layout-1", &[project], &settings);

            assert!(result.is_ok());
        }
    }
}
