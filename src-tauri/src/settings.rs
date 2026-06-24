use serde::{Deserialize, Serialize};

/// Fallback color for a priority level that has no color of its own: used
/// for newly-created custom levels before the user picks a color, and as a
/// placeholder for `settings.json` files written before `PriorityLevel.color`
/// was mandatory (see `Settings::normalize`). Hex, to match the format
/// produced by the `ColorPicker` UI and `Project.color`.
fn default_priority_color() -> String {
    "#807973".to_string()
}

/// Fallback color for a status that has no color of its own: used for
/// newly-created custom statuses before the user picks a color, and as a
/// placeholder for `settings.json` files written before
/// `StatusDefinition.color` was mandatory (see `Settings::normalize`). Hex,
/// to match the format produced by the `ColorPicker` UI and `Project.color`.
fn default_status_color() -> String {
    "#807973".to_string()
}

/// Default OKLCH lightness for "color code" mode's Kanban card background.
/// Matches the value this was hardcoded to before it became configurable
/// (`NEON_CARD_LIGHTNESS` in the frontend's `colorPresets.ts`), so existing
/// users see no visual change after upgrading.
fn default_card_lightness() -> f64 {
    0.5
}

/// Default OKLCH lightness for "color code" mode's week/calendar-view bar
/// background — deliberately darker than the card default, matching the
/// value this was hardcoded to before it became configurable
/// (`WEEK_BAR_LIGHTNESS` in the frontend's `colorPresets.ts`).
fn default_bar_lightness() -> f64 {
    0.38
}

/// The fixed set of values accepted by `Settings.ink_mode` and
/// `crate::project::ProjectBoard.ink_mode`. `"auto"` picks whichever of a
/// dark/light ink color has the higher real WCAG contrast against the
/// resolved background (see the frontend's `legibleInkColor`); `"white"`/
/// `"black"` force that choice regardless of contrast.
pub const INK_MODES: &[&str] = &["auto", "white", "black"];

/// Default `Settings.ink_mode`: preserves the contrast-computed behavior
/// this shipped with before the setting existed.
fn default_ink_mode() -> String {
    "auto".to_string()
}

/// Default `Settings.max_visible_subtasks`: a parent card's nested subtask
/// preview truncates beyond this many rows, picked as a reasonable card
/// height before the rest collapse into a "+N more" line.
fn default_max_visible_subtasks() -> u32 {
    5
}

/// The fixed set of values accepted by `Settings.card_tracked_time_display`.
/// `"total"`: a card's live ticker (while its timer is running) shows the
/// task's cumulative tracked time across every past session plus the
/// current one — see the frontend's `liveTrackedSecondsFor`. `"session"`:
/// the live ticker shows only the current session's own elapsed time,
/// restarting from zero on every resume. Either way, the *stopped*-state
/// chip (no timer currently running) always shows the lifetime total
/// regardless of this setting — there is no "current session" once
/// stopped, so showing anything else there wouldn't be meaningful.
pub const CARD_TRACKED_TIME_DISPLAY_MODES: &[&str] = &["total", "session"];

/// Default `Settings.card_tracked_time_display`: matches the behavior this
/// shipped with before the setting existed (the live ticker shows the
/// cumulative total, never resetting on resume).
fn default_card_tracked_time_display() -> String {
    "total".to_string()
}

/// A user-defined priority level: an id stored in `Task.priority`, a display
/// label, a `color` used to render that priority throughout the UI, and a
/// `rank` used to sort tasks by priority (lower `rank` sorts first / is
/// considered higher priority).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriorityLevel {
    pub id: String,
    pub label: String,
    #[serde(default = "default_priority_color")]
    pub color: String,
    pub rank: i64,
}

/// A user-defined task status: an id stored in `Task.status`, a display
/// label, `order`, its position in the global status list, and a `color`
/// used to style Kanban columns for this status throughout the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatusDefinition {
    pub id: String,
    pub label: String,
    pub order: i64,
    #[serde(default = "default_status_color")]
    pub color: String,
}

/// Default task attributes. Used both as the global defaults and as a
/// project's per-field overrides of those global defaults: any field left
/// `None`/empty here falls back to the corresponding global value.
///
/// `scheduled`, if set, must be one of [`SCHEDULED_RELATIVE_DATE_CODES`]
/// rather than an absolute date: it's resolved to an absolute date relative
/// to "today" at task-creation time (see [`resolve_scheduled_relative_date`]).
/// The global `defaults.scheduled` is always set (see [`Settings::default`]
/// and [`Settings::normalize`]) since every task must have a scheduled date.
///
/// `due`, if set, must be one of [`DUE_RELATIVE_DATE_CODES`] rather than an
/// absolute date: it's resolved to an absolute date relative to the task's
/// *scheduled* date (not "today") at task-creation time (see
/// [`resolve_due_relative_date`]). `"none"` means "never due".
///
/// `estimated_minutes`, if set, seeds `Task.estimated_minutes` for a newly
/// created task that doesn't specify its own estimate.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TaskDefaults {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
}

