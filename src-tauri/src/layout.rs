use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::settings::Settings;

/// The `StatLayout.kind` value for project status-line layouts — validated
/// by [`validate_status_layout`].
pub const STATUS_LINE_LAYOUT_KIND: &str = "status_line";

/// The `StatLayout.kind` value for analytics dashboard layouts — validated
/// by [`validate_dashboard_layout`].
pub const DASHBOARD_LAYOUT_KIND: &str = "dashboard";

/// The `StatLayout.kind` value for project-scoped dashboard layouts — one
/// per project, stored in the same `layouts.json` file, identified by the
/// `project_id` field. Validated by [`validate_project_dashboard_layout`].
pub const PROJECT_DASHBOARD_LAYOUT_KIND: &str = "project_dashboard";

/// The fixed set of widget ids a `"project_dashboard"` `StatLayout` entry
/// may reference — see the project-widgets feature spec for each widget's role.
/// Phase A implements W1–W6; Phase B slots are reserved here for validation
/// but not yet implemented in the frontend.
pub const KNOWN_PROJECT_WIDGET_IDS: &[&str] = &[
    "p_scoreboard",          // W1
    "p_health_pulse",        // W2
    "p_velocity",            // W3
    "p_completion_dial",     // W4
    "p_fuel_gauge",          // W5
    "p_effort_balance",      // W6
    "p_weekly_rhythm",       // W7 (Phase B)
    "p_time_donut",          // W9 (Phase B)
    "p_status_radial",       // W10 (Phase B)
    "p_due_timeline",        // W12 (Phase B)
    "p_burndown",            // W13 (Phase B)
    "p_completion_trend",    // W14 (Phase B)
    "p_subproject_tree",     // W16 (Phase B)
    "p_subproject_bars",     // W17 (Phase B)
    "p_subproject_sunburst", // W18 (Phase B)
];

/// The fixed set of widget ids a `"dashboard"` `StatLayout.stat_ids` entry
/// may reference — see the analytics dashboard feature spec for each widget's
/// role.  Order here matches the spec's catalog listing order and is also the
/// order seeded into the default layout by
/// [`ensure_default_dashboard_layout`].
pub const KNOWN_DASHBOARD_WIDGET_IDS: &[&str] = &[
    "completion_overview",
    "project_scale",
    "status_by_project",
    "project_health",
    "productivity",
];

/// The fixed set of stat ids a `"status_line"` `StatLayout.stat_ids` entry
/// may reference — see `crate::status_stats` for each stat's computation and
/// `docs/features/project-status-line.md`'s "Stat catalog". Order here
/// matches the spec's own catalog listing order, which is also the order
/// seeded into the default layout by [`ensure_default_status_line_layout`].
pub const KNOWN_STATUS_LINE_STAT_IDS: &[&str] = &[
    "status_badge",
    "estimated_time_left",
    "total_time_tracked",
    "avg_time_per_week",
    "completion_pct",
    "weighted_completion_pct",
    "mini_health",
    "mini_completion",
    "mini_fuel",
    "mini_sparkline",
];

/// A named, reusable arrangement of stats: today only the project status
/// line (`kind == "status_line"`), with the project-status-line spec's
/// "Layout system" section noting `"dashboard"` as a Phase 3 extension of
/// this same entity. `stat_ids` is ordered and only lists the stats
/// currently shown — toggling a stat off removes it from this list rather
/// than marking it disabled in place.
///
/// Editing a layout (rename, add/remove/reorder `stat_ids`) is meant to
/// mutate it in place so every `Settings.default_status_line_layout_id`/
/// `ProjectBoard.status_line_layout_id` referencing it sees the change
/// immediately ("named presets, shared on edit") — this struct itself has no
/// opinion on that; it's enforced by whichever command-layer save path
/// writes `layouts.json` (Milestone 2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatLayout {
    pub id: String,
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub stat_ids: Vec<String>,
    /// Superseded by `dashboard_widgets` for dashboard layouts. Kept for
    /// backward-compatible deserialization of Phase 3 layouts already on disk.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub widget_widths: HashMap<String, String>,
    /// Per-widget grid position, size, and config for dashboard layouts.
    /// Empty on status-line layouts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dashboard_widgets: Vec<DashboardWidget>,
    /// Visual theme for dashboard layouts: `"dark"` (default), `"app"`, or `"glass"`.
    /// `None` on status-line layouts and on dashboard layouts using the default theme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dashboard_theme: Option<String>,
    /// For `kind == "project_dashboard"` layouts: the id of the project this
    /// layout belongs to. `None` on status-line and global dashboard layouts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

