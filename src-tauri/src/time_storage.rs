use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum TimeStorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("failed to parse stored RFC3339 timestamp '{value}': {source}")]
    InvalidTimestamp {
        value: String,
        source: chrono::ParseError,
    },
}

/// A single time-tracking session against a task. `ended_at: None` means the
/// session is currently running — see the module-level schema comment on
/// [`init_schema`] for the full column semantics. `Serialize`: returned
/// directly from the `get_active_sessions`/`list_time_entries` Tauri
/// commands (`commands.rs`, Milestone 2).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimeEntry {
    pub id: String,
    pub task_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub last_heartbeat_at: Option<String>,
    pub created_at: String,
}

/// Creates the `time_entries` table and its `task_id` index if they don't
/// already exist. Safe to call on every app launch — `CREATE TABLE IF NOT
/// EXISTS`/`CREATE INDEX IF NOT EXISTS` make this idempotent.
///
/// At most one row per `task_id` may have `ended_at IS NULL` at a time (a
/// task can't have two concurrently-running sessions); this invariant is
/// enforced by [`start_entry`] rather than a DB-level constraint, since
/// SQLite has no portable way to express "unique among NULL-excluded rows"
/// without a partial index, which `bundled` SQLite does support but which
/// would make the no-op-on-already-running behavior in [`start_entry`] an
/// error instead of the desired no-op return.
pub fn init_schema(conn: &Connection) -> Result<(), TimeStorageError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS time_entries (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT,
            last_heartbeat_at TEXT,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_time_entries_task_id ON time_entries(task_id);",
    )?;
    Ok(())
}

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<TimeEntry> {
    Ok(TimeEntry {
        id: row.get(0)?,
        task_id: row.get(1)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        last_heartbeat_at: row.get(4)?,
        created_at: row.get(5)?,
    })
}

const SELECT_COLUMNS: &str = "id, task_id, started_at, ended_at, last_heartbeat_at, created_at";

/// Starts a new tracking session for `task_id`. If `task_id` already has an
/// active session (`ended_at IS NULL`), this is a no-op that returns the
/// existing active entry rather than creating a duplicate — "track this task
/// twice" has no meaning, per the time-tracking-engine spec.
pub fn start_entry(
    conn: &Connection,
    task_id: &str,
    started_at: DateTime<Utc>,
) -> Result<TimeEntry, TimeStorageError> {
    if let Some(existing) = get_active_entry_for_task(conn, task_id)? {
        return Ok(existing);
    }

    let entry = TimeEntry {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_string(),
        started_at: started_at.to_rfc3339(),
        ended_at: None,
        last_heartbeat_at: None,
        created_at: Utc::now().to_rfc3339(),
    };

    conn.execute(
        "INSERT INTO time_entries (id, task_id, started_at, ended_at, last_heartbeat_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            entry.id,
            entry.task_id,
            entry.started_at,
            entry.ended_at,
            entry.last_heartbeat_at,
            entry.created_at,
        ],
    )?;

    Ok(entry)
}

/// Ends the active session for `task_id` by setting `ended_at`. Returns
/// `Ok(None)` (not an error) if `task_id` has no active session — stopping a
/// non-running task is a valid no-op per the spec.
pub fn end_entry(
    conn: &Connection,
    task_id: &str,
    ended_at: DateTime<Utc>,
) -> Result<Option<TimeEntry>, TimeStorageError> {
    let Some(mut active) = get_active_entry_for_task(conn, task_id)? else {
        return Ok(None);
    };

    let ended_at_str = ended_at.to_rfc3339();
    conn.execute(
        "UPDATE time_entries SET ended_at = ?1 WHERE id = ?2",
        params![ended_at_str, active.id],
    )?;
    active.ended_at = Some(ended_at_str);

    Ok(Some(active))
}

/// Returns the currently-active session (`ended_at IS NULL`) for `task_id`,
/// if any.
pub fn get_active_entry_for_task(
    conn: &Connection,
    task_id: &str,
) -> Result<Option<TimeEntry>, TimeStorageError> {
    let sql = format!(
        "SELECT {SELECT_COLUMNS} FROM time_entries WHERE task_id = ?1 AND ended_at IS NULL"
    );
    let entry = conn
        .query_row(&sql, params![task_id], row_to_entry)
        .optional()?;
    Ok(entry)
}

/// Returns every currently-active session (`ended_at IS NULL`) across all
/// tasks. Used on app launch to detect orphaned sessions (see the
/// time-tracking-engine spec's "Orphaned sessions across app restarts").
pub fn list_active_entries(conn: &Connection) -> Result<Vec<TimeEntry>, TimeStorageError> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM time_entries WHERE ended_at IS NULL");
    let mut stmt = conn.prepare(&sql)?;
    let entries = stmt
        .query_map([], row_to_entry)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(entries)
}

/// Returns every session (active or completed) for `task_id`, in no
/// particular guaranteed order beyond SQLite's default row order.
pub fn list_entries_for_task(
    conn: &Connection,
    task_id: &str,
) -> Result<Vec<TimeEntry>, TimeStorageError> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM time_entries WHERE task_id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let entries = stmt
        .query_map(params![task_id], row_to_entry)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(entries)
}

/// Returns the entry with id `entry_id`, or `Ok(None)` if no such entry
/// exists. Used by the Tauri command layer (Milestone 2) wherever an
/// operation is keyed by entry id but needs to know that entry's `task_id`
/// or timestamps first (e.g. [`crate::time_tracking::recompute_and_persist_tracked_minutes`]
/// needs a `task_id`, not an `entry_id`).
pub fn get_entry(conn: &Connection, entry_id: &str) -> Result<Option<TimeEntry>, TimeStorageError> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM time_entries WHERE id = ?1");
    let entry = conn
        .query_row(&sql, params![entry_id], row_to_entry)
        .optional()?;
    Ok(entry)
}

