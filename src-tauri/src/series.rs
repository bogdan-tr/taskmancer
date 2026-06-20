// Not yet wired into any `#[tauri::command]` — that's the next increment
// (creating a recurring task, the edit/delete "this and future" prompts,
// and the scroll-triggered lookahead command). Remove this once that
// command-layer work lands and makes these reachable.
#![allow(dead_code)]

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::settings::validate_due_relative_date_code;

/// A day of the week, `0` (Sunday) through `6` (Saturday) — matches the
/// frontend's `Date.getDay()` convention (see `weekRange.ts`) so the two
/// sides need no translation when mirroring this logic.
pub type Weekday = u8;

/// How often a series repeats. Each variant carries only the fields that
/// are actually meaningful for it (no shared generic "interval" field),
/// since the unit an interval counts in differs per variant — a shared
/// field would make invalid combinations representable, e.g. a
/// monthly-by-day rule with a meaningless "every N weeks".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum RecurrenceFrequency {
    /// Every `interval` days. `1` is daily, `2` is "every other day".
    EveryNDays { interval: u32 },
    /// On the given weekdays, every `interval_weeks` weeks. `1` is every
    /// week, `2` is "every other <weekday>". "Every weekday"/"every
    /// weekend" are expressed here too, as `weekdays: [1,2,3,4,5]` /
    /// `[0,6]` respectively — they're NL/UI shortcuts, not a distinct rule
    /// shape.
    Weekly {
        weekdays: Vec<Weekday>,
        interval_weeks: u32,
    },
    /// On the given day-of-month (`1..=31`). A month without that day
    /// (e.g. the 31st in February) is skipped entirely for that month, per
    /// the user's explicit choice — never clamped to the month's last day
    /// or rolled into the next month.
    MonthlyByDay { day: u32 },
}

/// Returns `Ok(())` if `frequency` is internally well-formed (non-empty
/// weekday list, in-range weekday/day-of-month values, a positive
/// interval), or an error describing the problem otherwise.
pub fn validate_recurrence_frequency(frequency: &RecurrenceFrequency) -> Result<(), String> {
    match frequency {
        RecurrenceFrequency::EveryNDays { interval } => {
            if *interval == 0 {
                return Err("recurrence interval must be at least 1 day".to_string());
            }
        }
        RecurrenceFrequency::Weekly {
            weekdays,
            interval_weeks,
        } => {
            if weekdays.is_empty() {
                return Err("a weekly recurrence must specify at least one weekday".to_string());
            }
            if weekdays.iter().any(|day| *day > 6) {
                return Err("weekday must be 0 (Sunday) through 6 (Saturday)".to_string());
            }
            if *interval_weeks == 0 {
                return Err("recurrence interval must be at least 1 week".to_string());
            }
        }
        RecurrenceFrequency::MonthlyByDay { day } => {
            if *day == 0 || *day > 31 {
                return Err("day-of-month must be between 1 and 31".to_string());
            }
        }
    }
    Ok(())
}

/// A recurring task's template and rule, mirroring `Project`/`Settings` in
/// being persisted as structured JSON (`series.json`) rather than markdown —
/// this is configuration the app generates and consumes, not a user note.
/// Each occurrence the rule produces is a fully independent, normal `Task`
/// (see `Task::series_id`), not a virtual/lazily-computed entry — see the
/// project log for why a dedicated template record beat the alternatives
/// (a single self-rescheduling task, or lazy virtual occurrences).
///
/// Template fields (`title`, `project`, `priority`, `tags`,
/// `estimated_minutes`, `notes`) are copied into every newly generated
/// occurrence and are what "this and future" edits update. `status` is
/// deliberately *not* a template field: a freshly generated occurrence gets
/// the normal default-status resolution every new task gets, but an
/// occurrence's status afterward (e.g. marking one instance done) is purely
/// its own and never bulk-applied to other occurrences, the same way
/// dragging one occurrence to a different date never prompts or affects any
/// other occurrence either.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Series {
    pub id: String,
    pub frequency: RecurrenceFrequency,
    /// The first occurrence's scheduled date (`YYYY-MM-DD`) — every later
    /// occurrence's date is computed relative to this anchor, not to
    /// "today", so regenerating or extending the lookahead window later
    /// always reproduces the same sequence of dates.
    pub anchor_date: String,
    /// Optional end date (`YYYY-MM-DD`, inclusive) — no occurrences are
    /// generated for a date after this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    /// A [`crate::settings::DUE_RELATIVE_DATE_CODES`] code each occurrence's
    /// due date is resolved against, relative to that occurrence's own
    /// scheduled date (via [`crate::settings::resolve_due_relative_date`]) —
    /// `None` means occurrences have no due date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_code: Option<String>,
    /// The last date (`YYYY-MM-DD`) occurrences have been generated through
    /// — the watermark the rolling lookahead window extends forward from.
    /// Generation only ever moves this forward and never revisits a date at
    /// or before it, so an occurrence the user deleted is never silently
    /// regenerated.
    pub generated_until: String,
    /// `false` once recurrence has been removed from this series via Edit.
    /// Stops generation of any further occurrences, but existing
    /// occurrences keep their `series_id` rather than having it severed —
    /// the user's explicit choice, so a future series-level report could
    /// still group them, even though they behave as fully independent
    /// tasks from this point on (no more "this and future" prompts, since
    /// those are keyed on the series being active).
    #[serde(default = "default_active")]
    pub active: bool,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub priority: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
    #[serde(default)]
    pub notes: String,
    pub created: String,
}

