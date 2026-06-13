use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Accent color assigned to newly created or backfilled projects that don't
/// specify their own. Matches `--color-accent` in `tokens.css`.
pub const DEFAULT_PROJECT_COLOR: &str = "#3b82f6";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub order: i64,
    pub created: String,
}

impl Project {
    /// Creates a new project with a freshly generated id and the current
    /// time as `created`.
    pub fn new(name: String, color: String, order: i64) -> Self {
        Project {
            id: Uuid::new_v4().to_string(),
            name,
            color,
            order,
            created: Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_has_generated_id_and_created_timestamp() {
        let project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);

        assert_eq!(project.name, "Homework");
        assert_eq!(project.color, "#ff0000");
        assert_eq!(project.order, 1);
        assert!(!project.id.is_empty());
        assert!(!project.created.is_empty());
    }

    #[test]
    fn to_json_then_from_json_round_trips() {
        let project = Project::new("Homework".to_string(), "#ff0000".to_string(), 5);

        let json = serde_json::to_string(&project).expect("serialization should succeed");
        let parsed: Project = serde_json::from_str(&json).expect("parsing should succeed");

        assert_eq!(parsed, project);
    }

    #[test]
    fn order_defaults_to_zero_when_absent_from_json() {
        let json = r##"{"id":"abc","name":"Inbox","color":"#000000","created":"2026-06-11T10:00:00+00:00"}"##;

        let project: Project = serde_json::from_str(json).expect("parsing should succeed");

        assert_eq!(project.order, 0);
    }
}