/// Global, app-wide settings: the available priority levels, the global list
/// of statuses (from which each project's board is configured), and the
/// global default task attributes.
///
/// `done_status` and `cancelled_status` mark which entries in `statuses`
/// represent a task being finished or abandoned. Exactly one status must
/// always be the done status (enforced by [`validate_settings`]); the
/// cancelled status is optional and, if set, must differ from the done
/// status — a single status can't mean both "done" and "cancelled".
///
/// `default_project_id` is the id of the project a new task is filed under
/// when no project was specified (and no project-scoped board supplied
/// one); it must be non-empty (enforced by [`validate_settings`]) so a task
/// can never be created or saved without a project. Empty only transiently,
/// before the startup migration ensures a real default project exists and
/// sets this to its id.
///
/// `show_previous_weeks_column` is the global default for whether the Week
/// view shows an extra leading column listing unfinished tasks scheduled or
/// due before the visible week. A project's `ProjectBoard.show_previous_weeks`
/// overrides this when set (see [`crate::project::ProjectBoard`]).
///
/// `card_lightness`/`bar_lightness` are the global OKLCH lightness (see
/// [`validate_lightness`]) used for "color code" mode's Kanban card and
/// week/calendar-view bar backgrounds, respectively — two separate values
/// since the bar treatment is deliberately darker by default. A project's
/// `ProjectBoard.card_lightness`/`.bar_lightness` overrides these
/// individually when set.
///
/// `ink_mode` is the global default text-color mode (one of [`INK_MODES`])
/// for "color code" mode's Kanban card/bar text. A project's
/// `ProjectBoard.ink_mode` overrides this when set.
///
/// `show_subproject_tasks_default` is the global default for whether
/// viewing a project's board/week/calendar rolls up its descendant
/// subprojects' tasks too. A project's `ProjectBoard.show_subproject_tasks`
/// overrides this when set (see [`crate::project::ProjectBoard`]). Defaults
/// to `false` — rollup is opt-in per project, not automatic.
///
/// `parent_estimate_includes_own_value` controls how a task with subtasks'
/// *displayed* estimated time is computed on the frontend: when `true`, its
/// own `estimated_minutes` is added on top of its subtasks' total; when
/// `false` (the default), the subtasks' total replaces it entirely. Purely
/// a display preference — nothing in this struct or the backend ever
/// recomputes or overwrites a task's stored `estimated_minutes` based on
/// this; see the frontend's `effectiveEstimatedMinutes`.
///
/// `max_visible_subtasks` caps how many subtask rows a parent card's nested
/// preview shows at once before collapsing the rest into a "+N more" line —
/// also purely a frontend display concern (see the frontend's
/// `taskSubtasks`/`hiddenSubtaskCount` in `TaskCard.svelte`).
///
/// `tracking_auto_transition_enabled`/`tracking_auto_transition_status_id`
/// are the time-tracking engine's "auto-transition status when tracking
/// starts" setting (see `docs/features/time-tracking-engine.md`):
/// `tracking_auto_transition_enabled` defaults to `false` (no automatic
/// status change); when `true` and `tracking_auto_transition_status_id`
/// names a defined status, starting a task's timer moves that task to that
/// status. When enabled but `tracking_auto_transition_status_id` is unset,
/// the frontend falls back at runtime to the first status in the global
/// status list that isn't backlog/done/cancelled — that fallback resolution
/// itself is later-milestone frontend logic, not implemented here; this
/// struct only stores the two raw settings values.
///
/// `card_tracked_time_display` is one of [`CARD_TRACKED_TIME_DISPLAY_MODES`]
/// — see that constant's own doc comment for what each value means.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    #[serde(default)]
    pub priorities: Vec<PriorityLevel>,
    #[serde(default)]
    pub statuses: Vec<StatusDefinition>,
    #[serde(default)]
    pub defaults: TaskDefaults,
    #[serde(default)]
    pub done_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancelled_status: Option<String>,
    #[serde(default)]
    pub default_project_id: String,
    #[serde(default)]
    pub show_previous_weeks_column: bool,
    #[serde(default = "default_card_lightness")]
    pub card_lightness: f64,
    #[serde(default = "default_bar_lightness")]
    pub bar_lightness: f64,
    #[serde(default = "default_ink_mode")]
    pub ink_mode: String,
    #[serde(default)]
    pub show_subproject_tasks_default: bool,
    #[serde(default)]
    pub parent_estimate_includes_own_value: bool,
    #[serde(default = "default_max_visible_subtasks")]
    pub max_visible_subtasks: u32,
    #[serde(default)]
    pub tracking_auto_transition_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracking_auto_transition_status_id: Option<String>,
    #[serde(default = "default_card_tracked_time_display")]
    pub card_tracked_time_display: String,
}

impl Default for Settings {
    /// Seeds settings matching the app's previously hardcoded priority
    /// levels (low/medium/high) and statuses (backlog/do/in-progress/
    /// blocked/done), with new tasks defaulting to medium priority and the
    /// backlog status, as before. Colors are the hex equivalents of the
    /// app's original OKLCH seed colors, to match the format produced by the
    /// `ColorPicker` UI and `Project.color`.
    fn default() -> Self {
        Settings {
            priorities: vec![
                PriorityLevel {
                    id: "high".to_string(),
                    label: "High".to_string(),
                    color: "#bc267f".to_string(),
                    rank: 1,
                },
                PriorityLevel {
                    id: "medium".to_string(),
                    label: "Medium".to_string(),
                    color: "#aa6a00".to_string(),
                    rank: 2,
                },
                PriorityLevel {
                    id: "low".to_string(),
                    label: "Low".to_string(),
                    color: "#0e9254".to_string(),
                    rank: 3,
                },
            ],
            statuses: vec![
                StatusDefinition {
                    id: "backlog".to_string(),
                    label: "Backlog".to_string(),
                    order: 1,
                    color: "#6f7178".to_string(),
                },
                StatusDefinition {
                    id: "do".to_string(),
                    label: "Do".to_string(),
                    order: 2,
                    color: "#0073b6".to_string(),
                },
                StatusDefinition {
                    id: "in-progress".to_string(),
                    label: "In Progress".to_string(),
                    order: 3,
                    color: "#bd7d00".to_string(),
                },
                StatusDefinition {
                    id: "blocked".to_string(),
                    label: "Blocked".to_string(),
                    order: 4,
                    color: "#bc267f".to_string(),
                },
                StatusDefinition {
                    id: "done".to_string(),
                    label: "Done".to_string(),
                    order: 5,
                    color: "#0e9254".to_string(),
                },
            ],
            defaults: TaskDefaults {
                tags: Vec::new(),
                priority: Some("medium".to_string()),
                status: Some("backlog".to_string()),
                due: Some("none".to_string()),
                scheduled: Some("today".to_string()),
                estimated_minutes: None,
            },
            done_status: "done".to_string(),
            cancelled_status: None,
            default_project_id: String::new(),
            show_previous_weeks_column: false,
            card_lightness: default_card_lightness(),
            bar_lightness: default_bar_lightness(),
            ink_mode: default_ink_mode(),
            show_subproject_tasks_default: false,
            parent_estimate_includes_own_value: false,
            max_visible_subtasks: default_max_visible_subtasks(),
            tracking_auto_transition_enabled: false,
            tracking_auto_transition_status_id: None,
            card_tracked_time_display: default_card_tracked_time_display(),
        }
    }
}