fn default_active() -> bool {
    true
}

impl Series {
    /// Creates a new active series with a freshly generated id and the
    /// current time as `created`. `generated_until` starts at `anchor_date`
    /// itself — the caller is expected to generate the first occurrence (at
    /// `anchor_date`) separately and then extend the window forward from
    /// there, the same way [`crate::project::Project::new`] doesn't create
    /// any tasks itself.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frequency: RecurrenceFrequency,
        anchor_date: String,
        end_date: Option<String>,
        due_code: Option<String>,
        title: String,
        project: Option<String>,
        priority: String,
        tags: Vec<String>,
        estimated_minutes: Option<u32>,
        notes: String,
    ) -> Self {
        Series {
            id: Uuid::new_v4().to_string(),
            frequency,
            generated_until: anchor_date.clone(),
            anchor_date,
            end_date,
            due_code,
            active: true,
            title,
            project,
            priority,
            tags,
            estimated_minutes,
            notes,
            created: Utc::now().to_rfc3339(),
        }
    }
}

/// Returns `Ok(())` if `series` is internally well-formed — its frequency
/// is valid (see [`validate_recurrence_frequency`]) and its `due_code`, if
/// set, is a recognized code (see
/// [`crate::settings::validate_due_relative_date_code`]) — or an error
/// describing the first problem found otherwise.
pub fn validate_series(series: &Series) -> Result<(), String> {
    validate_recurrence_frequency(&series.frequency)?;
    if let Some(due_code) = &series.due_code {
        validate_due_relative_date_code(due_code)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weekly(weekdays: Vec<Weekday>, interval_weeks: u32) -> RecurrenceFrequency {
        RecurrenceFrequency::Weekly {
            weekdays,
            interval_weeks,
        }
    }

    fn new_series(frequency: RecurrenceFrequency) -> Series {
        Series::new(
            frequency,
            "2026-06-15".to_string(),
            None,
            None,
            "Water the plants".to_string(),
            None,
            "medium".to_string(),
            vec![],
            None,
            String::new(),
        )
    }

    #[test]
    fn new_series_has_generated_id_and_created_timestamp() {
        let series = new_series(RecurrenceFrequency::EveryNDays { interval: 1 });

        assert!(!series.id.is_empty());
        assert!(!series.created.is_empty());
    }

    #[test]
    fn new_series_is_active_with_generated_until_at_the_anchor_date() {
        let series = new_series(RecurrenceFrequency::EveryNDays { interval: 1 });

        assert!(series.active);
        assert_eq!(series.generated_until, "2026-06-15");
    }

    #[test]
    fn new_series_has_no_end_date_or_due_code_by_default() {
        let series = new_series(RecurrenceFrequency::EveryNDays { interval: 1 });

        assert_eq!(series.end_date, None);
        assert_eq!(series.due_code, None);
    }

    #[test]
    fn to_json_then_from_json_round_trips() {
        let series = new_series(weekly(vec![1, 3, 5], 2));

        let json = serde_json::to_string(&series).expect("serialization should succeed");
        let parsed: Series = serde_json::from_str(&json).expect("parsing should succeed");

        assert_eq!(parsed, series);
    }

    #[test]
    fn active_defaults_to_true_when_absent_from_json() {
        let json = r##"{"id":"abc","frequency":{"kind":"EveryNDays","interval":1},"anchor_date":"2026-06-15","generated_until":"2026-06-15","title":"Water the plants","priority":"medium","created":"2026-06-15T10:00:00+00:00"}"##;

        let series: Series = serde_json::from_str(json).expect("parsing should succeed");

        assert!(series.active);
    }

    #[test]
    fn tags_and_notes_default_to_empty_when_absent_from_json() {
        let json = r##"{"id":"abc","frequency":{"kind":"EveryNDays","interval":1},"anchor_date":"2026-06-15","generated_until":"2026-06-15","title":"Water the plants","priority":"medium","created":"2026-06-15T10:00:00+00:00"}"##;

        let series: Series = serde_json::from_str(json).expect("parsing should succeed");

        assert_eq!(series.tags, Vec::<String>::new());
        assert_eq!(series.notes, "");
    }

    #[test]
    fn validate_recurrence_frequency_accepts_every_n_days_with_a_positive_interval() {
        assert!(
            validate_recurrence_frequency(&RecurrenceFrequency::EveryNDays { interval: 1 }).is_ok()
        );
        assert!(
            validate_recurrence_frequency(&RecurrenceFrequency::EveryNDays { interval: 5 }).is_ok()
        );
    }

    #[test]
    fn validate_recurrence_frequency_rejects_every_n_days_with_a_zero_interval() {
        let result =
            validate_recurrence_frequency(&RecurrenceFrequency::EveryNDays { interval: 0 });

        assert!(result.is_err());
    }

    #[test]
    fn validate_recurrence_frequency_accepts_a_well_formed_weekly_rule() {
        assert!(validate_recurrence_frequency(&weekly(vec![1, 3, 5], 1)).is_ok());
    }

    #[test]
    fn validate_recurrence_frequency_rejects_an_empty_weekday_list() {
        let result = validate_recurrence_frequency(&weekly(vec![], 1));

        assert!(result.is_err());
    }

    #[test]
    fn validate_recurrence_frequency_rejects_an_out_of_range_weekday() {
        let result = validate_recurrence_frequency(&weekly(vec![7], 1));

        assert!(result.is_err());
    }

    #[test]
    fn validate_recurrence_frequency_rejects_a_zero_week_interval() {
        let result = validate_recurrence_frequency(&weekly(vec![1], 0));

        assert!(result.is_err());
    }

    #[test]
    fn validate_recurrence_frequency_accepts_in_range_day_of_month() {
        assert!(
            validate_recurrence_frequency(&RecurrenceFrequency::MonthlyByDay { day: 1 }).is_ok()
        );
        assert!(
            validate_recurrence_frequency(&RecurrenceFrequency::MonthlyByDay { day: 31 }).is_ok()
        );
    }

    #[test]
    fn validate_recurrence_frequency_rejects_day_of_month_zero() {
        let result = validate_recurrence_frequency(&RecurrenceFrequency::MonthlyByDay { day: 0 });

        assert!(result.is_err());
    }

    #[test]
    fn validate_recurrence_frequency_rejects_day_of_month_above_31() {
        let result = validate_recurrence_frequency(&RecurrenceFrequency::MonthlyByDay { day: 32 });

        assert!(result.is_err());
    }

    #[test]
    fn validate_series_rejects_an_invalid_frequency() {
        let mut series = new_series(RecurrenceFrequency::EveryNDays { interval: 0 });
        series.due_code = None;

        let result = validate_series(&series);

        assert!(result.is_err());
    }

    #[test]
    fn validate_series_rejects_an_unrecognized_due_code() {
        let mut series = new_series(RecurrenceFrequency::EveryNDays { interval: 1 });
        series.due_code = Some("not_a_real_code".to_string());

        let result = validate_series(&series);

        assert!(result.is_err());
    }

    #[test]
    fn validate_series_accepts_a_recognized_due_code() {
        let mut series = new_series(RecurrenceFrequency::EveryNDays { interval: 1 });
        series.due_code = Some("next_day".to_string());

        assert!(validate_series(&series).is_ok());
    }

    #[test]
    fn validate_series_accepts_no_due_code() {
        let series = new_series(RecurrenceFrequency::EveryNDays { interval: 1 });

        assert!(validate_series(&series).is_ok());
    }
}