/// Inserts a fully-completed entry (both `started_at` and `ended_at` set
/// immediately) with a freshly generated id, for manual time-entry creation
/// via the Tauri command layer's `add_manual_time_entry` (Milestone 2) —
/// unlike [`start_entry`], this never produces an active (`ended_at IS
/// NULL`) row, so it doesn't participate in the "at most one active session
/// per task" invariant at all.
pub fn insert_completed_entry(
    conn: &Connection,
    task_id: &str,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
) -> Result<TimeEntry, TimeStorageError> {
    let entry = TimeEntry {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_string(),
        started_at: started_at.to_rfc3339(),
        ended_at: Some(ended_at.to_rfc3339()),
        last_heartbeat_at: None,
        created_at: Utc::now().to_rfc3339(),
    };

    conn.execute(
        "INSERT INTO time_entries (id, task_id, started_at, ended_at, last_heartbeat_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            entry.id,
            entry.task_id,
            entry.started_at,
            entry.ended_at,
            entry.last_heartbeat_at,
            entry.created_at,
        ],
    )?;

    Ok(entry)
}

/// Updates `last_heartbeat_at` on the active session for `task_id`. A no-op
/// (not an error) if `task_id` has no active session — the heartbeat command
/// runs unconditionally every ~30s regardless of which tasks happen to be
/// running.
pub fn update_heartbeat(
    conn: &Connection,
    task_id: &str,
    at: DateTime<Utc>,
) -> Result<(), TimeStorageError> {
    conn.execute(
        "UPDATE time_entries SET last_heartbeat_at = ?1 WHERE task_id = ?2 AND ended_at IS NULL",
        params![at.to_rfc3339(), task_id],
    )?;
    Ok(())
}

/// Overwrites `started_at`/`ended_at` on an existing entry by id, for manual
/// correction. Does not validate that `started_at <= ended_at` — the Tauri
/// command wrapping this (Milestone 2) is responsible for any such
/// validation before calling this primitive.
pub fn update_entry_times(
    conn: &Connection,
    entry_id: &str,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
) -> Result<(), TimeStorageError> {
    let ended_at_str = ended_at.map(|dt| dt.to_rfc3339());
    conn.execute(
        "UPDATE time_entries SET started_at = ?1, ended_at = ?2 WHERE id = ?3",
        params![started_at.to_rfc3339(), ended_at_str, entry_id],
    )?;
    Ok(())
}

/// Deletes the entry with id `entry_id`. A no-op if no such entry exists —
/// mirrors the idempotent-delete-by-id style used elsewhere isn't required
/// here since callers always have a concrete entry id in hand from a prior
/// list/get call.
pub fn delete_entry(conn: &Connection, entry_id: &str) -> Result<(), TimeStorageError> {
    conn.execute("DELETE FROM time_entries WHERE id = ?1", params![entry_id])?;
    Ok(())
}

/// Sums `(ended_at - started_at)` in whole seconds across only the
/// *completed* sessions (`ended_at IS NOT NULL`) for `task_id`. A
/// still-running session contributes nothing here — the live "currently
/// running" ticker is computed client-side from `started_at` separately per
/// the spec, and this function feeds only the persisted `tracked_minutes`
/// cache, which must reflect just what's actually finished.
pub fn total_tracked_seconds_for_task(
    conn: &Connection,
    task_id: &str,
) -> Result<i64, TimeStorageError> {
    let mut stmt = conn.prepare(
        "SELECT started_at, ended_at FROM time_entries
         WHERE task_id = ?1 AND ended_at IS NOT NULL",
    )?;
    let rows = stmt.query_map(params![task_id], |row| {
        let started_at: String = row.get(0)?;
        let ended_at: String = row.get(1)?;
        Ok((started_at, ended_at))
    })?;

    let mut total_seconds: i64 = 0;
    for row in rows {
        let (started_at, ended_at) = row?;
        let started = parse_rfc3339(&started_at)?;
        let ended = parse_rfc3339(&ended_at)?;
        total_seconds += (ended - started).num_seconds();
    }

    Ok(total_seconds)
}

fn parse_rfc3339(value: &str) -> Result<DateTime<Utc>, TimeStorageError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|source| TimeStorageError::InvalidTimestamp {
            value: value.to_string(),
            source,
        })
}

/// Returns the start-of-day `DateTime<Utc>` for `date` — calendar-day
/// boundaries are computed in UTC throughout this module, matching how every
/// stored timestamp is already UTC RFC3339 (see [`init_schema`]'s column
/// doc comment); there is no separate "local day" concept anywhere else in
/// this codebase's time-tracking storage layer.
#[allow(dead_code)]
fn start_of_day_utc(date: chrono::NaiveDate) -> DateTime<Utc> {
    date.and_hms_opt(0, 0, 0)
        .expect("midnight is always a valid time")
        .and_utc()
}

/// Clips `[started_at, ended_at)` against `[day_start, day_start + 1 day)`
/// and returns the overlap in whole seconds (`0` if there's no overlap at
/// all). Shared by [`tracked_seconds_per_day`]'s day-bucketing loop — the
/// core of the "day-splitting" query-time clip described in the
/// time-tracking-engine spec's Storage section.
#[allow(dead_code)]
fn overlap_seconds_with_day(
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    day_start: DateTime<Utc>,
) -> i64 {
    let day_end = day_start + chrono::Duration::days(1);
    let overlap_start = started_at.max(day_start);
    let overlap_end = ended_at.min(day_end);
    if overlap_end <= overlap_start {
        return 0;
    }
    (overlap_end - overlap_start).num_seconds()
}