/// One widget's position, size, and optional config in a dashboard grid.
/// `x` and `y` are zero-based column/row indices; `w` and `h` are the column-
/// span and row-span.  `include_subprojects` is only used by the
/// `"project_health"` widget (ignored by all others). `config` holds optional
/// per-widget settings (e.g. animation style for `"p_health_pulse"`), stored
/// as raw JSON so new fields don't require schema changes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardWidget {
    pub widget_type: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_subprojects: Option<bool>,
    /// Free-form per-widget config. Currently only used by `"p_health_pulse"`
    /// (`{"style":"static"|"ecg"|"pulse"}`). `None` means use defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

impl StatLayout {
    /// Creates a new status-line layout with a freshly generated id.
    pub fn new_status_line(name: String, stat_ids: Vec<String>) -> Self {
        StatLayout {
            id: Uuid::new_v4().to_string(),
            name,
            kind: STATUS_LINE_LAYOUT_KIND.to_string(),
            stat_ids,
            widget_widths: HashMap::new(),
            dashboard_widgets: Vec::new(),
            dashboard_theme: None,
            project_id: None,
        }
    }

    /// Creates a new dashboard layout with a freshly generated id.
    pub fn new_dashboard(name: String, widgets: Vec<DashboardWidget>) -> Self {
        StatLayout {
            id: Uuid::new_v4().to_string(),
            name,
            kind: DASHBOARD_LAYOUT_KIND.to_string(),
            stat_ids: Vec::new(),
            widget_widths: HashMap::new(),
            dashboard_widgets: widgets,
            dashboard_theme: None,
            project_id: None,
        }
    }

    /// Creates a new project-dashboard layout for `project_id` with a freshly
    /// generated id and the Phase A default four-widget arrangement.
    pub fn new_project_dashboard(project_id: &str) -> Self {
        StatLayout {
            id: Uuid::new_v4().to_string(),
            name: "Project Dashboard".to_string(),
            kind: PROJECT_DASHBOARD_LAYOUT_KIND.to_string(),
            stat_ids: Vec::new(),
            widget_widths: HashMap::new(),
            dashboard_widgets: vec![
                DashboardWidget { widget_type: "p_completion_dial".to_string(), x: 0, y: 0, w: 4, h: 4, include_subprojects: None, config: None },
                DashboardWidget { widget_type: "p_health_pulse".to_string(),    x: 4, y: 0, w: 4, h: 4, include_subprojects: None, config: None },
                DashboardWidget { widget_type: "p_scoreboard".to_string(),      x: 8, y: 0, w: 4, h: 4, include_subprojects: None, config: None },
                DashboardWidget { widget_type: "p_weekly_rhythm".to_string(),   x: 0, y: 4, w: 6, h: 4, include_subprojects: None, config: None },
            ],
            dashboard_theme: None,
            project_id: Some(project_id.to_string()),
        }
    }
}

