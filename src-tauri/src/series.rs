use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A day of the week, `0` (Sunday) through `6` (Saturday) — matches the
/// frontend's `Date.getDay()` convention (see `weekRange.ts`) so the two
/// sides need no translation when mirroring this logic.
pub type Weekday = u8;

/// How a recurring task's due date relates to each occurrence's own
/// scheduled date (never to "today", and never a single fixed absolute
/// date — that wouldn't make sense once there's more than one occurrence).
/// Resolved per-occurrence by [`crate::recurrence::resolve_due_rule`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum DueRule {
    /// No due date for any occurrence — the `due na`/`due:na` sentinel,
    /// generalized to every occurrence the series produces. A distinct
    /// variant (rather than wrapping the whole field in `Option`) so a
    /// command parameter can tell "explicitly never due" apart from "no
    /// override given, use the configured default" — `Series.due_rule`
    /// itself is never optional; every series has *some* rule, even if
    /// that rule is "never".
    Never,
    /// The project/global default due-code (see
    /// [`crate::settings::DUE_RELATIVE_DATE_CODES`]), resolved per-
    /// occurrence via [`crate::settings::resolve_due_relative_date`] exactly
    /// like a normal (non-recurring) task's default due code already is.
    /// Used when no explicit due phrase was typed when creating the
    /// recurring task — kept as a distinct variant (rather than converting
    /// it to a fixed [`DueRule::AfterScheduled`] offset up front) so
    /// `"in_1_month"` keeps its true calendar-month arithmetic for every
    /// occurrence, instead of an approximation baked in from whichever
    /// month the series happened to start in.
    DefaultCode { code: String },
    /// Due `days` days after the occurrence's own scheduled date (`0` is
    /// the same day). Covers every NL due phrase that doesn't name a
    /// weekday-based rule explicitly — `due tomorrow`, `due in 5 days`,
    /// `due jul 31`, even a singular weekday like `due monday` — each
    /// resolves to a date once, and the *gap* between that date and the
    /// scheduled date becomes this offset, applied consistently to every
    /// future occurrence. May be negative (due before scheduled) — unusual
    /// but not rejected, since a workflow that wants it isn't unreasonable
    /// and there's nothing to clamp it against.
    AfterScheduled { days: i32 },
    /// Due on the next occurrence of `weekday` on or after the occurrence's
    /// own scheduled date, every `interval_weeks` weeks (`1` is the very
    /// next occurrence; `2` skips that one, due on the one after — "every
    /// other <weekday>", mirroring the scheduling side's same concept).
    /// Only reachable via the dedicated `due <weekday>s`/`due every
    /// <weekday>` NL phrase, never via a plain singular weekday (which
    /// resolves through `AfterScheduled` instead, to avoid two different
    /// rules silently producing the same date for the common case).
    Weekday {
        weekday: Weekday,
        interval_weeks: u32,
    },
}

/// Returns `Ok(())` if `rule` is internally well-formed — a recognized code
/// for [`DueRule::DefaultCode`] (see
/// [`crate::settings::validate_due_relative_date_code`]), an in-range
/// weekday and a positive interval for [`DueRule::Weekday`], and always
/// valid for [`DueRule::AfterScheduled`] including a negative offset (see
/// its own doc comment) — or an error describing the problem otherwise.
pub fn validate_due_rule(rule: &DueRule) -> Result<(), String> {
    match rule {
        DueRule::Never => Ok(()),
        DueRule::DefaultCode { code } => crate::settings::validate_due_relative_date_code(code),
        DueRule::AfterScheduled { .. } => Ok(()),
        DueRule::Weekday {
            weekday,
            interval_weeks,
        } => {
            if *weekday > 6 {
                return Err("weekday must be 0 (Sunday) through 6 (Saturday)".to_string());
            }
            if *interval_weeks == 0 {
                return Err("due interval must be at least 1 week".to_string());
            }
            Ok(())
        }
    }
}

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
/// Template fields (`title`, `project_id`, `priority`, `tags`,
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
    /// How each occurrence's due date relates to its own scheduled date —
    /// see [`DueRule`], including its own [`DueRule::Never`] variant for "no
    /// due date at all" (so this field is never itself optional).
    #[serde(default = "default_due_rule")]
    pub due_rule: DueRule,
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
    /// The id of the `Project` this series' occurrences belong to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
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

fn default_due_rule() -> DueRule {
    DueRule::Never
}

