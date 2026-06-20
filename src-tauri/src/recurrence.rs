use chrono::{Datelike, Days, NaiveDate};

use crate::series::{DueRule, RecurrenceFrequency, Series};
use crate::settings::resolve_due_relative_date;

fn parse_date(iso: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok()
}

/// Resolves `rule` to an absolute due date for an occurrence scheduled on
/// `scheduled`. Returns `None` for "never due" — [`DueRule::Never`],
/// [`DueRule::DefaultCode`]'s `"none"` sentinel (mirroring
/// [`resolve_due_relative_date`]'s own handling of it), or an otherwise
/// unrecognized default code (defensive — should be unreachable for a rule
/// that passed [`crate::series::validate_due_rule`] at write time).
pub fn resolve_due_rule(rule: &DueRule, scheduled: NaiveDate) -> Option<NaiveDate> {
    match rule {
        DueRule::Never => None,
        DueRule::DefaultCode { code } => resolve_due_relative_date(code, scheduled),
        DueRule::AfterScheduled { days } => {
            if *days >= 0 {
                scheduled.checked_add_days(Days::new(*days as u64))
            } else {
                scheduled.checked_sub_days(Days::new((-*days) as u64))
            }
        }
        DueRule::Weekday {
            weekday,
            interval_weeks,
        } => {
            let scheduled_weekday = scheduled.weekday().num_days_from_sunday() as i64;
            let days_until_next = (*weekday as i64 - scheduled_weekday).rem_euclid(7);
            let next = scheduled.checked_add_days(Days::new(days_until_next as u64))?;
            let extra_weeks = interval_weeks.saturating_sub(1) as u64;
            if extra_weeks == 0 {
                Some(next)
            } else {
                next.checked_add_days(Days::new(7 * extra_weeks))
            }
        }
    }
}

/// Whether `anchor` itself satisfies `frequency` — i.e. would be a valid
/// occurrence date if the rule were applied to it directly. Always `true`
/// for [`RecurrenceFrequency::EveryNDays`] (the anchor trivially defines
/// "day 0" of that pattern, so it can never mismatch), but `Weekly`/
/// `MonthlyByDay` can disagree with an explicitly chosen scheduled date —
/// e.g. scheduling a Mon/Tue/Wed series for a Saturday. Used by
/// `create_recurring_task` to decide whether the literal scheduled date
/// itself becomes the series' first occurrence, or whether the first
/// *real* occurrence is later — found by the normal generation step
/// (`occurrence_dates_in_range`), which already searches strictly after
/// the anchor regardless of whether the anchor itself matches.
pub fn anchor_matches_frequency(frequency: &RecurrenceFrequency, anchor: NaiveDate) -> bool {
    match frequency {
        RecurrenceFrequency::EveryNDays { .. } => true,
        RecurrenceFrequency::Weekly { weekdays, .. } => {
            weekdays.contains(&(anchor.weekday().num_days_from_sunday() as u8))
        }
        RecurrenceFrequency::MonthlyByDay { day } => anchor.day() == *day,
    }
}

/// Computes the occurrence dates `series` produces strictly after `after`
/// and up to (and including) `through`, additionally clamped to the
/// series' own `end_date` if it has one. Always empty if `series.anchor_date`
/// fails to parse (defensive — should be unreachable for a series that
/// passed validation at write time, but a corrupted `series.json` shouldn't
/// crash the app). Dates at or before `series.anchor_date` are never
/// produced by *this* function — `create_recurring_task` creates the
/// anchor date's own occurrence directly, separately, but only when
/// [`anchor_matches_frequency`] confirms the anchor itself satisfies the
/// rule (e.g. its weekday is one of `Weekly`'s `weekdays`). When it
/// doesn't (scheduling a Mon/Tue/Wed series for a Saturday), no occurrence
/// exists on the anchor date at all, and the first occurrence this
/// function produces (the first one strictly after the anchor) is the
/// series' real first occurrence.
pub fn occurrence_dates_in_range(
    series: &Series,
    after: NaiveDate,
    through: NaiveDate,
) -> Vec<NaiveDate> {
    let Some(anchor) = parse_date(&series.anchor_date) else {
        return Vec::new();
    };
    let through = match series.end_date.as_deref().and_then(parse_date) {
        Some(end) => through.min(end),
        None => through,
    };
    let after = after.max(anchor);
    if through <= after {
        return Vec::new();
    }

    match &series.frequency {
        RecurrenceFrequency::EveryNDays { interval } => {
            every_n_days_in_range(anchor, *interval, after, through)
        }
        RecurrenceFrequency::Weekly {
            weekdays,
            interval_weeks,
        } => weekly_in_range(anchor, weekdays, *interval_weeks, after, through),
        RecurrenceFrequency::MonthlyByDay { day } => {
            monthly_by_day_in_range(anchor, *day, after, through)
        }
    }
}

