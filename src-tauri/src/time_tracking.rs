use std::path::Path;

use rusqlite::Connection;

use crate::storage::{self, StorageError};
use crate::time_storage::{self, TimeStorageError};

#[derive(Debug, thiserror::Error)]
pub enum TimeTrackingError {
    #[error("time storage error: {0}")]
    TimeStorage(#[from] TimeStorageError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
}

/// Converts a duration in whole seconds to whole minutes, rounding to the
/// nearest minute (round-half-up: an exact 30s rounds up to 1 minute, not
/// down to 0). Nearest-rounding — rather than floor/truncate — is
/// deliberate: SQLite holds the exact, second-precision source of truth, and
/// `Task.tracked_minutes` is recomputed fresh from it on every start/stop
/// rather than incremented in place, specifically so repeated short sessions
/// don't accumulate a systematic downward bias the way floor-rounding would.
pub fn minutes_from_seconds(total_seconds: i64) -> u32 {
    if total_seconds <= 0 {
        return 0;
    }
    // Integer round-half-up: (n + half_divisor) / divisor.
    let minutes = (total_seconds + 30) / 60;
    minutes as u32
}

/// Recomputes `tracked_minutes` for `task_id` from the exact SQLite-stored
/// completed sessions (see [`time_storage::total_tracked_seconds_for_task`]),
/// then loads the task's markdown file from `tasks_dir`, rewrites its
/// `tracked_minutes` to the new value, and saves it back. Returns the newly
/// persisted value. Called after every session start/stop, per the
/// time-tracking-engine spec's "never incremented in place" rule.
pub fn recompute_and_persist_tracked_minutes(
    conn: &Connection,
    tasks_dir: &Path,
    task_id: &str,
) -> Result<u32, TimeTrackingError> {
    let total_seconds = time_storage::total_tracked_seconds_for_task(conn, task_id)?;
    let minutes = minutes_from_seconds(total_seconds);

    let mut task = storage::load_task(tasks_dir, task_id)?;
    task.tracked_minutes = minutes;
    storage::update_task(tasks_dir, &task)?;

    Ok(minutes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Task;
    use tempfile::tempdir;

    mod minutes_from_seconds_tests {
        use super::*;

        #[test]
        fn zero_seconds_is_zero_minutes() {
            assert_eq!(minutes_from_seconds(0), 0);
        }

        #[test]
        fn twenty_nine_seconds_rounds_down_to_zero_minutes() {
            assert_eq!(minutes_from_seconds(29), 0);
        }

        #[test]
        fn thirty_seconds_rounds_up_to_one_minute() {
            // Round-half-up: an exact half-minute rounds to the next minute.
            assert_eq!(minutes_from_seconds(30), 1);
        }

        #[test]
        fn thirty_one_seconds_rounds_up_to_one_minute() {
            assert_eq!(minutes_from_seconds(31), 1);
        }

        #[test]
        fn fifty_nine_seconds_rounds_up_to_one_minute() {
            assert_eq!(minutes_from_seconds(59), 1);
        }

        #[test]
        fn sixty_seconds_is_exactly_one_minute() {
            assert_eq!(minutes_from_seconds(60), 1);
        }

        #[test]
        fn ninety_seconds_rounds_up_to_two_minutes() {
            // Round-half-up rule applied explicitly at the 1.5-minute boundary.
            assert_eq!(minutes_from_seconds(90), 2);
        }

        #[test]
        fn eighty_nine_seconds_rounds_down_to_one_minute() {
            assert_eq!(minutes_from_seconds(89), 1);
        }

        #[test]
        fn negative_seconds_clamps_to_zero_minutes() {
            assert_eq!(minutes_from_seconds(-100), 0);
        }

        #[test]
        fn large_value_rounds_correctly() {
            // 36000s = 600 minutes exactly (10 hours).
            assert_eq!(minutes_from_seconds(36_000), 600);
        }

        #[test]
        fn large_value_with_remainder_rounds_up() {
            // 36031s -> 600 min 31s -> rounds up to 601 minutes.
            assert_eq!(minutes_from_seconds(36_031), 601);
        }
    }

    fn dt(rfc3339: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(rfc3339)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn recompute_and_persist_tracked_minutes_writes_the_rounded_total_to_the_task_file() {
        let tasks_dir = tempdir().unwrap();
        let conn = Connection::open_in_memory().unwrap();
        time_storage::init_schema(&conn).unwrap();

        let task = Task::new("Write report".to_string());
        storage::save_task(tasks_dir.path(), &task).unwrap();

        time_storage::start_entry(&conn, &task.id, dt("2026-06-15T09:00:00+00:00")).unwrap();
        time_storage::end_entry(&conn, &task.id, dt("2026-06-15T09:10:00+00:00")).unwrap();

        let minutes =
            recompute_and_persist_tracked_minutes(&conn, tasks_dir.path(), &task.id).unwrap();

        assert_eq!(minutes, 10);
        let reloaded = storage::load_task(tasks_dir.path(), &task.id).unwrap();
        assert_eq!(reloaded.tracked_minutes, 10);
    }

    #[test]
    fn recompute_and_persist_tracked_minutes_overwrites_a_stale_cached_value() {
        let tasks_dir = tempdir().unwrap();
        let conn = Connection::open_in_memory().unwrap();
        time_storage::init_schema(&conn).unwrap();

        let mut task = Task::new("Write report".to_string());
        task.tracked_minutes = 999; // stale/incorrect cached value
        storage::save_task(tasks_dir.path(), &task).unwrap();

        time_storage::start_entry(&conn, &task.id, dt("2026-06-15T09:00:00+00:00")).unwrap();
        time_storage::end_entry(&conn, &task.id, dt("2026-06-15T09:05:00+00:00")).unwrap();

        let minutes =
            recompute_and_persist_tracked_minutes(&conn, tasks_dir.path(), &task.id).unwrap();

        assert_eq!(minutes, 5);
    }

    #[test]
    fn recompute_and_persist_tracked_minutes_is_zero_for_a_task_with_no_sessions() {
        let tasks_dir = tempdir().unwrap();
        let conn = Connection::open_in_memory().unwrap();
        time_storage::init_schema(&conn).unwrap();

        let task = Task::new("Untouched task".to_string());
        storage::save_task(tasks_dir.path(), &task).unwrap();

        let minutes =
            recompute_and_persist_tracked_minutes(&conn, tasks_dir.path(), &task.id).unwrap();

        assert_eq!(minutes, 0);
        let reloaded = storage::load_task(tasks_dir.path(), &task.id).unwrap();
        assert_eq!(reloaded.tracked_minutes, 0);
    }

    #[test]
    fn recompute_and_persist_tracked_minutes_excludes_a_still_running_session() {
        let tasks_dir = tempdir().unwrap();
        let conn = Connection::open_in_memory().unwrap();
        time_storage::init_schema(&conn).unwrap();

        let task = Task::new("In progress".to_string());
        storage::save_task(tasks_dir.path(), &task).unwrap();
        time_storage::start_entry(&conn, &task.id, dt("2026-06-15T09:00:00+00:00")).unwrap();

        let minutes =
            recompute_and_persist_tracked_minutes(&conn, tasks_dir.path(), &task.id).unwrap();

        assert_eq!(minutes, 0);
    }

    #[test]
    fn recompute_and_persist_tracked_minutes_returns_not_found_for_a_missing_task() {
        let tasks_dir = tempdir().unwrap();
        let conn = Connection::open_in_memory().unwrap();
        time_storage::init_schema(&conn).unwrap();

        let result =
            recompute_and_persist_tracked_minutes(&conn, tasks_dir.path(), "missing-task-id");

        assert!(matches!(
            result,
            Err(TimeTrackingError::Storage(StorageError::NotFound(_)))
        ));
    }

    #[test]
    fn recompute_and_persist_tracked_minutes_sums_across_multiple_completed_sessions() {
        let tasks_dir = tempdir().unwrap();
        let conn = Connection::open_in_memory().unwrap();
        time_storage::init_schema(&conn).unwrap();

        let task = Task::new("Multi-session task".to_string());
        storage::save_task(tasks_dir.path(), &task).unwrap();

        time_storage::start_entry(&conn, &task.id, dt("2026-06-15T09:00:00+00:00")).unwrap();
        time_storage::end_entry(&conn, &task.id, dt("2026-06-15T09:10:00+00:00")).unwrap(); // 10 min
        time_storage::start_entry(&conn, &task.id, dt("2026-06-15T12:00:00+00:00")).unwrap();
        time_storage::end_entry(&conn, &task.id, dt("2026-06-15T12:20:00+00:00")).unwrap(); // 20 min

        let minutes =
            recompute_and_persist_tracked_minutes(&conn, tasks_dir.path(), &task.id).unwrap();

        assert_eq!(minutes, 30);
    }
}
