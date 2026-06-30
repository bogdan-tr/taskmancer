use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user-saved filter view. `filter_config` and `sort_config` are opaque
/// JSON blobs — Rust stores and retrieves them as strings; the frontend
/// owns the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedView {
    pub id: String,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub filter_config: String,
    pub sort_config: String,
    pub display_order: i64,
    pub created_at: String,
}

pub fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS saved_views (
            id           TEXT PRIMARY KEY,
            name         TEXT NOT NULL,
            color        TEXT NOT NULL DEFAULT '#3b82f6',
            icon         TEXT NOT NULL DEFAULT 'star',
            filter_config TEXT NOT NULL DEFAULT '{}',
            sort_config  TEXT NOT NULL DEFAULT '{\"levels\":[]}',
            display_order INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL
        );",
    )
}

pub fn list_views(conn: &Connection) -> rusqlite::Result<Vec<SavedView>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, icon, filter_config, sort_config, display_order, created_at
         FROM saved_views ORDER BY display_order ASC, created_at ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(SavedView {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            icon: row.get(3)?,
            filter_config: row.get(4)?,
            sort_config: row.get(5)?,
            display_order: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn create_view(
    conn: &Connection,
    name: &str,
    color: &str,
    icon: &str,
    filter_config: &str,
    sort_config: &str,
) -> rusqlite::Result<SavedView> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let display_order = next_display_order(conn)?;
    conn.execute(
        "INSERT INTO saved_views
             (id, name, color, icon, filter_config, sort_config, display_order, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, name, color, icon, filter_config, sort_config, display_order, now],
    )?;
    Ok(SavedView {
        id,
        name: name.to_string(),
        color: color.to_string(),
        icon: icon.to_string(),
        filter_config: filter_config.to_string(),
        sort_config: sort_config.to_string(),
        display_order,
        created_at: now,
    })
}

pub fn update_view(
    conn: &Connection,
    id: &str,
    name: &str,
    color: &str,
    icon: &str,
    filter_config: &str,
    sort_config: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE saved_views
         SET name = ?2, color = ?3, icon = ?4, filter_config = ?5, sort_config = ?6
         WHERE id = ?1",
        params![id, name, color, icon, filter_config, sort_config],
    )?;
    Ok(())
}

pub fn delete_view(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM saved_views WHERE id = ?1", params![id])?;
    Ok(())
}

/// Sets `display_order` for each view id in `ids` using its 0-based position.
pub fn reorder_views(conn: &Connection, ids: &[String]) -> rusqlite::Result<()> {
    for (i, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE saved_views SET display_order = ?2 WHERE id = ?1",
            params![id, i as i64],
        )?;
    }
    Ok(())
}

fn next_display_order(conn: &Connection) -> rusqlite::Result<i64> {
    let max: Option<i64> = conn
        .query_row("SELECT MAX(display_order) FROM saved_views", [], |row| {
            row.get(0)
        })
        .optional()?;
    Ok(max.unwrap_or(-1) + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn open_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn create_and_list_view() {
        let conn = open_db();
        let v = create_view(&conn, "My View", "#ff0000", "star", "{}", r#"{"levels":[]}"#).unwrap();
        assert_eq!(v.name, "My View");
        assert_eq!(v.color, "#ff0000");

        let views = list_views(&conn).unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].id, v.id);
    }

    #[test]
    fn update_view_fields() {
        let conn = open_db();
        let v = create_view(&conn, "Old", "#aaa", "star", "{}", r#"{"levels":[]}"#).unwrap();
        update_view(&conn, &v.id, "New", "#bbb", "filter", r#"{"text":"x"}"#, r#"{"levels":[]}"#)
            .unwrap();
        let views = list_views(&conn).unwrap();
        assert_eq!(views[0].name, "New");
        assert_eq!(views[0].filter_config, r#"{"text":"x"}"#);
    }

    #[test]
    fn delete_view_removes_row() {
        let conn = open_db();
        let v = create_view(&conn, "A", "#aaa", "star", "{}", r#"{"levels":[]}"#).unwrap();
        delete_view(&conn, &v.id).unwrap();
        assert!(list_views(&conn).unwrap().is_empty());
    }

    #[test]
    fn reorder_views_updates_order() {
        let conn = open_db();
        let v1 = create_view(&conn, "A", "#aaa", "star", "{}", r#"{"levels":[]}"#).unwrap();
        let v2 = create_view(&conn, "B", "#bbb", "star", "{}", r#"{"levels":[]}"#).unwrap();
        // Swap order: v2 first, v1 second
        reorder_views(&conn, &[v2.id.clone(), v1.id.clone()]).unwrap();
        let views = list_views(&conn).unwrap();
        assert_eq!(views[0].id, v2.id);
        assert_eq!(views[1].id, v1.id);
    }

    #[test]
    fn create_increments_display_order() {
        let conn = open_db();
        let v1 = create_view(&conn, "A", "#aaa", "star", "{}", r#"{"levels":[]}"#).unwrap();
        let v2 = create_view(&conn, "B", "#bbb", "star", "{}", r#"{"levels":[]}"#).unwrap();
        assert!(v2.display_order > v1.display_order);
    }
}
