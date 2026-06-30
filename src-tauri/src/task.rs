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
    /// Marks this task as the hidden time-tracking anchor for a `Project`
    /// (see `Project::tracking_task_id`) — set once when that project's
    /// lazily-created tracker task is generated, and never toggled
    /// afterward. `false` for every ordinary task. Hidden tasks are excluded
    /// from every kanban/week/calendar view and task picker by the frontend,
    /// but otherwise remain normal addressable markdown files: they save,
    /// load, and back up exactly like any other task, and `time_entries`
    /// rows can reference their id identically to a non-hidden task's.
    #[serde(default)]
    pub hidden: bool,
    /// ISO 8601 datetime when this task entered the done status (see
    /// `Settings::done_status`). Set automatically by [`crate::commands::update_task`]
    /// on status transition to done, cleared when the task leaves done, and
    /// left `None` for tasks that have never been marked done.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    /// ISO 8601 datetime when this task entered the cancelled status (see
    /// `Settings::cancelled_status`). Set automatically by
    /// [`crate::commands::update_task`] on status transition to cancelled,
    /// cleared when the task leaves cancelled, and left `None` for tasks
    /// that have never been cancelled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<String>,
    /// Free-form markdown body of the task file. Stored after the YAML
    /// frontmatter block (never inside it) — `to_markdown` strips this field
    /// from the YAML output and writes it as the file body instead.
    /// `#[serde(default)]` (not `skip`) so notes travel through Tauri's
    /// JSON IPC in both directions.
    #[serde(default)]
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