/// Dates `anchor + k * interval` days (`k >= 1`) that fall in `(after, through]`.
fn every_n_days_in_range(
    anchor: NaiveDate,
    interval: u32,
    after: NaiveDate,
    through: NaiveDate,
) -> Vec<NaiveDate> {
    let interval = interval.max(1) as i64;
    let mut dates = Vec::new();
    let days_since_anchor = (after - anchor).num_days();
    // The smallest k >= 1 such that anchor + k*interval > after.
    let mut k = (days_since_anchor / interval) + 1;
    if days_since_anchor < 0 {
        k = 1;
    }
    loop {
        let Some(candidate) = anchor.checked_add_days(Days::new((k * interval) as u64)) else {
            break;
        };
        if candidate > through {
            break;
        }
        dates.push(candidate);
        k += 1;
    }
    dates
}

/// Dates on one of `weekdays` (`0` = Sunday .. `6` = Saturday), in weeks
/// `interval_weeks` apart starting from the week containing `anchor`, that
/// fall in `(after, through]`.
fn weekly_in_range(
    anchor: NaiveDate,
    weekdays: &[u8],
    interval_weeks: u32,
    after: NaiveDate,
    through: NaiveDate,
) -> Vec<NaiveDate> {
    if weekdays.is_empty() {
        return Vec::new();
    }
    let interval_weeks = interval_weeks.max(1) as i64;
    let anchor_week_start =
        anchor - chrono::Duration::days(anchor.weekday().num_days_from_sunday() as i64);

    let mut dates = Vec::new();
    let mut week_index = 0i64;
    loop {
        let Some(week_start) =
            anchor_week_start.checked_add_signed(chrono::Duration::days(week_index * 7))
        else {
            break;
        };
        if week_start > through {
            break;
        }
        if week_index % interval_weeks == 0 {
            for weekday in weekdays {
                if let Some(candidate) = week_start.checked_add_days(Days::new(*weekday as u64)) {
                    if candidate > after && candidate <= through {
                        dates.push(candidate);
                    }
                }
            }
        }
        week_index += 1;
    }
    dates.sort();
    dates
}