/// Returns `Ok(())` if `layout` is internally well-formed: `kind` must be
/// [`STATUS_LINE_LAYOUT_KIND`] (the only kind currently supported — see that
/// constant's doc comment), and every entry in `stat_ids` must be one of
/// [`KNOWN_STATUS_LINE_STAT_IDS`]. Returns an error naming the first problem
/// found otherwise.
pub fn validate_status_layout(layout: &StatLayout) -> Result<(), String> {
    if layout.kind != STATUS_LINE_LAYOUT_KIND {
        return Err(format!("'{}' is not a supported layout kind", layout.kind));
    }

    if let Some(unknown) = layout
        .stat_ids
        .iter()
        .find(|id| !KNOWN_STATUS_LINE_STAT_IDS.contains(&id.as_str()))
    {
        return Err(format!("'{unknown}' is not a recognized stat id"));
    }

    Ok(())
}

/// Returns the seeded default status-line layout: all 6 known stats, in the
/// spec's own catalog order (see [`KNOWN_STATUS_LINE_STAT_IDS`]).
fn default_status_line_layout() -> StatLayout {
    StatLayout::new_status_line(
        "Default".to_string(),
        KNOWN_STATUS_LINE_STAT_IDS
            .iter()
            .map(|id| id.to_string())
            .collect(),
    )
}

/// Ensures `settings.default_status_line_layout_id` references a layout that
/// actually exists in `layouts`, creating the seeded default layout (see
/// [`default_status_line_layout`]) and pointing `settings` at it if it
/// doesn't. Mirrors `commands::ensure_default_project`'s "ensure a referenced
/// default resource actually exists" pattern exactly: covers both a
/// brand-new install (`default_status_line_layout_id` seeds as an empty
/// string — see `Settings::default`) and an upgrade from before this field
/// existed, where the id is just stale. Returns
/// `Some((updated_layouts, updated_settings))` if a layout was created (so
/// the caller knows both files need saving), or `None` if
/// `default_status_line_layout_id` already pointed at a real layout.
pub fn ensure_default_status_line_layout(
    layouts: Vec<StatLayout>,
    settings: Settings,
) -> Option<(Vec<StatLayout>, Settings)> {
    if layouts
        .iter()
        .any(|l| l.id == settings.default_status_line_layout_id)
    {
        return None;
    }

    let mut layouts = layouts;
    let mut settings = settings;
    let layout = default_status_line_layout();
    settings.default_status_line_layout_id = layout.id.clone();
    layouts.push(layout);
    Some((layouts, settings))
}

/// Returns `Ok(())` if `layout` is internally well-formed as a dashboard
/// layout: `kind` must be [`DASHBOARD_LAYOUT_KIND`] and every entry in
/// `dashboard_widgets` must have a `widget_type` that is one of
/// [`KNOWN_DASHBOARD_WIDGET_IDS`].
#[allow(dead_code)]
pub fn validate_dashboard_layout(layout: &StatLayout) -> Result<(), String> {
    if layout.kind != DASHBOARD_LAYOUT_KIND {
        return Err(format!("'{}' is not a dashboard layout kind", layout.kind));
    }

    if let Some(unknown) = layout
        .dashboard_widgets
        .iter()
        .find(|w| !KNOWN_DASHBOARD_WIDGET_IDS.contains(&w.widget_type.as_str()))
    {
        return Err(format!(
            "'{}' is not a recognized dashboard widget type",
            unknown.widget_type
        ));
    }

    Ok(())
}

/// Returns the seeded default dashboard layout: 5 widgets pre-placed on a
/// 12-column grid (see [`KNOWN_DASHBOARD_WIDGET_IDS`] for the full catalog).
fn default_dashboard_layout() -> StatLayout {
    StatLayout::new_dashboard(
        "Default".to_string(),
        vec![
            DashboardWidget { widget_type: "completion_overview".to_string(), x: 0, y: 0, w: 6, h: 4, include_subprojects: None, config: None },
            DashboardWidget { widget_type: "project_scale".to_string(), x: 6, y: 0, w: 6, h: 4, include_subprojects: None, config: None },
            DashboardWidget { widget_type: "status_by_project".to_string(), x: 0, y: 4, w: 4, h: 4, include_subprojects: None, config: None },
            DashboardWidget { widget_type: "project_health".to_string(), x: 4, y: 4, w: 3, h: 4, include_subprojects: Some(false), config: None },
            DashboardWidget { widget_type: "productivity".to_string(), x: 7, y: 4, w: 5, h: 4, include_subprojects: None, config: None },
        ],
    )
}