/// Removes the `notes:` key (and any continuation lines for block scalars)
/// from a YAML string produced by `serde_yaml::to_string`. Notes live in the
/// markdown body after the frontmatter closing `---`, not in the YAML itself.
fn strip_notes_from_yaml(yaml: &str) -> String {
    let mut result: Vec<&str> = Vec::new();
    let mut in_notes_block = false;

    for line in yaml.lines() {
        if line.starts_with("notes:") {
            in_notes_block = true;
            continue;
        }
        if in_notes_block {
            // Continuation lines of a YAML block scalar are indented
            if line.starts_with(' ') || line.is_empty() {
                continue;
            }
            in_notes_block = false;
        }
        result.push(line);
    }

    let mut out = result.join("\n");
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
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
            hidden: false,
            completed_at: None,
            cancelled_at: None,
            notes: String::new(),
        }
    }

    /// Serializes the task to a markdown string with a YAML frontmatter
    /// block followed by the task's free-form notes.
    pub fn to_markdown(&self) -> Result<String, TaskError> {
        let frontmatter = serde_yaml::to_string(self).map_err(TaskError::Serialize)?;
        // `notes` is included in serde JSON (for Tauri IPC) but must not
        // appear in the YAML frontmatter — notes live in the file body.
        let frontmatter = strip_notes_from_yaml(&frontmatter);
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
        task.hidden = true;
        task.completed_at = Some("2026-06-25T12:34:56+00:00".to_string());
        task.cancelled_at = None;
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
        assert!(!task.hidden);
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
    fn new_task_is_not_hidden() {
        let task = Task::new("Write report".to_string());

        assert!(!task.hidden);
    }

    #[test]
    fn to_markdown_then_from_markdown_round_trips_hidden_true() {
        let mut task = Task::new("Project — General time".to_string());
        task.hidden = true;

        let markdown = task.to_markdown().expect("serialization should succeed");
        let parsed = Task::from_markdown(&markdown).expect("parsing should succeed");

        assert!(parsed.hidden);
    }

    #[test]
    fn from_markdown_defaults_hidden_to_false_when_absent() {
        let markdown = "---\nid: abc123\ntitle: Demo\ncreated: 2026-06-11T10:00:00+00:00\n---\n\n";

        let task = Task::from_markdown(markdown).expect("parsing should succeed");

        assert!(!task.hidden);
    }

    #[test]
    fn new_task_has_no_completed_at_or_cancelled_at() {
        let task = Task::new("Write report".to_string());

        assert_eq!(task.completed_at, None);
        assert_eq!(task.cancelled_at, None);
    }

    #[test]
    fn to_markdown_omits_completed_at_when_unset() {
        let task = Task::new("Demo".to_string());

        let markdown = task.to_markdown().expect("serialization should succeed");

        assert!(!markdown.contains("completed_at"));
    }

    #[test]
    fn to_markdown_omits_cancelled_at_when_unset() {
        let task = Task::new("Demo".to_string());

        let markdown = task.to_markdown().expect("serialization should succeed");

        assert!(!markdown.contains("cancelled_at"));
    }

    #[test]
    fn to_markdown_then_from_markdown_round_trips_completed_at() {
        let mut task = Task::new("Done task".to_string());
        task.completed_at = Some("2026-06-25T12:34:56+00:00".to_string());

        let markdown = task.to_markdown().expect("serialization should succeed");
        let parsed = Task::from_markdown(&markdown).expect("parsing should succeed");

        assert_eq!(parsed.completed_at, Some("2026-06-25T12:34:56+00:00".to_string()));
        assert_eq!(parsed.cancelled_at, None);
    }

    #[test]
    fn to_markdown_then_from_markdown_round_trips_cancelled_at() {
        let mut task = Task::new("Cancelled task".to_string());
        task.cancelled_at = Some("2026-06-25T09:00:00+00:00".to_string());

        let markdown = task.to_markdown().expect("serialization should succeed");
        let parsed = Task::from_markdown(&markdown).expect("parsing should succeed");

        assert_eq!(parsed.cancelled_at, Some("2026-06-25T09:00:00+00:00".to_string()));
        assert_eq!(parsed.completed_at, None);
    }

    #[test]
    fn from_markdown_defaults_completed_at_to_none_when_absent() {
        let markdown = "---\nid: abc123\ntitle: Demo\ncreated: 2026-06-11T10:00:00+00:00\n---\n\n";

        let task = Task::from_markdown(markdown).expect("parsing should succeed");

        assert_eq!(task.completed_at, None);
    }

    #[test]
    fn from_markdown_defaults_cancelled_at_to_none_when_absent() {
        let markdown = "---\nid: abc123\ntitle: Demo\ncreated: 2026-06-11T10:00:00+00:00\n---\n\n";

        let task = Task::from_markdown(markdown).expect("parsing should succeed");

        assert_eq!(task.cancelled_at, None);
    }

    #[test]
    fn from_markdown_applies_defaults_includes_no_completed_or_cancelled_at() {
        let markdown = "---\nid: abc123\ntitle: Demo\ncreated: 2026-06-11T10:00:00+00:00\n---\n\n";

        let task = Task::from_markdown(markdown).expect("parsing should succeed");

        assert_eq!(task.completed_at, None);
        assert_eq!(task.cancelled_at, None);
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

    #[test]
    fn to_markdown_does_not_include_notes_in_yaml_frontmatter() {
        let mut task = Task::new("Demo".to_string());
        task.notes = "These are my notes.\nThey span multiple lines.".to_string();

        let markdown = task.to_markdown().expect("serialization should succeed");

        // Extract only the YAML frontmatter block (between the two `---` lines)
        let frontmatter = markdown
            .strip_prefix("---\n")
            .and_then(|s| s.split_once("\n---\n"))
            .map(|(fm, _)| fm)
            .expect("markdown should have frontmatter");

        assert!(
            !frontmatter.contains("notes"),
            "frontmatter should not contain 'notes' key, but got:\n{frontmatter}"
        );
    }

    #[test]
    fn to_markdown_writes_notes_in_body() {
        let mut task = Task::new("Demo".to_string());
        task.notes = "Important note content.".to_string();

        let markdown = task.to_markdown().expect("serialization should succeed");

        // Body is everything after the closing `---`
        let body = markdown
            .split_once("\n---\n\n")
            .map(|(_, body)| body)
            .expect("markdown should have body section");

        assert_eq!(body.trim(), "Important note content.");
    }

    #[test]
    fn notes_survive_serde_json_round_trip() {
        let mut task = Task::new("Test task".to_string());
        task.notes = "important notes".to_string();

        let json = serde_json::to_string(&task).expect("should serialize to JSON");
        let parsed: Task = serde_json::from_str(&json).expect("should deserialize from JSON");

        assert_eq!(parsed.notes, "important notes");
    }

    #[test]
    fn to_markdown_strips_multiline_notes_from_frontmatter() {
        let mut task = Task::new("Demo".to_string());
        // Use a string with newlines that serde_yaml might render as a block scalar
        task.notes = "line one\nline two\nline three".to_string();

        let markdown = task.to_markdown().expect("serialization should succeed");
        let frontmatter = markdown
            .strip_prefix("---\n")
            .and_then(|s| s.split_once("\n---\n"))
            .map(|(fm, _)| fm)
            .expect("markdown should have frontmatter");

        assert!(
            !frontmatter.contains("notes"),
            "frontmatter must not contain notes key even for multiline notes"
        );
        assert!(markdown.contains("line one\nline two\nline three"));
    }
}
