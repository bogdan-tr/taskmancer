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

/// The only accepted value for `Settings.status_bar_style`. Chips and tint
/// were removed after the initial implementation in favor of a per-bar tint
/// toggle (`Settings.status_bar_tile_tint`). Legacy settings files that still
/// carry `"chips"` or `"tint"` are migrated to `"tiles"` by
/// [`Settings::normalize`] on load.
pub const STATUS_BAR_STYLES: &[&str] = &["tiles"];

/// Default `Settings.status_bar_style`: the spec's seeded default.
fn default_status_bar_style() -> String {
    "tiles".to_string()
}

/// Default `Settings.avg_time_per_week_window`: the trailing-week count the
/// `avg_time_per_week` status-line stat averages over, per the
/// project-status-line spec's stat catalog entry.
fn default_avg_time_per_week_window() -> u32 {
    4
}

/// Default `Settings.status_bar_enabled`: the status bar is shown by default.
fn default_status_bar_enabled() -> bool {
    true
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

/// One status-line health tier's condition set (see
/// `crate::status_tier` for the evaluator and
/// `docs/features/project-status-line.md`'s "Status algorithm"). Every
/// condition the tier has set must match for the tier to match (AND); an
/// unset (`None`) condition is simply skipped for that tier, so a tier can
/// use just one of the three fields if that's all it needs.
///
/// `due_within_days`: matches if any of the project's own qualifying tasks
/// has a `due` date `<= today + due_within_days` (zero or negative catches
/// overdue/due-today).
///
/// `min_priority`: matches if any qualifying task's priority has a `rank`
/// less-than-or-equal-to (i.e. at least as severe as) the named
/// `PriorityLevel.id`'s `rank`. An `id` that doesn't match any current
/// priority level never matches (fails closed rather than erroring), mirroring
/// how this codebase already degrades gracefully when a setting references a
/// since-removed id elsewhere (e.g. `Task.priority`/`Task.status` reads).
///
/// `estimated_time_left_exceeds_minutes`: matches if the project's own
/// already-computed `estimated_time_left` stat value is strictly greater
/// than this many minutes.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StatusTierRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_within_days: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_priority: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_time_left_exceeds_minutes: Option<u32>,
}

/// Seeds `Settings.default_status_tier_rules`: exactly 4 entries in
/// `[severe, critical, needs_attention, on_track]` order, matching the
/// project-status-line spec's "Seeded defaults" table.
fn default_status_tier_rules() -> Vec<StatusTierRule> {
    vec![
        StatusTierRule {
            due_within_days: Some(0),
            min_priority: None,
            estimated_time_left_exceeds_minutes: None,
        },
        StatusTierRule {
            due_within_days: Some(1),
            min_priority: Some("high".to_string()),
            estimated_time_left_exceeds_minutes: None,
        },
        StatusTierRule {
            due_within_days: Some(3),
            min_priority: None,
            estimated_time_left_exceeds_minutes: None,
        },
        StatusTierRule {
            due_within_days: Some(7),
            min_priority: None,
            estimated_time_left_exceeds_minutes: None,
        },
    ]
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
///
/// `default_status_tier_rules` is the project-status-line feature's global
/// tier thresholds (see [`StatusTierRule`] and `crate::status_tier`): always
/// exactly 4 entries in `[severe, critical, needs_attention, on_track]`
/// order (enforced by [`validate_settings`]). A project's
/// `ProjectBoard.status_tier_rule_overrides` overrides individual slots when
/// set — see that field's own doc comment.
///
/// `avg_time_per_week_window` is how many trailing complete weeks the
/// `avg_time_per_week` status-line stat averages over; must be `> 0`
/// (enforced by [`validate_settings`]). No per-project override, per the
/// project-status-line spec.
///
/// `default_status_line_layout_id` points at the seeded built-in
/// `StatLayout` (see `crate::layout`) shown by default on a project's status
/// line. Empty only transiently, before the startup migration ensures a
/// real default layout exists and sets this to its id — mirrors
/// `default_project_id`/`commands::ensure_default_project` exactly.
///
/// `status_bar_enabled` — when `false`, the project status bar is hidden on
/// every project board. Per-project boards can override this with
/// `ProjectBoard.status_bar_enabled_override` (`None` = inherit, `Some(true)`
/// = force on, `Some(false)` = force off). Defaults to `true`.
///
/// `status_bar_tile_tint` — when `true`, the tiles-style bar's background
/// carries a soft color wash that shifts with the current status tier (the
/// visual effect formerly provided by the removed `"tint"` bar style, now
/// available as an opt-in toggle on the only remaining style). Defaults to
/// `false`.
///
/// `status_bar_style` is one of [`STATUS_BAR_STYLES`] (always `"tiles"` after
/// the chips/tint styles were removed — kept in the struct for legacy
/// round-trip compatibility and migrated to `"tiles"` on load by
/// [`Settings::normalize`]).
///
/// `vim` holds the global vim-navigation settings (master enable switch and
/// optional per-action keybinding overrides). Absent from settings files
/// written before this field was added; [`VimSettings::default`] supplies the
/// correct zero-state (`enabled: false`, no overrides) via `#[serde(default)]`.

/// One action → zero-or-more key-combo mapping for vim navigation.
/// `action_id` matches the `VimActionId` constants on the frontend.
/// `combos` format: `"j"`, `"G"`, `"Ctrl+k"`, `"Shift+D"`, `"ArrowLeft"`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct VimKeybinding {
    pub action_id: String,
    pub combos: Vec<String>,
}

