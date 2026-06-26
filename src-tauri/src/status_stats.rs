use chrono::{DateTime, Datelike, NaiveDate, Utc};
use rusqlite::Connection;

use crate::commands::effective_default_estimated_minutes;
use crate::project::Project;
use crate::project_tree;
use crate::settings::Settings;
use crate::task::Task;
use crate::time_storage::{self, TimeStorageError};

/// Returns `true` if `task` belongs directly to `project_id` (no rollup —
/// every caller in this module that needs subproject rollup does so
/// explicitly via [`project_and_descendant_ids`] rather than baking rollup
/// into this check itself).
fn belongs_to_project(task: &Task, project_id: &str) -> bool {
    task.project_id.as_deref() == Some(project_id)
}

/// Returns `true` if `task.status` is neither `settings.done_status` nor
/// `settings.cancelled_status` (when one is configured) — i.e. still
/// "incomplete" work, the population the status-tier evaluator and
/// `estimated_time_left` scan over. `pub(crate)` so `commands::
/// build_project_status_stats` can reuse this exact rule for its own
/// incomplete-task scan rather than re-deriving it inline, the same reuse
/// reason `effective_default_estimated_minutes` was promoted for.
pub(crate) fn is_incomplete(task: &Task, settings: &Settings) -> bool {
    task.status != settings.done_status
        && settings
            .cancelled_status
            .as_deref()
            .is_none_or(|cancelled| task.status != cancelled)
}

/// Returns `project_id` followed by every transitive descendant's id, when
/// `include_subprojects` is `true`; otherwise just `project_id` alone. Shared
/// by every stat in this module that respects the `show_subproject_tasks`
/// rollup toggle (`total_time_tracked`, `completion_pct`,
/// `weighted_completion_pct`, `avg_time_per_week`) — `estimated_time_left`
/// and the status-tier evaluator deliberately never call this with `true`,
/// per the project-status-line spec's resolved rollup-scope decision.
fn project_and_descendant_ids(
    projects: &[Project],
    project_id: &str,
    include_subprojects: bool,
) -> Vec<String> {
    let mut ids = vec![project_id.to_string()];
    if include_subprojects {
        ids.extend(
            project_tree::descendants_of(projects, project_id)
                .into_iter()
                .map(|p| p.id.clone()),
        );
    }
    ids
}

/// `estimated_time_left`: sum of `max(0, estimated_minutes - tracked_minutes)`
/// over `project_id`'s own non-hidden, incomplete tasks that have *both* an
/// estimate and a `scheduled` date set — a task with no estimate at all is
/// skipped entirely (not counted as `0`), so the metric isn't skewed by
/// unestimated work. Never rolled up into subprojects, even when
/// `show_subproject_tasks` is on — per the spec's resolved rollup-scope
/// decision, a busy subproject can never flip its parent's badge or time-left
/// figure.
pub fn estimated_time_left(project_id: &str, tasks: &[Task], settings: &Settings) -> u32 {
    // Include tasks inside subtask containers owned by tasks directly in project_id,
    // so parent-task estimates and their subtask estimates are both counted.
    let subtask_container_ids: std::collections::HashSet<&str> = tasks
        .iter()
        .filter(|t| belongs_to_project(t, project_id) && !t.hidden)
        .filter_map(|t| t.subtask_project_id.as_deref())
        .collect();

    tasks
        .iter()
        .filter(|task| {
            let in_scope = belongs_to_project(task, project_id)
                || task
                    .project_id
                    .as_deref()
                    .map_or(false, |pid| subtask_container_ids.contains(pid));
            in_scope
                && !task.hidden
                && task.series_id.is_none()
                && is_incomplete(task, settings)
                && task.scheduled.is_some()
        })
        .filter_map(|task| {
            task.estimated_minutes
                .map(|estimate| (estimate, task.tracked_minutes))
        })
        .map(|(estimate, tracked)| estimate.saturating_sub(tracked))
        .sum()
}

/// `total_time_tracked`: sum of `tracked_minutes` across `project_id`'s own
/// non-hidden tasks, plus its own hidden tracker task's `tracked_minutes`
/// (via `Project.tracking_task_id`) — this is the one stat where the hidden
/// tracker's own time explicitly counts, since "track the project as a
/// whole" is the whole point of that hidden task existing. When
/// `include_subprojects` is `true` (mirrors the caller's resolved
/// `show_subproject_tasks` setting), the same calculation (own tasks + own
/// hidden tracker) is added recursively for every descendant subproject too.
pub fn total_time_tracked(
    project_id: &str,
    projects: &[Project],
    tasks: &[Task],
    include_subprojects: bool,
) -> u32 {
    project_and_descendant_ids(projects, project_id, include_subprojects)
        .iter()
        .map(|id| total_time_tracked_for_single_project(id, projects, tasks))
        .sum()
}

/// The non-recursive "this one project's own tasks plus its own hidden
/// tracker" calculation that [`total_time_tracked`] sums across a project
/// and (optionally) its descendants.
fn total_time_tracked_for_single_project(
    project_id: &str,
    projects: &[Project],
    tasks: &[Task],
) -> u32 {
    let own_tasks_total: u32 = tasks
        .iter()
        .filter(|task| belongs_to_project(task, project_id) && !task.hidden)
        .map(|task| task.tracked_minutes)
        .sum();

    let tracker_total = projects
        .iter()
        .find(|p| p.id == project_id)
        .and_then(|p| p.tracking_task_id.as_deref())
        .and_then(|tracker_id| tasks.iter().find(|t| t.id == tracker_id))
        .map(|tracker| tracker.tracked_minutes)
        .unwrap_or(0);

    own_tasks_total + tracker_total
}

/// Returns `true` if `task` should be dropped entirely (both numerator and
/// denominator) from the `completion_pct`/`weighted_completion_pct`
/// population: hidden, any recurring occurrence (`series_id.is_some()`, no
/// exceptions), or cancelled (`status == cancelled_status`, when configured).
fn excluded_from_completion_population(task: &Task, settings: &Settings) -> bool {
    task.hidden
        || task.series_id.is_some()
        || settings
            .cancelled_status
            .as_deref()
            .is_some_and(|cancelled| task.status == cancelled)
}

