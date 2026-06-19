use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::settings::TaskDefaults;

/// Accent color assigned to newly created or backfilled projects that don't
/// specify their own. Matches `--color-accent` in `tokens.css`.
pub const DEFAULT_PROJECT_COLOR: &str = "#3b82f6";

/// A project's Kanban board configuration: the subset and order of the
/// global status list (see `settings::Settings::statuses`) shown on this
/// project's board, and the status new tasks in this project get by
/// default. An empty `statuses` list means the project hasn't customized
/// its board and shows the global status list as-is; a `None`
/// `default_status` falls back to the global `Settings::defaults.status`.
///
/// `show_previous_weeks` overrides `Settings::show_previous_weeks_column`
/// for this project's Week view when set; `None` inherits the global
/// default.
///
/// `card_lightness`/`bar_lightness` override
/// `Settings::card_lightness`/`.bar_lightness` for this project's Kanban
/// cards and week/calendar-view bars, respectively, when set — each `None`
/// independently inherits its own global default. Must be a valid OKLCH
/// lightness (`0.0..=1.0`, see `settings::validate_lightness`) when set.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProjectBoard {
    #[serde(default)]
    pub statuses: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_previous_weeks: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_lightness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_lightness: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub order: i64,
    pub created: String,
    /// This project's Kanban board configuration. Defaults to an empty
    /// (uncustomized) board for projects created or persisted before this
    /// field existed.
    #[serde(default)]
    pub board: ProjectBoard,
    /// Per-project overrides of the global default task attributes. Any
    /// field left unset falls back to `settings::Settings::defaults`.
    #[serde(default)]
    pub defaults: TaskDefaults,
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
            board: ProjectBoard::default(),
            defaults: TaskDefaults::default(),
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

    #[test]
    fn board_and_defaults_are_empty_when_absent_from_json() {
        let json = r##"{"id":"abc","name":"Inbox","color":"#000000","created":"2026-06-11T10:00:00+00:00"}"##;

        let project: Project = serde_json::from_str(json).expect("parsing should succeed");

        assert_eq!(project.board, ProjectBoard::default());
        assert_eq!(project.defaults, TaskDefaults::default());
    }

    #[test]
    fn new_project_has_empty_board_and_defaults() {
        let project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);

        assert!(project.board.statuses.is_empty());
        assert_eq!(project.board.default_status, None);
        assert_eq!(project.defaults, TaskDefaults::default());
    }
}