/// Returns, for every task id in `task_ids`, tracked seconds bucketed per
/// calendar day (UTC) over the half-open window `[range_start, range_end)`
/// — the day-clipping aggregation the project-status-line feature's
/// `avg_time_per_week` stat is built on (see the time-tracking-engine spec's
/// "Day-splitting" paragraph: rows are never physically split at midnight,
/// so any day-bucketed report clips at query time instead). Days with zero
/// tracked seconds are simply absent from the returned map rather than
/// present with a `0` entry.
///
/// A still-running entry (`ended_at IS NULL`) has its open end clipped
/// against `now` rather than treated as unbounded — mirrors how the
/// frontend's live ticker already treats a running session (computed
/// client-side from `started_at`, capped at "now"). An entry that starts
/// before `range_start` or ends after `range_end` is clipped to the window;
/// one that doesn't overlap `[range_start, range_end)` at all contributes
/// nothing. `range_start >= range_end` (an empty or inverted window) always
/// returns an empty map.
#[allow(dead_code)]
pub fn tracked_seconds_per_day(
    conn: &Connection,
    task_ids: &[String],
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<std::collections::HashMap<chrono::NaiveDate, i64>, TimeStorageError> {
    let mut buckets: std::collections::HashMap<chrono::NaiveDate, i64> =
        std::collections::HashMap::new();

    if task_ids.is_empty() || range_start >= range_end {
        return Ok(buckets);
    }

    let placeholders = task_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql =
        format!("SELECT started_at, ended_at FROM time_entries WHERE task_id IN ({placeholders})");
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::ToSql> = task_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect();
    let rows = stmt.query_map(params.as_slice(), |row| {
        let started_at: String = row.get(0)?;
        let ended_at: Option<String> = row.get(1)?;
        Ok((started_at, ended_at))
    })?;

    for row in rows {
        let (started_at, ended_at) = row?;
        let started = parse_rfc3339(&started_at)?;
        let ended = match ended_at {
            Some(value) => parse_rfc3339(&value)?,
            None => now,
        };

        // Clip the whole entry to the query window first, then walk
        // day-by-day within that already-clipped range.
        let clipped_start = started.max(range_start);
        let clipped_end = ended.min(range_end);
        if clipped_end <= clipped_start {
            continue;
        }

        let mut day = clipped_start.date_naive();
        let last_day = (clipped_end - chrono::Duration::nanoseconds(1)).date_naive();
        while day <= last_day {
            let day_start = start_of_day_utc(day);
            let seconds = overlap_seconds_with_day(clipped_start, clipped_end, day_start);
            if seconds > 0 {
                *buckets.entry(day).or_insert(0) += seconds;
            }
            day += chrono::Duration::days(1);
        }
    }

    Ok(buckets)
}

/// Total tracked seconds across **all** tasks within `[range_start, range_end)`,
/// clipping still-running entries against `now`. Used for the global status
/// bar's "time tracked today/this week" stats, where filtering by a set of
/// task ids would require loading every task first.
pub fn total_tracked_seconds_all_tasks_in_range(
    conn: &Connection,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<i64, TimeStorageError> {
    if range_start >= range_end {
        return Ok(0);
    }

    let mut stmt = conn.prepare("SELECT started_at, ended_at FROM time_entries")?;
    let rows = stmt.query_map([], |row| {
        let started_at: String = row.get(0)?;
        let ended_at: Option<String> = row.get(1)?;
        Ok((started_at, ended_at))
    })?;

    let mut total: i64 = 0;
    for row in rows {
        let (started_at, ended_at) = row?;
        let started = parse_rfc3339(&started_at)?;
        let ended = match ended_at {
            Some(v) => parse_rfc3339(&v)?,
            None => now,
        };
        let clipped_start = started.max(range_start);
        let clipped_end = ended.min(range_end);
        if clipped_end > clipped_start {
            total += (clipped_end - clipped_start).num_seconds();
        }
    }

    Ok(total)
}

/// Total tracked seconds for the specified `task_ids` within
/// `[range_start, range_end)`, clipping still-running entries against `now`.
/// Returns `0` immediately when `task_ids` is empty or the window is empty/
/// inverted. Mirrors [`total_tracked_seconds_all_tasks_in_range`] but filters
/// by a caller-supplied id list, as needed by the analytics dashboard's
/// per-project and per-tag time aggregations.
#[allow(dead_code)]
pub fn total_tracked_seconds_for_task_ids_in_range(
    conn: &Connection,
    task_ids: &[String],
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<i64, TimeStorageError> {
    if task_ids.is_empty() || range_start >= range_end {
        return Ok(0);
    }

    let placeholders = task_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql =
        format!("SELECT started_at, ended_at FROM time_entries WHERE task_id IN ({placeholders})");
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::ToSql> = task_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect();
    let rows = stmt.query_map(params.as_slice(), |row| {
        let started_at: String = row.get(0)?;
        let ended_at: Option<String> = row.get(1)?;
        Ok((started_at, ended_at))
    })?;

    let mut total: i64 = 0;
    for row in rows {
        let (started_at, ended_at) = row?;
        let started = parse_rfc3339(&started_at)?;
        let ended = match ended_at {
            Some(v) => parse_rfc3339(&v)?,
            None => now,
        };
        let clipped_start = started.max(range_start);
        let clipped_end = ended.min(range_end);
        if clipped_end > clipped_start {
            total += (clipped_end - clipped_start).num_seconds();
        }
    }

    Ok(total)
}

/// Returns every time-entry row that overlaps `[range_start, range_end)` as a
/// `(task_id, clamped_start, clamped_end)` triple, with the open endpoint of
/// still-running entries clamped to `now` and each entry additionally clamped
/// to the query window.  Used by the analytics dashboard's busy-histogram
/// command, which needs per-entry timestamps (not just totals) to bucket work
/// by day-of-week and hour-of-day.
#[allow(dead_code, clippy::type_complexity)]
pub fn list_time_entries_in_range(
    conn: &Connection,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<Vec<(String, DateTime<Utc>, DateTime<Utc>)>, TimeStorageError> {
    if range_start >= range_end {
        return Ok(Vec::new());
    }

    let mut stmt = conn.prepare("SELECT task_id, started_at, ended_at FROM time_entries")?;
    let rows = stmt.query_map([], |row| {
        let task_id: String = row.get(0)?;
        let started_at: String = row.get(1)?;
        let ended_at: Option<String> = row.get(2)?;
        Ok((task_id, started_at, ended_at))
    })?;

    let mut entries = Vec::new();
    for row in rows {
        let (task_id, started_at, ended_at) = row?;
        let started = parse_rfc3339(&started_at)?;
        let ended = match ended_at {
            Some(v) => parse_rfc3339(&v)?,
            None => now,
        };
        let clipped_start = started.max(range_start);
        let clipped_end = ended.min(range_end);
        if clipped_end > clipped_start {
            entries.push((task_id, clipped_start, clipped_end));
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    fn dt(rfc3339: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(rfc3339)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn init_schema_is_idempotent() {
        let conn = setup();

        let result = init_schema(&conn);

        assert!(result.is_ok());
    }

    #[test]
    fn start_entry_creates_an_active_session() {
        let conn = setup();
        let started_at = dt("2026-06-15T09:00:00+00:00");

        let entry = start_entry(&conn, "task-1", started_at).unwrap();

        assert_eq!(entry.task_id, "task-1");
        assert_eq!(entry.started_at, "2026-06-15T09:00:00+00:00");
        assert_eq!(entry.ended_at, None);
        assert!(!entry.id.is_empty());
    }

    #[test]
    fn start_entry_for_an_already_running_task_is_a_no_op_and_returns_existing_entry() {
        let conn = setup();
        let first_start = dt("2026-06-15T09:00:00+00:00");
        let second_start = dt("2026-06-15T09:30:00+00:00");

        let first = start_entry(&conn, "task-1", first_start).unwrap();
        let second = start_entry(&conn, "task-1", second_start).unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(second.started_at, "2026-06-15T09:00:00+00:00");
        let all_entries = list_entries_for_task(&conn, "task-1").unwrap();
        assert_eq!(all_entries.len(), 1);
    }

    #[test]
    fn end_entry_stops_the_active_session() {
        let conn = setup();
        let started_at = dt("2026-06-15T09:00:00+00:00");
        let ended_at = dt("2026-06-15T10:00:00+00:00");
        start_entry(&conn, "task-1", started_at).unwrap();

        let ended = end_entry(&conn, "task-1", ended_at).unwrap();

        let ended = ended.expect("expected an active session to end");
        assert_eq!(
            ended.ended_at,
            Some("2026-06-15T10:00:00+00:00".to_string())
        );
        assert_eq!(get_active_entry_for_task(&conn, "task-1").unwrap(), None);
    }

    #[test]
    fn end_entry_returns_none_when_no_active_session_exists() {
        let conn = setup();

        let result = end_entry(
            &conn,
            "task-with-no-sessions",
            dt("2026-06-15T10:00:00+00:00"),
        );

        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn end_entry_is_not_repeatable() {
        let conn = setup();
        start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
        end_entry(&conn, "task-1", dt("2026-06-15T10:00:00+00:00")).unwrap();

        let second_stop = end_entry(&conn, "task-1", dt("2026-06-15T11:00:00+00:00"));

        assert_eq!(second_stop.unwrap(), None);
    }

    #[test]
    fn get_active_entry_for_task_returns_none_for_a_task_with_zero_entries() {
        let conn = setup();

        let result = get_active_entry_for_task(&conn, "never-tracked-task").unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn get_active_entry_for_task_returns_none_after_completed_session() {
        let conn = setup();
        start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
        end_entry(&conn, "task-1", dt("2026-06-15T10:00:00+00:00")).unwrap();

        let result = get_active_entry_for_task(&conn, "task-1").unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn multiple_tasks_can_each_have_their_own_active_session_simultaneously() {
        let conn = setup();

        start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
        start_entry(&conn, "task-2", dt("2026-06-15T09:05:00+00:00")).unwrap();
        start_entry(&conn, "task-3", dt("2026-06-15T09:10:00+00:00")).unwrap();

        let active = list_active_entries(&conn).unwrap();

        assert_eq!(active.len(), 3);
        let task_ids: Vec<&str> = active.iter().map(|e| e.task_id.as_str()).collect();
        assert!(task_ids.contains(&"task-1"));
        assert!(task_ids.contains(&"task-2"));
        assert!(task_ids.contains(&"task-3"));
    }

    #[test]
    fn list_active_entries_excludes_completed_sessions() {
        let conn = setup();
        start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
        end_entry(&conn, "task-1", dt("2026-06-15T10:00:00+00:00")).unwrap();
        start_entry(&conn, "task-2", dt("2026-06-15T11:00:00+00:00")).unwrap();

        let active = list_active_entries(&conn).unwrap();

        assert_eq!(active.len(), 1);
        assert_eq!(active[0].task_id, "task-2");
    }

    #[test]
    fn list_active_entries_returns_empty_when_nothing_is_running() {
        let conn = setup();

        let active = list_active_entries(&conn).unwrap();

        assert!(active.is_empty());
    }

    #[test]
    fn list_entries_for_task_returns_empty_for_a_task_with_zero_entries() {
        let conn = setup();

        let entries = list_entries_for_task(&conn, "never-tracked-task").unwrap();

        assert!(entries.is_empty());
    }

    #[test]
    fn list_entries_for_task_includes_both_active_and_completed_sessions() {
        let conn = setup();
        start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
        end_entry(&conn, "task-1", dt("2026-06-15T10:00:00+00:00")).unwrap();
        start_entry(&conn, "task-1", dt("2026-06-15T11:00:00+00:00")).unwrap();

        let entries = list_entries_for_task(&conn, "task-1").unwrap();

        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn list_entries_for_task_does_not_return_other_tasks_entries() {
        let conn = setup();
        start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
        start_entry(&conn, "task-2", dt("2026-06-15T09:00:00+00:00")).unwrap();

        let entries = list_entries_for_task(&conn, "task-1").unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].task_id, "task-1");
    }

    #[test]
    fn update_heartbeat_sets_last_heartbeat_at_on_the_active_session() {
        let conn = setup();
        start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();

        update_heartbeat(&conn, "task-1", dt("2026-06-15T09:00:30+00:00")).unwrap();

        let active = get_active_entry_for_task(&conn, "task-1").unwrap().unwrap();
        assert_eq!(
            active.last_heartbeat_at,
            Some("2026-06-15T09:00:30+00:00".to_string())
        );
    }

    #[test]
    fn update_heartbeat_is_a_no_op_when_no_active_session_exists() {
        let conn = setup();

        let result = update_heartbeat(&conn, "idle-task", dt("2026-06-15T09:00:30+00:00"));

        assert!(result.is_ok());
    }

    #[test]
    fn update_heartbeat_does_not_affect_other_tasks_active_sessions() {
        let conn = setup();
        start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
        start_entry(&conn, "task-2", dt("2026-06-15T09:00:00+00:00")).unwrap();

        update_heartbeat(&conn, "task-1", dt("2026-06-15T09:00:30+00:00")).unwrap();

        let task_2_active = get_active_entry_for_task(&conn, "task-2").unwrap().unwrap();
        assert_eq!(task_2_active.last_heartbeat_at, None);
    }

    #[test]
    fn update_entry_times_overwrites_started_and_ended_at() {
        let conn = setup();
        let entry = start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
        end_entry(&conn, "task-1", dt("2026-06-15T10:00:00+00:00")).unwrap();

        update_entry_times(
            &conn,
            &entry.id,
            dt("2026-06-15T08:00:00+00:00"),
            Some(dt("2026-06-15T08:30:00+00:00")),
        )
        .unwrap();

        let entries = list_entries_for_task(&conn, "task-1").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].started_at, "2026-06-15T08:00:00+00:00");
        assert_eq!(
            entries[0].ended_at,
            Some("2026-06-15T08:30:00+00:00".to_string())
        );
    }

    #[test]
    fn update_entry_times_can_clear_ended_at_back_to_running() {
        let conn = setup();
        let entry = start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
        end_entry(&conn, "task-1", dt("2026-06-15T10:00:00+00:00")).unwrap();

        update_entry_times(&conn, &entry.id, dt("2026-06-15T09:00:00+00:00"), None).unwrap();

        let active = get_active_entry_for_task(&conn, "task-1").unwrap();
        assert!(active.is_some());
    }

    #[test]
    fn update_entry_times_is_a_no_op_for_an_unknown_entry_id() {
        let conn = setup();

        let result = update_entry_times(
            &conn,
            "does-not-exist",
            dt("2026-06-15T09:00:00+00:00"),
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn delete_entry_removes_the_row() {
        let conn = setup();
        let entry = start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();

        delete_entry(&conn, &entry.id).unwrap();

        let entries = list_entries_for_task(&conn, "task-1").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn delete_entry_is_a_no_op_for_an_unknown_entry_id() {
        let conn = setup();

        let result = delete_entry(&conn, "does-not-exist");

        assert!(result.is_ok());
    }

    #[test]
    fn delete_entry_only_removes_the_targeted_entry() {
        let conn = setup();
        let entry_1 = start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
        end_entry(&conn, "task-1", dt("2026-06-15T09:30:00+00:00")).unwrap();
        let entry_2 = start_entry(&conn, "task-1", dt("2026-06-15T10:00:00+00:00")).unwrap();

        delete_entry(&conn, &entry_1.id).unwrap();

        let entries = list_entries_for_task(&conn, "task-1").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, entry_2.id);
    }

    #[test]
    fn total_tracked_seconds_for_task_is_zero_for_a_task_with_no_entries() {
        let conn = setup();

        let total = total_tracked_seconds_for_task(&conn, "never-tracked-task").unwrap();

        assert_eq!(total, 0);
    }

    #[test]
    fn total_tracked_seconds_for_task_excludes_a_still_running_session() {
        let conn = setup();
        start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();

        let total = total_tracked_seconds_for_task(&conn, "task-1").unwrap();

        assert_eq!(total, 0);
    }

    #[test]
    fn total_tracked_seconds_for_task_sums_only_completed_sessions() {
        let conn = setup();
        start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
        end_entry(&conn, "task-1", dt("2026-06-15T09:10:00+00:00")).unwrap(); // 600s
        start_entry(&conn, "task-1", dt("2026-06-15T10:00:00+00:00")).unwrap();
        end_entry(&conn, "task-1", dt("2026-06-15T10:05:00+00:00")).unwrap(); // 300s
        start_entry(&conn, "task-1", dt("2026-06-15T11:00:00+00:00")).unwrap(); // still running

        let total = total_tracked_seconds_for_task(&conn, "task-1").unwrap();

        assert_eq!(total, 900);
    }

    #[test]
    fn total_tracked_seconds_for_task_does_not_include_other_tasks_sessions() {
        let conn = setup();
        start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
        end_entry(&conn, "task-1", dt("2026-06-15T09:10:00+00:00")).unwrap(); // 600s
        start_entry(&conn, "task-2", dt("2026-06-15T09:00:00+00:00")).unwrap();
        end_entry(&conn, "task-2", dt("2026-06-15T09:30:00+00:00")).unwrap(); // 1800s

        let total = total_tracked_seconds_for_task(&conn, "task-1").unwrap();

        assert_eq!(total, 600);
    }

    #[test]
    fn total_tracked_seconds_for_task_handles_a_session_spanning_midnight() {
        let conn = setup();
        start_entry(&conn, "task-1", dt("2026-06-15T23:30:00+00:00")).unwrap();
        end_entry(&conn, "task-1", dt("2026-06-16T00:15:00+00:00")).unwrap(); // 45 min = 2700s

        let total = total_tracked_seconds_for_task(&conn, "task-1").unwrap();

        assert_eq!(total, 2700);
    }

    #[test]
    fn get_entry_returns_none_for_an_unknown_id() {
        let conn = setup();

        let result = get_entry(&conn, "does-not-exist").unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn get_entry_returns_the_matching_entry() {
        let conn = setup();
        let created = start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();

        let found = get_entry(&conn, &created.id).unwrap();

        assert_eq!(found, Some(created));
    }

    #[test]
    fn get_entry_finds_a_completed_entry_too() {
        let conn = setup();
        let created = start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
        end_entry(&conn, "task-1", dt("2026-06-15T10:00:00+00:00")).unwrap();

        let found = get_entry(&conn, &created.id).unwrap().unwrap();

        assert_eq!(
            found.ended_at,
            Some("2026-06-15T10:00:00+00:00".to_string())
        );
    }

    #[test]
    fn insert_completed_entry_creates_a_row_with_both_timestamps_set() {
        let conn = setup();

        let entry = insert_completed_entry(
            &conn,
            "task-1",
            dt("2026-06-15T09:00:00+00:00"),
            dt("2026-06-15T09:45:00+00:00"),
        )
        .unwrap();

        assert_eq!(entry.task_id, "task-1");
        assert_eq!(entry.started_at, "2026-06-15T09:00:00+00:00");
        assert_eq!(
            entry.ended_at,
            Some("2026-06-15T09:45:00+00:00".to_string())
        );
        assert!(!entry.id.is_empty());
    }

    #[test]
    fn insert_completed_entry_does_not_count_as_an_active_session() {
        let conn = setup();

        insert_completed_entry(
            &conn,
            "task-1",
            dt("2026-06-15T09:00:00+00:00"),
            dt("2026-06-15T09:45:00+00:00"),
        )
        .unwrap();

        let active = get_active_entry_for_task(&conn, "task-1").unwrap();
        assert_eq!(active, None);
    }

    #[test]
    fn insert_completed_entry_is_independent_of_an_existing_active_session() {
        let conn = setup();
        start_entry(&conn, "task-1", dt("2026-06-15T08:00:00+00:00")).unwrap();

        insert_completed_entry(
            &conn,
            "task-1",
            dt("2026-06-14T09:00:00+00:00"),
            dt("2026-06-14T09:30:00+00:00"),
        )
        .unwrap();

        let entries = list_entries_for_task(&conn, "task-1").unwrap();
        assert_eq!(entries.len(), 2);
        assert!(get_active_entry_for_task(&conn, "task-1")
            .unwrap()
            .is_some());
    }

    #[test]
    fn insert_completed_entry_contributes_to_total_tracked_seconds() {
        let conn = setup();

        insert_completed_entry(
            &conn,
            "task-1",
            dt("2026-06-15T09:00:00+00:00"),
            dt("2026-06-15T09:45:00+00:00"),
        )
        .unwrap();

        let total = total_tracked_seconds_for_task(&conn, "task-1").unwrap();
        assert_eq!(total, 2700);
    }

    mod tracked_seconds_per_day_tests {
        use super::*;

        fn day(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
            chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
        }

        fn window(start: &str, end: &str) -> (DateTime<Utc>, DateTime<Utc>) {
            (dt(start), dt(end))
        }

        #[test]
        fn empty_task_ids_returns_an_empty_map() {
            let conn = setup();
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-08T00:00:00+00:00");

            let buckets =
                tracked_seconds_per_day(&conn, &[], start, end, dt("2026-06-15T00:00:00+00:00"))
                    .unwrap();

            assert!(buckets.is_empty());
        }

        #[test]
        fn empty_window_returns_an_empty_map() {
            let conn = setup();
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-06-15T09:00:00+00:00"),
                dt("2026-06-15T10:00:00+00:00"),
            )
            .unwrap();
            let task_ids = vec!["task-1".to_string()];
            // range_start == range_end: an empty half-open window.
            let now = dt("2026-06-20T00:00:00+00:00");

            let buckets = tracked_seconds_per_day(
                &conn,
                &task_ids,
                dt("2026-06-15T00:00:00+00:00"),
                dt("2026-06-15T00:00:00+00:00"),
                now,
            )
            .unwrap();

            assert!(buckets.is_empty());
        }

        #[test]
        fn an_inverted_window_returns_an_empty_map() {
            let conn = setup();
            let task_ids = vec!["task-1".to_string()];
            let now = dt("2026-06-20T00:00:00+00:00");

            let buckets = tracked_seconds_per_day(
                &conn,
                &task_ids,
                dt("2026-06-20T00:00:00+00:00"),
                dt("2026-06-10T00:00:00+00:00"),
                now,
            )
            .unwrap();

            assert!(buckets.is_empty());
        }

        #[test]
        fn a_task_with_no_entries_contributes_nothing() {
            let conn = setup();
            let task_ids = vec!["never-tracked".to_string()];
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-08T00:00:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let buckets = tracked_seconds_per_day(&conn, &task_ids, start, end, now).unwrap();

            assert!(buckets.is_empty());
        }

        #[test]
        fn a_session_entirely_within_one_day_buckets_into_that_day_only() {
            let conn = setup();
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-06-15T09:00:00+00:00"),
                dt("2026-06-15T10:00:00+00:00"), // 3600s
            )
            .unwrap();
            let task_ids = vec!["task-1".to_string()];
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-30T00:00:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let buckets = tracked_seconds_per_day(&conn, &task_ids, start, end, now).unwrap();

            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets.get(&day(2026, 6, 15)), Some(&3600));
        }

        #[test]
        fn a_session_spanning_midnight_splits_proportionally_across_both_days() {
            let conn = setup();
            // 23:30 -> 00:15 next day: 30 min in day 1, 15 min in day 2.
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-06-15T23:30:00+00:00"),
                dt("2026-06-16T00:15:00+00:00"),
            )
            .unwrap();
            let task_ids = vec!["task-1".to_string()];
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-30T00:00:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let buckets = tracked_seconds_per_day(&conn, &task_ids, start, end, now).unwrap();

            assert_eq!(buckets.get(&day(2026, 6, 15)), Some(&1800));
            assert_eq!(buckets.get(&day(2026, 6, 16)), Some(&900));
            assert_eq!(buckets.len(), 2);
        }

        #[test]
        fn a_session_spanning_multiple_days_splits_across_every_day_it_touches() {
            let conn = setup();
            // 2026-06-10 12:00 -> 2026-06-13 06:00: 4 calendar days touched.
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-06-10T12:00:00+00:00"),
                dt("2026-06-13T06:00:00+00:00"),
            )
            .unwrap();
            let task_ids = vec!["task-1".to_string()];
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-30T00:00:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let buckets = tracked_seconds_per_day(&conn, &task_ids, start, end, now).unwrap();

            // Day 1 (06-10): 12:00 -> 24:00 = 12h = 43200s
            assert_eq!(buckets.get(&day(2026, 6, 10)), Some(&43200));
            // Day 2 (06-11): full day = 86400s
            assert_eq!(buckets.get(&day(2026, 6, 11)), Some(&86400));
            // Day 3 (06-12): full day = 86400s
            assert_eq!(buckets.get(&day(2026, 6, 12)), Some(&86400));
            // Day 4 (06-13): 00:00 -> 06:00 = 6h = 21600s
            assert_eq!(buckets.get(&day(2026, 6, 13)), Some(&21600));
            assert_eq!(buckets.len(), 4);

            let total: i64 = buckets.values().sum();
            assert_eq!(total, (3 * 86400) - (12 * 3600) + (6 * 3600)); // sanity cross-check
        }

        #[test]
        fn a_still_running_session_is_clipped_against_now() {
            let conn = setup();
            // Started 09:00, still running; "now" is 09:30 on the same day.
            time_storage_start_then_no_end(&conn, "task-1", "2026-06-15T09:00:00+00:00");
            let task_ids = vec!["task-1".to_string()];
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-30T00:00:00+00:00");
            let now = dt("2026-06-15T09:30:00+00:00");

            let buckets = tracked_seconds_per_day(&conn, &task_ids, start, end, now).unwrap();

            assert_eq!(buckets.get(&day(2026, 6, 15)), Some(&1800));
        }

        #[test]
        fn a_still_running_session_clipped_against_now_can_span_into_a_later_day() {
            let conn = setup();
            time_storage_start_then_no_end(&conn, "task-1", "2026-06-15T23:00:00+00:00");
            let task_ids = vec!["task-1".to_string()];
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-30T00:00:00+00:00");
            let now = dt("2026-06-16T01:00:00+00:00");

            let buckets = tracked_seconds_per_day(&conn, &task_ids, start, end, now).unwrap();

            assert_eq!(buckets.get(&day(2026, 6, 15)), Some(&3600));
            assert_eq!(buckets.get(&day(2026, 6, 16)), Some(&3600));
        }

        #[test]
        fn a_session_starting_before_and_ending_after_the_query_window_is_clipped_to_it() {
            let conn = setup();
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-05-01T00:00:00+00:00"),
                dt("2026-07-01T00:00:00+00:00"),
            )
            .unwrap();
            let task_ids = vec!["task-1".to_string()];
            // Window is just one day: 06-15 00:00 -> 06-16 00:00.
            let (start, end) = window("2026-06-15T00:00:00+00:00", "2026-06-16T00:00:00+00:00");
            let now = dt("2026-08-01T00:00:00+00:00");

            let buckets = tracked_seconds_per_day(&conn, &task_ids, start, end, now).unwrap();

            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets.get(&day(2026, 6, 15)), Some(&86400));
        }

        #[test]
        fn a_session_entirely_before_the_window_contributes_nothing() {
            let conn = setup();
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-01-01T00:00:00+00:00"),
                dt("2026-01-02T00:00:00+00:00"),
            )
            .unwrap();
            let task_ids = vec!["task-1".to_string()];
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-30T00:00:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let buckets = tracked_seconds_per_day(&conn, &task_ids, start, end, now).unwrap();

            assert!(buckets.is_empty());
        }

        #[test]
        fn a_session_entirely_after_the_window_contributes_nothing() {
            let conn = setup();
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-12-01T00:00:00+00:00"),
                dt("2026-12-02T00:00:00+00:00"),
            )
            .unwrap();
            let task_ids = vec!["task-1".to_string()];
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-30T00:00:00+00:00");
            let now = dt("2026-12-20T00:00:00+00:00");

            let buckets = tracked_seconds_per_day(&conn, &task_ids, start, end, now).unwrap();

            assert!(buckets.is_empty());
        }

        #[test]
        fn multiple_tasks_entries_are_interleaved_and_summed_per_day() {
            let conn = setup();
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-06-15T09:00:00+00:00"),
                dt("2026-06-15T10:00:00+00:00"), // 3600s on 06-15
            )
            .unwrap();
            insert_completed_entry(
                &conn,
                "task-2",
                dt("2026-06-15T11:00:00+00:00"),
                dt("2026-06-15T11:30:00+00:00"), // 1800s on 06-15
            )
            .unwrap();
            insert_completed_entry(
                &conn,
                "task-2",
                dt("2026-06-16T08:00:00+00:00"),
                dt("2026-06-16T08:10:00+00:00"), // 600s on 06-16
            )
            .unwrap();
            let task_ids = vec!["task-1".to_string(), "task-2".to_string()];
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-30T00:00:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let buckets = tracked_seconds_per_day(&conn, &task_ids, start, end, now).unwrap();

            assert_eq!(buckets.get(&day(2026, 6, 15)), Some(&5400));
            assert_eq!(buckets.get(&day(2026, 6, 16)), Some(&600));
            assert_eq!(buckets.len(), 2);
        }

        #[test]
        fn a_task_id_not_in_the_requested_list_is_excluded() {
            let conn = setup();
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-06-15T09:00:00+00:00"),
                dt("2026-06-15T10:00:00+00:00"),
            )
            .unwrap();
            insert_completed_entry(
                &conn,
                "task-not-requested",
                dt("2026-06-15T09:00:00+00:00"),
                dt("2026-06-15T20:00:00+00:00"),
            )
            .unwrap();
            let task_ids = vec!["task-1".to_string()];
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-30T00:00:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let buckets = tracked_seconds_per_day(&conn, &task_ids, start, end, now).unwrap();

            assert_eq!(buckets.get(&day(2026, 6, 15)), Some(&3600));
        }

        #[test]
        fn a_zero_length_session_contributes_nothing() {
            let conn = setup();
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-06-15T09:00:00+00:00"),
                dt("2026-06-15T09:00:00+00:00"),
            )
            .unwrap();
            let task_ids = vec!["task-1".to_string()];
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-30T00:00:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let buckets = tracked_seconds_per_day(&conn, &task_ids, start, end, now).unwrap();

            assert!(buckets.is_empty());
        }

        /// Test-only helper: starts an active (never-ended) session directly
        /// via [`start_entry`], named to read clearly at each call site above
        /// without repeating the "still running" framing every time.
        fn time_storage_start_then_no_end(conn: &Connection, task_id: &str, started_at: &str) {
            start_entry(conn, task_id, dt(started_at)).unwrap();
        }
    }

    mod total_tracked_seconds_for_task_ids_in_range_tests {
        use super::*;

        fn window(start: &str, end: &str) -> (DateTime<Utc>, DateTime<Utc>) {
            (dt(start), dt(end))
        }

        #[test]
        fn returns_zero_for_empty_task_ids() {
            let conn = setup();
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-06-15T09:00:00+00:00"),
                dt("2026-06-15T10:00:00+00:00"),
            )
            .unwrap();
            let (start, end) = window("2026-06-15T00:00:00+00:00", "2026-06-16T00:00:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let total =
                total_tracked_seconds_for_task_ids_in_range(&conn, &[], start, end, now).unwrap();

            assert_eq!(total, 0);
        }

        #[test]
        fn returns_zero_for_empty_window() {
            let conn = setup();
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-06-15T09:00:00+00:00"),
                dt("2026-06-15T10:00:00+00:00"),
            )
            .unwrap();
            let task_ids = vec!["task-1".to_string()];
            let ts = dt("2026-06-15T12:00:00+00:00");

            let total =
                total_tracked_seconds_for_task_ids_in_range(&conn, &task_ids, ts, ts, ts).unwrap();

            assert_eq!(total, 0);
        }

        #[test]
        fn sums_entries_for_requested_task_ids_only() {
            let conn = setup();
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-06-15T09:00:00+00:00"),
                dt("2026-06-15T09:30:00+00:00"), // 1800s
            )
            .unwrap();
            insert_completed_entry(
                &conn,
                "task-2",
                dt("2026-06-15T10:00:00+00:00"),
                dt("2026-06-15T11:00:00+00:00"), // 3600s — excluded
            )
            .unwrap();
            let task_ids = vec!["task-1".to_string()];
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-30T00:00:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let total =
                total_tracked_seconds_for_task_ids_in_range(&conn, &task_ids, start, end, now)
                    .unwrap();

            assert_eq!(total, 1800);
        }

        #[test]
        fn clips_entries_to_the_query_window() {
            let conn = setup();
            // Entry spans three days but window is just one hour.
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-06-14T00:00:00+00:00"),
                dt("2026-06-16T00:00:00+00:00"),
            )
            .unwrap();
            let task_ids = vec!["task-1".to_string()];
            let (start, end) = window("2026-06-15T10:00:00+00:00", "2026-06-15T11:00:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let total =
                total_tracked_seconds_for_task_ids_in_range(&conn, &task_ids, start, end, now)
                    .unwrap();

            assert_eq!(total, 3600);
        }
    }

    mod list_time_entries_in_range_tests {
        use super::*;

        fn window(start: &str, end: &str) -> (DateTime<Utc>, DateTime<Utc>) {
            (dt(start), dt(end))
        }

        #[test]
        fn returns_empty_for_inverted_window() {
            let conn = setup();
            let ts = dt("2026-06-15T12:00:00+00:00");

            let entries = list_time_entries_in_range(&conn, ts, ts, ts).unwrap();

            assert!(entries.is_empty());
        }

        #[test]
        fn returns_empty_when_no_entries_exist() {
            let conn = setup();
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-30T00:00:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let entries = list_time_entries_in_range(&conn, start, end, now).unwrap();

            assert!(entries.is_empty());
        }

        #[test]
        fn returns_entry_clamped_to_window() {
            let conn = setup();
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-06-15T09:00:00+00:00"),
                dt("2026-06-15T10:00:00+00:00"),
            )
            .unwrap();
            let (start, end) = window("2026-06-15T09:30:00+00:00", "2026-06-15T10:30:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let entries = list_time_entries_in_range(&conn, start, end, now).unwrap();

            assert_eq!(entries.len(), 1);
            let (task_id, clipped_start, clipped_end) = &entries[0];
            assert_eq!(task_id, "task-1");
            assert_eq!(*clipped_start, dt("2026-06-15T09:30:00+00:00"));
            assert_eq!(*clipped_end, dt("2026-06-15T10:00:00+00:00"));
        }

        #[test]
        fn excludes_entries_entirely_outside_window() {
            let conn = setup();
            insert_completed_entry(
                &conn,
                "task-1",
                dt("2026-01-01T09:00:00+00:00"),
                dt("2026-01-01T10:00:00+00:00"),
            )
            .unwrap();
            let (start, end) = window("2026-06-01T00:00:00+00:00", "2026-06-30T00:00:00+00:00");
            let now = dt("2026-06-20T00:00:00+00:00");

            let entries = list_time_entries_in_range(&conn, start, end, now).unwrap();

            assert!(entries.is_empty());
        }

        #[test]
        fn clamps_running_entry_to_now() {
            let conn = setup();
            start_entry(&conn, "task-1", dt("2026-06-15T09:00:00+00:00")).unwrap();
            let (start, end) = window("2026-06-15T00:00:00+00:00", "2026-06-16T00:00:00+00:00");
            let now = dt("2026-06-15T09:30:00+00:00");

            let entries = list_time_entries_in_range(&conn, start, end, now).unwrap();

            assert_eq!(entries.len(), 1);
            let (_, clipped_start, clipped_end) = &entries[0];
            assert_eq!(*clipped_start, dt("2026-06-15T09:00:00+00:00"));
            assert_eq!(*clipped_end, now);
        }
    }
}
