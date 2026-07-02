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

// ── Subtask containers & effective project ──────────────────────────────────

/// Ids of every auto-generated subtask-container project referenced by any
/// task's `subtask_project_id`. These are real `Project` records parented
/// under the owning task's project, but they are an implementation detail of
/// the subtask system — widgets must never present them as subprojects.
pub fn subtask_container_ids(tasks: &[Task]) -> HashSet<&str> {
    tasks
        .iter()
        .filter_map(|t| t.subtask_project_id.as_deref())
        .collect()
}

/// Maps each subtask-container project id to the task that owns it.
fn container_owners(tasks: &[Task]) -> HashMap<&str, &Task> {
    tasks
        .iter()
        .filter_map(|t| t.subtask_project_id.as_deref().map(|cid| (cid, t)))
        .collect()
}

/// Resolves the project a task effectively belongs to for widget purposes:
/// its own `project_id`, unless that project is a subtask container — then
/// the owning (parent) task's project, resolved transitively for nested
/// containers (guarded against cycles). Returns `None` for tasks with no
/// project or whose container chain is orphaned.
fn resolve_effective_project<'a>(
    task: &'a Task,
    owners: &HashMap<&'a str, &'a Task>,
) -> Option<&'a str> {
    let mut pid = task.project_id.as_deref()?;
    let mut guard = 32u32;
    while let Some(owner) = owners.get(pid) {
        if guard == 0 {
            return None;
        }
        guard -= 1;
        pid = owner.project_id.as_deref()?;
    }
    Some(pid)
}

/// Public single-task convenience wrapper around
/// [`resolve_effective_project`]. Builds the owner map internally — when
/// resolving many tasks, prefer the pool functions, which build it once.
pub fn effective_project_id<'a>(task: &'a Task, tasks: &'a [Task]) -> Option<&'a str> {
    let owners = container_owners(tasks);
    resolve_effective_project(task, &owners)
}

// ── Real (non-container) subproject listings ────────────────────────────────

/// Like `project_tree::children_of`, but excluding subtask-container
/// projects. This is what subproject widgets must use.
pub fn real_children_of<'a>(
    projects: &'a [Project],
    tasks: &[Task],
    parent_id: &str,
) -> Vec<&'a Project> {
    let containers = subtask_container_ids(tasks);
    project_tree::children_of(projects, Some(parent_id))
        .into_iter()
        .filter(|p| !containers.contains(p.id.as_str()))
        .collect()
}

/// Like `project_tree::descendants_of`, but excluding subtask-container
/// projects. This is what subproject widgets must use.
pub fn real_descendants_of<'a>(
    projects: &'a [Project],
    tasks: &[Task],
    id: &str,
) -> Vec<&'a Project> {
    let containers = subtask_container_ids(tasks);
    project_tree::descendants_of(projects, id)
        .into_iter()
        .filter(|p| !containers.contains(p.id.as_str()))
        .collect()
}

// ── Project task pools ───────────────────────────────────────────────────────

/// Returns the non-hidden tasks that effectively belong to `project_id`
/// (subtasks roll up to the owning task's project), with NO series
/// deduplication — every recurring occurrence is kept. This is the pool for
/// occurrence-counting widgets (completions over time, due-date timelines).
pub fn project_task_pool_raw<'a>(project_id: &str, tasks: &'a [Task]) -> Vec<&'a Task> {
    let owners = container_owners(tasks);
    tasks
        .iter()
        .filter(|t| !t.hidden)
        .filter(|t| resolve_effective_project(t, &owners) == Some(project_id))
        .collect()
}

/// Returns the non-hidden, series-deduplicated task pool for a single project
/// (not rolling up subprojects). Subtasks count toward the owning task's
/// project. This is the pool for entity-counting widgets (task counts,
/// status distribution, completion denominators).
pub fn project_task_pool<'a>(project_id: &str, tasks: &'a [Task]) -> Vec<&'a Task> {
    deduplicate_series(&project_task_pool_raw(project_id, tasks))
}

/// Raw (occurrence-level) variant of
/// [`project_task_pool_with_subprojects`] — no series deduplication.
pub fn project_task_pool_with_subprojects_raw<'a>(
    project_id: &str,
    projects: &'a [Project],
    tasks: &'a [Task],
) -> Vec<&'a Task> {
    let containers = subtask_container_ids(tasks);
    let descendants = project_tree::descendants_of(projects, project_id);
    let all_ids: HashSet<&str> = std::iter::once(project_id)
        .chain(
            descendants
                .iter()
                .filter(|p| !containers.contains(p.id.as_str()))
                .map(|p| p.id.as_str()),
        )
        .collect();

    let owners = container_owners(tasks);
    tasks
        .iter()
        .filter(|t| !t.hidden)
        .filter(|t| {
            resolve_effective_project(t, &owners).map_or(false, |pid| all_ids.contains(pid))
        })
        .collect()
}