impl Series {
    /// Creates a new active series with a freshly generated id and the
    /// current time as `created`. `generated_until` starts at `anchor_date`
    /// itself — the caller is expected to generate the window forward from
    /// there (via the normal generation step, see
    /// `crate::recurrence::occurrence_dates_in_range`), and separately
    /// create an occurrence directly at `anchor_date` itself *only* if
    /// `crate::recurrence::anchor_matches_frequency` confirms the anchor
    /// satisfies the rule — this constructor doesn't create any tasks
    /// itself, the same way [`crate::project::Project::new`] doesn't.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frequency: RecurrenceFrequency,
        anchor_date: String,
        end_date: Option<String>,
        due_rule: DueRule,
        title: String,
        project_id: Option<String>,
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
            due_rule,
            active: true,
            title,
            project_id,
            priority,
            tags,
            estimated_minutes,
            notes,
            created: Utc::now().to_rfc3339(),
        }
    }
}

/// Returns `Ok(())` if `series` is internally well-formed — its frequency
/// is valid (see [`validate_recurrence_frequency`]) and its `due_rule` is
/// well-formed (see [`validate_due_rule`]) — or an error describing the
/// first problem found otherwise.
pub fn validate_series(series: &Series) -> Result<(), String> {
    validate_recurrence_frequency(&series.frequency)?;
    validate_due_rule(&series.due_rule)?;
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
            DueRule::Never,
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
    fn new_series_has_no_end_date_and_a_never_due_rule_by_default() {
        let series = new_series(RecurrenceFrequency::EveryNDays { interval: 1 });

        assert_eq!(series.end_date, None);
        assert_eq!(series.due_rule, DueRule::Never);
    }

    #[test]
    fn due_rule_defaults_to_never_when_absent_from_json() {
        let json = r##"{"id":"abc","frequency":{"kind":"EveryNDays","interval":1},"anchor_date":"2026-06-15","generated_until":"2026-06-15","title":"Water the plants","priority":"medium","created":"2026-06-15T10:00:00+00:00"}"##;

        let series: Series = serde_json::from_str(json).expect("parsing should succeed");

        assert_eq!(series.due_rule, DueRule::Never);
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
        series.due_rule = DueRule::Never;

        let result = validate_series(&series);

        assert!(result.is_err());
    }

    #[test]
    fn validate_series_rejects_an_invalid_due_rule() {
        let mut series = new_series(RecurrenceFrequency::EveryNDays { interval: 1 });
        series.due_rule = DueRule::Weekday {
            weekday: 7,
            interval_weeks: 1,
        };

        let result = validate_series(&series);

        assert!(result.is_err());
    }

    #[test]
    fn validate_series_accepts_a_well_formed_due_rule() {
        let mut series = new_series(RecurrenceFrequency::EveryNDays { interval: 1 });
        series.due_rule = DueRule::AfterScheduled { days: 1 };

        assert!(validate_series(&series).is_ok());
    }

    #[test]
    fn validate_series_accepts_the_never_due_rule() {
        let series = new_series(RecurrenceFrequency::EveryNDays { interval: 1 });

        assert!(validate_series(&series).is_ok());
    }

    #[test]
    fn validate_due_rule_accepts_never() {
        assert!(validate_due_rule(&DueRule::Never).is_ok());
    }

    #[test]
    fn validate_due_rule_accepts_any_after_scheduled_offset_including_negative() {
        assert!(validate_due_rule(&DueRule::AfterScheduled { days: 0 }).is_ok());
        assert!(validate_due_rule(&DueRule::AfterScheduled { days: 30 }).is_ok());
        assert!(validate_due_rule(&DueRule::AfterScheduled { days: -1 }).is_ok());
    }

    #[test]
    fn validate_due_rule_accepts_a_well_formed_weekday_rule() {
        assert!(validate_due_rule(&DueRule::Weekday {
            weekday: 1,
            interval_weeks: 1,
        })
        .is_ok());
        assert!(validate_due_rule(&DueRule::Weekday {
            weekday: 6,
            interval_weeks: 2,
        })
        .is_ok());
    }

    #[test]
    fn validate_due_rule_rejects_an_out_of_range_weekday() {
        let result = validate_due_rule(&DueRule::Weekday {
            weekday: 7,
            interval_weeks: 1,
        });

        assert!(result.is_err());
    }

    #[test]
    fn validate_due_rule_rejects_a_zero_week_interval() {
        let result = validate_due_rule(&DueRule::Weekday {
            weekday: 1,
            interval_weeks: 0,
        });

        assert!(result.is_err());
    }

    #[test]
    fn validate_due_rule_accepts_a_recognized_default_code() {
        assert!(validate_due_rule(&DueRule::DefaultCode {
            code: "in_1_month".to_string()
        })
        .is_ok());
    }

    #[test]
    fn validate_due_rule_rejects_an_unrecognized_default_code() {
        let result = validate_due_rule(&DueRule::DefaultCode {
            code: "not_a_real_code".to_string(),
        });

        assert!(result.is_err());
    }
}
