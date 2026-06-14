use serde::{Deserialize, Serialize};

/// Fallback color for a priority level that has no color of its own: used
/// for newly-created custom levels before the user picks a color, and as a
/// placeholder for `settings.json` files written before `PriorityLevel.color`
/// was mandatory (see `Settings::normalize`).
fn default_priority_color() -> String {
    "oklch(58% 0.012 60)".to_string()
}

/// Fallback color for a status that has no color of its own: used for
/// newly-created custom statuses before the user picks a color, and as a
/// placeholder for `settings.json` files written before
/// `StatusDefinition.color` was mandatory (see `Settings::normalize`).
fn default_status_color() -> String {
    "oklch(58% 0.012 60)".to_string()
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
}

/// Global, app-wide settings: the available priority levels, the global list
/// of statuses (from which each project's board is configured), and the
/// global default task attributes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    #[serde(default)]
    pub priorities: Vec<PriorityLevel>,
    #[serde(default)]
    pub statuses: Vec<StatusDefinition>,
    #[serde(default)]
    pub defaults: TaskDefaults,
}

impl Default for Settings {
    /// Seeds settings matching the app's previously hardcoded priority
    /// levels (low/medium/high) and statuses (backlog/do/in-progress/
    /// blocked/done), with new tasks defaulting to medium priority and the
    /// backlog status, as before.
    fn default() -> Self {
        Settings {
            priorities: vec![
                PriorityLevel {
                    id: "high".to_string(),
                    label: "High".to_string(),
                    color: "oklch(54% 0.2 350)".to_string(),
                    rank: 1,
                },
                PriorityLevel {
                    id: "medium".to_string(),
                    label: "Medium".to_string(),
                    color: "oklch(58% 0.13 70)".to_string(),
                    rank: 2,
                },
                PriorityLevel {
                    id: "low".to_string(),
                    label: "Low".to_string(),
                    color: "oklch(58% 0.14 155)".to_string(),
                    rank: 3,
                },
            ],
            statuses: vec![
                StatusDefinition {
                    id: "backlog".to_string(),
                    label: "Backlog".to_string(),
                    order: 1,
                    color: "oklch(55% 0.01 270)".to_string(),
                },
                StatusDefinition {
                    id: "do".to_string(),
                    label: "Do".to_string(),
                    order: 2,
                    color: "oklch(52% 0.16 235)".to_string(),
                },
                StatusDefinition {
                    id: "in-progress".to_string(),
                    label: "In Progress".to_string(),
                    order: 3,
                    color: "oklch(64% 0.14 75)".to_string(),
                },
                StatusDefinition {
                    id: "blocked".to_string(),
                    label: "Blocked".to_string(),
                    order: 4,
                    color: "oklch(54% 0.2 350)".to_string(),
                },
                StatusDefinition {
                    id: "done".to_string(),
                    label: "Done".to_string(),
                    order: 5,
                    color: "oklch(58% 0.14 155)".to_string(),
                },
            ],
            defaults: TaskDefaults {
                tags: Vec::new(),
                priority: Some("medium".to_string()),
                status: Some("backlog".to_string()),
                due: None,
            },
        }
    }
}

impl Settings {
    /// Fills in the seeded colors for the well-known default priority ids
    /// (`high`/`medium`/`low`) and status ids (`backlog`/`do`/`in-progress`/
    /// `blocked`/`done`) when they're still on the neutral fallback color.
    /// This recovers `settings.json` files written before
    /// `PriorityLevel.color`/`StatusDefinition.color` was mandatory, where
    /// the field was absent and serde supplied the fallback color on load.
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
        self
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

/// Validates settings before they're persisted: `priorities` and `statuses`
/// must each be non-empty with unique ids (an empty or duplicate-id list
/// would make `validate_priority_id`/`validate_status_id` reject every task
/// write, including the `resolve_default_priority`/`resolve_default_status`
/// fallbacks), and `defaults.priority`/`defaults.status`, if set, must
/// reference one of those ids.
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
        assert_eq!(settings.defaults.due, None);
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
        let settings = Settings::default();

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_rejects_empty_priorities() {
        let settings = Settings {
            priorities: Vec::new(),
            statuses: Vec::new(),
            defaults: TaskDefaults::default(),
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
        settings.defaults.priority = None;

        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn validate_settings_rejects_empty_statuses() {
        let settings = Settings {
            priorities: Settings::default().priorities,
            statuses: Vec::new(),
            defaults: TaskDefaults::default(),
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
        settings.defaults.status = None;

        assert!(validate_settings(&settings).is_ok());
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
    }

    #[test]
    fn settings_with_missing_optional_fields_deserializes_with_defaults() {
        let json = r#"{"priorities":[],"statuses":[]}"#;

        let settings: Settings = serde_json::from_str(json).expect("parsing should succeed");

        assert!(settings.priorities.is_empty());
        assert!(settings.statuses.is_empty());
        assert_eq!(settings.defaults, TaskDefaults::default());
    }
}