impl Settings {
    /// Fills in the seeded colors for the well-known default priority ids
    /// (`high`/`medium`/`low`) and status ids (`backlog`/`do`/`in-progress`/
    /// `blocked`/`done`) when they're still on the neutral fallback color,
    /// and repairs `done_status` if it doesn't reference a defined status.
    /// This recovers `settings.json` files written before
    /// `PriorityLevel.color`/`StatusDefinition.color`/`Settings.done_status`
    /// were mandatory, where the field was absent and serde supplied an
    /// empty/fallback value on load.
    ///
    /// When repairing `done_status` and no status has id `"done"`, the
    /// fallback is the status with the highest `order`; if multiple statuses
    /// tie on `order`, the last one encountered in `self.statuses` is used.
    ///
    /// Also backfills `defaults.scheduled`/`defaults.due` to `"today"`/
    /// `"none"` for `settings.json` files written before every task was
    /// required to have a scheduled date.
    pub fn normalize(mut self) -> Self {
        let seeded = Self::default();
        for level in &mut self.priorities {
            if level.color == default_priority_color() {
                if let Some(seed) = seeded.priorities.iter().find(|p| p.id == level.id) {
                    level.color = seed.color.clone();
                }
            }
        }
        for status in &mut self.statuses {
            if status.color == default_status_color() {
                if let Some(seed) = seeded.statuses.iter().find(|s| s.id == status.id) {
                    status.color = seed.color.clone();
                }
            }
        }

        if !self.statuses.iter().any(|s| s.id == self.done_status) {
            self.done_status = self
                .statuses
                .iter()
                .find(|s| s.id == "done")
                .or_else(|| self.statuses.iter().max_by_key(|s| s.order))
                .map(|s| s.id.clone())
                .unwrap_or_default();
        }

        if self.defaults.scheduled.is_none() {
            self.defaults.scheduled = Some("today".to_string());
        }
        if self.defaults.due.is_none() {
            self.defaults.due = Some("none".to_string());
        }

        self
    }
}

/// The fixed set of relative-date codes accepted by `TaskDefaults.scheduled`.
/// Mirrors `SCHEDULED_RELATIVE_DATE_OPTIONS` in the frontend's
/// `src/lib/relativeDates.ts` — keep both lists in sync.
pub const SCHEDULED_RELATIVE_DATE_CODES: &[&str] = &[
    "today",
    "tomorrow",
    "in_2_days",
    "in_3_days",
    "in_1_week",
    "in_1_month",
];

/// The fixed set of relative-date codes accepted by `TaskDefaults.due`.
/// `"none"` means "never due"; the rest are offsets from the task's
/// `scheduled` date (not "today") — see [`resolve_due_relative_date`]. Mirrors
/// `DUE_RELATIVE_DATE_OPTIONS` in the frontend's `src/lib/relativeDates.ts` —
/// keep both lists in sync.
pub const DUE_RELATIVE_DATE_CODES: &[&str] = &[
    "none",
    "same_day",
    "next_day",
    "in_2_days",
    "in_3_days",
    "in_1_week",
    "in_1_month",
];

/// Returns `Ok(())` if `code` is one of [`SCHEDULED_RELATIVE_DATE_CODES`], or
/// an error naming the unrecognized code otherwise.
pub fn validate_scheduled_relative_date_code(code: &str) -> Result<(), String> {
    if SCHEDULED_RELATIVE_DATE_CODES.contains(&code) {
        Ok(())
    } else {
        Err(format!(
            "'{code}' is not a recognized scheduled-date option"
        ))
    }
}

/// Returns `Ok(())` if `code` is one of [`DUE_RELATIVE_DATE_CODES`], or an
/// error naming the unrecognized code otherwise.
pub fn validate_due_relative_date_code(code: &str) -> Result<(), String> {
    if DUE_RELATIVE_DATE_CODES.contains(&code) {
        Ok(())
    } else {
        Err(format!("'{code}' is not a recognized due-date option"))
    }
}

/// Resolves a [`SCHEDULED_RELATIVE_DATE_CODES`] code to an absolute date
/// relative to `today`. Returns `None` for a code outside that list — this
/// should be unreachable for codes that passed
/// [`validate_scheduled_relative_date_code`] at write-time, but degrades
/// gracefully (no default date applied) for a stale code left over from a
/// future app version rather than panicking.
pub fn resolve_scheduled_relative_date(
    code: &str,
    today: chrono::NaiveDate,
) -> Option<chrono::NaiveDate> {
    use chrono::{Days, Months};
    match code {
        "today" => Some(today),
        "tomorrow" => today.checked_add_days(Days::new(1)),
        "in_2_days" => today.checked_add_days(Days::new(2)),
        "in_3_days" => today.checked_add_days(Days::new(3)),
        "in_1_week" => today.checked_add_days(Days::new(7)),
        "in_1_month" => today.checked_add_months(Months::new(1)),
        _ => None,
    }
}

/// Resolves a [`DUE_RELATIVE_DATE_CODES`] code to an absolute date relative to
/// `scheduled`. `"none"` always resolves to `None` (never due). Returns
/// `None` for any other code outside that list — this should be unreachable
/// for codes that passed [`validate_due_relative_date_code`] at write-time,
/// but degrades gracefully (no due date applied) for a stale code left over
/// from a future app version rather than panicking.
pub fn resolve_due_relative_date(
    code: &str,
    scheduled: chrono::NaiveDate,
) -> Option<chrono::NaiveDate> {
    use chrono::{Days, Months};
    match code {
        "none" => None,
        "same_day" => Some(scheduled),
        "next_day" => scheduled.checked_add_days(Days::new(1)),
        "in_2_days" => scheduled.checked_add_days(Days::new(2)),
        "in_3_days" => scheduled.checked_add_days(Days::new(3)),
        "in_1_week" => scheduled.checked_add_days(Days::new(7)),
        "in_1_month" => scheduled.checked_add_months(Months::new(1)),
        _ => None,
    }
}

/// Returns `Ok(())` if `id` matches a `PriorityLevel` in `settings.priorities`,
/// or an error naming the unknown id otherwise. Used by the command layer to
/// reject writes that reference an undefined priority level.
pub fn validate_priority_id(settings: &Settings, id: &str) -> Result<(), String> {
    if settings.priorities.iter().any(|level| level.id == id) {
        Ok(())
    } else {
        Err(format!("priority '{id}' is not a defined priority level"))
    }
}