/// Global vim navigation settings. `enabled` is the master switch.
/// Keybindings here OVERRIDE the frontend defaults; absent bindings use defaults.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct VimSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub keybindings: Vec<VimKeybinding>,
    #[serde(default)]
    pub status_keybindings: Vec<VimKeybinding>,
}

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
    #[serde(default = "default_status_tier_rules")]
    pub default_status_tier_rules: Vec<StatusTierRule>,
    #[serde(default = "default_avg_time_per_week_window")]
    pub avg_time_per_week_window: u32,
    #[serde(default)]
    pub default_status_line_layout_id: String,
    #[serde(default = "default_status_bar_style")]
    pub status_bar_style: String,
    #[serde(default = "default_status_bar_enabled")]
    pub status_bar_enabled: bool,
    #[serde(default)]
    pub status_bar_tile_tint: bool,
    #[serde(default)]
    pub default_dashboard_layout_id: String,
    #[serde(default)]
    pub vim: VimSettings,
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
            default_status_tier_rules: default_status_tier_rules(),
            avg_time_per_week_window: default_avg_time_per_week_window(),
            default_status_line_layout_id: String::new(),
            status_bar_style: default_status_bar_style(),
            status_bar_enabled: default_status_bar_enabled(),
            status_bar_tile_tint: false,
            default_dashboard_layout_id: String::new(),
            vim: VimSettings::default(),
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

        // Migrate removed bar styles to the only remaining style "tiles".
        if !STATUS_BAR_STYLES.contains(&self.status_bar_style.as_str()) {
            self.status_bar_style = default_status_bar_style();
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

/// Returns `Ok(())` if `value` is one of [`STATUS_BAR_STYLES`], or an error
/// naming the unrecognized value otherwise. Used for `Settings.status_bar_style`.
pub fn validate_status_bar_style(value: &str) -> Result<(), String> {
    if STATUS_BAR_STYLES.contains(&value) {
        Ok(())
    } else {
        Err(format!("'{value}' is not a recognized status bar style"))
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
/// `tracking_auto_transition_status_id`, if set, must reference a
/// defined status, `default_status_tier_rules` must have exactly 4 entries
/// (`[severe, critical, needs_attention, on_track]` — a `min_priority` on
/// any entry is deliberately *not* validated against `priorities` here, since
/// it's meant to fail closed rather than reject a save when it references a
/// since-removed level, per `crate::status_tier`'s evaluator),
/// `avg_time_per_week_window` must be `> 0`, `default_status_line_layout_id`
/// must be non-empty, and `status_bar_style` must be one of
/// [`STATUS_BAR_STYLES`].
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

    if settings.default_status_tier_rules.len() != 4 {
        return Err(format!(
            "default_status_tier_rules must have exactly 4 entries, got {}",
            settings.default_status_tier_rules.len()
        ));
    }

    if settings.avg_time_per_week_window == 0 {
        return Err("avg_time_per_week_window must be greater than 0".to_string());
    }

    if settings.default_status_line_layout_id.is_empty() {
        return Err("a default status line layout must be defined".to_string());
    }

    validate_status_bar_style(&settings.status_bar_style)?;

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

    /// Fills in every other entity-existence field [`validate_settings`]
    /// requires to be non-empty (`default_project_id`,
    /// `default_status_line_layout_id`) so tests targeting one specific
    /// validation rule don't have to restate this boilerplate.
    fn settings_with_required_ids_set() -> Settings {
        Settings {
            default_project_id: "some-project-id".to_string(),
            default_status_line_layout_id: "some-layout-id".to_string(),
            ..Settings::default()
        }
    }

    #[test]
    fn validate_settings_accepts_default_settings() {
        let settings = settings_with_required_ids_set();

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
        let mut settings = settings_with_required_ids_set();
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
        let mut settings = settings_with_required_ids_set();
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
        let mut settings = settings_with_required_ids_set();
        settings.done_status = "backlog".to_string();

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_accepts_a_missing_cancelled_status() {
        let mut settings = settings_with_required_ids_set();
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
        let mut settings = settings_with_required_ids_set();
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
        let mut settings = settings_with_required_ids_set();
        settings.defaults.due = Some("next_day".to_string());

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_accepts_a_missing_default_due() {
        let mut settings = settings_with_required_ids_set();
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
        let mut settings = settings_with_required_ids_set();
        settings.defaults.scheduled = Some("in_1_week".to_string());

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_accepts_a_missing_default_scheduled() {
        let mut settings = settings_with_required_ids_set();
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
        let settings = settings_with_required_ids_set();

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
        let settings = settings_with_required_ids_set();

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
                ..settings_with_required_ids_set()
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
            tracking_auto_transition_enabled: true,
            tracking_auto_transition_status_id: Some("in-progress".to_string()),
            ..settings_with_required_ids_set()
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
            tracking_auto_transition_enabled: false,
            tracking_auto_transition_status_id: None,
            ..settings_with_required_ids_set()
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
            card_tracked_time_display: "always-on".to_string(),
            ..settings_with_required_ids_set()
        };

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("always-on"));
    }

    #[test]
    fn default_settings_seed_has_four_status_tier_rules_in_severity_order() {
        let settings = Settings::default();

        assert_eq!(settings.default_status_tier_rules.len(), 4);
        // [severe, critical, needs_attention, on_track]
        assert_eq!(
            settings.default_status_tier_rules[0].due_within_days,
            Some(0)
        );
        assert_eq!(
            settings.default_status_tier_rules[1].due_within_days,
            Some(1)
        );
        assert_eq!(
            settings.default_status_tier_rules[1].min_priority,
            Some("high".to_string())
        );
        assert_eq!(
            settings.default_status_tier_rules[2].due_within_days,
            Some(3)
        );
        assert_eq!(
            settings.default_status_tier_rules[3].due_within_days,
            Some(7)
        );
    }

    #[test]
    fn default_status_tier_rules_have_no_estimated_time_left_or_min_priority_except_critical() {
        let settings = Settings::default();

        for (index, rule) in settings.default_status_tier_rules.iter().enumerate() {
            assert_eq!(rule.estimated_time_left_exceeds_minutes, None);
            if index != 1 {
                assert_eq!(rule.min_priority, None);
            }
        }
    }

    #[test]
    fn deserializing_settings_without_default_status_tier_rules_seeds_the_defaults() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert_eq!(
            settings.default_status_tier_rules,
            default_status_tier_rules()
        );
    }

    #[test]
    fn status_tier_rule_round_trips_through_json_when_every_field_is_set() {
        let rule = StatusTierRule {
            due_within_days: Some(2),
            min_priority: Some("high".to_string()),
            estimated_time_left_exceeds_minutes: Some(120),
        };

        let json = serde_json::to_string(&rule).expect("serialization should succeed");
        let parsed: StatusTierRule = serde_json::from_str(&json).expect("parsing should succeed");

        assert_eq!(parsed, rule);
    }

    #[test]
    fn status_tier_rule_defaults_to_every_condition_unset() {
        let rule = StatusTierRule::default();

        assert_eq!(rule.due_within_days, None);
        assert_eq!(rule.min_priority, None);
        assert_eq!(rule.estimated_time_left_exceeds_minutes, None);
    }

    #[test]
    fn status_tier_rule_omits_unset_fields_when_serialized() {
        let rule = StatusTierRule::default();

        let json = serde_json::to_string(&rule).expect("serialization should succeed");

        assert_eq!(json, "{}");
    }

    #[test]
    fn validate_settings_rejects_default_status_tier_rules_with_too_few_entries() {
        let settings = Settings {
            default_status_tier_rules: vec![StatusTierRule::default(); 3],
            ..settings_with_required_ids_set()
        };

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("default_status_tier_rules"));
    }

    #[test]
    fn validate_settings_rejects_default_status_tier_rules_with_too_many_entries() {
        let settings = Settings {
            default_status_tier_rules: vec![StatusTierRule::default(); 5],
            ..settings_with_required_ids_set()
        };

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("default_status_tier_rules"));
    }

    #[test]
    fn validate_settings_accepts_exactly_four_status_tier_rules() {
        let settings = Settings {
            default_status_tier_rules: vec![StatusTierRule::default(); 4],
            ..settings_with_required_ids_set()
        };

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_accepts_a_status_tier_rule_with_min_priority_referencing_a_removed_level()
    {
        // min_priority is deliberately not cross-checked against `priorities`
        // here — it's meant to fail closed at evaluation time, not reject
        // the settings save, per crate::status_tier's evaluator.
        let mut rules = default_status_tier_rules();
        rules[0].min_priority = Some("a-priority-that-was-deleted".to_string());
        let settings = Settings {
            default_status_tier_rules: rules,
            ..settings_with_required_ids_set()
        };

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn default_settings_seed_has_an_avg_time_per_week_window_of_four() {
        let settings = Settings::default();

        assert_eq!(settings.avg_time_per_week_window, 4);
    }

    #[test]
    fn deserializing_settings_without_avg_time_per_week_window_defaults_to_four() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert_eq!(settings.avg_time_per_week_window, 4);
    }

    #[test]
    fn validate_settings_rejects_a_zero_avg_time_per_week_window() {
        let settings = Settings {
            avg_time_per_week_window: 0,
            ..settings_with_required_ids_set()
        };

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("avg_time_per_week_window"));
    }

    #[test]
    fn validate_settings_accepts_an_avg_time_per_week_window_of_one() {
        let settings = Settings {
            avg_time_per_week_window: 1,
            ..settings_with_required_ids_set()
        };

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn default_settings_seed_has_an_empty_default_status_line_layout_id() {
        let settings = Settings::default();

        assert_eq!(settings.default_status_line_layout_id, "");
    }

    #[test]
    fn deserializing_settings_without_default_status_line_layout_id_leaves_it_empty() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert_eq!(settings.default_status_line_layout_id, "");
    }

    #[test]
    fn validate_settings_rejects_an_empty_default_status_line_layout_id() {
        let settings = Settings {
            default_status_line_layout_id: String::new(),
            ..settings_with_required_ids_set()
        };

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("status line layout"));
    }

    #[test]
    fn validate_settings_accepts_a_non_empty_default_status_line_layout_id() {
        let settings = settings_with_required_ids_set();

        assert!(validate_settings(&settings).is_ok());
        assert_eq!(settings.default_status_line_layout_id, "some-layout-id");
    }

    #[test]
    fn default_settings_seed_has_a_status_bar_style_of_tiles() {
        let settings = Settings::default();

        assert_eq!(settings.status_bar_style, "tiles");
    }

    #[test]
    fn deserializing_settings_without_status_bar_style_defaults_to_tiles() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert_eq!(settings.status_bar_style, "tiles");
    }

    #[test]
    fn validate_status_bar_style_accepts_every_defined_style() {
        for style in STATUS_BAR_STYLES {
            assert!(
                validate_status_bar_style(style).is_ok(),
                "{style} should be valid"
            );
        }
    }

    #[test]
    fn validate_status_bar_style_rejects_an_unknown_style() {
        let err = validate_status_bar_style("neon").unwrap_err();
        assert!(err.contains("neon"));
    }

    #[test]
    fn validate_settings_rejects_an_unknown_status_bar_style() {
        let settings = Settings {
            status_bar_style: "neon".to_string(),
            ..settings_with_required_ids_set()
        };

        let err = validate_settings(&settings).unwrap_err();
        assert!(err.contains("status bar style"));
    }

    #[test]
    fn validate_settings_accepts_every_defined_status_bar_style() {
        for style in STATUS_BAR_STYLES {
            let settings = Settings {
                status_bar_style: style.to_string(),
                ..settings_with_required_ids_set()
            };

            assert!(
                validate_settings(&settings).is_ok(),
                "{style} should be valid"
            );
        }
    }

    #[test]
    fn normalize_migrates_removed_chips_style_to_tiles() {
        let settings = Settings {
            status_bar_style: "chips".to_string(),
            ..Settings::default()
        };
        let normalized = settings.normalize();
        assert_eq!(normalized.status_bar_style, "tiles");
    }

    #[test]
    fn normalize_migrates_removed_tint_style_to_tiles() {
        let settings = Settings {
            status_bar_style: "tint".to_string(),
            ..Settings::default()
        };
        let normalized = settings.normalize();
        assert_eq!(normalized.status_bar_style, "tiles");
    }

    #[test]
    fn default_settings_have_status_bar_enabled_true() {
        assert!(Settings::default().status_bar_enabled);
    }

    #[test]
    fn default_settings_have_status_bar_tile_tint_false() {
        assert!(!Settings::default().status_bar_tile_tint);
    }

    #[test]
    fn deserializing_settings_without_status_bar_enabled_defaults_to_true() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(settings.status_bar_enabled);
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

    #[test]
    fn default_settings_seed_has_an_empty_default_dashboard_layout_id() {
        let settings = Settings::default();

        assert_eq!(settings.default_dashboard_layout_id, "");
    }

    // ── VimSettings tests ────────────────────────────────────────────────────

    #[test]
    fn vim_settings_default_is_disabled_with_no_keybindings() {
        let vim = VimSettings::default();

        assert!(!vim.enabled);
        assert!(vim.keybindings.is_empty());
        assert!(vim.status_keybindings.is_empty());
    }

    #[test]
    fn deserializing_settings_without_vim_key_supplies_disabled_defaults() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert!(!settings.vim.enabled);
        assert!(settings.vim.keybindings.is_empty());
        assert!(settings.vim.status_keybindings.is_empty());
    }

    #[test]
    fn vim_settings_with_enabled_and_keybinding_round_trips_through_serde() {
        let vim = VimSettings {
            enabled: true,
            keybindings: vec![VimKeybinding {
                action_id: "move_down".to_string(),
                combos: vec!["j".to_string(), "Ctrl+j".to_string()],
            }],
            status_keybindings: vec![],
        };

        let json = serde_json::to_string(&vim).expect("serialization should succeed");
        let parsed: VimSettings = serde_json::from_str(&json).expect("parsing should succeed");

        assert_eq!(parsed, vim);
        assert!(parsed.enabled);
        assert_eq!(parsed.keybindings.len(), 1);
        assert_eq!(parsed.keybindings[0].action_id, "move_down");
        assert_eq!(parsed.keybindings[0].combos, vec!["j", "Ctrl+j"]);
    }

    #[test]
    fn vim_keybinding_serializes_with_action_id_and_combos_keys() {
        let binding = VimKeybinding {
            action_id: "go_to_top".to_string(),
            combos: vec!["gg".to_string()],
        };

        let json = serde_json::to_string(&binding).expect("serialization should succeed");

        assert!(json.contains("\"action_id\""));
        assert!(json.contains("\"combos\""));
        assert!(json.contains("\"go_to_top\""));
        assert!(json.contains("\"gg\""));
    }

    #[test]
    fn settings_with_vim_field_round_trips_through_json() {
        let settings = Settings {
            vim: VimSettings {
                enabled: true,
                keybindings: vec![VimKeybinding {
                    action_id: "edit_task".to_string(),
                    combos: vec!["e".to_string()],
                }],
                status_keybindings: vec![VimKeybinding {
                    action_id: "done-status-id".to_string(),
                    combos: vec!["d".to_string()],
                }],
            },
            ..Settings::default()
        };

        let json = serde_json::to_string(&settings).expect("serialization should succeed");
        let parsed: Settings = serde_json::from_str(&json).expect("parsing should succeed");

        assert_eq!(parsed.vim, settings.vim);
        assert!(parsed.vim.enabled);
        assert_eq!(parsed.vim.keybindings[0].action_id, "edit_task");
        assert_eq!(parsed.vim.status_keybindings[0].action_id, "done-status-id");
    }
}
