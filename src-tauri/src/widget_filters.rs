//! Canonical filter and aggregation functions used by every widget backend.
//!
//! All functions operate purely on in-memory data (no I/O) so they are easy
//! to test and compose. Each widget's compute function should delegate its
//! task-pool selection here rather than re-implementing the same hidden-task
//! and series-deduplication logic inline.

use std::collections::{HashMap, HashSet};

use crate::project::Project;
use crate::project_tree;
use crate::settings::Settings;
use crate::task::Task;

// ── Status predicates ────────────────────────────────────────────────────────

/// Returns `true` if the task is in the "done" completed state (matches
/// `settings.done_status` only — NOT cancelled).
pub fn is_completed(task: &Task, settings: &Settings) -> bool {
    task.status == settings.done_status
}

/// Returns `true` if the task is in the cancelled state (matches
/// `settings.cancelled_status`, if configured). Returns `false` when no
/// cancelled status is configured.
pub fn is_cancelled(task: &Task, settings: &Settings) -> bool {
    settings
        .cancelled_status
        .as_deref()
        .map(|cs| task.status == cs)
        .unwrap_or(false)
}

/// Returns `true` if the task is in any terminal state (done OR cancelled).
pub fn is_terminal(task: &Task, settings: &Settings) -> bool {
    is_completed(task, settings) || is_cancelled(task, settings)
}

// ── Series deduplication ────────────────────────────────────────────────────

/// From a slice of task references, keeps one representative per `series_id`
/// (the occurrence with the latest `due` date; ties broken by latest
/// `created`). Tasks with no `series_id` are kept individually.
///
/// The order of the output is not guaranteed.
pub fn deduplicate_series<'a>(tasks: &[&'a Task]) -> Vec<&'a Task> {
    let mut result: Vec<&'a Task> = Vec::new();
    let mut series_map: HashMap<&str, &'a Task> = HashMap::new();

    for &task in tasks {
        match task.series_id.as_deref() {
            None => result.push(task),
            Some(sid) => {
                let should_replace = series_map.get(sid).map_or(true, |&prev| {
                    let new_due = task.due.as_deref().unwrap_or("");
                    let prev_due = prev.due.as_deref().unwrap_or("");
                    if new_due != prev_due {
                        new_due > prev_due
                    } else {
                        task.created > prev.created
                    }
                });
                if should_replace {
                    series_map.insert(sid, task);
                }
            }
        }
    }

    result.extend(series_map.into_values());
    result
}

// ── Project task pools ───────────────────────────────────────────────────────

/// Returns the non-hidden, series-deduplicated task pool for a single project
/// (not rolling up subprojects).
pub fn project_task_pool<'a>(project_id: &str, tasks: &'a [Task]) -> Vec<&'a Task> {
    let visible: Vec<&Task> = tasks
        .iter()
        .filter(|t| !t.hidden && t.project_id.as_deref() == Some(project_id))
        .collect();
    deduplicate_series(&visible)
}

/// Returns the non-hidden, series-deduplicated task pool for a project and
/// all its transitive descendant subprojects combined.
pub fn project_task_pool_with_subprojects<'a>(
    project_id: &str,
    projects: &'a [Project],
    tasks: &'a [Task],
) -> Vec<&'a Task> {
    let descendants = project_tree::descendants_of(projects, project_id);
    let all_ids: HashSet<&str> = std::iter::once(project_id)
        .chain(descendants.iter().map(|p| p.id.as_str()))
        .collect();

    let visible: Vec<&Task> = tasks
        .iter()
        .filter(|t| {
            !t.hidden
                && t.project_id
                    .as_deref()
                    .map_or(false, |pid| all_ids.contains(pid))
        })
        .collect();
    deduplicate_series(&visible)
}

// ── Date-range filters ───────────────────────────────────────────────────────

/// From a slice of task references, returns only those whose `completed_at`
/// date falls within `[range_start, range_end]` (inclusive, YYYY-MM-DD).
/// Tasks with `completed_at = None` are excluded.
pub fn tasks_completed_in_range<'a>(
    tasks: &[&'a Task],
    range_start: &str,
    range_end: &str,
) -> Vec<&'a Task> {
    tasks
        .iter()
        .filter(|&&t| {
            t.completed_at.as_deref().map_or(false, |cat| {
                let date = &cat[..10.min(cat.len())];
                date >= range_start && date <= range_end
            })
        })
        .copied()
        .collect()
}