/// Ensures `settings.default_dashboard_layout_id` references a layout that
/// actually exists in `layouts`, creating the seeded default dashboard layout
/// (see [`default_dashboard_layout`]) and pointing `settings` at it if it
/// doesn't.  Mirrors [`ensure_default_status_line_layout`]'s pattern exactly.
/// Returns `Some((updated_layouts, updated_settings))` if a layout was created,
/// or `None` if `default_dashboard_layout_id` already pointed at a real layout.
pub fn ensure_default_dashboard_layout(
    layouts: Vec<StatLayout>,
    settings: Settings,
) -> Option<(Vec<StatLayout>, Settings)> {
    if layouts
        .iter()
        .any(|l| l.id == settings.default_dashboard_layout_id)
    {
        return None;
    }

    let mut layouts = layouts;
    let mut settings = settings;
    let layout = default_dashboard_layout();
    settings.default_dashboard_layout_id = layout.id.clone();
    layouts.push(layout);
    Some((layouts, settings))
}

/// Returns `Ok(())` if `layout` is internally well-formed as a project
/// dashboard layout: `kind` must be [`PROJECT_DASHBOARD_LAYOUT_KIND`],
/// `project_id` must be `Some`, and every entry in `dashboard_widgets` must
/// have a `widget_type` that is one of [`KNOWN_PROJECT_WIDGET_IDS`].
pub fn validate_project_dashboard_layout(layout: &StatLayout) -> Result<(), String> {
    if layout.kind != PROJECT_DASHBOARD_LAYOUT_KIND {
        return Err(format!(
            "'{}' is not a project_dashboard layout kind",
            layout.kind
        ));
    }
    if layout.project_id.is_none() {
        return Err("project_dashboard layout must have a project_id".to_string());
    }
    if let Some(unknown) = layout
        .dashboard_widgets
        .iter()
        .find(|w| !KNOWN_PROJECT_WIDGET_IDS.contains(&w.widget_type.as_str()))
    {
        return Err(format!(
            "'{}' is not a recognized project widget type",
            unknown.widget_type
        ));
    }
    Ok(())
}