/// Returns the non-hidden, series-deduplicated task pool for a project and
/// all its transitive descendant subprojects combined (subtask containers
/// excluded as projects; their subtasks roll up to the owning task's
/// project, which keeps them in scope).
pub fn project_task_pool_with_subprojects<'a>(
    project_id: &str,
    projects: &'a [Project],
    tasks: &'a [Task],
) -> Vec<&'a Task> {
    deduplicate_series(&project_task_pool_with_subprojects_raw(
        project_id, projects, tasks,
    ))
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
        .filter(|&&t| task_date_proxy(t).map_or(false, |d| d >= range_start && d <= range_end))
        .copied()
        .collect()
}

// ── Occurrence-level completion counting ─────────────────────────────────────

/// Completed tasks at the *occurrence* level (NO series deduplication —
/// every finished occurrence of a recurring series counts individually).
///
/// - `range = None` means all-time: every completed task is returned,
///   including tasks whose `completed_at` is `None` (pre-timestamp tasks).
/// - `range = Some((start, end))` (inclusive `YYYY-MM-DD`): only tasks whose
///   `completed_at` date falls inside the range; `completed_at = None` tasks
///   are excluded because their completion date is unknown.
///
/// Cancelled tasks are never included — completion means `done_status` only.
pub fn completed_occurrences_in_range<'a>(
    tasks: &[&'a Task],
    settings: &Settings,
    range: Option<(&str, &str)>,
) -> Vec<&'a Task> {
    tasks
        .iter()
        .filter(|&&t| is_completed(t, settings))
        .filter(|&&t| match range {
            None => true,
            Some((start, end)) => t.completed_at.as_deref().map_or(false, |cat| {
                let date = &cat[..10.min(cat.len())];
                date >= start && date <= end
            }),
        })
        .copied()
        .collect()
}

/// Cancelled tasks at the *occurrence* level — same contract as
/// [`completed_occurrences_in_range`] but for `cancelled_status` /
/// `cancelled_at`. Returns an empty list when no cancelled status is
/// configured.
pub fn cancelled_occurrences_in_range<'a>(
    tasks: &[&'a Task],
    settings: &Settings,
    range: Option<(&str, &str)>,
) -> Vec<&'a Task> {
    tasks
        .iter()
        .filter(|&&t| is_cancelled(t, settings))
        .filter(|&&t| match range {
            None => true,
            Some((start, end)) => t.cancelled_at.as_deref().map_or(false, |cat| {
                let date = &cat[..10.min(cat.len())];
                date >= start && date <= end
            }),
        })
        .copied()
        .collect()
}

// ── Tracked-time aggregation ─────────────────────────────────────────────────

