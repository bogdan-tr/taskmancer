use rusqlite::{params, Connection};
use serde::Serialize;
use uuid::Uuid;

use crate::settings::Settings;
use crate::task::Task;

#[derive(Debug, thiserror::Error)]
pub enum StatusHistoryError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// A single recorded status transition for a task.
///
/// `from_status` is `None` for the initial creation event (no prior status)
/// and for seed events where the previous status is unknown. `source` is one
/// of `"user"` (explicit edit), `"cascade"` (automated cascade), or `"seed"`
/// (back-filled from existing task timestamps on first launch — shown with a
/// `~` prefix in the UI to indicate approximate data).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusHistoryEntry {
    pub id: String,
    pub task_id: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub changed_at: String,
    pub source: String,
}

/// Creates the `task_status_history` table and its indexes if they don't
/// already exist. Safe to call on every launch — `IF NOT EXISTS` is
/// idempotent.
pub fn init_schema(conn: &Connection) -> Result<(), StatusHistoryError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_status_history (
            id          TEXT PRIMARY KEY,
            task_id     TEXT NOT NULL,
            from_status TEXT,
            to_status   TEXT NOT NULL,
            changed_at  TEXT NOT NULL,
            source      TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_tsh_task_id
            ON task_status_history(task_id);
        CREATE INDEX IF NOT EXISTS idx_tsh_changed_at
            ON task_status_history(changed_at);",
    )?;
    Ok(())
}

/// Records a single status transition. `from_status` is `None` for the
/// initial creation row. `changed_at` is an RFC3339 UTC string.
pub fn record_transition(
    conn: &Connection,
    task_id: &str,
    from_status: Option<&str>,
    to_status: &str,
    changed_at: &str,
    source: &str,
) -> Result<StatusHistoryEntry, StatusHistoryError> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO task_status_history
            (id, task_id, from_status, to_status, changed_at, source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, task_id, from_status, to_status, changed_at, source],
    )?;
    Ok(StatusHistoryEntry {
        id,
        task_id: task_id.to_string(),
        from_status: from_status.map(str::to_string),
        to_status: to_status.to_string(),
        changed_at: changed_at.to_string(),
        source: source.to_string(),
    })
}