/// Ensures a `project_dashboard` layout exists in `layouts` for `project_id`,
/// creating the default four-widget layout (see [`StatLayout::new_project_dashboard`])
/// if none is found. Returns the layout's `id` (whether pre-existing or freshly
/// created). If a new layout is created, it is pushed onto `layouts` so the
/// caller can persist the updated list.
pub fn ensure_default_project_dashboard_layout(
    project_id: &str,
    layouts: &mut Vec<StatLayout>,
) -> String {
    if let Some(existing) = layouts.iter().find(|l| {
        l.kind == PROJECT_DASHBOARD_LAYOUT_KIND
            && l.project_id.as_deref() == Some(project_id)
    }) {
        return existing.id.clone();
    }
    let layout = StatLayout::new_project_dashboard(project_id);
    let id = layout.id.clone();
    layouts.push(layout);
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_status_line_has_generated_id_and_status_line_kind() {
        let layout = StatLayout::new_status_line("My Layout".to_string(), vec![]);

        assert!(!layout.id.is_empty());
        assert_eq!(layout.kind, "status_line");
        assert_eq!(layout.name, "My Layout");
    }

    #[test]
    fn to_json_then_from_json_round_trips() {
        let layout = StatLayout::new_status_line(
            "Default".to_string(),
            vec!["status_badge".to_string(), "completion_pct".to_string()],
        );

        let json = serde_json::to_string(&layout).expect("serialization should succeed");
        let parsed: StatLayout = serde_json::from_str(&json).expect("parsing should succeed");

        assert_eq!(parsed, layout);
    }

    #[test]
    fn stat_ids_defaults_to_empty_when_absent_from_json() {
        let json = r#"{"id":"abc","name":"Default","kind":"status_line"}"#;

        let layout: StatLayout = serde_json::from_str(json).expect("parsing should succeed");

        assert!(layout.stat_ids.is_empty());
    }

    #[test]
    fn validate_status_layout_accepts_a_well_formed_status_line_layout() {
        let layout = StatLayout::new_status_line(
            "Default".to_string(),
            vec!["status_badge".to_string(), "completion_pct".to_string()],
        );

        assert!(validate_status_layout(&layout).is_ok());
    }

    #[test]
    fn validate_status_layout_accepts_an_empty_stat_ids_list() {
        let layout = StatLayout::new_status_line("Empty".to_string(), vec![]);

        assert!(validate_status_layout(&layout).is_ok());
    }

    #[test]
    fn validate_status_layout_accepts_every_known_stat_id() {
        let layout = StatLayout::new_status_line(
            "All".to_string(),
            KNOWN_STATUS_LINE_STAT_IDS
                .iter()
                .map(|id| id.to_string())
                .collect(),
        );

        assert!(validate_status_layout(&layout).is_ok());
    }

    #[test]
    fn validate_status_layout_rejects_an_unknown_stat_id() {
        let layout =
            StatLayout::new_status_line("Bad".to_string(), vec!["not_a_real_stat".to_string()]);

        let err = validate_status_layout(&layout).unwrap_err();
        assert!(err.contains("not_a_real_stat"));
    }

    #[test]
    fn validate_status_layout_rejects_an_arbitrary_unknown_kind() {
        let layout = StatLayout {
            id: "layout-1".to_string(),
            name: "Mystery".to_string(),
            kind: "widget_grid".to_string(),
            stat_ids: vec![],
            widget_widths: HashMap::new(),
            dashboard_widgets: Vec::new(),
            dashboard_theme: None,
            project_id: None,
        };

        let err = validate_status_layout(&layout).unwrap_err();
        assert!(err.contains("widget_grid"));
    }

    #[test]
    fn default_status_line_layout_contains_all_six_known_stats_in_catalog_order() {
        let layout = default_status_line_layout();

        assert_eq!(layout.stat_ids, KNOWN_STATUS_LINE_STAT_IDS);
    }

    #[test]
    fn ensure_default_status_line_layout_creates_one_when_none_exists() {
        let layouts: Vec<StatLayout> = Vec::new();
        let settings = Settings::default();

        let result = ensure_default_status_line_layout(layouts, settings);

        let (layouts, settings) = result.expect("should have created a default layout");
        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].stat_ids, KNOWN_STATUS_LINE_STAT_IDS);
        assert_eq!(settings.default_status_line_layout_id, layouts[0].id);
    }

    #[test]
    fn ensure_default_status_line_layout_does_nothing_when_default_already_exists() {
        let layout = StatLayout::new_status_line("Custom".to_string(), vec![]);
        let settings = Settings {
            default_status_line_layout_id: layout.id.clone(),
            ..Settings::default()
        };
        let layouts = vec![layout];

        let result = ensure_default_status_line_layout(layouts, settings);

        assert!(result.is_none());
    }

    #[test]
    fn ensure_default_status_line_layout_creates_one_when_configured_id_is_stale() {
        let layout = StatLayout::new_status_line("Custom".to_string(), vec![]);
        let settings = Settings {
            default_status_line_layout_id: "a-deleted-layout-id".to_string(),
            ..Settings::default()
        };
        let layouts = vec![layout];

        let result = ensure_default_status_line_layout(layouts, settings);

        let (layouts, settings) = result.expect("should have created a default layout");
        assert_eq!(layouts.len(), 2);
        assert!(layouts
            .iter()
            .any(|l| l.id == settings.default_status_line_layout_id));
    }

    #[test]
    fn ensure_default_status_line_layout_creates_one_for_a_brand_new_install() {
        // Settings::default()'s default_status_line_layout_id seeds empty,
        // mirroring default_project_id before ensure_default_project runs.
        let layouts: Vec<StatLayout> = Vec::new();
        let settings = Settings::default();
        assert_eq!(settings.default_status_line_layout_id, "");

        let result = ensure_default_status_line_layout(layouts, settings);

        assert!(result.is_some());
    }

    // --- Dashboard layout tests ---

    #[test]
    fn new_dashboard_has_generated_id_and_dashboard_kind() {
        let layout = StatLayout::new_dashboard("My Dashboard".to_string(), vec![]);

        assert!(!layout.id.is_empty());
        assert_eq!(layout.kind, DASHBOARD_LAYOUT_KIND);
        assert_eq!(layout.name, "My Dashboard");
        assert!(layout.widget_widths.is_empty());
        assert!(layout.dashboard_widgets.is_empty());
        assert!(layout.dashboard_theme.is_none());
    }

    #[test]
    fn widget_widths_omitted_from_json_when_empty() {
        let layout = StatLayout::new_dashboard("D".to_string(), vec![]);

        let json = serde_json::to_string(&layout).expect("should serialize");

        assert!(!json.contains("widget_widths"));
    }

    #[test]
    fn widget_widths_included_in_json_when_nonempty() {
        let mut layout = StatLayout::new_dashboard("D".to_string(), vec![]);
        layout
            .widget_widths
            .insert("completion_overview".to_string(), "2".to_string());

        let json = serde_json::to_string(&layout).expect("should serialize");

        assert!(json.contains("widget_widths"));
        assert!(json.contains("completion_overview"));
    }

    #[test]
    fn validate_dashboard_layout_accepts_a_well_formed_layout() {
        let widgets: Vec<DashboardWidget> = KNOWN_DASHBOARD_WIDGET_IDS
            .iter()
            .enumerate()
            .map(|(i, id)| DashboardWidget {
                widget_type: id.to_string(),
                x: (i as i32) * 2,
                y: 0,
                w: 2,
                h: 4,
                include_subprojects: None,
                config: None,
            })
            .collect();
        let layout = StatLayout::new_dashboard("Full".to_string(), widgets);

        assert!(validate_dashboard_layout(&layout).is_ok());
    }

    #[test]
    fn validate_dashboard_layout_accepts_an_empty_widget_list() {
        let layout = StatLayout::new_dashboard("Empty".to_string(), vec![]);

        assert!(validate_dashboard_layout(&layout).is_ok());
    }

    #[test]
    fn validate_dashboard_layout_rejects_a_non_dashboard_kind() {
        let layout = StatLayout {
            id: "x".to_string(),
            name: "X".to_string(),
            kind: "status_line".to_string(),
            stat_ids: vec![],
            widget_widths: HashMap::new(),
            dashboard_widgets: Vec::new(),
            dashboard_theme: None,
            project_id: None,
        };

        let err = validate_dashboard_layout(&layout).unwrap_err();
        assert!(err.contains("status_line"));
    }

    #[test]
    fn validate_dashboard_layout_rejects_an_unknown_widget_type() {
        let layout = StatLayout::new_dashboard(
            "Bad".to_string(),
            vec![DashboardWidget {
                widget_type: "not_a_widget".to_string(),
                x: 0,
                y: 0,
                w: 2,
                h: 2,
                include_subprojects: None,
                config: None,
            }],
        );

        let err = validate_dashboard_layout(&layout).unwrap_err();
        assert!(err.contains("not_a_widget"));
    }

    #[test]
    fn ensure_default_dashboard_layout_creates_one_when_none_exists() {
        let layouts: Vec<StatLayout> = Vec::new();
        let settings = Settings::default();

        let result = ensure_default_dashboard_layout(layouts, settings);

        let (layouts, settings) = result.expect("should have created a default layout");
        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].kind, DASHBOARD_LAYOUT_KIND);
        assert_eq!(layouts[0].dashboard_widgets.len(), 5);
        assert_eq!(settings.default_dashboard_layout_id, layouts[0].id);
    }

    #[test]
    fn ensure_default_dashboard_layout_does_nothing_when_default_already_exists() {
        let layout = StatLayout::new_dashboard("Custom".to_string(), vec![]);
        let settings = Settings {
            default_dashboard_layout_id: layout.id.clone(),
            ..Settings::default()
        };
        let layouts = vec![layout];

        let result = ensure_default_dashboard_layout(layouts, settings);

        assert!(result.is_none());
    }

    // --- Project dashboard layout tests ---

    #[test]
    fn new_project_dashboard_has_project_dashboard_kind_and_project_id() {
        let layout = StatLayout::new_project_dashboard("proj-1");

        assert!(!layout.id.is_empty());
        assert_eq!(layout.kind, PROJECT_DASHBOARD_LAYOUT_KIND);
        assert_eq!(layout.project_id, Some("proj-1".to_string()));
        assert_eq!(layout.dashboard_widgets.len(), 4);
    }

    #[test]
    fn validate_project_dashboard_layout_accepts_a_well_formed_layout() {
        let layout = StatLayout::new_project_dashboard("proj-1");

        assert!(validate_project_dashboard_layout(&layout).is_ok());
    }

    #[test]
    fn validate_project_dashboard_layout_rejects_wrong_kind() {
        let mut layout = StatLayout::new_project_dashboard("proj-1");
        layout.kind = "dashboard".to_string();

        let err = validate_project_dashboard_layout(&layout).unwrap_err();
        assert!(err.contains("project_dashboard"));
    }

    #[test]
    fn validate_project_dashboard_layout_rejects_missing_project_id() {
        let mut layout = StatLayout::new_project_dashboard("proj-1");
        layout.project_id = None;

        let err = validate_project_dashboard_layout(&layout).unwrap_err();
        assert!(err.contains("project_id"));
    }

    #[test]
    fn validate_project_dashboard_layout_rejects_unknown_widget_type() {
        let mut layout = StatLayout::new_project_dashboard("proj-1");
        layout.dashboard_widgets.push(DashboardWidget {
            widget_type: "unknown_widget".to_string(),
            x: 0, y: 0, w: 4, h: 4,
            include_subprojects: None,
            config: None,
        });

        let err = validate_project_dashboard_layout(&layout).unwrap_err();
        assert!(err.contains("unknown_widget"));
    }

    #[test]
    fn ensure_default_project_dashboard_layout_creates_one_when_none_exists() {
        let mut layouts: Vec<StatLayout> = Vec::new();

        let id = ensure_default_project_dashboard_layout("proj-1", &mut layouts);

        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].kind, PROJECT_DASHBOARD_LAYOUT_KIND);
        assert_eq!(layouts[0].project_id, Some("proj-1".to_string()));
        assert_eq!(id, layouts[0].id);
    }

    #[test]
    fn ensure_default_project_dashboard_layout_returns_existing_without_creating() {
        let existing = StatLayout::new_project_dashboard("proj-1");
        let existing_id = existing.id.clone();
        let mut layouts = vec![existing];

        let id = ensure_default_project_dashboard_layout("proj-1", &mut layouts);

        assert_eq!(layouts.len(), 1);
        assert_eq!(id, existing_id);
    }

    #[test]
    fn ensure_default_project_dashboard_layout_creates_separately_per_project() {
        let layout_a = StatLayout::new_project_dashboard("proj-a");
        let mut layouts = vec![layout_a];

        ensure_default_project_dashboard_layout("proj-b", &mut layouts);

        assert_eq!(layouts.len(), 2);
        assert!(layouts.iter().any(|l| l.project_id.as_deref() == Some("proj-a")));
        assert!(layouts.iter().any(|l| l.project_id.as_deref() == Some("proj-b")));
    }
}