/// Returns `Ok(())` if `id` matches a `StatusDefinition` in `settings.statuses`,
/// or an error naming the unknown id otherwise. Used by the command layer to
/// reject writes that reference an undefined status.
pub fn validate_status_id(settings: &Settings, id: &str) -> Result<(), String> {
    if settings.statuses.iter().any(|status| status.id == id) {
        Ok(())
    } else {
        Err(format!("status '{id}' is not a defined status"))
    }
}

/// Returns `Ok(())` if `value` is a valid OKLCH lightness (`0.0..=1.0`), or
/// an error otherwise. Used for `Settings.card_lightness`/`.bar_lightness`
/// and their per-project `ProjectBoard` overrides — see
/// [`crate::project::ProjectBoard`].
pub fn validate_lightness(value: f64) -> Result<(), String> {
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "lightness must be between 0.0 and 1.0, got {value}"
        ))
    }
}

/// Returns `Ok(())` if `value` is one of [`INK_MODES`], or an error naming
/// the unrecognized value otherwise. Used for `Settings.ink_mode` and its
/// per-project `ProjectBoard` override — see [`crate::project::ProjectBoard`].
pub fn validate_ink_mode(value: &str) -> Result<(), String> {
    if INK_MODES.contains(&value) {
        Ok(())
    } else {
        Err(format!("'{value}' is not a recognized ink mode"))
    }
}

/// Returns `Ok(())` if `value` is one of [`CARD_TRACKED_TIME_DISPLAY_MODES`],
/// or an error naming the invalid value otherwise. Used for
/// `Settings.card_tracked_time_display`.
pub fn validate_card_tracked_time_display(value: &str) -> Result<(), String> {
    if CARD_TRACKED_TIME_DISPLAY_MODES.contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "'{value}' is not a recognized card tracked-time display mode"
        ))
    }
}

