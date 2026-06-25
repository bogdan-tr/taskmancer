use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::settings::Settings;

/// The only `StatLayout.kind` value accepted today. `"dashboard"` is named in
/// the project-status-line spec as a future `kind` sharing this same entity
/// (Phase 3's analytics dashboards), but that kind's widget catalog doesn't
/// exist yet — validating for it now would be speculative, so
/// [`validate_status_layout`] rejects it like any other unrecognized kind
/// until Phase 3 actually extends this list.
pub const STATUS_LINE_LAYOUT_KIND: &str = "status_line";

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
}

impl StatLayout {
    /// Creates a new status-line layout with a freshly generated id.
    pub fn new_status_line(name: String, stat_ids: Vec<String>) -> Self {
        StatLayout {
            id: Uuid::new_v4().to_string(),
            name,
            kind: STATUS_LINE_LAYOUT_KIND.to_string(),
            stat_ids,
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
    fn validate_status_layout_rejects_the_dashboard_kind() {
        let layout = StatLayout {
            id: "layout-1".to_string(),
            name: "Future Dashboard".to_string(),
            kind: "dashboard".to_string(),
            stat_ids: vec![],
        };

        let err = validate_status_layout(&layout).unwrap_err();
        assert!(err.contains("dashboard"));
    }

    #[test]
    fn validate_status_layout_rejects_an_arbitrary_unknown_kind() {
        let layout = StatLayout {
            id: "layout-1".to_string(),
            name: "Mystery".to_string(),
            kind: "widget_grid".to_string(),
            stat_ids: vec![],
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
}