/// Returns the best date proxy for a task: `due` → `scheduled` → first 10
/// chars of `created`.
fn task_date_proxy(task: &Task) -> Option<&str> {
    task.due
        .as_deref()
        .or(task.scheduled.as_deref())
        .or_else(|| task.created.get(..10))
}

/// From a slice of task references, returns only those whose best date proxy
/// (due → scheduled → created[0..10]) falls within `[range_start, range_end]`
/// (inclusive, YYYY-MM-DD).
pub fn tasks_active_in_range<'a>(
    tasks: &[&'a Task],
    range_start: &str,
    range_end: &str,
) -> Vec<&'a Task> {
    tasks
        .iter()
        .filter(|&&t| {
            task_date_proxy(t).map_or(false, |d| d >= range_start && d <= range_end)
        })
        .copied()
        .collect()
}

// ── Series aggregation ───────────────────────────────────────────────────────

/// Sums `tracked_minutes` across **all** occurrences of each recurring series
/// (not just the deduplication representative). Returns a map of
/// `series_id → total_tracked_minutes` as `i64` for every series that has at
/// least one task in `tasks`. Non-recurring tasks (series_id = None) are
/// ignored.
pub fn series_tracked_totals(tasks: &[Task]) -> HashMap<String, i64> {
    let mut map: HashMap<String, i64> = HashMap::new();
    for task in tasks {
        if let Some(sid) = &task.series_id {
            *map.entry(sid.clone()).or_insert(0) += task.tracked_minutes as i64;
        }
    }
    map
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use crate::task::Task;

    fn settings_with_cancelled() -> Settings {
        Settings {
            cancelled_status: Some("cancelled".to_string()),
            ..Settings::default()
        }
    }

    fn task_with_status(status: &str) -> Task {
        Task {
            status: status.to_string(),
            ..Task::new("Test".to_string())
        }
    }

    fn task_in_project(project_id: &str) -> Task {
        Task {
            project_id: Some(project_id.to_string()),
            ..Task::new("Test".to_string())
        }
    }

    // ── is_completed ─────────────────────────────────────────────────────────

    #[test]
    fn is_completed_returns_true_for_done_status() {
        let settings = Settings::default(); // done_status = "done"
        let task = task_with_status("done");
        assert!(is_completed(&task, &settings));
    }

    #[test]
    fn is_completed_returns_false_for_non_done_status() {
        let settings = Settings::default();
        let task = task_with_status("backlog");
        assert!(!is_completed(&task, &settings));
    }

    #[test]
    fn is_completed_returns_false_for_cancelled_status() {
        let settings = settings_with_cancelled();
        let task = task_with_status("cancelled");
        // cancelled ≠ done_status → not completed
        assert!(!is_completed(&task, &settings));
    }

    #[test]
    fn all_tasks_in_done_status_are_completed() {
        let settings = Settings::default();
        let tasks: Vec<Task> = (0..5).map(|_| task_with_status("done")).collect();
        assert!(tasks.iter().all(|t| is_completed(t, &settings)));
    }

    #[test]
    fn is_completed_empty_input_no_crash() {
        let settings = Settings::default();
        let tasks: Vec<Task> = vec![];
        assert_eq!(tasks.iter().filter(|t| is_completed(t, &settings)).count(), 0);
    }

    // ── is_cancelled ─────────────────────────────────────────────────────────

    #[test]
    fn is_cancelled_returns_false_when_no_cancelled_status_configured() {
        let settings = Settings::default(); // cancelled_status = None
        let task = task_with_status("cancelled");
        assert!(!is_cancelled(&task, &settings));
    }

    #[test]
    fn is_cancelled_returns_true_when_status_matches_configured_cancelled() {
        let settings = settings_with_cancelled();
        let task = task_with_status("cancelled");
        assert!(is_cancelled(&task, &settings));
    }

    #[test]
    fn is_cancelled_returns_false_for_non_cancelled_status() {
        let settings = settings_with_cancelled();
        let task = task_with_status("backlog");
        assert!(!is_cancelled(&task, &settings));
    }

    #[test]
    fn is_cancelled_empty_input_no_crash() {
        let settings = Settings::default();
        let tasks: Vec<Task> = vec![];
        assert_eq!(tasks.iter().filter(|t| is_cancelled(t, &settings)).count(), 0);
    }

    // ── is_terminal ──────────────────────────────────────────────────────────

    #[test]
    fn is_terminal_true_for_done() {
        let settings = Settings::default();
        let task = task_with_status("done");
        assert!(is_terminal(&task, &settings));
    }

    #[test]
    fn is_terminal_true_for_cancelled() {
        let settings = settings_with_cancelled();
        let task = task_with_status("cancelled");
        assert!(is_terminal(&task, &settings));
    }

    #[test]
    fn is_terminal_false_for_active_status() {
        let settings = Settings::default();
        let task = task_with_status("in-progress");
        assert!(!is_terminal(&task, &settings));
    }

    #[test]
    fn is_terminal_false_for_cancelled_string_when_none_configured() {
        let settings = Settings::default(); // no cancelled_status
        let task = task_with_status("cancelled");
        assert!(!is_terminal(&task, &settings)); // "cancelled" matches nothing
    }

    // ── deduplicate_series ───────────────────────────────────────────────────

    #[test]
    fn deduplicate_series_empty_input_returns_empty() {
        let result = deduplicate_series(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn deduplicate_series_non_recurring_tasks_all_kept() {
        let t1 = Task::new("A".to_string());
        let t2 = Task::new("B".to_string());
        let refs = [&t1, &t2];
        let result = deduplicate_series(&refs);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn deduplicate_series_keeps_one_representative_per_series_id() {
        let mut t1 = Task::new("Instance 1".to_string());
        t1.series_id = Some("series-abc".to_string());
        t1.due = Some("2026-06-01".to_string());

        let mut t2 = Task::new("Instance 2".to_string());
        t2.series_id = Some("series-abc".to_string());
        t2.due = Some("2026-06-08".to_string()); // later due → this is the representative

        let refs = [&t1, &t2];
        let result = deduplicate_series(&refs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].due.as_deref(), Some("2026-06-08"));
    }

    #[test]
    fn deduplicate_series_mix_of_oneoff_and_recurring() {
        let mut recurring = Task::new("Recurring".to_string());
        recurring.series_id = Some("series-1".to_string());

        let oneoff = Task::new("One-off".to_string());

        let refs = [&recurring, &oneoff];
        let result = deduplicate_series(&refs);
        assert_eq!(result.len(), 2);
    }

    // ── project_task_pool ─────────────────────────────────────────────────────

    #[test]
    fn project_task_pool_empty_tasks_returns_empty() {
        let result = project_task_pool("proj-1", &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn project_task_pool_excludes_hidden_tasks() {
        let mut visible = task_in_project("proj-1");
        visible.hidden = false;

        let mut hidden = task_in_project("proj-1");
        hidden.hidden = true;

        let tasks = [visible, hidden];
        let result = project_task_pool("proj-1", &tasks);
        assert_eq!(result.len(), 1);
        assert!(!result[0].hidden);
    }

    #[test]
    fn project_task_pool_excludes_wrong_project_id() {
        let t = task_in_project("proj-2");
        let tasks = [t];
        let result = project_task_pool("proj-1", &tasks);
        assert!(result.is_empty());
    }

    #[test]
    fn project_task_pool_deduplicates_series() {
        let mut t1 = task_in_project("proj-1");
        t1.series_id = Some("series-x".to_string());
        t1.due = Some("2026-06-01".to_string());

        let mut t2 = task_in_project("proj-1");
        t2.series_id = Some("series-x".to_string());
        t2.due = Some("2026-06-15".to_string());

        let tasks = [t1, t2];
        let result = project_task_pool("proj-1", &tasks);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn project_task_pool_wrong_project_id_returns_empty() {
        let t = task_in_project("proj-A");
        let tasks = [t];
        let result = project_task_pool("proj-B", &tasks);
        assert!(result.is_empty());
    }

    // ── tasks_completed_in_range ──────────────────────────────────────────────

    #[test]
    fn tasks_completed_in_range_empty_returns_empty() {
        let result = tasks_completed_in_range(&[], "2026-01-01", "2026-12-31");
        assert!(result.is_empty());
    }

    #[test]
    fn tasks_completed_in_range_excludes_tasks_with_no_completed_at() {
        let t = Task::new("No timestamp".to_string()); // completed_at = None
        let refs = [&t];
        let result = tasks_completed_in_range(&refs, "2026-01-01", "2026-12-31");
        assert!(result.is_empty());
    }

    #[test]
    fn tasks_completed_in_range_includes_tasks_in_range() {
        let mut t = Task::new("Done in range".to_string());
        t.completed_at = Some("2026-06-15T10:00:00Z".to_string());
        let refs = [&t];
        let result = tasks_completed_in_range(&refs, "2026-06-01", "2026-06-30");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn tasks_completed_in_range_excludes_tasks_outside_range() {
        let mut t = Task::new("Done outside range".to_string());
        t.completed_at = Some("2026-07-15T10:00:00Z".to_string());
        let refs = [&t];
        let result = tasks_completed_in_range(&refs, "2026-06-01", "2026-06-30");
        assert!(result.is_empty());
    }

    #[test]
    fn tasks_completed_in_range_inclusive_at_start_boundary() {
        let mut t = Task::new("Start".to_string());
        t.completed_at = Some("2026-06-01T00:00:00Z".to_string());
        let refs = [&t];
        let result = tasks_completed_in_range(&refs, "2026-06-01", "2026-06-30");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn tasks_completed_in_range_inclusive_at_end_boundary() {
        let mut t = Task::new("End".to_string());
        t.completed_at = Some("2026-06-30T23:59:59Z".to_string());
        let refs = [&t];
        let result = tasks_completed_in_range(&refs, "2026-06-01", "2026-06-30");
        assert_eq!(result.len(), 1);
    }

    // ── tasks_active_in_range ─────────────────────────────────────────────────

    #[test]
    fn tasks_active_in_range_empty_returns_empty() {
        let result = tasks_active_in_range(&[], "2026-01-01", "2026-12-31");
        assert!(result.is_empty());
    }

    #[test]
    fn tasks_active_in_range_uses_due_date_first() {
        let mut t = Task::new("Task with due date".to_string());
        t.due = Some("2026-06-15".to_string());
        t.scheduled = Some("2026-05-01".to_string()); // outside range if used as proxy
        let refs = [&t];
        let result = tasks_active_in_range(&refs, "2026-06-01", "2026-06-30");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn tasks_active_in_range_falls_back_to_scheduled_when_no_due() {
        let mut t = Task::new("Task without due date".to_string());
        t.due = None;
        t.scheduled = Some("2026-06-15".to_string());
        let refs = [&t];
        let result = tasks_active_in_range(&refs, "2026-06-01", "2026-06-30");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn tasks_active_in_range_excludes_tasks_outside_range() {
        let mut t = Task::new("Old task".to_string());
        t.due = Some("2026-01-01".to_string());
        let refs = [&t];
        let result = tasks_active_in_range(&refs, "2026-06-01", "2026-06-30");
        assert!(result.is_empty());
    }

    // ── series_tracked_totals ────────────────────────────────────────────────

    #[test]
    fn series_tracked_totals_empty_returns_empty_map() {
        let result = series_tracked_totals(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn series_tracked_totals_sums_all_instances_for_a_series() {
        let mut t1 = Task::new("Instance 1".to_string());
        t1.series_id = Some("series-1".to_string());
        t1.tracked_minutes = 30;

        let mut t2 = Task::new("Instance 2".to_string());
        t2.series_id = Some("series-1".to_string());
        t2.tracked_minutes = 45;

        let tasks = [t1, t2];
        let result = series_tracked_totals(&tasks);
        assert_eq!(result.get("series-1"), Some(&75i64));
    }

    #[test]
    fn series_tracked_totals_ignores_non_recurring_tasks() {
        let t = Task::new("One-off".to_string()); // series_id = None
        let tasks = [t];
        let result = series_tracked_totals(&tasks);
        assert!(result.is_empty());
    }

    #[test]
    fn series_tracked_totals_handles_multiple_series_independently() {
        let mut a1 = Task::new("A1".to_string());
        a1.series_id = Some("series-A".to_string());
        a1.tracked_minutes = 20;

        let mut a2 = Task::new("A2".to_string());
        a2.series_id = Some("series-A".to_string());
        a2.tracked_minutes = 10;

        let mut b1 = Task::new("B1".to_string());
        b1.series_id = Some("series-B".to_string());
        b1.tracked_minutes = 60;

        let tasks = [a1, a2, b1];
        let result = series_tracked_totals(&tasks);
        assert_eq!(result.get("series-A"), Some(&30i64));
        assert_eq!(result.get("series-B"), Some(&60i64));
    }
}