/// Returns all history entries for `task_id`, ordered oldest-first.
pub fn list_for_task(
    conn: &Connection,
    task_id: &str,
) -> Result<Vec<StatusHistoryEntry>, StatusHistoryError> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, from_status, to_status, changed_at, source
         FROM task_status_history
         WHERE task_id = ?1
         ORDER BY changed_at ASC",
    )?;
    let entries = stmt
        .query_map(params![task_id], |row| {
            Ok(StatusHistoryEntry {
                id: row.get(0)?,
                task_id: row.get(1)?,
                from_status: row.get(2)?,
                to_status: row.get(3)?,
                changed_at: row.get(4)?,
                source: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

/// Returns the total count of rows in `task_status_history`.
fn total_row_count(conn: &Connection) -> Result<i64, StatusHistoryError> {
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM task_status_history", [], |r| r.get(0))?;
    Ok(count)
}

/// Back-fills `task_status_history` from the task list's existing timestamp
/// fields — runs only when the table is completely empty (first launch after
/// the feature is added). Idempotent: a no-op if any rows already exist.
///
/// For each task, generates:
/// - A creation event (`from_status = NULL`) at `task.created` with
///   `to_status` = the task's current status (or `"backlog"` for terminal
///   tasks where the initial status is unknown).
/// - If `task.completed_at` is set: a completion event at that timestamp.
/// - If `task.cancelled_at` is set: a cancellation event at that timestamp.
pub fn seed_if_empty(
    conn: &Connection,
    live_tasks: &[Task],
    archived_tasks: &[Task],
    settings: &Settings,
) -> Result<(), StatusHistoryError> {
    if total_row_count(conn)? > 0 {
        return Ok(());
    }
    let all_tasks = live_tasks.iter().chain(archived_tasks.iter());
    for task in all_tasks {
        seed_task(conn, task, settings)?;
    }
    Ok(())
}

/// Seeds history for a single task. Called from [`seed_if_empty`].
fn seed_task(
    conn: &Connection,
    task: &Task,
    settings: &Settings,
) -> Result<(), StatusHistoryError> {
    let is_done = task.status == settings.done_status;
    let is_cancelled = settings
        .cancelled_status
        .as_deref()
        .map(|cs| task.status == cs)
        .unwrap_or(false);

    // For terminal tasks we can't know the initial status — use "backlog" as
    // the best approximation. For active tasks use the current status.
    let creation_to_status = if is_done || is_cancelled {
        "backlog"
    } else {
        &task.status
    };

    record_transition(
        conn,
        &task.id,
        None,
        creation_to_status,
        &task.created,
        "seed",
    )?;

    if let Some(completed_at) = &task.completed_at {
        if is_done {
            record_transition(
                conn,
                &task.id,
                None,
                &settings.done_status,
                completed_at,
                "seed",
            )?;
        }
    }

    if let Some(cancelled_at) = &task.cancelled_at {
        if is_cancelled {
            if let Some(cs) = &settings.cancelled_status {
                record_transition(conn, &task.id, None, cs, cancelled_at, "seed")?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use crate::task::Task;

    fn in_memory_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        init_schema(&conn).expect("schema");
        conn
    }

    fn default_settings() -> Settings {
        Settings::default()
    }

    fn make_task(id: &str, status: &str, created: &str) -> Task {
        let mut t = Task::new("Test task".to_string());
        t.id = id.to_string();
        t.status = status.to_string();
        t.created = created.to_string();
        t
    }

    #[test]
    fn init_schema_is_idempotent() {
        let conn = Connection::open_in_memory().expect("in-memory db");
        init_schema(&conn).expect("first call");
        init_schema(&conn).expect("second call should not fail");
    }

    #[test]
    fn record_transition_inserts_a_row() {
        let conn = in_memory_conn();

        let entry = record_transition(
            &conn,
            "task-1",
            Some("backlog"),
            "in-progress",
            "2026-01-01T10:00:00Z",
            "user",
        )
        .unwrap();

        assert_eq!(entry.task_id, "task-1");
        assert_eq!(entry.from_status, Some("backlog".to_string()));
        assert_eq!(entry.to_status, "in-progress");
        assert_eq!(entry.source, "user");
    }

    #[test]
    fn record_transition_from_status_can_be_null() {
        let conn = in_memory_conn();

        let entry = record_transition(&conn, "task-1", None, "backlog", "2026-01-01T00:00:00Z", "seed")
            .unwrap();

        assert_eq!(entry.from_status, None);
        assert_eq!(entry.to_status, "backlog");
    }

    #[test]
    fn list_for_task_returns_entries_oldest_first() {
        let conn = in_memory_conn();

        record_transition(
            &conn, "t1", Some("backlog"), "in-progress", "2026-01-02T00:00:00Z", "user",
        )
        .unwrap();
        record_transition(&conn, "t1", None, "backlog", "2026-01-01T00:00:00Z", "seed")
            .unwrap();

        let entries = list_for_task(&conn, "t1").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].changed_at, "2026-01-01T00:00:00Z");
        assert_eq!(entries[1].changed_at, "2026-01-02T00:00:00Z");
    }

    #[test]
    fn list_for_task_only_returns_entries_for_that_task() {
        let conn = in_memory_conn();

        record_transition(&conn, "t1", None, "backlog", "2026-01-01T00:00:00Z", "seed").unwrap();
        record_transition(&conn, "t2", None, "backlog", "2026-01-01T00:00:00Z", "seed").unwrap();

        let entries = list_for_task(&conn, "t1").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].task_id, "t1");
    }

    #[test]
    fn list_for_task_returns_empty_for_unknown_task() {
        let conn = in_memory_conn();
        let entries = list_for_task(&conn, "no-such-task").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn seed_if_empty_inserts_creation_event_for_each_task() {
        let conn = in_memory_conn();
        let settings = default_settings();
        let tasks = vec![
            make_task("t1", "backlog", "2026-01-01T00:00:00Z"),
            make_task("t2", "in-progress", "2026-01-02T00:00:00Z"),
        ];

        seed_if_empty(&conn, &tasks, &[], &settings).unwrap();

        let t1_history = list_for_task(&conn, "t1").unwrap();
        assert_eq!(t1_history.len(), 1);
        assert_eq!(t1_history[0].from_status, None);
        assert_eq!(t1_history[0].to_status, "backlog");
        assert_eq!(t1_history[0].changed_at, "2026-01-01T00:00:00Z");
        assert_eq!(t1_history[0].source, "seed");
    }

    #[test]
    fn seed_if_empty_seeds_completed_at_event_for_done_task() {
        let conn = in_memory_conn();
        let settings = default_settings();
        let mut task = make_task("t1", &settings.done_status, "2026-01-01T00:00:00Z");
        task.completed_at = Some("2026-01-05T15:30:00Z".to_string());

        seed_if_empty(&conn, &[task], &[], &settings).unwrap();

        let history = list_for_task(&conn, "t1").unwrap();
        assert_eq!(history.len(), 2);
        // Creation event first
        assert_eq!(history[0].to_status, "backlog");
        assert_eq!(history[0].source, "seed");
        // Terminal event
        assert_eq!(history[1].to_status, settings.done_status);
        assert_eq!(history[1].changed_at, "2026-01-05T15:30:00Z");
        assert_eq!(history[1].source, "seed");
    }

    #[test]
    fn seed_if_empty_seeds_cancelled_at_event_for_cancelled_task() {
        let conn = in_memory_conn();
        let mut settings = default_settings();
        settings.cancelled_status = Some("cancelled".to_string());

        let mut task = make_task("t1", "cancelled", "2026-01-01T00:00:00Z");
        task.cancelled_at = Some("2026-01-03T12:00:00Z".to_string());

        seed_if_empty(&conn, &[task], &[], &settings).unwrap();

        let history = list_for_task(&conn, "t1").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[1].to_status, "cancelled");
        assert_eq!(history[1].changed_at, "2026-01-03T12:00:00Z");
    }

    #[test]
    fn seed_if_empty_is_noop_when_rows_already_exist() {
        let conn = in_memory_conn();
        let settings = default_settings();
        let tasks = vec![make_task("t1", "backlog", "2026-01-01T00:00:00Z")];

        seed_if_empty(&conn, &tasks, &[], &settings).unwrap();
        seed_if_empty(&conn, &tasks, &[], &settings).unwrap();

        let history = list_for_task(&conn, "t1").unwrap();
        assert_eq!(history.len(), 1, "seeding twice should not duplicate rows");
    }

    #[test]
    fn seed_if_empty_includes_archived_tasks() {
        let conn = in_memory_conn();
        let settings = default_settings();
        let live = vec![make_task("live-1", "backlog", "2026-01-01T00:00:00Z")];
        let archived = vec![make_task("archived-1", &settings.done_status, "2026-01-02T00:00:00Z")];

        seed_if_empty(&conn, &live, &archived, &settings).unwrap();

        assert_eq!(list_for_task(&conn, "live-1").unwrap().len(), 1);
        assert_eq!(list_for_task(&conn, "archived-1").unwrap().len(), 1);
    }

    #[test]
    fn seed_if_empty_uses_current_status_for_active_non_backlog_task() {
        let conn = in_memory_conn();
        let settings = default_settings();
        let tasks = vec![make_task("t1", "in-progress", "2026-01-01T00:00:00Z")];

        seed_if_empty(&conn, &tasks, &[], &settings).unwrap();

        let history = list_for_task(&conn, "t1").unwrap();
        assert_eq!(history[0].to_status, "in-progress");
    }
}