/// Validates settings before they're persisted: `priorities` and `statuses`
/// must each be non-empty with unique ids (an empty or duplicate-id list
/// would make `validate_priority_id`/`validate_status_id` reject every task
/// write, including the `resolve_default_priority`/`resolve_default_status`
/// fallbacks), `defaults.priority`/`defaults.status`, if set, must reference
/// one of those ids, `defaults.due`, if set, must be one of
/// [`DUE_RELATIVE_DATE_CODES`], `defaults.scheduled`, if set, must be one of
/// [`SCHEDULED_RELATIVE_DATE_CODES`], `done_status` must be non-empty and
/// reference a defined status, `cancelled_status`, if set, must
/// reference a defined status distinct from `done_status`,
/// `card_lightness`/`bar_lightness` must each be a valid OKLCH lightness
/// (see [`validate_lightness`]), `default_project_id` must be non-empty,
/// and `tracking_auto_transition_status_id`, if set, must reference a
/// defined status.
pub fn validate_settings(settings: &Settings) -> Result<(), String> {
    if settings.priorities.is_empty() {
        return Err("at least one priority level must be defined".to_string());
    }

    let mut seen = std::collections::HashSet::new();
    if let Some(duplicate) = settings
        .priorities
        .iter()
        .find(|level| !seen.insert(&level.id))
    {
        return Err(format!("duplicate priority id '{}'", duplicate.id));
    }

    if let Some(default_priority) = &settings.defaults.priority {
        validate_priority_id(settings, default_priority)?;
    }

    if settings.statuses.is_empty() {
        return Err("at least one status must be defined".to_string());
    }

    let mut seen = std::collections::HashSet::new();
    if let Some(duplicate) = settings
        .statuses
        .iter()
        .find(|status| !seen.insert(&status.id))
    {
        return Err(format!("duplicate status id '{}'", duplicate.id));
    }

    if let Some(default_status) = &settings.defaults.status {
        validate_status_id(settings, default_status)?;
    }

    if settings.done_status.is_empty() {
        return Err("a done status must be defined".to_string());
    }
    validate_status_id(settings, &settings.done_status)?;

    if let Some(cancelled_status) = &settings.cancelled_status {
        validate_status_id(settings, cancelled_status)?;
        if cancelled_status == &settings.done_status {
            return Err("the cancelled status can't be the same as the done status".to_string());
        }
    }

    if let Some(due) = &settings.defaults.due {
        validate_due_relative_date_code(due)?;
    }

    if let Some(scheduled) = &settings.defaults.scheduled {
        validate_scheduled_relative_date_code(scheduled)?;
    }

    if settings.default_project_id.is_empty() {
        return Err("a default project must be defined".to_string());
    }

    validate_lightness(settings.card_lightness)?;
    validate_lightness(settings.bar_lightness)?;
    validate_ink_mode(&settings.ink_mode)?;

    if let Some(status_id) = &settings.tracking_auto_transition_status_id {
        validate_status_id(settings, status_id)?;
    }

    validate_card_tracked_time_display(&settings.card_tracked_time_display)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_seed_matches_previous_hardcoded_values() {
        let settings = Settings::default();

        let priority_ids: Vec<&str> = settings.priorities.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(priority_ids, vec!["high", "medium", "low"]);

        let status_ids: Vec<&str> = settings.statuses.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            status_ids,
            vec!["backlog", "do", "in-progress", "blocked", "done"]
        );

        assert_eq!(settings.defaults.priority, Some("medium".to_string()));
        assert_eq!(settings.defaults.status, Some("backlog".to_string()));
        assert!(settings.defaults.tags.is_empty());
        assert_eq!(settings.defaults.due, Some("none".to_string()));
        assert_eq!(settings.defaults.scheduled, Some("today".to_string()));
    }

    #[test]
    fn default_settings_seed_has_a_done_status_and_no_cancelled_status() {
        let settings = Settings::default();

        assert_eq!(settings.done_status, "done");
        assert_eq!(settings.cancelled_status, None);
    }

    #[test]
    fn default_settings_seed_has_an_empty_default_project_id() {
        let settings = Settings::default();

        assert_eq!(settings.default_project_id, "");
    }

    #[test]
    fn deserializing_settings_without_default_project_id_leaves_it_empty() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert_eq!(settings.default_project_id, "");
    }

    #[test]
    fn default_settings_seed_priorities_have_distinct_non_empty_colors() {
        let settings = Settings::default();

        let colors: Vec<&str> = settings
            .priorities
            .iter()
            .map(|p| p.color.as_str())
            .collect();
        assert!(colors.iter().all(|c| !c.is_empty()));
        assert_eq!(
            colors.len(),
            colors
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
        );
    }

    #[test]
    fn default_settings_seed_statuses_have_distinct_non_empty_colors_unequal_to_the_fallback() {
        let settings = Settings::default();

        let colors: Vec<&str> = settings.statuses.iter().map(|s| s.color.as_str()).collect();
        assert!(colors.iter().all(|c| !c.is_empty()));
        assert!(colors.iter().all(|c| *c != default_status_color()));
        assert_eq!(
            colors.len(),
            colors
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
        );
    }

    /// Returns `true` if `color` is a 6-digit hex color (`#RRGGBB`).
    fn is_six_digit_hex(color: &str) -> bool {
        color.len() == 7
            && color.starts_with('#')
            && color[1..].chars().all(|c| c.is_ascii_hexdigit())
    }

    #[test]
    fn default_settings_seed_colors_and_fallback_are_six_digit_hex() {
        let settings = Settings::default();

        assert!(is_six_digit_hex(&default_priority_color()));
        assert!(is_six_digit_hex(&default_status_color()));
        assert!(settings
            .priorities
            .iter()
            .all(|p| is_six_digit_hex(&p.color)));
        assert!(settings.statuses.iter().all(|s| is_six_digit_hex(&s.color)));
    }

    #[test]
    fn validate_priority_id_accepts_a_defined_id() {
        let settings = Settings::default();

        assert!(validate_priority_id(&settings, "medium").is_ok());
    }

    #[test]
    fn validate_priority_id_rejects_an_unknown_id() {
        let settings = Settings::default();

        let err = validate_priority_id(&settings, "urgent").unwrap_err();
        assert!(err.contains("urgent"));
    }

    #[test]
    fn validate_status_id_accepts_a_defined_id() {
        let settings = Settings::default();

        assert!(validate_status_id(&settings, "backlog").is_ok());
    }

    #[test]
    fn validate_status_id_rejects_an_unknown_id() {
        let settings = Settings::default();

        let err = validate_status_id(&settings, "on-hold").unwrap_err();
        assert!(err.contains("on-hold"));
    }

    #[test]
    fn validate_settings_accepts_default_settings() {
        let mut settings = Settings::default();
        settings.default_project_id = "some-project-id".to_string();

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_rejects_empty_priorities() {
        let settings = Settings {
            priorities: Vec::new(),
            statuses: Vec::new(),
            defaults: TaskDefaults::default(),
            ..Default::default()
        };

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("at least one priority level"));
    }

    #[test]
    fn validate_settings_rejects_duplicate_priority_ids() {
        let mut settings = Settings::default();
        settings.priorities[1].id = settings.priorities[0].id.clone();

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("duplicate priority id"));
    }

    #[test]
    fn validate_settings_rejects_an_unknown_default_priority() {
        let mut settings = Settings::default();
        settings.defaults.priority = Some("urgent".to_string());

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("urgent"));
    }

    #[test]
    fn validate_settings_accepts_a_missing_default_priority() {
        let mut settings = Settings::default();
        settings.default_project_id = "some-project-id".to_string();
        settings.defaults.priority = None;

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_rejects_empty_statuses() {
        let settings = Settings {
            priorities: Settings::default().priorities,
            statuses: Vec::new(),
            defaults: TaskDefaults::default(),
            ..Default::default()
        };

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("at least one status"));
    }

    #[test]
    fn validate_settings_rejects_duplicate_status_ids() {
        let mut settings = Settings::default();
        settings.statuses[1].id = settings.statuses[0].id.clone();

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("duplicate status id"));
    }

    #[test]
    fn validate_settings_rejects_an_unknown_default_status() {
        let mut settings = Settings::default();
        settings.defaults.status = Some("on-hold".to_string());

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("on-hold"));
    }

    #[test]
    fn validate_settings_accepts_a_missing_default_status() {
        let mut settings = Settings::default();
        settings.default_project_id = "some-project-id".to_string();
        settings.defaults.status = None;

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_rejects_an_empty_done_status() {
        let mut settings = Settings::default();
        settings.done_status = String::new();

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("done status"));
    }

    #[test]
    fn validate_settings_rejects_an_unknown_done_status() {
        let mut settings = Settings::default();
        settings.done_status = "on-hold".to_string();

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("on-hold"));
    }

    #[test]
    fn validate_settings_accepts_a_valid_done_status() {
        let mut settings = Settings::default();
        settings.default_project_id = "some-project-id".to_string();
        settings.done_status = "backlog".to_string();

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_accepts_a_missing_cancelled_status() {
        let mut settings = Settings::default();
        settings.default_project_id = "some-project-id".to_string();
        settings.cancelled_status = None;

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_rejects_an_unknown_cancelled_status() {
        let mut settings = Settings::default();
        settings.cancelled_status = Some("on-hold".to_string());

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("on-hold"));
    }

    #[test]
    fn validate_settings_rejects_a_cancelled_status_equal_to_the_done_status() {
        let mut settings = Settings::default();
        settings.cancelled_status = Some(settings.done_status.clone());

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("cancelled"));
    }

    #[test]
    fn validate_settings_accepts_a_valid_distinct_cancelled_status() {
        let mut settings = Settings::default();
        settings.default_project_id = "some-project-id".to_string();
        settings.cancelled_status = Some("blocked".to_string());

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_rejects_an_unknown_default_due() {
        let mut settings = Settings::default();
        settings.defaults.due = Some("next_quarter".to_string());

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("next_quarter"));
    }

    #[test]
    fn validate_settings_accepts_a_valid_default_due() {
        let mut settings = Settings::default();
        settings.default_project_id = "some-project-id".to_string();
        settings.defaults.due = Some("next_day".to_string());

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_accepts_a_missing_default_due() {
        let mut settings = Settings::default();
        settings.default_project_id = "some-project-id".to_string();
        settings.defaults.due = None;

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_rejects_an_unknown_default_scheduled() {
        let mut settings = Settings::default();
        settings.defaults.scheduled = Some("next_quarter".to_string());

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("next_quarter"));
    }

    #[test]
    fn validate_settings_accepts_a_valid_default_scheduled() {
        let mut settings = Settings::default();
        settings.default_project_id = "some-project-id".to_string();
        settings.defaults.scheduled = Some("in_1_week".to_string());

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_accepts_a_missing_default_scheduled() {
        let mut settings = Settings::default();
        settings.default_project_id = "some-project-id".to_string();
        settings.defaults.scheduled = None;

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_rejects_an_empty_default_project() {
        let settings = Settings {
            default_project_id: String::new(),
            ..Default::default()
        };

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("default project"));
    }

    #[test]
    fn validate_settings_accepts_a_valid_default_project() {
        let settings = Settings {
            default_project_id: "some-project-id".to_string(),
            ..Default::default()
        };

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_lightness_accepts_the_full_valid_range() {
        assert!(validate_lightness(0.0).is_ok());
        assert!(validate_lightness(0.5).is_ok());
        assert!(validate_lightness(1.0).is_ok());
    }

    #[test]
    fn validate_lightness_rejects_a_negative_value() {
        assert!(validate_lightness(-0.01).is_err());
    }

    #[test]
    fn validate_lightness_rejects_a_value_above_one() {
        assert!(validate_lightness(1.01).is_err());
    }

    #[test]
    fn validate_settings_rejects_an_out_of_range_card_lightness() {
        let settings = Settings {
            card_lightness: 1.5,
            default_project_id: "some-project-id".to_string(),
            ..Default::default()
        };

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("lightness"));
    }

    #[test]
    fn validate_settings_rejects_an_out_of_range_bar_lightness() {
        let settings = Settings {
            bar_lightness: -0.5,
            ..Default::default()
        };

        assert!(validate_settings(&settings).is_err());
    }

    #[test]
    fn validate_settings_accepts_the_default_card_and_bar_lightness() {
        let mut settings = Settings::default();
        settings.default_project_id = "some-project-id".to_string();

        assert!(validate_settings(&settings).is_ok());
        assert_eq!(settings.card_lightness, 0.5);
        assert_eq!(settings.bar_lightness, 0.38);
    }

    #[test]
    fn default_settings_seed_does_not_include_a_tasks_own_estimate_in_its_parent_rollup() {
        let settings = Settings::default();

        assert!(!settings.parent_estimate_includes_own_value);
    }

    #[test]
    fn deserializing_settings_without_parent_estimate_includes_own_value_defaults_to_false() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert!(!settings.parent_estimate_includes_own_value);
    }

    #[test]
    fn default_settings_seed_has_a_max_visible_subtasks_of_five() {
        let settings = Settings::default();

        assert_eq!(settings.max_visible_subtasks, 5);
    }

    #[test]
    fn deserializing_settings_without_max_visible_subtasks_defaults_to_five() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert_eq!(settings.max_visible_subtasks, 5);
    }

    #[test]
    fn default_settings_seed_has_an_ink_mode_of_auto() {
        let settings = Settings::default();

        assert_eq!(settings.ink_mode, "auto");
    }

    #[test]
    fn deserializing_settings_without_ink_mode_defaults_to_auto() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert_eq!(settings.ink_mode, "auto");
    }

    #[test]
    fn validate_ink_mode_accepts_every_defined_mode() {
        for mode in INK_MODES {
            assert!(validate_ink_mode(mode).is_ok(), "{mode} should be valid");
        }
    }

    #[test]
    fn validate_ink_mode_rejects_an_unknown_mode() {
        let err = validate_ink_mode("sepia").unwrap_err();
        assert!(err.contains("sepia"));
    }

    #[test]
    fn validate_settings_rejects_an_unknown_ink_mode() {
        let settings = Settings {
            ink_mode: "sepia".to_string(),
            default_project_id: "some-project-id".to_string(),
            ..Settings::default()
        };

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("ink mode"));
    }

    #[test]
    fn validate_settings_accepts_every_defined_ink_mode() {
        for mode in INK_MODES {
            let settings = Settings {
                ink_mode: mode.to_string(),
                default_project_id: "some-project-id".to_string(),
                ..Settings::default()
            };

            assert!(
                validate_settings(&settings).is_ok(),
                "{mode} should be valid"
            );
        }
    }

    #[test]
    fn default_settings_seed_has_tracking_auto_transition_disabled() {
        let settings = Settings::default();

        assert!(!settings.tracking_auto_transition_enabled);
        assert_eq!(settings.tracking_auto_transition_status_id, None);
    }

    #[test]
    fn deserializing_settings_without_tracking_auto_transition_fields_defaults_to_disabled() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert!(!settings.tracking_auto_transition_enabled);
        assert_eq!(settings.tracking_auto_transition_status_id, None);
    }

    #[test]
    fn tracking_auto_transition_fields_round_trip_when_set() {
        let settings = Settings {
            tracking_auto_transition_enabled: true,
            tracking_auto_transition_status_id: Some("in-progress".to_string()),
            ..Settings::default()
        };

        let json = serde_json::to_string(&settings).expect("serialization should succeed");
        let parsed: Settings = serde_json::from_str(&json).expect("parsing should succeed");

        assert!(parsed.tracking_auto_transition_enabled);
        assert_eq!(
            parsed.tracking_auto_transition_status_id,
            Some("in-progress".to_string())
        );
    }

    #[test]
    fn validate_settings_accepts_a_valid_tracking_auto_transition_status_id() {
        let settings = Settings {
            default_project_id: "some-project-id".to_string(),
            tracking_auto_transition_enabled: true,
            tracking_auto_transition_status_id: Some("in-progress".to_string()),
            ..Settings::default()
        };

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_rejects_an_unknown_tracking_auto_transition_status_id() {
        let settings = Settings {
            default_project_id: "some-project-id".to_string(),
            tracking_auto_transition_enabled: true,
            tracking_auto_transition_status_id: Some("on-hold".to_string()),
            ..Settings::default()
        };

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("on-hold"));
    }

    #[test]
    fn validate_settings_accepts_a_missing_tracking_auto_transition_status_id() {
        let settings = Settings {
            default_project_id: "some-project-id".to_string(),
            tracking_auto_transition_enabled: false,
            tracking_auto_transition_status_id: None,
            ..Settings::default()
        };

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn default_settings_seed_shows_the_cumulative_total_on_cards() {
        let settings = Settings::default();

        assert_eq!(settings.card_tracked_time_display, "total");
    }

    #[test]
    fn deserializing_settings_without_card_tracked_time_display_defaults_to_total() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert_eq!(settings.card_tracked_time_display, "total");
    }

    #[test]
    fn card_tracked_time_display_round_trips_when_set_to_session() {
        let settings = Settings {
            card_tracked_time_display: "session".to_string(),
            ..Settings::default()
        };

        let json = serde_json::to_string(&settings).expect("serialization should succeed");
        let parsed: Settings = serde_json::from_str(&json).expect("parsing should succeed");

        assert_eq!(parsed.card_tracked_time_display, "session");
    }

    #[test]
    fn validate_card_tracked_time_display_accepts_every_defined_mode() {
        for mode in CARD_TRACKED_TIME_DISPLAY_MODES {
            assert!(
                validate_card_tracked_time_display(mode).is_ok(),
                "{mode} should be valid"
            );
        }
    }

    #[test]
    fn validate_card_tracked_time_display_rejects_an_unknown_mode() {
        let err = validate_card_tracked_time_display("always-on").unwrap_err();
        assert!(err.contains("always-on"));
    }

    #[test]
    fn validate_settings_rejects_an_unknown_card_tracked_time_display() {
        let settings = Settings {
            default_project_id: "some-project-id".to_string(),
            card_tracked_time_display: "always-on".to_string(),
            ..Settings::default()
        };

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("always-on"));
    }

    #[test]
    fn validate_scheduled_relative_date_code_accepts_every_defined_code() {
        for code in SCHEDULED_RELATIVE_DATE_CODES {
            assert!(
                validate_scheduled_relative_date_code(code).is_ok(),
                "{code} should be valid"
            );
        }
    }

    #[test]
    fn validate_scheduled_relative_date_code_rejects_an_unknown_code() {
        let err = validate_scheduled_relative_date_code("next_quarter").unwrap_err();
        assert!(err.contains("next_quarter"));
    }

    #[test]
    fn validate_due_relative_date_code_accepts_every_defined_code() {
        for code in DUE_RELATIVE_DATE_CODES {
            assert!(
                validate_due_relative_date_code(code).is_ok(),
                "{code} should be valid"
            );
        }
    }

    #[test]
    fn validate_due_relative_date_code_rejects_an_unknown_code() {
        let err = validate_due_relative_date_code("next_quarter").unwrap_err();
        assert!(err.contains("next_quarter"));
    }

    #[test]
    fn resolve_scheduled_relative_date_resolves_today() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(resolve_scheduled_relative_date("today", today), Some(today));
    }

    #[test]
    fn resolve_scheduled_relative_date_resolves_tomorrow() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(
            resolve_scheduled_relative_date("tomorrow", today),
            chrono::NaiveDate::from_ymd_opt(2026, 6, 15)
        );
    }

    #[test]
    fn resolve_scheduled_relative_date_resolves_in_2_days() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(
            resolve_scheduled_relative_date("in_2_days", today),
            chrono::NaiveDate::from_ymd_opt(2026, 6, 16)
        );
    }

    #[test]
    fn resolve_scheduled_relative_date_resolves_in_3_days() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(
            resolve_scheduled_relative_date("in_3_days", today),
            chrono::NaiveDate::from_ymd_opt(2026, 6, 17)
        );
    }

    #[test]
    fn resolve_scheduled_relative_date_resolves_in_1_week() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(
            resolve_scheduled_relative_date("in_1_week", today),
            chrono::NaiveDate::from_ymd_opt(2026, 6, 21)
        );
    }

    #[test]
    fn resolve_scheduled_relative_date_resolves_in_1_month() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(
            resolve_scheduled_relative_date("in_1_month", today),
            chrono::NaiveDate::from_ymd_opt(2026, 7, 14)
        );
    }

    #[test]
    fn resolve_scheduled_relative_date_clamps_in_1_month_to_the_shorter_target_month() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();

        assert_eq!(
            resolve_scheduled_relative_date("in_1_month", today),
            chrono::NaiveDate::from_ymd_opt(2026, 2, 28)
        );
    }

    #[test]
    fn resolve_scheduled_relative_date_returns_none_for_an_unrecognized_code() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(resolve_scheduled_relative_date("next_quarter", today), None);
    }

    #[test]
    fn resolve_due_relative_date_resolves_none_as_never_due() {
        let scheduled = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(resolve_due_relative_date("none", scheduled), None);
    }

    #[test]
    fn resolve_due_relative_date_resolves_same_day() {
        let scheduled = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(
            resolve_due_relative_date("same_day", scheduled),
            Some(scheduled)
        );
    }

    #[test]
    fn resolve_due_relative_date_resolves_next_day() {
        let scheduled = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(
            resolve_due_relative_date("next_day", scheduled),
            chrono::NaiveDate::from_ymd_opt(2026, 6, 15)
        );
    }

    #[test]
    fn resolve_due_relative_date_resolves_in_2_days() {
        let scheduled = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(
            resolve_due_relative_date("in_2_days", scheduled),
            chrono::NaiveDate::from_ymd_opt(2026, 6, 16)
        );
    }

    #[test]
    fn resolve_due_relative_date_resolves_in_3_days() {
        let scheduled = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(
            resolve_due_relative_date("in_3_days", scheduled),
            chrono::NaiveDate::from_ymd_opt(2026, 6, 17)
        );
    }

    #[test]
    fn resolve_due_relative_date_resolves_in_1_week() {
        let scheduled = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(
            resolve_due_relative_date("in_1_week", scheduled),
            chrono::NaiveDate::from_ymd_opt(2026, 6, 21)
        );
    }

    #[test]
    fn resolve_due_relative_date_resolves_in_1_month() {
        let scheduled = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(
            resolve_due_relative_date("in_1_month", scheduled),
            chrono::NaiveDate::from_ymd_opt(2026, 7, 14)
        );
    }

    #[test]
    fn resolve_due_relative_date_clamps_in_1_month_to_the_shorter_target_month() {
        let scheduled = chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();

        assert_eq!(
            resolve_due_relative_date("in_1_month", scheduled),
            chrono::NaiveDate::from_ymd_opt(2026, 2, 28)
        );
    }

    #[test]
    fn resolve_due_relative_date_returns_none_for_an_unrecognized_code() {
        let scheduled = chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        assert_eq!(resolve_due_relative_date("next_quarter", scheduled), None);
    }

    #[test]
    fn normalize_fills_in_seeded_colors_for_legacy_settings_missing_color() {
        let json = r#"{
            "priorities": [
                {"id": "high", "label": "High", "rank": 1},
                {"id": "medium", "label": "Medium", "rank": 2},
                {"id": "low", "label": "Low", "rank": 3}
            ],
            "statuses": []
        }"#;
        let settings: Settings = serde_json::from_str(json).expect("parsing should succeed");
        assert!(settings
            .priorities
            .iter()
            .all(|p| p.color == default_priority_color()));

        let normalized = settings.normalize();

        assert_eq!(normalized.priorities, Settings::default().priorities);
    }

    #[test]
    fn normalize_leaves_a_custom_color_on_a_known_id_untouched() {
        let mut settings = Settings::default();
        settings.priorities[0].color = "#ff0000".to_string();

        let normalized = settings.clone().normalize();

        assert_eq!(normalized.priorities[0].color, "#ff0000");
    }

    #[test]
    fn normalize_leaves_an_unknown_id_on_the_fallback_color_unchanged() {
        let settings = Settings {
            priorities: vec![PriorityLevel {
                id: "urgent".to_string(),
                label: "Urgent".to_string(),
                color: default_priority_color(),
                rank: 1,
            }],
            statuses: Vec::new(),
            defaults: TaskDefaults::default(),
            ..Default::default()
        };

        let normalized = settings.normalize();

        assert_eq!(normalized.priorities[0].color, default_priority_color());
    }

    #[test]
    fn normalize_fills_in_seeded_colors_for_legacy_settings_missing_status_color() {
        let json = r#"{
            "priorities": [],
            "statuses": [
                {"id": "backlog", "label": "Backlog", "order": 1},
                {"id": "do", "label": "Do", "order": 2},
                {"id": "in-progress", "label": "In Progress", "order": 3},
                {"id": "blocked", "label": "Blocked", "order": 4},
                {"id": "done", "label": "Done", "order": 5}
            ]
        }"#;
        let settings: Settings = serde_json::from_str(json).expect("parsing should succeed");
        assert!(settings
            .statuses
            .iter()
            .all(|s| s.color == default_status_color()));

        let normalized = settings.normalize();

        assert_eq!(normalized.statuses, Settings::default().statuses);
    }

    #[test]
    fn normalize_leaves_a_custom_status_color_on_a_known_id_untouched() {
        let mut settings = Settings::default();
        settings.statuses[0].color = "#ff0000".to_string();

        let normalized = settings.clone().normalize();

        assert_eq!(normalized.statuses[0].color, "#ff0000");
    }

    #[test]
    fn normalize_leaves_an_unknown_status_id_on_the_fallback_color_unchanged() {
        let settings = Settings {
            priorities: Vec::new(),
            statuses: vec![StatusDefinition {
                id: "on-hold".to_string(),
                label: "On Hold".to_string(),
                order: 1,
                color: default_status_color(),
            }],
            defaults: TaskDefaults::default(),
            ..Default::default()
        };

        let normalized = settings.normalize();

        assert_eq!(normalized.statuses[0].color, default_status_color());
    }

    #[test]
    fn settings_round_trip_through_json() {
        let settings = Settings::default();

        let json = serde_json::to_string(&settings).expect("serialization should succeed");
        let parsed: Settings = serde_json::from_str(&json).expect("parsing should succeed");

        assert_eq!(parsed, settings);
    }

    #[test]
    fn task_defaults_defaults_to_empty_with_no_overrides() {
        let defaults = TaskDefaults::default();

        assert!(defaults.tags.is_empty());
        assert_eq!(defaults.priority, None);
        assert_eq!(defaults.status, None);
        assert_eq!(defaults.due, None);
        assert_eq!(defaults.scheduled, None);
        assert_eq!(defaults.estimated_minutes, None);
    }

    #[test]
    fn task_defaults_deserializes_an_estimated_minutes_override() {
        let json = r#"{"estimated_minutes": 30}"#;

        let defaults: TaskDefaults = serde_json::from_str(json).expect("parsing should succeed");

        assert_eq!(defaults.estimated_minutes, Some(30));
    }

    #[test]
    fn settings_with_missing_optional_fields_deserializes_with_defaults() {
        let json = r#"{"priorities":[],"statuses":[]}"#;

        let settings: Settings = serde_json::from_str(json).expect("parsing should succeed");

        assert!(settings.priorities.is_empty());
        assert!(settings.statuses.is_empty());
        assert_eq!(settings.defaults, TaskDefaults::default());
        assert_eq!(settings.done_status, "");
        assert_eq!(settings.cancelled_status, None);
    }

    #[test]
    fn normalize_migrates_legacy_settings_missing_done_status_to_status_id_done() {
        let json = r#"{
            "priorities": [],
            "statuses": [
                {"id": "backlog", "label": "Backlog", "order": 1},
                {"id": "do", "label": "Do", "order": 2},
                {"id": "done", "label": "Done", "order": 3}
            ]
        }"#;
        let settings: Settings = serde_json::from_str(json).expect("parsing should succeed");
        assert_eq!(settings.done_status, "");

        let normalized = settings.normalize();

        assert_eq!(normalized.done_status, "done");
    }

    #[test]
    fn normalize_falls_back_to_the_last_status_by_order_when_no_status_id_done_exists() {
        let json = r#"{
            "priorities": [],
            "statuses": [
                {"id": "backlog", "label": "Backlog", "order": 1},
                {"id": "complete", "label": "Complete", "order": 2}
            ]
        }"#;
        let settings: Settings = serde_json::from_str(json).expect("parsing should succeed");

        let normalized = settings.normalize();

        assert_eq!(normalized.done_status, "complete");
    }

    #[test]
    fn normalize_leaves_a_valid_done_status_unchanged() {
        let mut settings = Settings::default();
        settings.done_status = "backlog".to_string();

        let normalized = settings.normalize();

        assert_eq!(normalized.done_status, "backlog");
    }

    #[test]
    fn normalize_replaces_an_invalid_done_status_with_status_id_done() {
        let mut settings = Settings::default();
        settings.done_status = "nonexistent".to_string();

        let normalized = settings.normalize();

        assert_eq!(normalized.done_status, "done");
    }

    #[test]
    fn normalize_leaves_done_status_empty_when_there_are_no_statuses() {
        let settings = Settings {
            priorities: Vec::new(),
            statuses: Vec::new(),
            defaults: TaskDefaults::default(),
            done_status: String::new(),
            cancelled_status: None,
            default_project_id: String::new(),
            ..Default::default()
        };

        let normalized = settings.normalize();

        assert_eq!(normalized.done_status, "");
    }
}