/// Returns the surviving population for `completion_pct`/
/// `weighted_completion_pct`: every task (from the combined `tasks_dir` +
/// `archive_dir` lists the caller already merged into `all_tasks`) belonging
/// to `project_id` or — when `include_subprojects` is `true` — one of its
/// descendants, with hidden/recurring/cancelled tasks dropped (see
/// [`excluded_from_completion_population`]).
fn completion_population<'a>(
    project_id: &str,
    projects: &[Project],
    all_tasks: &'a [Task],
    include_subprojects: bool,
    settings: &Settings,
) -> Vec<&'a Task> {
    let ids = project_and_descendant_ids(projects, project_id, include_subprojects);
    all_tasks
        .iter()
        .filter(|task| {
            task.project_id
                .as_deref()
                .is_some_and(|id| ids.iter().any(|p| p == id))
                && !excluded_from_completion_population(task, settings)
        })
        .collect()
}

/// `completion_pct`: `(completed tasks) / (surviving tasks)` over the
/// project's (and, when rolled up, its descendants') combined `tasks_dir` +
/// `archive_dir` population, after dropping hidden/recurring/cancelled tasks
/// (see [`completion_population`]). `None` when the surviving population is
/// empty — there's nothing meaningful to divide by, so this deliberately
/// doesn't return a misleading `0.0` or `100.0`.
pub fn completion_pct(
    project_id: &str,
    projects: &[Project],
    all_tasks: &[Task],
    include_subprojects: bool,
    settings: &Settings,
) -> Option<f64> {
    let population = completion_population(
        project_id,
        projects,
        all_tasks,
        include_subprojects,
        settings,
    );
    if population.is_empty() {
        return None;
    }

    let completed = population
        .iter()
        .filter(|task| task.status == settings.done_status)
        .count();

    Some(completed as f64 / population.len() as f64)
}

/// Resolves the weight `weighted_completion_pct` uses for `task`: the task's
/// own `estimated_minutes` if set, else
/// [`effective_default_estimated_minutes`] for the task's own project chain,
/// else `None` if even that's unset — meaning the task is dropped from both
/// the numerator and denominator entirely, since there's truly no number to
/// weight it by. Reuses `commands::effective_default_estimated_minutes`
/// rather than reimplementing the project-then-global fallback chain.
fn resolve_completion_weight(
    task: &Task,
    projects: &[Project],
    settings: &Settings,
) -> Option<u32> {
    if let Some(own_estimate) = task.estimated_minutes {
        return Some(own_estimate);
    }

    let chain = task
        .project_id
        .as_deref()
        .map(|id| project_tree::self_and_ancestors(projects, id))
        .unwrap_or_default();
    effective_default_estimated_minutes(&settings.defaults, &chain)
}

/// `weighted_completion_pct`: same population and numerator condition as
/// [`completion_pct`] (status == done), but each task is weighted by
/// [`resolve_completion_weight`] instead of counting `1` per task. A task
/// that resolves to no weight at all (own estimate unset, project default
/// unset, global default unset) is dropped from both the numerator and
/// denominator. `None` when the resulting denominator is `0` — either no
/// surviving tasks at all, or none of them could be weighted.
pub fn weighted_completion_pct(
    project_id: &str,
    projects: &[Project],
    all_tasks: &[Task],
    include_subprojects: bool,
    settings: &Settings,
) -> Option<f64> {
    let population = completion_population(
        project_id,
        projects,
        all_tasks,
        include_subprojects,
        settings,
    );

    let mut numerator: u64 = 0;
    let mut denominator: u64 = 0;
    for task in population {
        let Some(weight) = resolve_completion_weight(task, projects, settings) else {
            continue;
        };
        denominator += weight as u64;
        if task.status == settings.done_status {
            numerator += weight as u64;
        }
    }

    if denominator == 0 {
        return None;
    }

    Some(numerator as f64 / denominator as f64)
}

/// `active_completion_pct`: the same population and numerator rule as
/// [`completion_pct`], but restricted to `active_tasks` only — no archived
/// tasks are included. "What fraction of tasks currently on the board are
/// done?" rather than "what fraction of all tasks ever created are done?".
/// `None` when the surviving population (after dropping hidden/recurring/
/// cancelled tasks from `active_tasks`) is empty.
pub fn active_completion_pct(
    project_id: &str,
    projects: &[Project],
    active_tasks: &[Task],
    include_subprojects: bool,
    settings: &Settings,
) -> Option<f64> {
    completion_pct(
        project_id,
        projects,
        active_tasks,
        include_subprojects,
        settings,
    )
}

/// Returns the set of task ids `avg_time_per_week` aggregates tracked time
/// over for `project_id`: its own tasks' ids, plus its own hidden tracker
/// task's id (if set), plus — when `include_subprojects` is `true` — the
/// same for every descendant subproject. Matches
/// [`total_time_tracked_for_single_project`]'s exact population, just
/// expressed as ids rather than summed minutes, since the day-clipping
/// aggregator (`time_storage::tracked_seconds_per_day`) needs raw ids.
fn avg_time_per_week_task_ids(
    project_id: &str,
    projects: &[Project],
    tasks: &[Task],
    include_subprojects: bool,
) -> Vec<String> {
    let project_ids = project_and_descendant_ids(projects, project_id, include_subprojects);

    let mut ids: Vec<String> = tasks
        .iter()
        .filter(|task| {
            task.project_id
                .as_deref()
                .is_some_and(|id| project_ids.iter().any(|p| p == id))
                && !task.hidden
        })
        .map(|task| task.id.clone())
        .collect();

    for id in &project_ids {
        if let Some(tracker_id) = projects
            .iter()
            .find(|p| &p.id == id)
            .and_then(|p| p.tracking_task_id.as_deref())
        {
            ids.push(tracker_id.to_string());
        }
    }

    ids
}

