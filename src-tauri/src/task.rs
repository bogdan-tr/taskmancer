use chrono::Utc;
use gray_matter::engine::YAML;
use gray_matter::Matter;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The id of a `PriorityLevel` (see `crate::settings`) when no other value
/// is specified. Matches the `id` of the "medium" entry in
/// `Settings::default()`'s seeded priority list.
fn default_priority() -> String {
    "medium".to_string()
}

/// The id of a `StatusDefinition` (see `crate::settings`) when no other
/// value is specified. Matches the `id` of the "backlog" entry in
/// `Settings::default()`'s seeded status list.
fn default_status() -> String {
    "backlog".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub title: String,
    /// The id of a user-defined `StatusDefinition` (see `crate::settings`).
    /// Validated against the current `Settings.statuses` by the command
    /// layer on write; read paths accept any string so tasks referencing a
    /// since-removed status remain visible.
    #[serde(default = "default_status")]
    pub status: String,
    /// The id of the `Project` this task belongs to. `None` only
    /// transiently — every task gets a concrete project id at creation
    /// time (see `commands::resolve_project_id`), falling back to
    /// `Settings.default_project_id` when none is given explicitly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// The id of a user-defined `PriorityLevel` (see `crate::settings`).
    /// Validated against the current `Settings.priorities` by the command
    /// layer on write; read paths accept any string so tasks referencing a
    /// since-removed priority level remain visible.
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled: Option<String>,
    #[serde(default)]
    pub order: i64,
    pub created: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// User-editable estimate of how long this task will take, in minutes.
    /// `None` means no estimate has been set. Settable via `create_task`/
    /// `update_task`, mirroring `priority`/`due`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
    /// Total time tracked against this task so far, in minutes. Always
    /// present (defaults to `0`, not `None`, since "no time tracked yet" is
    /// itself a meaningful value to display) and not settable through
    /// `update_task` — only the (future) time-tracking infrastructure writes
    /// this, mirroring how `id`/`created`/`depends_on` are preserved from
    /// disk rather than taken from the request payload.
    #[serde(default)]
    pub tracked_minutes: u32,
    /// The id of the `Series` (see `crate::series`) this task was generated
    /// from, if any. `None` for a normal, non-recurring task. Set once at
    /// generation time and otherwise only cleared (never reassigned) — see
    /// `apply_task_update`'s "just this one" handling in the command layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series_id: Option<String>,
    /// The id of this task's auto-generated "subtask container" `Project`,
    /// if it has ever had a subtask (see
    /// `commands::get_or_create_subtask_container`). `None` until the
    /// first subtask is created, and reset back to `None` when the
    /// container becomes empty and is cleaned up (see
    /// `commands::owning_task_if_container_now_empty`). The container
    /// itself stores no back-pointer to this task — callers needing the
    /// reverse direction (a project id -> its owning task) do a reverse
    /// scan over the task list instead, since there's no other place a
    /// subtask container's identity is recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtask_project_id: Option<String>,
    #[serde(skip)]
    pub notes: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("frontmatter is missing or not valid YAML")]
    MissingFrontmatter,
    /// `gray_matter`'s `Pod::deserialize` surfaces `serde_json::Error` even
    /// though the source frontmatter is YAML, since `Pod` is deserialized
    /// via an internal JSON-shaped representation.
    #[error("failed to parse task frontmatter: {0}")]
    InvalidFrontmatter(#[from] serde_json::Error),
    #[error("failed to serialize task frontmatter: {0}")]
    Serialize(serde_yaml::Error),
}

impl Task {
    /// Creates a new task with a freshly generated id, the given title,
    /// and default status/priority/created timestamp.
    pub fn new(title: String) -> Self {
        Task {
            id: Uuid::new_v4().to_string(),
            title,
            status: default_status(),
            project_id: None,
            tags: Vec::new(),
            priority: default_priority(),
            due: None,
            scheduled: None,
            order: Utc::now().timestamp_millis(),
            created: Utc::now().to_rfc3339(),
            depends_on: Vec::new(),
            estimated_minutes: None,
            tracked_minutes: 0,
            series_id: None,
            subtask_project_id: None,
            notes: String::new(),
        }
    }