/// The `day`-th of each month from `after`'s month through `through`'s
/// month that falls in `(after, through]`. A month with fewer than `day`
/// days (e.g. the 31st in February) is skipped entirely for that month.
fn monthly_by_day_in_range(
    anchor: NaiveDate,
    day: u32,
    after: NaiveDate,
    through: NaiveDate,
) -> Vec<NaiveDate> {
    let mut dates = Vec::new();
    let mut year = after.year();
    let mut month = after.month();
    loop {
        if let Some(candidate) = NaiveDate::from_ymd_opt(year, month, day) {
            if candidate > after && candidate <= through && candidate >= anchor {
                dates.push(candidate);
            }
        }
        let Some(next_month_first) = NaiveDate::from_ymd_opt(year, month, 1)
            .and_then(|d| d.checked_add_months(chrono::Months::new(1)))
        else {
            break;
        };
        if next_month_first > through {
            break;
        }
        year = next_month_first.year();
        month = next_month_first.month();
    }
    dates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::series::Series;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn series_with(
        frequency: RecurrenceFrequency,
        anchor_date: &str,
        end_date: Option<&str>,
    ) -> Series {
        let mut series = Series::new(
            frequency,
            anchor_date.to_string(),
            end_date.map(|s| s.to_string()),
            DueRule::Never,
            "Water the plants".to_string(),
            None,
            "medium".to_string(),
            vec![],
            None,
            String::new(),
        );
        series.generated_until = anchor_date.to_string();
        series
    }

    mod every_n_days {
        use super::*;

        #[test]
        fn daily_produces_every_day_after_the_anchor() {
            let series = series_with(
                RecurrenceFrequency::EveryNDays { interval: 1 },
                "2026-06-15",
                None,
            );

            let dates = occurrence_dates_in_range(&series, date("2026-06-15"), date("2026-06-18"));

            assert_eq!(
                dates,
                vec![date("2026-06-16"), date("2026-06-17"), date("2026-06-18")]
            );
        }

        #[test]
        fn every_other_day_skips_one_day_between_occurrences() {
            let series = series_with(
                RecurrenceFrequency::EveryNDays { interval: 2 },
                "2026-06-15",
                None,
            );

            let dates = occurrence_dates_in_range(&series, date("2026-06-15"), date("2026-06-21"));

            assert_eq!(
                dates,
                vec![date("2026-06-17"), date("2026-06-19"), date("2026-06-21")]
            );
        }

        #[test]
        fn never_produces_a_date_at_or_before_the_anchor() {
            let series = series_with(
                RecurrenceFrequency::EveryNDays { interval: 1 },
                "2026-06-15",
                None,
            );

            // `after` predates the anchor — occurrences still only start from the anchor forward.
            let dates = occurrence_dates_in_range(&series, date("2026-06-10"), date("2026-06-17"));

            assert_eq!(dates, vec![date("2026-06-16"), date("2026-06-17")]);
        }

        #[test]
        fn resuming_from_a_later_watermark_only_returns_newer_dates() {
            let series = series_with(
                RecurrenceFrequency::EveryNDays { interval: 1 },
                "2026-06-15",
                None,
            );

            let dates = occurrence_dates_in_range(&series, date("2026-06-17"), date("2026-06-19"));

            assert_eq!(dates, vec![date("2026-06-18"), date("2026-06-19")]);
        }

        #[test]
        fn clamps_to_the_series_end_date() {
            let series = series_with(
                RecurrenceFrequency::EveryNDays { interval: 1 },
                "2026-06-15",
                Some("2026-06-17"),
            );

            let dates = occurrence_dates_in_range(&series, date("2026-06-15"), date("2026-06-30"));

            assert_eq!(dates, vec![date("2026-06-16"), date("2026-06-17")]);
        }

        #[test]
        fn returns_empty_when_through_is_at_or_before_after() {
            let series = series_with(
                RecurrenceFrequency::EveryNDays { interval: 1 },
                "2026-06-15",
                None,
            );

            let dates = occurrence_dates_in_range(&series, date("2026-06-18"), date("2026-06-18"));

            assert!(dates.is_empty());
        }
    }

    mod weekly {
        use super::*;

        #[test]
        fn single_weekday_every_week_produces_that_weekday_each_week() {
            // 2026-06-15 is a Monday.
            let series = series_with(
                RecurrenceFrequency::Weekly {
                    weekdays: vec![1],
                    interval_weeks: 1,
                },
                "2026-06-15",
                None,
            );

            let dates = occurrence_dates_in_range(&series, date("2026-06-15"), date("2026-06-29"));

            assert_eq!(dates, vec![date("2026-06-22"), date("2026-06-29")]);
        }

        #[test]
        fn multiple_weekdays_produce_each_one_within_the_same_week() {
            // Mon/Wed/Fri starting Monday 2026-06-15.
            let series = series_with(
                RecurrenceFrequency::Weekly {
                    weekdays: vec![1, 3, 5],
                    interval_weeks: 1,
                },
                "2026-06-15",
                None,
            );

            let dates = occurrence_dates_in_range(&series, date("2026-06-15"), date("2026-06-19"));

            assert_eq!(dates, vec![date("2026-06-17"), date("2026-06-19")]);
        }

        #[test]
        fn every_other_weekday_skips_alternating_weeks() {
            // Every other Saturday, starting Saturday 2026-06-13.
            let series = series_with(
                RecurrenceFrequency::Weekly {
                    weekdays: vec![6],
                    interval_weeks: 2,
                },
                "2026-06-13",
                None,
            );

            let dates = occurrence_dates_in_range(&series, date("2026-06-13"), date("2026-07-25"));

            assert_eq!(
                dates,
                vec![date("2026-06-27"), date("2026-07-11"), date("2026-07-25")]
            );
        }

        #[test]
        fn every_weekday_shortcut_excludes_the_weekend() {
            // Mon-Fri starting Monday 2026-06-15.
            let series = series_with(
                RecurrenceFrequency::Weekly {
                    weekdays: vec![1, 2, 3, 4, 5],
                    interval_weeks: 1,
                },
                "2026-06-15",
                None,
            );

            let dates = occurrence_dates_in_range(&series, date("2026-06-15"), date("2026-06-21"));

            assert_eq!(
                dates,
                vec![
                    date("2026-06-16"),
                    date("2026-06-17"),
                    date("2026-06-18"),
                    date("2026-06-19"),
                ]
            );
        }

        #[test]
        fn does_not_produce_a_weekday_occurrence_before_the_anchor_in_the_anchors_own_week() {
            // Anchor is Wednesday 2026-06-17, rule is Mon/Wed/Fri — Monday the
            // 15th is in the same week but predates the anchor and must not appear.
            let series = series_with(
                RecurrenceFrequency::Weekly {
                    weekdays: vec![1, 3, 5],
                    interval_weeks: 1,
                },
                "2026-06-17",
                None,
            );

            let dates = occurrence_dates_in_range(&series, date("2026-06-17"), date("2026-06-19"));

            assert_eq!(dates, vec![date("2026-06-19")]);
        }
    }

    mod monthly_by_day {
        use super::*;

        #[test]
        fn produces_the_given_day_in_each_month() {
            let series = series_with(
                RecurrenceFrequency::MonthlyByDay { day: 4 },
                "2026-06-04",
                None,
            );

            let dates = occurrence_dates_in_range(&series, date("2026-06-04"), date("2026-09-04"));

            assert_eq!(
                dates,
                vec![date("2026-07-04"), date("2026-08-04"), date("2026-09-04")]
            );
        }

        #[test]
        fn skips_a_month_that_does_not_have_that_day() {
            // The 31st: April and June don't have one; May and July do.
            let series = series_with(
                RecurrenceFrequency::MonthlyByDay { day: 31 },
                "2026-03-31",
                None,
            );

            let dates = occurrence_dates_in_range(&series, date("2026-03-31"), date("2026-07-31"));

            assert_eq!(dates, vec![date("2026-05-31"), date("2026-07-31")]);
        }

        #[test]
        fn skips_february_for_the_31st() {
            let series = series_with(
                RecurrenceFrequency::MonthlyByDay { day: 31 },
                "2026-01-31",
                None,
            );

            let dates = occurrence_dates_in_range(&series, date("2026-01-31"), date("2026-03-31"));

            // No February 31st — only March's.
            assert_eq!(dates, vec![date("2026-03-31")]);
        }
    }

    #[test]
    fn returns_empty_when_the_anchor_date_is_unparseable() {
        let mut series = series_with(
            RecurrenceFrequency::EveryNDays { interval: 1 },
            "2026-06-15",
            None,
        );
        series.anchor_date = "not-a-date".to_string();

        let dates = occurrence_dates_in_range(&series, date("2026-06-15"), date("2026-06-20"));

        assert!(dates.is_empty());
    }

    #[test]
    fn ignores_an_unparseable_end_date_rather_than_failing() {
        let mut series = series_with(
            RecurrenceFrequency::EveryNDays { interval: 1 },
            "2026-06-15",
            None,
        );
        series.end_date = Some("not-a-date".to_string());

        let dates = occurrence_dates_in_range(&series, date("2026-06-15"), date("2026-06-17"));

        assert_eq!(dates, vec![date("2026-06-16"), date("2026-06-17")]);
    }

    mod resolve_due_rule_tests {
        use super::*;

        #[test]
        fn after_scheduled_zero_days_is_the_same_day() {
            let resolved =
                resolve_due_rule(&DueRule::AfterScheduled { days: 0 }, date("2026-06-15"));

            assert_eq!(resolved, Some(date("2026-06-15")));
        }

        #[test]
        fn after_scheduled_positive_days_is_later() {
            let resolved =
                resolve_due_rule(&DueRule::AfterScheduled { days: 5 }, date("2026-06-15"));

            assert_eq!(resolved, Some(date("2026-06-20")));
        }

        #[test]
        fn after_scheduled_negative_days_is_earlier() {
            let resolved =
                resolve_due_rule(&DueRule::AfterScheduled { days: -3 }, date("2026-06-15"));

            assert_eq!(resolved, Some(date("2026-06-12")));
        }

        #[test]
        fn weekday_rule_resolves_to_the_same_day_when_scheduled_is_already_that_weekday() {
            // 2026-06-15 is a Monday.
            let resolved = resolve_due_rule(
                &DueRule::Weekday {
                    weekday: 1,
                    interval_weeks: 1,
                },
                date("2026-06-15"),
            );

            assert_eq!(resolved, Some(date("2026-06-15")));
        }

        #[test]
        fn weekday_rule_resolves_to_the_next_occurrence_of_that_weekday() {
            // 2026-06-15 is a Monday; the next Friday is 2026-06-19.
            let resolved = resolve_due_rule(
                &DueRule::Weekday {
                    weekday: 5,
                    interval_weeks: 1,
                },
                date("2026-06-15"),
            );

            assert_eq!(resolved, Some(date("2026-06-19")));
        }

        #[test]
        fn weekday_rule_wraps_around_to_next_week_when_the_weekday_already_passed() {
            // 2026-06-19 is a Friday; the next Monday is 2026-06-22, not earlier.
            let resolved = resolve_due_rule(
                &DueRule::Weekday {
                    weekday: 1,
                    interval_weeks: 1,
                },
                date("2026-06-19"),
            );

            assert_eq!(resolved, Some(date("2026-06-22")));
        }

        #[test]
        fn weekday_rule_with_interval_two_skips_the_next_occurrence() {
            // 2026-06-15 is a Monday; the next Friday is 2026-06-19, the one after is 2026-06-26.
            let resolved = resolve_due_rule(
                &DueRule::Weekday {
                    weekday: 5,
                    interval_weeks: 2,
                },
                date("2026-06-15"),
            );

            assert_eq!(resolved, Some(date("2026-06-26")));
        }

        #[test]
        fn never_resolves_to_no_due_date() {
            let resolved = resolve_due_rule(&DueRule::Never, date("2026-06-15"));

            assert_eq!(resolved, None);
        }

        #[test]
        fn default_code_resolves_via_the_settings_relative_date_resolver() {
            let resolved = resolve_due_rule(
                &DueRule::DefaultCode {
                    code: "next_day".to_string(),
                },
                date("2026-06-15"),
            );

            assert_eq!(resolved, Some(date("2026-06-16")));
        }

        #[test]
        fn default_code_none_sentinel_resolves_to_never_due() {
            let resolved = resolve_due_rule(
                &DueRule::DefaultCode {
                    code: "none".to_string(),
                },
                date("2026-06-15"),
            );

            assert_eq!(resolved, None);
        }

        #[test]
        fn default_code_unrecognized_resolves_to_none_rather_than_panicking() {
            let resolved = resolve_due_rule(
                &DueRule::DefaultCode {
                    code: "not_a_real_code".to_string(),
                },
                date("2026-06-15"),
            );

            assert_eq!(resolved, None);
        }
    }

    mod anchor_matches_frequency_tests {
        use super::*;

        #[test]
        fn every_n_days_always_matches_the_anchor() {
            // 2026-06-20 is a Saturday — irrelevant for this frequency, the
            // anchor always trivially defines "day 0" of an N-day pattern.
            let frequency = RecurrenceFrequency::EveryNDays { interval: 3 };

            assert!(anchor_matches_frequency(&frequency, date("2026-06-20")));
        }

        #[test]
        fn weekly_matches_when_the_anchor_weekday_is_in_the_list() {
            // 2026-06-15 is a Monday.
            let frequency = RecurrenceFrequency::Weekly {
                weekdays: vec![1, 2, 3],
                interval_weeks: 1,
            };

            assert!(anchor_matches_frequency(&frequency, date("2026-06-15")));
        }

        #[test]
        fn weekly_does_not_match_when_the_anchor_weekday_is_not_in_the_list() {
            // 2026-06-20 is a Saturday, not in [Mon, Tue, Wed].
            let frequency = RecurrenceFrequency::Weekly {
                weekdays: vec![1, 2, 3],
                interval_weeks: 1,
            };

            assert!(!anchor_matches_frequency(&frequency, date("2026-06-20")));
        }

        #[test]
        fn weekly_does_not_match_regardless_of_interval_weeks() {
            // The interval only affects which *future* weeks count, not
            // whether the anchor's own weekday is in the list at all.
            let frequency = RecurrenceFrequency::Weekly {
                weekdays: vec![1, 2, 3],
                interval_weeks: 2,
            };

            assert!(!anchor_matches_frequency(&frequency, date("2026-06-20")));
        }

        #[test]
        fn monthly_by_day_matches_when_the_anchor_day_of_month_equals_day() {
            let frequency = RecurrenceFrequency::MonthlyByDay { day: 20 };

            assert!(anchor_matches_frequency(&frequency, date("2026-06-20")));
        }

        #[test]
        fn monthly_by_day_does_not_match_when_the_anchor_day_of_month_differs() {
            let frequency = RecurrenceFrequency::MonthlyByDay { day: 5 };

            assert!(!anchor_matches_frequency(&frequency, date("2026-06-20")));
        }
    }
}
