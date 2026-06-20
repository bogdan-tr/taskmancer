use chrono::{Datelike, Days, NaiveDate};

use crate::series::{RecurrenceFrequency, Series};

fn parse_date(iso: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok()
}

/// Computes the occurrence dates `series` produces strictly after `after`
/// and up to (and including) `through`, additionally clamped to the
/// series' own `end_date` if it has one. Always empty if `series.anchor_date`
/// fails to parse (defensive — should be unreachable for a series that
/// passed validation at write time, but a corrupted `series.json` shouldn't
/// crash the app). Dates at or before `series.anchor_date` are never
/// produced, since the anchor itself is always the first occurrence
/// (created directly alongside the series, not through this function).
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
            None,
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
}