/// Returns the most recent Monday/Sunday (per `week_start`) on or before
/// `date` — the start of the calendar week containing `date`, per the
/// caller-supplied week-start convention. `week_start` must be `"monday"` or
/// `"sunday"`; any other value is treated as `"sunday"` (chrono's own
/// `Weekday` default ordering), degrading gracefully rather than panicking
/// on an unrecognized string from a frontend `localStorage` value this
/// backend never validates.
fn start_of_week(date: NaiveDate, week_start: &str) -> NaiveDate {
    use chrono::Weekday;

    let week_start_day = if week_start == "monday" {
        Weekday::Mon
    } else {
        Weekday::Sun
    };

    let days_since_start = (date.weekday().num_days_from_monday() as i64
        - week_start_day.num_days_from_monday() as i64)
        .rem_euclid(7);
    date - chrono::Duration::days(days_since_start)
}

/// `avg_time_per_week`: trailing-`N`-week average tracked time (in seconds),
/// `N = settings.avg_time_per_week_window`, over the exact task population
/// [`avg_time_per_week_task_ids`] resolves (own tasks + own hidden tracker,
/// plus descendants' when rolled up — matching `total_time_tracked`'s
/// population). Weeks align to `week_start` (`"monday"` or `"sunday"`, per
/// the caller's resolved Week-view display setting — this backend has no
/// persisted notion of that setting itself, see the project-status-line
/// spec's resolved clarification).
///
/// The current, still-in-progress week is always excluded — only complete
/// past weeks count, so a Wednesday doesn't make the average look
/// artificially low. The "how many complete weeks have actually elapsed"
/// judgment call: this looks at the earliest `started_at` among the
/// project's own tracked sessions (not the project's `created` date, since a
/// project can exist for a long time before anyone starts tracking it, and
/// `created` would then understate how few complete weeks of *tracking*
/// data actually exist) and counts complete weeks from there up to (but
/// excluding) the current week, capped at `N`. Returns `Some(0)` — not
/// `None` — when there's no tracked time at all yet, since "the average
/// over zero activity is zero" is itself a meaningful, displayable answer
/// rather than a missing one.
#[allow(clippy::too_many_arguments)]
pub fn avg_time_per_week(
    conn: &Connection,
    project_id: &str,
    projects: &[Project],
    tasks: &[Task],
    include_subprojects: bool,
    settings: &Settings,
    week_start: &str,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<f64, TimeStorageError> {
    let task_ids = avg_time_per_week_task_ids(project_id, projects, tasks, include_subprojects);
    if task_ids.is_empty() {
        return Ok(0.0);
    }

    let current_week_start = start_of_week(today, week_start);
    let window = settings.avg_time_per_week_window;

    // Look back up to `window` complete weeks to find the earliest one with
    // any tracked activity, so a brand-new project doesn't get penalized by
    // averaging over weeks that simply don't exist yet for it.
    let earliest_candidate_start = current_week_start - chrono::Duration::weeks(window as i64);
    let range_start = start_of_day_utc_naive(earliest_candidate_start);
    let range_end = start_of_day_utc_naive(current_week_start);

    let buckets =
        time_storage::tracked_seconds_per_day(conn, &task_ids, range_start, range_end, now)?;

    if buckets.is_empty() {
        return Ok(0.0);
    }

    let earliest_active_day = *buckets.keys().min().expect("buckets is non-empty");
    let earliest_active_week_start = start_of_week(earliest_active_day, week_start);
    let complete_weeks_elapsed =
        ((current_week_start - earliest_active_week_start).num_days() / 7).clamp(1, window as i64);

    let total_seconds: i64 = buckets.values().sum();
    Ok(total_seconds as f64 / complete_weeks_elapsed as f64)
}

/// Converts a `NaiveDate` to a UTC midnight `DateTime<Utc>` — a free
/// function (rather than reusing `time_storage`'s private
/// `start_of_day_utc`) since this module has no access to that private
/// helper and the conversion is one line either way.
fn start_of_day_utc_naive(date: NaiveDate) -> DateTime<Utc> {
    date.and_hms_opt(0, 0, 0)
        .expect("midnight is always a valid time")
        .and_utc()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::TaskDefaults;
    use crate::time_storage as ts;

    fn task_in_project(project_id: &str) -> Task {
        let mut task = Task::new("Demo".to_string());
        task.project_id = Some(project_id.to_string());
        task
    }

    fn project(id: &str) -> Project {
        let mut p = Project::new(id.to_string(), "#111111".to_string(), 1);
        p.id = id.to_string();
        p
    }

    mod estimated_time_left_tests {
        use super::*;

        #[test]
        fn sums_estimate_minus_tracked_floored_at_zero() {
            let mut t1 = task_in_project("p1");
            t1.scheduled = Some("2026-06-24".to_string());
            t1.estimated_minutes = Some(100);
            t1.tracked_minutes = 30;
            let mut t2 = task_in_project("p1");
            t2.scheduled = Some("2026-06-25".to_string());
            t2.estimated_minutes = Some(50);
            t2.tracked_minutes = 80; // over-tracked: floors at 0
            let settings = Settings::default();

            let result = estimated_time_left("p1", &[t1, t2], &settings);

            assert_eq!(result, 70);
        }

        #[test]
        fn skips_tasks_with_no_estimate_rather_than_counting_as_zero() {
            let mut t1 = task_in_project("p1");
            t1.scheduled = Some("2026-06-24".to_string());
            t1.estimated_minutes = None;
            t1.tracked_minutes = 0;
            let mut t2 = task_in_project("p1");
            t2.scheduled = Some("2026-06-24".to_string());
            t2.estimated_minutes = Some(60);
            t2.tracked_minutes = 0;
            let settings = Settings::default();

            let result = estimated_time_left("p1", &[t1, t2], &settings);

            assert_eq!(result, 60);
        }

        #[test]
        fn skips_tasks_with_no_scheduled_date_even_if_estimated() {
            let mut t = task_in_project("p1");
            t.scheduled = None;
            t.estimated_minutes = Some(60);
            let settings = Settings::default();

            let result = estimated_time_left("p1", &[t], &settings);

            assert_eq!(result, 0);
        }

        #[test]
        fn excludes_hidden_tasks() {
            let mut t = task_in_project("p1");
            t.scheduled = Some("2026-06-24".to_string());
            t.estimated_minutes = Some(60);
            t.hidden = true;
            let settings = Settings::default();

            let result = estimated_time_left("p1", &[t], &settings);

            assert_eq!(result, 0);
        }

        #[test]
        fn excludes_completed_tasks() {
            let settings = Settings {
                done_status: "done".to_string(),
                ..Settings::default()
            };
            let mut t = task_in_project("p1");
            t.scheduled = Some("2026-06-24".to_string());
            t.estimated_minutes = Some(60);
            t.status = "done".to_string();

            let result = estimated_time_left("p1", &[t], &settings);

            assert_eq!(result, 0);
        }

        #[test]
        fn excludes_cancelled_tasks() {
            let settings = Settings {
                cancelled_status: Some("cancelled".to_string()),
                ..Settings::default()
            };
            let mut t = task_in_project("p1");
            t.scheduled = Some("2026-06-24".to_string());
            t.estimated_minutes = Some(60);
            t.status = "cancelled".to_string();

            let result = estimated_time_left("p1", &[t], &settings);

            assert_eq!(result, 0);
        }

        #[test]
        fn excludes_tasks_from_other_projects() {
            let mut t = task_in_project("other-project");
            t.scheduled = Some("2026-06-24".to_string());
            t.estimated_minutes = Some(60);
            let settings = Settings::default();

            let result = estimated_time_left("p1", &[t], &settings);

            assert_eq!(result, 0);
        }

        #[test]
        fn returns_zero_for_zero_estimate_exactly_at_boundary() {
            let mut t = task_in_project("p1");
            t.scheduled = Some("2026-06-24".to_string());
            t.estimated_minutes = Some(30);
            t.tracked_minutes = 30; // exactly equal: 0 left
            let settings = Settings::default();

            let result = estimated_time_left("p1", &[t], &settings);

            assert_eq!(result, 0);
        }

        #[test]
        fn empty_task_list_is_zero() {
            let settings = Settings::default();

            let result = estimated_time_left("p1", &[], &settings);

            assert_eq!(result, 0);
        }

        #[test]
        fn excludes_recurring_tasks_even_when_scheduled_and_estimated() {
            let mut t = task_in_project("p1");
            t.scheduled = Some("2026-06-24".to_string());
            t.estimated_minutes = Some(60);
            t.series_id = Some("series-1".to_string());
            let settings = Settings::default();

            let result = estimated_time_left("p1", &[t], &settings);

            assert_eq!(result, 0);
        }

        #[test]
        fn never_rolls_up_into_subprojects() {
            let mut sub_task = task_in_project("sub");
            sub_task.scheduled = Some("2026-06-24".to_string());
            sub_task.estimated_minutes = Some(500);
            let settings = Settings::default();

            // Even if a caller mistakenly passed a subproject's tasks in
            // here, estimated_time_left has no rollup parameter at all —
            // this test documents that calling it with "p1" while only
            // "sub"'s tasks exist correctly yields 0, since there's no
            // mechanism for it to look at any project but its own.
            let result = estimated_time_left("p1", &[sub_task], &settings);

            assert_eq!(result, 0);
        }

        #[test]
        fn includes_subtask_estimates_from_container_project() {
            let mut parent = task_in_project("p1");
            parent.scheduled = Some("2026-06-24".to_string());
            parent.estimated_minutes = Some(30);
            parent.subtask_project_id = Some("container-1".to_string());

            let mut subtask = task_in_project("container-1");
            subtask.scheduled = Some("2026-06-24".to_string());
            subtask.estimated_minutes = Some(40);
            subtask.tracked_minutes = 10;

            let settings = Settings::default();
            let result = estimated_time_left("p1", &[parent, subtask], &settings);

            // 30 (parent, nothing tracked) + (40-10) (subtask) = 60
            assert_eq!(result, 60);
        }

        #[test]
        fn does_not_include_containers_owned_by_tasks_in_other_projects() {
            let mut other_parent = task_in_project("other");
            other_parent.scheduled = Some("2026-06-24".to_string());
            other_parent.subtask_project_id = Some("container-other".to_string());

            let mut subtask = task_in_project("container-other");
            subtask.scheduled = Some("2026-06-24".to_string());
            subtask.estimated_minutes = Some(100);

            let settings = Settings::default();
            let result = estimated_time_left("p1", &[other_parent, subtask], &settings);

            assert_eq!(result, 0);
        }

        #[test]
        fn subtask_without_scheduled_date_is_excluded() {
            let mut parent = task_in_project("p1");
            parent.scheduled = Some("2026-06-24".to_string());
            parent.estimated_minutes = Some(30);
            parent.subtask_project_id = Some("container-1".to_string());

            let mut subtask = task_in_project("container-1");
            subtask.scheduled = None;
            subtask.estimated_minutes = Some(40);

            let settings = Settings::default();
            let result = estimated_time_left("p1", &[parent, subtask], &settings);

            // unscheduled subtask is excluded, only parent counted
            assert_eq!(result, 30);
        }
    }

    mod total_time_tracked_tests {
        use super::*;

        #[test]
        fn sums_tracked_minutes_across_own_non_hidden_tasks() {
            let mut t1 = task_in_project("p1");
            t1.tracked_minutes = 30;
            let mut t2 = task_in_project("p1");
            t2.tracked_minutes = 45;
            let projects = vec![project("p1")];

            let result = total_time_tracked("p1", &projects, &[t1, t2], false);

            assert_eq!(result, 75);
        }

        #[test]
        fn includes_the_hidden_trackers_own_time() {
            let mut tracker = task_in_project("p1");
            tracker.hidden = true;
            tracker.tracked_minutes = 200;
            let mut p1 = project("p1");
            p1.tracking_task_id = Some(tracker.id.clone());
            let mut normal_task = task_in_project("p1");
            normal_task.tracked_minutes = 10;

            let result = total_time_tracked("p1", &[p1], &[tracker, normal_task], false);

            assert_eq!(result, 210);
        }

        #[test]
        fn excludes_other_hidden_tasks_that_arent_the_tracker() {
            // A hidden task that isn't *this* project's tracking_task_id
            // (e.g. a subtask container's stray hidden task) never counts.
            let mut hidden_other = task_in_project("p1");
            hidden_other.hidden = true;
            hidden_other.tracked_minutes = 999;
            let p1 = project("p1"); // tracking_task_id left None
            let mut normal_task = task_in_project("p1");
            normal_task.tracked_minutes = 10;

            let result = total_time_tracked("p1", &[p1], &[hidden_other, normal_task], false);

            assert_eq!(result, 10);
        }

        #[test]
        fn is_zero_when_there_is_no_tracker_and_no_tasks() {
            let p1 = project("p1");

            let result = total_time_tracked("p1", &[p1], &[], false);

            assert_eq!(result, 0);
        }

        #[test]
        fn excludes_subprojects_when_rollup_is_off() {
            let mut parent = project("parent");
            parent.id = "parent".to_string();
            let mut child = project("child");
            child.id = "child".to_string();
            child.parent_id = Some("parent".to_string());
            let mut parent_task = task_in_project("parent");
            parent_task.tracked_minutes = 10;
            let mut child_task = task_in_project("child");
            child_task.tracked_minutes = 100;

            let result = total_time_tracked(
                "parent",
                &[parent, child],
                &[parent_task, child_task],
                false,
            );

            assert_eq!(result, 10);
        }

        #[test]
        fn includes_subprojects_recursively_when_rollup_is_on() {
            let mut parent = project("parent");
            parent.id = "parent".to_string();
            let mut child = project("child");
            child.id = "child".to_string();
            child.parent_id = Some("parent".to_string());
            let mut grandchild = project("grandchild");
            grandchild.id = "grandchild".to_string();
            grandchild.parent_id = Some("child".to_string());

            let mut parent_task = task_in_project("parent");
            parent_task.tracked_minutes = 10;
            let mut child_task = task_in_project("child");
            child_task.tracked_minutes = 20;
            let mut grandchild_task = task_in_project("grandchild");
            grandchild_task.tracked_minutes = 30;

            let result = total_time_tracked(
                "parent",
                &[parent, child, grandchild],
                &[parent_task, child_task, grandchild_task],
                true,
            );

            assert_eq!(result, 60);
        }

        #[test]
        fn includes_each_subprojects_own_hidden_tracker_when_rolled_up() {
            let mut child_tracker = task_in_project("child");
            child_tracker.hidden = true;
            child_tracker.tracked_minutes = 500;
            let mut parent = project("parent");
            parent.id = "parent".to_string();
            let mut child = project("child");
            child.id = "child".to_string();
            child.parent_id = Some("parent".to_string());
            child.tracking_task_id = Some(child_tracker.id.clone());

            let result = total_time_tracked("parent", &[parent, child], &[child_tracker], true);

            assert_eq!(result, 500);
        }
    }

    mod completion_pct_tests {
        use super::*;

        fn settings_with_done(done: &str) -> Settings {
            Settings {
                done_status: done.to_string(),
                ..Settings::default()
            }
        }

        #[test]
        fn returns_none_for_an_empty_population() {
            let settings = settings_with_done("done");
            let projects = vec![project("p1")];

            let result = completion_pct("p1", &projects, &[], false, &settings);

            assert_eq!(result, None);
        }

        #[test]
        fn computes_the_basic_ratio() {
            let settings = settings_with_done("done");
            let projects = vec![project("p1")];
            let mut done_task = task_in_project("p1");
            done_task.status = "done".to_string();
            let not_done_task = task_in_project("p1");

            let result = completion_pct(
                "p1",
                &projects,
                &[done_task, not_done_task],
                false,
                &settings,
            );

            assert_eq!(result, Some(0.5));
        }

        #[test]
        fn excludes_hidden_tasks_from_both_sides() {
            let settings = settings_with_done("done");
            let projects = vec![project("p1")];
            let mut hidden = task_in_project("p1");
            hidden.hidden = true;
            hidden.status = "done".to_string();
            let mut done_task = task_in_project("p1");
            done_task.status = "done".to_string();

            let result = completion_pct("p1", &projects, &[hidden, done_task], false, &settings);

            assert_eq!(result, Some(1.0));
        }

        #[test]
        fn excludes_every_recurring_occurrence_with_no_exceptions() {
            let settings = settings_with_done("done");
            let projects = vec![project("p1")];
            let mut recurring_done = task_in_project("p1");
            recurring_done.status = "done".to_string();
            recurring_done.series_id = Some("series-1".to_string());
            let mut recurring_not_done = task_in_project("p1");
            recurring_not_done.series_id = Some("series-1".to_string());
            let normal_done = {
                let mut t = task_in_project("p1");
                t.status = "done".to_string();
                t
            };

            let result = completion_pct(
                "p1",
                &projects,
                &[recurring_done, recurring_not_done, normal_done],
                false,
                &settings,
            );

            // Only the one normal, non-recurring done task should survive.
            assert_eq!(result, Some(1.0));
        }

        #[test]
        fn excludes_cancelled_tasks_from_both_sides() {
            let mut settings = settings_with_done("done");
            settings.cancelled_status = Some("cancelled".to_string());
            let projects = vec![project("p1")];
            let mut cancelled = task_in_project("p1");
            cancelled.status = "cancelled".to_string();
            let mut done_task = task_in_project("p1");
            done_task.status = "done".to_string();

            let result = completion_pct("p1", &projects, &[cancelled, done_task], false, &settings);

            assert_eq!(result, Some(1.0));
        }

        #[test]
        fn includes_archived_tasks_in_the_population() {
            // Modeled by simply mixing tasks from both directories into the
            // single all_tasks slice the caller is expected to merge —
            // this function makes no distinction between "still active" and
            // "archived" beyond what's already in the slice it receives.
            let settings = settings_with_done("done");
            let projects = vec![project("p1")];
            let mut archived_done = task_in_project("p1");
            archived_done.status = "done".to_string();
            let active_not_done = task_in_project("p1");

            let result = completion_pct(
                "p1",
                &projects,
                &[archived_done, active_not_done],
                false,
                &settings,
            );

            assert_eq!(result, Some(0.5));
        }

        #[test]
        fn rolls_up_subprojects_when_enabled() {
            let settings = settings_with_done("done");
            let mut parent = project("parent");
            parent.id = "parent".to_string();
            let mut child = project("child");
            child.id = "child".to_string();
            child.parent_id = Some("parent".to_string());
            let mut parent_done = task_in_project("parent");
            parent_done.status = "done".to_string();
            let child_not_done = task_in_project("child");

            let result = completion_pct(
                "parent",
                &[parent, child],
                &[parent_done, child_not_done],
                true,
                &settings,
            );

            assert_eq!(result, Some(0.5));
        }

        #[test]
        fn does_not_roll_up_when_disabled() {
            let settings = settings_with_done("done");
            let mut parent = project("parent");
            parent.id = "parent".to_string();
            let mut child = project("child");
            child.id = "child".to_string();
            child.parent_id = Some("parent".to_string());
            let mut parent_done = task_in_project("parent");
            parent_done.status = "done".to_string();
            let child_not_done = task_in_project("child");

            let result = completion_pct(
                "parent",
                &[parent, child],
                &[parent_done, child_not_done],
                false,
                &settings,
            );

            assert_eq!(result, Some(1.0));
        }

        #[test]
        fn excludes_tasks_belonging_to_unrelated_projects() {
            let settings = settings_with_done("done");
            let projects = vec![project("p1"), project("p2")];
            let mut other_done = task_in_project("p2");
            other_done.status = "done".to_string();
            let mine_not_done = task_in_project("p1");

            let result = completion_pct(
                "p1",
                &projects,
                &[other_done, mine_not_done],
                false,
                &settings,
            );

            assert_eq!(result, Some(0.0));
        }
    }

    mod weighted_completion_pct_tests {
        use super::*;

        fn settings_with_done(done: &str) -> Settings {
            Settings {
                done_status: done.to_string(),
                ..Settings::default()
            }
        }

        #[test]
        fn returns_none_for_an_empty_population() {
            let settings = settings_with_done("done");
            let projects = vec![project("p1")];

            let result = weighted_completion_pct("p1", &projects, &[], false, &settings);

            assert_eq!(result, None);
        }

        #[test]
        fn weights_by_own_estimated_minutes_when_set() {
            let settings = settings_with_done("done");
            let projects = vec![project("p1")];
            let mut done_task = task_in_project("p1");
            done_task.status = "done".to_string();
            done_task.estimated_minutes = Some(30);
            let mut not_done_task = task_in_project("p1");
            not_done_task.estimated_minutes = Some(90);

            let result = weighted_completion_pct(
                "p1",
                &projects,
                &[done_task, not_done_task],
                false,
                &settings,
            );

            assert_eq!(result, Some(30.0 / 120.0));
        }

        #[test]
        fn falls_back_to_the_projects_own_default_estimate_when_unset_on_the_task() {
            let settings = settings_with_done("done");
            let mut p1 = project("p1");
            p1.defaults = TaskDefaults {
                estimated_minutes: Some(45),
                ..TaskDefaults::default()
            };
            let projects = vec![p1];
            let mut done_task = task_in_project("p1");
            done_task.status = "done".to_string();
            done_task.estimated_minutes = None; // falls back to project default (45)

            let result = weighted_completion_pct("p1", &projects, &[done_task], false, &settings);

            assert_eq!(result, Some(1.0)); // sole task, done, weight 45/45
        }

        #[test]
        fn falls_back_to_the_global_default_when_neither_task_nor_project_set_an_estimate() {
            let mut settings = settings_with_done("done");
            settings.defaults.estimated_minutes = Some(20);
            let projects = vec![project("p1")]; // no project-level default
            let mut done_task = task_in_project("p1");
            done_task.status = "done".to_string();
            done_task.estimated_minutes = None;

            let result = weighted_completion_pct("p1", &projects, &[done_task], false, &settings);

            assert_eq!(result, Some(1.0));
        }

        #[test]
        fn drops_a_task_entirely_when_no_estimate_resolves_anywhere() {
            let settings = settings_with_done("done"); // no global default either
            let projects = vec![project("p1")];
            let mut undweighable_done = task_in_project("p1");
            undweighable_done.status = "done".to_string();
            undweighable_done.estimated_minutes = None;
            let mut weighable_not_done = task_in_project("p1");
            weighable_not_done.estimated_minutes = Some(50);

            let result = weighted_completion_pct(
                "p1",
                &projects,
                &[undweighable_done, weighable_not_done],
                false,
                &settings,
            );

            // The undweighable done task is dropped from both sides, so the
            // only surviving task is the un-done one with weight 50: 0/50.
            assert_eq!(result, Some(0.0));
        }

        #[test]
        fn returns_none_when_every_task_is_unweighable() {
            let settings = settings_with_done("done");
            let projects = vec![project("p1")];
            let mut t = task_in_project("p1");
            t.estimated_minutes = None;

            let result = weighted_completion_pct("p1", &projects, &[t], false, &settings);

            assert_eq!(result, None);
        }

        #[test]
        fn excludes_recurring_hidden_and_cancelled_tasks_like_completion_pct() {
            let mut settings = settings_with_done("done");
            settings.cancelled_status = Some("cancelled".to_string());
            let projects = vec![project("p1")];
            let mut recurring = task_in_project("p1");
            recurring.series_id = Some("series-1".to_string());
            recurring.estimated_minutes = Some(1000);
            let mut hidden = task_in_project("p1");
            hidden.hidden = true;
            hidden.estimated_minutes = Some(1000);
            let mut cancelled = task_in_project("p1");
            cancelled.status = "cancelled".to_string();
            cancelled.estimated_minutes = Some(1000);
            let mut surviving_done = task_in_project("p1");
            surviving_done.status = "done".to_string();
            surviving_done.estimated_minutes = Some(10);

            let result = weighted_completion_pct(
                "p1",
                &projects,
                &[recurring, hidden, cancelled, surviving_done],
                false,
                &settings,
            );

            assert_eq!(result, Some(1.0));
        }

        #[test]
        fn rolls_up_subprojects_when_enabled() {
            let settings = settings_with_done("done");
            let mut parent = project("parent");
            parent.id = "parent".to_string();
            let mut child = project("child");
            child.id = "child".to_string();
            child.parent_id = Some("parent".to_string());
            let mut parent_done = task_in_project("parent");
            parent_done.status = "done".to_string();
            parent_done.estimated_minutes = Some(10);
            let mut child_not_done = task_in_project("child");
            child_not_done.estimated_minutes = Some(30);

            let result = weighted_completion_pct(
                "parent",
                &[parent, child],
                &[parent_done, child_not_done],
                true,
                &settings,
            );

            assert_eq!(result, Some(10.0 / 40.0));
        }
    }

    mod avg_time_per_week_tests {
        use super::*;

        fn setup_conn() -> Connection {
            let conn = Connection::open_in_memory().unwrap();
            ts::init_schema(&conn).unwrap();
            conn
        }

        fn dt(rfc3339: &str) -> DateTime<Utc> {
            DateTime::parse_from_rfc3339(rfc3339)
                .unwrap()
                .with_timezone(&Utc)
        }

        #[test]
        fn returns_zero_when_the_project_has_no_tasks_and_no_tracker() {
            let conn = setup_conn();
            let p1 = project("p1");
            let settings = Settings::default();
            let today = NaiveDate::from_ymd_opt(2026, 6, 24).unwrap();

            let result = avg_time_per_week(
                &conn,
                "p1",
                &[p1],
                &[],
                false,
                &settings,
                "monday",
                today,
                dt("2026-06-24T12:00:00+00:00"),
            )
            .unwrap();

            assert_eq!(result, 0.0);
        }

        #[test]
        fn returns_zero_when_tasks_exist_but_nothing_was_ever_tracked() {
            let conn = setup_conn();
            let p1 = project("p1");
            let task = task_in_project("p1");
            let settings = Settings::default();
            let today = NaiveDate::from_ymd_opt(2026, 6, 24).unwrap();

            let result = avg_time_per_week(
                &conn,
                "p1",
                &[p1],
                &[task],
                false,
                &settings,
                "monday",
                today,
                dt("2026-06-24T12:00:00+00:00"),
            )
            .unwrap();

            assert_eq!(result, 0.0);
        }

        #[test]
        fn excludes_the_current_in_progress_week_entirely() {
            let conn = setup_conn();
            let p1 = project("p1");
            let task = task_in_project("p1");
            // today is a Wednesday in the week of 2026-06-22 (Monday).
            let today = NaiveDate::from_ymd_opt(2026, 6, 24).unwrap();
            assert_eq!(today.weekday(), chrono::Weekday::Wed);
            // Track a huge amount *today*, in the current week — should be
            // entirely excluded from the average.
            ts::insert_completed_entry(
                &conn,
                &task.id,
                dt("2026-06-24T08:00:00+00:00"),
                dt("2026-06-24T20:00:00+00:00"), // 12 hours
            )
            .unwrap();
            let settings = Settings::default();

            let result = avg_time_per_week(
                &conn,
                "p1",
                &[p1],
                &[task],
                false,
                &settings,
                "monday",
                today,
                dt("2026-06-24T21:00:00+00:00"),
            )
            .unwrap();

            assert_eq!(result, 0.0);
        }

        #[test]
        fn averages_over_a_single_complete_past_week() {
            let conn = setup_conn();
            let p1 = project("p1");
            let task = task_in_project("p1");
            // Current week starts Monday 2026-06-22. The prior week
            // (06-15 to 06-21) gets 7 hours tracked.
            ts::insert_completed_entry(
                &conn,
                &task.id,
                dt("2026-06-16T08:00:00+00:00"),
                dt("2026-06-16T15:00:00+00:00"), // 7h = 25200s
            )
            .unwrap();
            let today = NaiveDate::from_ymd_opt(2026, 6, 24).unwrap();
            let settings = Settings {
                avg_time_per_week_window: 4,
                ..Settings::default()
            };

            let result = avg_time_per_week(
                &conn,
                "p1",
                &[p1],
                &[task],
                false,
                &settings,
                "monday",
                today,
                dt("2026-06-24T21:00:00+00:00"),
            )
            .unwrap();

            // Only 1 complete week of activity exists (the week of 06-15),
            // so the average is over 1 week, not the full window of 4.
            assert_eq!(result, 25200.0);
        }

        #[test]
        fn averages_over_multiple_complete_weeks_within_the_window() {
            let conn = setup_conn();
            let p1 = project("p1");
            let task = task_in_project("p1");
            // Week of 06-08 (2 weeks back from current week 06-22): 2h.
            ts::insert_completed_entry(
                &conn,
                &task.id,
                dt("2026-06-09T08:00:00+00:00"),
                dt("2026-06-09T10:00:00+00:00"), // 2h = 7200s
            )
            .unwrap();
            // Week of 06-15 (1 week back): 4h.
            ts::insert_completed_entry(
                &conn,
                &task.id,
                dt("2026-06-16T08:00:00+00:00"),
                dt("2026-06-16T12:00:00+00:00"), // 4h = 14400s
            )
            .unwrap();
            let today = NaiveDate::from_ymd_opt(2026, 6, 24).unwrap();
            let settings = Settings {
                avg_time_per_week_window: 4,
                ..Settings::default()
            };

            let result = avg_time_per_week(
                &conn,
                "p1",
                &[p1],
                &[task],
                false,
                &settings,
                "monday",
                today,
                dt("2026-06-24T21:00:00+00:00"),
            )
            .unwrap();

            // Earliest activity is the week of 06-08, which is 2 complete
            // weeks before the current week (06-22) -> average over 2 weeks.
            let total = 7200.0 + 14400.0;
            assert_eq!(result, total / 2.0);
        }

        #[test]
        fn caps_the_averaging_window_at_n_even_with_older_activity() {
            let conn = setup_conn();
            let p1 = project("p1");
            let task = task_in_project("p1");
            // Activity 10 weeks back — far outside a window of 4 — should
            // not be included in the query range at all, and the divisor
            // should still cap at the window size.
            ts::insert_completed_entry(
                &conn,
                &task.id,
                dt("2026-04-14T08:00:00+00:00"),
                dt("2026-04-14T09:00:00+00:00"),
            )
            .unwrap();
            // One hour within the window, 1 week back.
            ts::insert_completed_entry(
                &conn,
                &task.id,
                dt("2026-06-16T08:00:00+00:00"),
                dt("2026-06-16T09:00:00+00:00"), // 1h = 3600s
            )
            .unwrap();
            let today = NaiveDate::from_ymd_opt(2026, 6, 24).unwrap();
            let settings = Settings {
                avg_time_per_week_window: 4,
                ..Settings::default()
            };

            let result = avg_time_per_week(
                &conn,
                "p1",
                &[p1],
                &[task],
                false,
                &settings,
                "monday",
                today,
                dt("2026-06-24T21:00:00+00:00"),
            )
            .unwrap();

            // Only the in-window hour is queried at all (the query range
            // itself is bounded to `window` weeks back), and the earliest
            // active day within that range is the week of 06-15 -> 1 week.
            assert_eq!(result, 3600.0);
        }

        #[test]
        fn respects_a_sunday_week_start() {
            let conn = setup_conn();
            let p1 = project("p1");
            let task = task_in_project("p1");
            // With sunday-start weeks, 2026-06-24 (Wed) belongs to the week
            // starting Sunday 2026-06-21. Track time in the prior Sunday
            // week (06-14 to 06-20).
            ts::insert_completed_entry(
                &conn,
                &task.id,
                dt("2026-06-15T08:00:00+00:00"),
                dt("2026-06-15T09:00:00+00:00"), // 1h
            )
            .unwrap();
            let today = NaiveDate::from_ymd_opt(2026, 6, 24).unwrap();
            let settings = Settings::default();

            let result = avg_time_per_week(
                &conn,
                "p1",
                &[p1],
                &[task],
                false,
                &settings,
                "sunday",
                today,
                dt("2026-06-24T21:00:00+00:00"),
            )
            .unwrap();

            assert_eq!(result, 3600.0);
        }

        #[test]
        fn includes_the_hidden_trackers_own_time() {
            let conn = setup_conn();
            let mut tracker = task_in_project("p1");
            tracker.hidden = true;
            let mut p1 = project("p1");
            p1.tracking_task_id = Some(tracker.id.clone());
            ts::insert_completed_entry(
                &conn,
                &tracker.id,
                dt("2026-06-16T08:00:00+00:00"),
                dt("2026-06-16T09:00:00+00:00"), // 1h
            )
            .unwrap();
            let today = NaiveDate::from_ymd_opt(2026, 6, 24).unwrap();
            let settings = Settings::default();

            let result = avg_time_per_week(
                &conn,
                "p1",
                &[p1],
                &[tracker],
                false,
                &settings,
                "monday",
                today,
                dt("2026-06-24T21:00:00+00:00"),
            )
            .unwrap();

            assert_eq!(result, 3600.0);
        }

        #[test]
        fn rolls_up_subproject_tracked_time_when_enabled() {
            let conn = setup_conn();
            let mut parent = project("parent");
            parent.id = "parent".to_string();
            let mut child = project("child");
            child.id = "child".to_string();
            child.parent_id = Some("parent".to_string());
            let child_task = task_in_project("child");
            ts::insert_completed_entry(
                &conn,
                &child_task.id,
                dt("2026-06-16T08:00:00+00:00"),
                dt("2026-06-16T09:00:00+00:00"), // 1h
            )
            .unwrap();
            let today = NaiveDate::from_ymd_opt(2026, 6, 24).unwrap();
            let settings = Settings::default();

            let result = avg_time_per_week(
                &conn,
                "parent",
                &[parent, child],
                &[child_task],
                true,
                &settings,
                "monday",
                today,
                dt("2026-06-24T21:00:00+00:00"),
            )
            .unwrap();

            assert_eq!(result, 3600.0);
        }

        #[test]
        fn does_not_roll_up_subproject_time_when_disabled() {
            let conn = setup_conn();
            let mut parent = project("parent");
            parent.id = "parent".to_string();
            let mut child = project("child");
            child.id = "child".to_string();
            child.parent_id = Some("parent".to_string());
            let child_task = task_in_project("child");
            ts::insert_completed_entry(
                &conn,
                &child_task.id,
                dt("2026-06-16T08:00:00+00:00"),
                dt("2026-06-16T09:00:00+00:00"),
            )
            .unwrap();
            let today = NaiveDate::from_ymd_opt(2026, 6, 24).unwrap();
            let settings = Settings::default();

            let result = avg_time_per_week(
                &conn,
                "parent",
                &[parent, child],
                &[child_task],
                false,
                &settings,
                "monday",
                today,
                dt("2026-06-24T21:00:00+00:00"),
            )
            .unwrap();

            assert_eq!(result, 0.0);
        }
    }

    mod start_of_week_tests {
        use super::*;

        #[test]
        fn monday_start_returns_the_date_itself_when_already_monday() {
            let monday = NaiveDate::from_ymd_opt(2026, 6, 22).unwrap();
            assert_eq!(monday.weekday(), chrono::Weekday::Mon);

            assert_eq!(start_of_week(monday, "monday"), monday);
        }

        #[test]
        fn monday_start_resolves_a_wednesday_back_to_that_weeks_monday() {
            let wednesday = NaiveDate::from_ymd_opt(2026, 6, 24).unwrap();
            let expected_monday = NaiveDate::from_ymd_opt(2026, 6, 22).unwrap();

            assert_eq!(start_of_week(wednesday, "monday"), expected_monday);
        }

        #[test]
        fn sunday_start_resolves_a_wednesday_back_to_that_weeks_sunday() {
            let wednesday = NaiveDate::from_ymd_opt(2026, 6, 24).unwrap();
            let expected_sunday = NaiveDate::from_ymd_opt(2026, 6, 21).unwrap();

            assert_eq!(start_of_week(wednesday, "sunday"), expected_sunday);
        }

        #[test]
        fn an_unrecognized_week_start_falls_back_to_sunday_rather_than_panicking() {
            let wednesday = NaiveDate::from_ymd_opt(2026, 6, 24).unwrap();
            let expected_sunday = NaiveDate::from_ymd_opt(2026, 6, 21).unwrap();

            assert_eq!(start_of_week(wednesday, "garbage"), expected_sunday);
        }
    }
}