    /// Serializes the task to a markdown string with a YAML frontmatter
    /// block followed by the task's free-form notes.
    pub fn to_markdown(&self) -> Result<String, TaskError> {
        let frontmatter = serde_yaml::to_string(self).map_err(TaskError::Serialize)?;
        Ok(format!("---\n{frontmatter}---\n\n{}", self.notes))
    }

    /// Parses a markdown string with a YAML frontmatter block into a Task.
    /// The remaining markdown body becomes the task's notes.
    pub fn from_markdown(content: &str) -> Result<Self, TaskError> {
        let matter = Matter::<YAML>::new();
        let parsed = matter.parse(content);
        let data = parsed.data.ok_or(TaskError::MissingFrontmatter)?;
        let mut task: Task = data.deserialize()?;
        task.notes = parsed.content.trim().to_string();
        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_has_sensible_defaults() {
        let task = Task::new("Write report".to_string());

        assert_eq!(task.title, "Write report");
        assert_eq!(task.status, "backlog");
        assert_eq!(task.priority, "medium");
        assert!(task.project_id.is_none());
        assert!(task.tags.is_empty());
        assert!(!task.id.is_empty());
        assert!(!task.created.is_empty());
        assert!(task.order > 0);
    }

    #[test]
    fn to_markdown_then_from_markdown_round_trips() {
        let mut task = Task::new("Assignment 4".to_string());
        task.status = "in-progress".to_string();
        task.project_id = Some("project-id-cs101-homework".to_string());
        task.tags = vec!["reading".to_string(), "urgent".to_string()];
        task.priority = "high".to_string();
        task.due = Some("2026-06-15".to_string());
        task.scheduled = Some("2026-06-10".to_string());
        task.order = 1234567890;
        task.depends_on = vec!["other-task-id".to_string()];
        task.estimated_minutes = Some(90);
        task.tracked_minutes = 45;
        task.series_id = Some("series-abc123".to_string());
        task.subtask_project_id = Some("container-project-id".to_string());
        task.notes = "Some free-form notes about the assignment.".to_string();

        let markdown = task.to_markdown().expect("serialization should succeed");
        let parsed = Task::from_markdown(&markdown).expect("parsing should succeed");

        assert_eq!(parsed, task);
    }

    #[test]
    fn from_markdown_parses_status_as_a_plain_string() {
        let markdown = "---\nid: abc123\ntitle: Demo\nstatus: in-progress\ncreated: 2026-06-11T10:00:00+00:00\n---\n\nBody text.";

        let task = Task::from_markdown(markdown).expect("parsing should succeed");

        assert_eq!(task.status, "in-progress");
        assert_eq!(task.notes, "Body text.");
    }

    #[test]
    fn from_markdown_applies_defaults_for_missing_optional_fields() {
        let markdown = "---\nid: abc123\ntitle: Demo\ncreated: 2026-06-11T10:00:00+00:00\n---\n\n";

        let task = Task::from_markdown(markdown).expect("parsing should succeed");

        assert_eq!(task.status, "backlog");
        assert_eq!(task.priority, "medium");
        assert!(task.tags.is_empty());
        assert!(task.depends_on.is_empty());
        assert!(task.project_id.is_none());
        assert_eq!(task.order, 0);
        assert_eq!(task.estimated_minutes, None);
        assert_eq!(task.tracked_minutes, 0);
        assert_eq!(task.series_id, None);
        assert_eq!(task.subtask_project_id, None);
    }

    #[test]
    fn new_task_has_no_estimate_and_zero_tracked_minutes() {
        let task = Task::new("Write report".to_string());

        assert_eq!(task.estimated_minutes, None);
        assert_eq!(task.tracked_minutes, 0);
    }

    #[test]
    fn new_task_has_no_series_id() {
        let task = Task::new("Write report".to_string());

        assert_eq!(task.series_id, None);
    }

    #[test]
    fn to_markdown_omits_series_id_when_unset() {
        let task = Task::new("Demo".to_string());

        let markdown = task.to_markdown().expect("serialization should succeed");

        assert!(!markdown.contains("series_id"));
    }

    #[test]
    fn to_markdown_then_from_markdown_round_trips_a_series_id() {
        let mut task = Task::new("Water the plants".to_string());
        task.series_id = Some("series-abc123".to_string());

        let markdown = task.to_markdown().expect("serialization should succeed");
        let parsed = Task::from_markdown(&markdown).expect("parsing should succeed");

        assert_eq!(parsed.series_id, Some("series-abc123".to_string()));
    }

    #[test]
    fn from_markdown_defaults_series_id_to_none_when_absent() {
        let markdown = "---\nid: abc123\ntitle: Demo\ncreated: 2026-06-11T10:00:00+00:00\n---\n\n";

        let task = Task::from_markdown(markdown).expect("parsing should succeed");

        assert_eq!(task.series_id, None);
    }

    #[test]
    fn new_task_has_no_subtask_project_id() {
        let task = Task::new("Write report".to_string());

        assert_eq!(task.subtask_project_id, None);
    }

    #[test]
    fn to_markdown_omits_subtask_project_id_when_unset() {
        let task = Task::new("Demo".to_string());

        let markdown = task.to_markdown().expect("serialization should succeed");

        assert!(!markdown.contains("subtask_project_id"));
    }

    #[test]
    fn to_markdown_then_from_markdown_round_trips_a_subtask_project_id() {
        let mut task = Task::new("Fix the bug".to_string());
        task.subtask_project_id = Some("container-project-id".to_string());

        let markdown = task.to_markdown().expect("serialization should succeed");
        let parsed = Task::from_markdown(&markdown).expect("parsing should succeed");

        assert_eq!(
            parsed.subtask_project_id,
            Some("container-project-id".to_string())
        );
    }

    #[test]
    fn from_markdown_defaults_subtask_project_id_to_none_when_absent() {
        let markdown = "---\nid: abc123\ntitle: Demo\ncreated: 2026-06-11T10:00:00+00:00\n---\n\n";

        let task = Task::from_markdown(markdown).expect("parsing should succeed");

        assert_eq!(task.subtask_project_id, None);
    }

    #[test]
    fn to_markdown_omits_estimated_minutes_when_unset() {
        let task = Task::new("Demo".to_string());

        let markdown = task.to_markdown().expect("serialization should succeed");

        assert!(!markdown.contains("estimated_minutes"));
    }

    #[test]
    fn to_markdown_then_from_markdown_round_trips_an_estimate_of_zero() {
        let mut task = Task::new("Demo".to_string());
        task.estimated_minutes = Some(0);

        let markdown = task.to_markdown().expect("serialization should succeed");
        let parsed = Task::from_markdown(&markdown).expect("parsing should succeed");

        assert_eq!(parsed.estimated_minutes, Some(0));
    }

    #[test]
    fn from_markdown_without_frontmatter_fails() {
        let result = Task::from_markdown("Just plain markdown, no frontmatter.");

        assert!(matches!(result, Err(TaskError::MissingFrontmatter)));
    }

    #[test]
    fn from_markdown_accepts_an_arbitrary_priority_string() {
        let markdown = "---\nid: abc123\ntitle: Demo\npriority: urgent\ncreated: 2026-06-11T10:00:00+00:00\n---\n\n";

        let task = Task::from_markdown(markdown).expect("parsing should succeed");

        assert_eq!(task.priority, "urgent");
    }

    #[test]
    fn to_markdown_emits_priority_as_a_plain_string() {
        let mut task = Task::new("Demo".to_string());
        task.priority = "urgent".to_string();

        let markdown = task.to_markdown().expect("serialization should succeed");

        assert!(markdown.contains("priority: urgent\n"));
    }

    #[test]
    fn from_markdown_accepts_an_arbitrary_status_string() {
        let markdown = "---\nid: abc123\ntitle: Demo\nstatus: on-hold\ncreated: 2026-06-11T10:00:00+00:00\n---\n\n";

        let task = Task::from_markdown(markdown).expect("parsing should succeed");

        assert_eq!(task.status, "on-hold");
    }

    #[test]
    fn to_markdown_emits_status_as_a_plain_string() {
        let mut task = Task::new("Demo".to_string());
        task.status = "on-hold".to_string();

        let markdown = task.to_markdown().expect("serialization should succeed");

        assert!(markdown.contains("status: on-hold\n"));
    }
}
