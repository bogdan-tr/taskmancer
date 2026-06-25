use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::settings::{StatusTierRule, TaskDefaults};

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
///
/// `ink_mode` overrides `Settings::ink_mode` for this project's color-coded
/// card/bar text when set (one of [`crate::settings::INK_MODES`]); `None`
/// inherits the global default.
///
/// `show_subproject_tasks` overrides `Settings::show_subproject_tasks_default`
/// for whether viewing *this* project's board/week/calendar rolls up its
/// descendant subprojects' tasks too; `None` inherits the global default
/// (itself defaulting to `false` — rollup is opt-in, not automatic, per the
/// Subtasks-feedback round that introduced this field).
///
/// `status_tier_rule_overrides` overrides individual slots of
/// `Settings::default_status_tier_rules` for this project's status-line
/// health badge (see `crate::status_tier`) when set. When `Some`, it always
/// has exactly 4 entries aligned to the same `[severe, critical,
/// needs_attention, on_track]` order as the global list — but each slot is
/// independently `Option<StatusTierRule>`: a `None` slot inherits *that one*
/// tier from the global default rather than requiring the whole override
/// list to be all-or-nothing. `None` (the whole field, not a slot) inherits
/// every tier from the global default.
///
/// `status_line_layout_id` overrides `Settings::default_status_line_layout_id`
/// for which `StatLayout` (see `crate::layout`) this project's status line
/// renders when set; `None` inherits the global default.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ink_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_subproject_tasks: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_tier_rule_overrides: Option<Vec<Option<StatusTierRule>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_line_layout_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String,
    /// The id of this project's parent, or `None` for a top-level project.
    /// Nesting is arbitrary depth — a project named here may itself have a
    /// non-`None` `parent_id`. See `crate::project_tree` for helpers that
    /// walk this relationship.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
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
    /// The id of this project's lazily-created hidden tracker `Task` (see
    /// `Task::hidden`), used to "track this project as a whole" rather than
    /// any single task within it. `None` until the project's own play
    /// button is pressed for the first time (see
    /// `commands::get_or_create_project_tracking_task`, Milestone 2), and
    /// otherwise only ever set once — this directly mirrors
    /// `Task::subtask_project_id`, just inverted (a project growing a hidden
    /// task instead of a task growing a hidden project).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracking_task_id: Option<String>,
}

impl Project {
    /// Creates a new top-level project with a freshly generated id and the
    /// current time as `created`. Callers creating a subproject set
    /// `parent_id` on the returned value directly (it's a public field).
    pub fn new(name: String, color: String, order: i64) -> Self {
        Project {
            id: Uuid::new_v4().to_string(),
            name,
            color,
            parent_id: None,
            order,
            created: Utc::now().to_rfc3339(),
            board: ProjectBoard::default(),
            defaults: TaskDefaults::default(),
            tracking_task_id: None,
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

    #[test]
    fn new_project_has_no_ink_mode_override() {
        let project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);

        assert_eq!(project.board.ink_mode, None);
    }

    #[test]
    fn board_ink_mode_is_none_when_absent_from_json() {
        let json = r##"{"id":"abc","name":"Inbox","color":"#000000","created":"2026-06-11T10:00:00+00:00"}"##;

        let project: Project = serde_json::from_str(json).expect("parsing should succeed");

        assert_eq!(project.board.ink_mode, None);
    }

    #[test]
    fn new_project_has_no_parent() {
        let project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);

        assert_eq!(project.parent_id, None);
    }

    #[test]
    fn parent_id_is_none_when_absent_from_json() {
        let json = r##"{"id":"abc","name":"Inbox","color":"#000000","created":"2026-06-11T10:00:00+00:00"}"##;

        let project: Project = serde_json::from_str(json).expect("parsing should succeed");

        assert_eq!(project.parent_id, None);
    }

    #[test]
    fn parent_id_round_trips_when_set() {
        let mut project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);
        project.parent_id = Some("parent-123".to_string());

        let json = serde_json::to_string(&project).expect("serialization should succeed");
        let parsed: Project = serde_json::from_str(&json).expect("parsing should succeed");

        assert_eq!(parsed.parent_id, Some("parent-123".to_string()));
    }

    #[test]
    fn new_project_has_no_tracking_task() {
        let project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);

        assert_eq!(project.tracking_task_id, None);
    }

    #[test]
    fn tracking_task_id_is_none_when_absent_from_json() {
        let json = r##"{"id":"abc","name":"Inbox","color":"#000000","created":"2026-06-11T10:00:00+00:00"}"##;

        let project: Project = serde_json::from_str(json).expect("parsing should succeed");

        assert_eq!(project.tracking_task_id, None);
    }

    #[test]
    fn tracking_task_id_round_trips_when_set() {
        let mut project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);
        project.tracking_task_id = Some("tracker-task-123".to_string());

        let json = serde_json::to_string(&project).expect("serialization should succeed");
        let parsed: Project = serde_json::from_str(&json).expect("parsing should succeed");

        assert_eq!(
            parsed.tracking_task_id,
            Some("tracker-task-123".to_string())
        );
    }

    #[test]
    fn new_project_has_no_status_tier_rule_overrides_or_layout_override() {
        let project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);

        assert_eq!(project.board.status_tier_rule_overrides, None);
        assert_eq!(project.board.status_line_layout_id, None);
    }

    #[test]
    fn status_tier_rule_overrides_is_none_when_absent_from_json() {
        let json = r##"{"id":"abc","name":"Inbox","color":"#000000","created":"2026-06-11T10:00:00+00:00"}"##;

        let project: Project = serde_json::from_str(json).expect("parsing should succeed");

        assert_eq!(project.board.status_tier_rule_overrides, None);
        assert_eq!(project.board.status_line_layout_id, None);
    }

    #[test]
    fn status_tier_rule_overrides_round_trips_with_a_mix_of_set_and_inherited_slots() {
        let mut project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);
        project.board.status_tier_rule_overrides = Some(vec![
            Some(StatusTierRule {
                due_within_days: Some(0),
                min_priority: None,
                estimated_time_left_exceeds_minutes: None,
            }),
            None,
            None,
            Some(StatusTierRule {
                due_within_days: Some(14),
                min_priority: None,
                estimated_time_left_exceeds_minutes: None,
            }),
        ]);

        let json = serde_json::to_string(&project).expect("serialization should succeed");
        let parsed: Project = serde_json::from_str(&json).expect("parsing should succeed");

        assert_eq!(
            parsed.board.status_tier_rule_overrides,
            project.board.status_tier_rule_overrides
        );
    }

    #[test]
    fn status_line_layout_id_round_trips_when_set() {
        let mut project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);
        project.board.status_line_layout_id = Some("layout-123".to_string());

        let json = serde_json::to_string(&project).expect("serialization should succeed");
        let parsed: Project = serde_json::from_str(&json).expect("parsing should succeed");

        assert_eq!(
            parsed.board.status_line_layout_id,
            Some("layout-123".to_string())
        );
    }
}