/// Total tracked minutes attributable to `project_id` for widget purposes:
/// every occurrence in the raw pool (subtasks rolled up, hidden excluded,
/// archived included when present in `tasks`) plus the project's own hidden
/// tracker task. When `include_subprojects` is `true`, descendant real
/// subprojects' pools and trackers are included too.
///
/// This deliberately does NOT reuse `status_stats::total_time_tracked` —
/// that function feeds the status bar and knows nothing about subtask
/// rollup; widgets need the rollup semantics.
pub fn project_tracked_minutes(
    project_id: &str,
    projects: &[Project],
    tasks: &[Task],
    include_subprojects: bool,
) -> i64 {
    let pool = if include_subprojects {
        project_task_pool_with_subprojects_raw(project_id, projects, tasks)
    } else {
        project_task_pool_raw(project_id, tasks)
    };
    let pool_minutes: i64 = pool.iter().map(|t| t.tracked_minutes as i64).sum();

    let mut tracker_project_ids: Vec<&str> = vec![project_id];
    if include_subprojects {
        tracker_project_ids.extend(
            real_descendants_of(projects, tasks, project_id)
                .iter()
                .map(|p| p.id.as_str()),
        );
    }
    let tracker_minutes: i64 = tracker_project_ids
        .iter()
        .filter_map(|pid| {
            let tracker_id = projects
                .iter()
                .find(|p| p.id == *pid)
                .and_then(|p| p.tracking_task_id.as_deref())?;
            tasks
                .iter()
                .find(|t| t.id == tracker_id)
                .map(|t| t.tracked_minutes as i64)
        })
        .sum();

    pool_minutes + tracker_minutes
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
        assert_eq!(
            tasks.iter().filter(|t| is_completed(t, &settings)).count(),
            0
        );
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
        assert_eq!(
            tasks.iter().filter(|t| is_cancelled(t, &settings)).count(),
            0
        );
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

    // ── subtask containers & effective project ──────────────────────────────

    /// A parent task in `project_id` owning subtask container `container_id`.
    fn parent_with_container(project_id: &str, container_id: &str) -> Task {
        Task {
            project_id: Some(project_id.to_string()),
            subtask_project_id: Some(container_id.to_string()),
            ..Task::new("Parent".to_string())
        }
    }

    fn project_with_id(id: &str) -> Project {
        Project {
            id: id.to_string(),
            ..Project::new("Test project".to_string(), "#ffffff".to_string(), 0)
        }
    }

    fn container_project(id: &str, parent_project_id: &str) -> Project {
        Project {
            parent_id: Some(parent_project_id.to_string()),
            ..project_with_id(id)
        }
    }

    fn child_project(id: &str, parent_project_id: &str) -> Project {
        Project {
            parent_id: Some(parent_project_id.to_string()),
            ..project_with_id(id)
        }
    }

    #[test]
    fn subtask_container_ids_empty_tasks_returns_empty() {
        assert!(subtask_container_ids(&[]).is_empty());
    }

    #[test]
    fn subtask_container_ids_collects_all_referenced_containers() {
        let tasks = [
            parent_with_container("proj-1", "cont-a"),
            parent_with_container("proj-1", "cont-b"),
            task_in_project("proj-1"),
        ];
        let ids = subtask_container_ids(&tasks);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains("cont-a") && ids.contains("cont-b"));
    }

    #[test]
    fn effective_project_id_plain_task_returns_own_project() {
        let tasks = [task_in_project("proj-1")];
        assert_eq!(effective_project_id(&tasks[0], &tasks), Some("proj-1"));
    }

    #[test]
    fn effective_project_id_subtask_resolves_to_owner_project() {
        let tasks = [
            parent_with_container("proj-1", "cont-a"),
            task_in_project("cont-a"), // subtask
        ];
        assert_eq!(effective_project_id(&tasks[1], &tasks), Some("proj-1"));
    }

    #[test]
    fn effective_project_id_nested_containers_resolve_transitively() {
        // Subtask of a subtask: cont-b owned by a task living inside cont-a,
        // which is owned by a task in proj-1.
        let mut mid = parent_with_container("cont-a", "cont-b");
        mid.project_id = Some("cont-a".to_string());
        let tasks = [
            parent_with_container("proj-1", "cont-a"),
            mid,
            task_in_project("cont-b"),
        ];
        assert_eq!(effective_project_id(&tasks[2], &tasks), Some("proj-1"));
    }

    #[test]
    fn effective_project_id_task_with_no_project_returns_none() {
        let tasks = [Task::new("Homeless".to_string())];
        assert_eq!(effective_project_id(&tasks[0], &tasks), None);
    }

    #[test]
    fn effective_project_id_orphaned_container_chain_returns_none() {
        // Owner task exists but itself has no project.
        let mut orphan_owner = parent_with_container("x", "cont-a");
        orphan_owner.project_id = None;
        let tasks = [orphan_owner, task_in_project("cont-a")];
        assert_eq!(effective_project_id(&tasks[1], &tasks), None);
    }

    #[test]
    fn effective_project_id_container_cycle_does_not_hang() {
        // Two containers owning each other's tasks — degenerate, must not loop.
        let mut a = parent_with_container("cont-b", "cont-a");
        a.project_id = Some("cont-b".to_string());
        let mut b = parent_with_container("cont-a", "cont-b");
        b.project_id = Some("cont-a".to_string());
        let tasks = [a, b, task_in_project("cont-a")];
        // Result value is unspecified for a cycle; the contract is termination.
        let _ = effective_project_id(&tasks[2], &tasks);
    }

    // ── real_children_of / real_descendants_of ───────────────────────────────

    #[test]
    fn real_children_of_excludes_subtask_containers() {
        let projects = [
            project_with_id("proj-1"),
            child_project("sub-1", "proj-1"),
            container_project("cont-a", "proj-1"),
        ];
        let tasks = [parent_with_container("proj-1", "cont-a")];
        let children = real_children_of(&projects, &tasks, "proj-1");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id, "sub-1");
    }

    #[test]
    fn real_descendants_of_excludes_containers_at_any_depth() {
        let projects = [
            project_with_id("proj-1"),
            child_project("sub-1", "proj-1"),
            container_project("cont-a", "sub-1"), // container inside subproject
        ];
        let mut owner = parent_with_container("sub-1", "cont-a");
        owner.project_id = Some("sub-1".to_string());
        let tasks = [owner];
        let descendants = real_descendants_of(&projects, &tasks, "proj-1");
        assert_eq!(descendants.len(), 1);
        assert_eq!(descendants[0].id, "sub-1");
    }

    // ── project_task_pool subtask rollup + raw variant ───────────────────────

    #[test]
    fn project_task_pool_rolls_subtasks_up_to_owner_project() {
        let tasks = [
            parent_with_container("proj-1", "cont-a"),
            task_in_project("cont-a"), // subtask — must count toward proj-1
        ];
        let pool = project_task_pool("proj-1", &tasks);
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn project_task_pool_raw_keeps_every_series_occurrence() {
        let mut o1 = task_in_project("proj-1");
        o1.series_id = Some("s-1".to_string());
        o1.due = Some("2026-06-01".to_string());
        let mut o2 = task_in_project("proj-1");
        o2.series_id = Some("s-1".to_string());
        o2.due = Some("2026-06-08".to_string());

        let tasks = [o1, o2];
        assert_eq!(project_task_pool_raw("proj-1", &tasks).len(), 2);
        assert_eq!(project_task_pool("proj-1", &tasks).len(), 1);
    }

    #[test]
    fn project_task_pool_raw_excludes_hidden() {
        let mut hidden = task_in_project("proj-1");
        hidden.hidden = true;
        let tasks = [hidden, task_in_project("proj-1")];
        assert_eq!(project_task_pool_raw("proj-1", &tasks).len(), 1);
    }

    #[test]
    fn project_task_pool_includes_archived_terminal_tasks() {
        // Archived tasks arrive pre-merged in `tasks`; the pool must not
        // treat `archived_at` as an exclusion criterion.
        let mut archived = task_in_project("proj-1");
        archived.status = "done".to_string();
        archived.archived_at = Some("2026-06-20T10:00:00Z".to_string());
        let tasks = [archived];
        assert_eq!(project_task_pool("proj-1", &tasks).len(), 1);
    }

    #[test]
    fn pool_with_subprojects_rolls_up_descendant_subtasks_but_not_containers() {
        let projects = [
            project_with_id("proj-1"),
            child_project("sub-1", "proj-1"),
            container_project("cont-a", "sub-1"),
        ];
        let mut owner = parent_with_container("sub-1", "cont-a");
        owner.project_id = Some("sub-1".to_string());
        let tasks = [
            owner,                     // parent task in sub-1
            task_in_project("cont-a"), // subtask → rolls up to sub-1
            task_in_project("proj-1"), // direct task
        ];
        let pool = project_task_pool_with_subprojects("proj-1", &projects, &tasks);
        assert_eq!(pool.len(), 3);
    }

    // ── completed_occurrences_in_range ───────────────────────────────────────

    #[test]
    fn completed_occurrences_all_time_includes_timestampless_done_tasks() {
        let settings = Settings::default();
        let mut done_old = task_with_status("done"); // completed_at = None
        done_old.completed_at = None;
        let refs = [&done_old];
        let result = completed_occurrences_in_range(&refs, &settings, None);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn completed_occurrences_ranged_excludes_timestampless() {
        let settings = Settings::default();
        let done_old = task_with_status("done");
        let refs = [&done_old];
        let result =
            completed_occurrences_in_range(&refs, &settings, Some(("2026-01-01", "2026-12-31")));
        assert!(result.is_empty());
    }

    #[test]
    fn completed_occurrences_counts_every_finished_series_occurrence() {
        let settings = Settings::default();
        let mut o1 = task_with_status("done");
        o1.series_id = Some("s-1".to_string());
        o1.completed_at = Some("2026-06-10T09:00:00Z".to_string());
        let mut o2 = task_with_status("done");
        o2.series_id = Some("s-1".to_string());
        o2.completed_at = Some("2026-06-17T09:00:00Z".to_string());
        let refs = [&o1, &o2];
        let result =
            completed_occurrences_in_range(&refs, &settings, Some(("2026-06-01", "2026-06-30")));
        assert_eq!(result.len(), 2, "each finished occurrence counts");
    }

    #[test]
    fn completed_occurrences_excludes_cancelled_tasks() {
        let settings = settings_with_cancelled();
        let cancelled = task_with_status("cancelled");
        let refs = [&cancelled];
        assert!(completed_occurrences_in_range(&refs, &settings, None).is_empty());
    }

    #[test]
    fn completed_occurrences_respects_range_boundaries() {
        let settings = Settings::default();
        let mut inside = task_with_status("done");
        inside.completed_at = Some("2026-06-30T23:59:59Z".to_string());
        let mut outside = task_with_status("done");
        outside.completed_at = Some("2026-07-01T00:00:00Z".to_string());
        let refs = [&inside, &outside];
        let result =
            completed_occurrences_in_range(&refs, &settings, Some(("2026-06-01", "2026-06-30")));
        assert_eq!(result.len(), 1);
    }

    // ── cancelled_occurrences_in_range ───────────────────────────────────────

    #[test]
    fn cancelled_occurrences_all_time_includes_timestampless() {
        let settings = settings_with_cancelled();
        let c = task_with_status("cancelled"); // cancelled_at = None
        let refs = [&c];
        assert_eq!(
            cancelled_occurrences_in_range(&refs, &settings, None).len(),
            1
        );
    }

    #[test]
    fn cancelled_occurrences_ranged_uses_cancelled_at() {
        let settings = settings_with_cancelled();
        let mut inside = task_with_status("cancelled");
        inside.cancelled_at = Some("2026-06-15T10:00:00Z".to_string());
        let no_ts = task_with_status("cancelled");
        let refs = [&inside, &no_ts];
        let result =
            cancelled_occurrences_in_range(&refs, &settings, Some(("2026-06-01", "2026-06-30")));
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn cancelled_occurrences_empty_when_no_cancelled_status_configured() {
        let settings = Settings::default();
        let c = task_with_status("cancelled");
        let refs = [&c];
        assert!(cancelled_occurrences_in_range(&refs, &settings, None).is_empty());
    }

    // ── project_tracked_minutes ──────────────────────────────────────────────

    #[test]
    fn project_tracked_minutes_sums_pool_and_subtasks() {
        let mut direct = task_in_project("proj-1");
        direct.tracked_minutes = 30;
        let mut parent = parent_with_container("proj-1", "cont-a");
        parent.tracked_minutes = 10;
        let mut subtask = task_in_project("cont-a");
        subtask.tracked_minutes = 20;
        let tasks = [direct, parent, subtask];
        let projects = [project_with_id("proj-1")];
        assert_eq!(
            project_tracked_minutes("proj-1", &projects, &tasks, false),
            60
        );
    }

    #[test]
    fn project_tracked_minutes_includes_hidden_tracker_task() {
        let mut tracker = Task::new("tracker".to_string());
        tracker.hidden = true;
        tracker.tracked_minutes = 45;
        let tracker_id = tracker.id.clone();
        let mut project = project_with_id("proj-1");
        project.tracking_task_id = Some(tracker_id);
        let tasks = [tracker];
        let projects = [project];
        assert_eq!(
            project_tracked_minutes("proj-1", &projects, &tasks, false),
            45
        );
    }

    #[test]
    fn project_tracked_minutes_with_subprojects_includes_descendants() {
        let mut child_task = task_in_project("sub-1");
        child_task.tracked_minutes = 25;
        let mut own_task = task_in_project("proj-1");
        own_task.tracked_minutes = 5;
        let tasks = [child_task, own_task];
        let projects = [project_with_id("proj-1"), child_project("sub-1", "proj-1")];
        assert_eq!(
            project_tracked_minutes("proj-1", &projects, &tasks, true),
            30
        );
        assert_eq!(
            project_tracked_minutes("proj-1", &projects, &tasks, false),
            5
        );
    }

    #[test]
    fn project_tracked_minutes_counts_all_series_occurrences() {
        let mut o1 = task_in_project("proj-1");
        o1.series_id = Some("s-1".to_string());
        o1.tracked_minutes = 15;
        let mut o2 = task_in_project("proj-1");
        o2.series_id = Some("s-1".to_string());
        o2.tracked_minutes = 15;
        let tasks = [o1, o2];
        let projects = [project_with_id("proj-1")];
        // Raw pool — both occurrences' time counts (Q-RECUR-1: time is summed).
        assert_eq!(
            project_tracked_minutes("proj-1", &projects, &tasks, false),
            30
        );
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
