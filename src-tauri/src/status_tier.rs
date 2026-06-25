use chrono::NaiveDate;
use serde::Serialize;

use crate::settings::{PriorityLevel, Settings, StatusTierRule};
use crate::task::Task;

/// The 4 status-line health tiers, most-severe-first — see
/// `docs/features/project-status-line.md`'s "Status algorithm". `Great` has
/// no rule of its own: it's the implicit fallback when no real tier matches.
///
/// Serializes as a stable snake_case string id (`"needs_attention"`, etc.) —
/// the form `get_project_status_stats` (Milestone 2) returns to the frontend,
/// since the frontend has no reason to know about Rust enum variant naming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusTier {
    Severe,
    Critical,
    NeedsAttention,
    OnTrack,
    Great,
}

/// Resolves the 4 effective tier rules for a project: `overrides[i]`
/// (when `Some`) wins for slot `i`, otherwise `global[i]` is used. `global`
/// is assumed to already have exactly 4 entries (enforced by
/// `settings::validate_settings`); `overrides`, when `Some`, is assumed to
/// also have exactly 4 entries (the per-slot-`Option` invariant documented
/// on `crate::project::ProjectBoard::status_tier_rule_overrides`) — a
/// shorter or longer override list just falls back to the global rule for
/// any out-of-bounds slot rather than panicking, so a malformed/legacy
/// project file degrades gracefully instead of crashing the evaluator.
pub fn effective_status_tier_rules<'a>(
    global: &'a [StatusTierRule],
    overrides: Option<&'a [Option<StatusTierRule>]>,
) -> Vec<&'a StatusTierRule> {
    global
        .iter()
        .enumerate()
        .map(|(index, global_rule)| {
            overrides
                .and_then(|slots| slots.get(index))
                .and_then(|slot| slot.as_ref())
                .unwrap_or(global_rule)
        })
        .collect()
}

/// Returns `true` if `rule.due_within_days` is unset, or any task in `tasks`
/// has a parseable `due` date `<= today + due_within_days`. An unparsable
/// `due` string is skipped (never matches) rather than erroring — mirrors
/// how date parsing degrades elsewhere in this codebase (e.g.
/// `commands::tally_statuses`-adjacent date handling) rather than failing
/// the whole evaluation over one malformed task.
fn matches_due_within_days(rule: &StatusTierRule, tasks: &[&Task], today: NaiveDate) -> bool {
    let Some(days) = rule.due_within_days else {
        return true;
    };
    let Some(threshold) = add_days_allow_negative(today, days) else {
        return false;
    };

    tasks.iter().any(|task| {
        task.due
            .as_deref()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .is_some_and(|due| due <= threshold)
    })
}

/// `NaiveDate` has no direct "add a possibly-negative i32 number of days"
/// helper that also handles the negative case without an awkward
/// branch at every call site, so this wraps `chrono::Days`'s `checked_add`/
/// `checked_sub` (each of which only accepts a `u64` magnitude).
fn add_days_allow_negative(date: NaiveDate, days: i32) -> Option<NaiveDate> {
    if days >= 0 {
        date.checked_add_days(chrono::Days::new(days as u64))
    } else {
        date.checked_sub_days(chrono::Days::new(days.unsigned_abs() as u64))
    }
}

/// Returns `true` if `rule.min_priority` is unset, or any task in `tasks`
/// has a priority whose `rank` is `<=` the named level's `rank` (lower rank =
/// more severe, so this is "at least as severe as"). An `id` that doesn't
/// match any current `PriorityLevel` never matches — fails closed rather
/// than erroring, per the project-status-line spec's clarification.
fn matches_min_priority(
    rule: &StatusTierRule,
    tasks: &[&Task],
    priorities: &[PriorityLevel],
) -> bool {
    let Some(min_priority_id) = &rule.min_priority else {
        return true;
    };
    let Some(threshold_rank) = priorities
        .iter()
        .find(|level| &level.id == min_priority_id)
        .map(|level| level.rank)
    else {
        return false;
    };

    tasks.iter().any(|task| {
        priorities
            .iter()
            .find(|level| level.id == task.priority)
            .is_some_and(|level| level.rank <= threshold_rank)
    })
}

/// Returns `Ok(())` if `overrides` has exactly 4 entries — the invariant
/// documented on `crate::project::ProjectBoard::status_tier_rule_overrides`
/// (one slot per `[severe, critical, needs_attention, on_track]` tier) that
/// [`effective_status_tier_rules`] otherwise just assumes. Called from
/// `commands::apply_project_update` whenever
/// `update.board.status_tier_rule_overrides` is `Some`.
pub fn validate_status_tier_rule_overrides(
    overrides: &[Option<StatusTierRule>],
) -> Result<(), String> {
    if overrides.len() != 4 {
        return Err(format!(
            "status_tier_rule_overrides must have exactly 4 entries (one per tier), got {}",
            overrides.len()
        ));
    }
    Ok(())
}

/// Returns `true` if `rule.estimated_time_left_exceeds_minutes` is unset, or
/// `estimated_time_left` (the project's already-computed
/// `status_stats::estimated_time_left` value — not recomputed here) is
/// strictly greater than the threshold.
fn matches_estimated_time_left_exceeds(rule: &StatusTierRule, estimated_time_left: u32) -> bool {
    rule.estimated_time_left_exceeds_minutes
        .is_none_or(|threshold| estimated_time_left > threshold)
}

/// Returns `true` if `rule` has every condition unset — a degenerate tier
/// that was never actually configured. [`rule_matches`] treats this as "can
/// never match" rather than vacuously true: an AND over zero active
/// conditions being `true` would make every tier with no conditions set
/// match unconditionally, which would make a freshly-added/blanked-out tier
/// silently swallow every project into its severity level instead of simply
/// never firing.
fn has_no_conditions_set(rule: &StatusTierRule) -> bool {
    rule.due_within_days.is_none()
        && rule.min_priority.is_none()
        && rule.estimated_time_left_exceeds_minutes.is_none()
}

/// Returns `true` if `rule` has at least one condition set, and every
/// condition it has set matches (AND semantics; an unset condition is
/// skipped — see [`has_no_conditions_set`] for why an all-unset rule is
/// `false` rather than vacuously `true`).
fn rule_matches(
    rule: &StatusTierRule,
    tasks: &[&Task],
    priorities: &[PriorityLevel],
    estimated_time_left: u32,
    today: NaiveDate,
) -> bool {
    if has_no_conditions_set(rule) {
        return false;
    }

    matches_due_within_days(rule, tasks, today)
        && matches_min_priority(rule, tasks, priorities)
        && matches_estimated_time_left_exceeds(rule, estimated_time_left)
}

/// Evaluates a project's status-line health tier: most-severe-first against
/// `effective_rules` (see [`effective_status_tier_rules`] —
/// `[severe, critical, needs_attention, on_track]` order), first full match
/// wins, implicit [`StatusTier::Great`] fallback when none match.
///
/// `tasks` must already be filtered to this project's own (never rolled up
/// into subprojects, per the spec's resolved rollup-scope decision)
/// non-hidden, incomplete tasks — this function does no status/hidden
/// filtering itself, since that depends on `Settings.done_status`/
/// `.cancelled_status`, which the caller already has in hand alongside the
/// task list. `estimated_time_left` is the project's already-computed
/// `status_stats::estimated_time_left` stat, passed in rather than
/// recomputed here (that stat has its own, different task-filtering rules —
/// see its own doc comment).
pub fn evaluate_status_tier(
    effective_rules: &[&StatusTierRule],
    tasks: &[&Task],
    settings: &Settings,
    estimated_time_left: u32,
    today: NaiveDate,
) -> StatusTier {
    const TIER_ORDER: [StatusTier; 4] = [
        StatusTier::Severe,
        StatusTier::Critical,
        StatusTier::NeedsAttention,
        StatusTier::OnTrack,
    ];

    for (rule, tier) in effective_rules.iter().zip(TIER_ORDER) {
        if rule_matches(
            rule,
            tasks,
            &settings.priorities,
            estimated_time_left,
            today,
        ) {
            return tier;
        }
    }

    StatusTier::Great
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::PriorityLevel;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 6, 24).unwrap()
    }

    fn task_due(due: &str) -> Task {
        let mut task = Task::new("Demo".to_string());
        task.due = Some(due.to_string());
        task
    }

    fn task_with_priority(priority: &str) -> Task {
        let mut task = Task::new("Demo".to_string());
        task.priority = priority.to_string();
        task
    }

    fn priorities() -> Vec<PriorityLevel> {
        vec![
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
        ]
    }

    fn settings_with_priorities() -> Settings {
        Settings {
            priorities: priorities(),
            ..Settings::default()
        }
    }

    mod effective_status_tier_rules_tests {
        use super::*;

        fn global() -> Vec<StatusTierRule> {
            vec![
                StatusTierRule {
                    due_within_days: Some(0),
                    ..Default::default()
                },
                StatusTierRule {
                    due_within_days: Some(1),
                    ..Default::default()
                },
                StatusTierRule {
                    due_within_days: Some(3),
                    ..Default::default()
                },
                StatusTierRule {
                    due_within_days: Some(7),
                    ..Default::default()
                },
            ]
        }

        #[test]
        fn returns_every_global_rule_when_no_overrides_exist() {
            let global = global();

            let effective = effective_status_tier_rules(&global, None);

            assert_eq!(effective.len(), 4);
            for (i, rule) in effective.iter().enumerate() {
                assert_eq!(rule.due_within_days, global[i].due_within_days);
            }
        }

        #[test]
        fn a_set_override_slot_wins_over_the_global_rule_for_that_slot_only() {
            let global = global();
            let overrides = vec![
                None,
                Some(StatusTierRule {
                    due_within_days: Some(2),
                    min_priority: Some("high".to_string()),
                    estimated_time_left_exceeds_minutes: None,
                }),
                None,
                None,
            ];

            let effective = effective_status_tier_rules(&global, Some(&overrides));

            assert_eq!(effective[0].due_within_days, Some(0)); // inherited
            assert_eq!(effective[1].due_within_days, Some(2)); // overridden
            assert_eq!(effective[2].due_within_days, Some(3)); // inherited
            assert_eq!(effective[3].due_within_days, Some(7)); // inherited
        }

        #[test]
        fn every_slot_can_be_overridden_independently() {
            let global = global();
            let overrides = vec![
                Some(StatusTierRule {
                    due_within_days: Some(-1),
                    ..Default::default()
                }),
                Some(StatusTierRule {
                    due_within_days: Some(2),
                    ..Default::default()
                }),
                Some(StatusTierRule {
                    due_within_days: Some(5),
                    ..Default::default()
                }),
                Some(StatusTierRule {
                    due_within_days: Some(14),
                    ..Default::default()
                }),
            ];

            let effective = effective_status_tier_rules(&global, Some(&overrides));

            assert_eq!(effective[0].due_within_days, Some(-1));
            assert_eq!(effective[1].due_within_days, Some(2));
            assert_eq!(effective[2].due_within_days, Some(5));
            assert_eq!(effective[3].due_within_days, Some(14));
        }

        #[test]
        fn a_too_short_overrides_list_falls_back_to_global_for_missing_slots() {
            let global = global();
            let overrides = vec![Some(StatusTierRule {
                due_within_days: Some(-1),
                ..Default::default()
            })];

            let effective = effective_status_tier_rules(&global, Some(&overrides));

            assert_eq!(effective[0].due_within_days, Some(-1));
            assert_eq!(effective[1].due_within_days, Some(1));
            assert_eq!(effective[2].due_within_days, Some(3));
            assert_eq!(effective[3].due_within_days, Some(7));
        }
    }

    mod validate_status_tier_rule_overrides_tests {
        use super::*;

        #[test]
        fn accepts_exactly_four_entries() {
            let overrides = vec![None, None, None, None];

            assert!(validate_status_tier_rule_overrides(&overrides).is_ok());
        }

        #[test]
        fn accepts_four_entries_with_some_set() {
            let overrides = vec![
                Some(StatusTierRule {
                    due_within_days: Some(0),
                    ..Default::default()
                }),
                None,
                None,
                None,
            ];

            assert!(validate_status_tier_rule_overrides(&overrides).is_ok());
        }

        #[test]
        fn rejects_fewer_than_four_entries() {
            let overrides = vec![None, None, None];

            let err = validate_status_tier_rule_overrides(&overrides).unwrap_err();
            assert!(err.contains('3'));
        }

        #[test]
        fn rejects_more_than_four_entries() {
            let overrides = vec![None, None, None, None, None];

            let err = validate_status_tier_rule_overrides(&overrides).unwrap_err();
            assert!(err.contains('5'));
        }

        #[test]
        fn rejects_an_empty_list() {
            let overrides: Vec<Option<StatusTierRule>> = vec![];

            assert!(validate_status_tier_rule_overrides(&overrides).is_err());
        }
    }

    mod evaluate_status_tier_tests {
        use super::*;

        fn rules_all_unset() -> Vec<StatusTierRule> {
            vec![StatusTierRule::default(); 4]
        }

        fn as_refs(rules: &[StatusTierRule]) -> Vec<&StatusTierRule> {
            rules.iter().collect()
        }

        #[test]
        fn a_tier_with_every_condition_unset_never_matches_even_with_overdue_tasks() {
            // Degenerate config: a tier configured with literally no
            // conditions must never fire — it shouldn't vacuously match
            // every project just because nothing was set.
            let rules = rules_all_unset();
            let very_overdue = task_due("2020-01-01");
            let settings = Settings::default();

            let tier =
                evaluate_status_tier(&as_refs(&rules), &[&very_overdue], &settings, 9999, today());

            assert_eq!(tier, StatusTier::Great);
        }

        #[test]
        fn falls_back_to_great_when_no_tasks_and_no_rules_match() {
            let rules = rules_all_unset();
            let settings = Settings::default();

            let tier = evaluate_status_tier(&as_refs(&rules), &[], &settings, 0, today());

            assert_eq!(tier, StatusTier::Great);
        }

        #[test]
        fn matches_severe_when_a_task_is_overdue() {
            let mut rules = rules_all_unset();
            rules[0].due_within_days = Some(0);
            let overdue = task_due("2026-06-20"); // before today (06-24)
            let settings = Settings::default();

            let tier = evaluate_status_tier(&as_refs(&rules), &[&overdue], &settings, 0, today());

            assert_eq!(tier, StatusTier::Severe);
        }

        #[test]
        fn due_within_days_zero_matches_a_task_due_exactly_today() {
            let mut rules = rules_all_unset();
            rules[0].due_within_days = Some(0);
            let due_today = task_due("2026-06-24");
            let settings = Settings::default();

            let tier = evaluate_status_tier(&as_refs(&rules), &[&due_today], &settings, 0, today());

            assert_eq!(tier, StatusTier::Severe);
        }

        #[test]
        fn due_within_days_does_not_match_a_task_due_one_day_after_the_threshold() {
            let mut rules = rules_all_unset();
            rules[0].due_within_days = Some(0);
            let due_tomorrow = task_due("2026-06-25");
            let settings = Settings::default();

            let tier =
                evaluate_status_tier(&as_refs(&rules), &[&due_tomorrow], &settings, 0, today());

            assert_eq!(tier, StatusTier::Great);
        }

        #[test]
        fn most_severe_tier_wins_when_multiple_tiers_would_match() {
            let mut rules = rules_all_unset();
            rules[0].due_within_days = Some(7); // severe: lenient, matches
            rules[1].due_within_days = Some(7); // critical: also matches
            let due_soon = task_due("2026-06-26");
            let settings = Settings::default();

            let tier = evaluate_status_tier(&as_refs(&rules), &[&due_soon], &settings, 0, today());

            assert_eq!(tier, StatusTier::Severe);
        }

        #[test]
        fn a_tier_with_multiple_conditions_requires_all_of_them_to_match() {
            let mut rules = rules_all_unset();
            rules[1].due_within_days = Some(1);
            rules[1].min_priority = Some("high".to_string());
            // Due soon but low priority: due_within_days matches, min_priority doesn't.
            let due_soon_low_priority = task_with_priority("low");
            let mut due_soon_low_priority = due_soon_low_priority;
            due_soon_low_priority.due = Some("2026-06-24".to_string());
            let settings = settings_with_priorities();

            let tier = evaluate_status_tier(
                &as_refs(&rules),
                &[&due_soon_low_priority],
                &settings,
                0,
                today(),
            );

            assert_eq!(tier, StatusTier::Great);
        }

        #[test]
        fn a_tier_with_multiple_conditions_matches_when_both_are_satisfied_by_different_tasks() {
            let mut rules = rules_all_unset();
            rules[1].due_within_days = Some(1);
            rules[1].min_priority = Some("high".to_string());
            let due_soon = task_due("2026-06-24");
            let high_priority = task_with_priority("high");
            let settings = settings_with_priorities();

            let tier = evaluate_status_tier(
                &as_refs(&rules),
                &[&due_soon, &high_priority],
                &settings,
                0,
                today(),
            );

            assert_eq!(tier, StatusTier::Critical);
        }

        #[test]
        fn min_priority_matches_an_exactly_equal_rank() {
            let mut rules = rules_all_unset();
            rules[1].min_priority = Some("high".to_string());
            let high_priority_task = task_with_priority("high");
            let settings = settings_with_priorities();

            let tier = evaluate_status_tier(
                &as_refs(&rules),
                &[&high_priority_task],
                &settings,
                0,
                today(),
            );

            assert_eq!(tier, StatusTier::Critical);
        }

        #[test]
        fn min_priority_matches_a_more_severe_custom_level_with_a_lower_rank() {
            let mut rules = rules_all_unset();
            rules[1].min_priority = Some("high".to_string());
            let mut settings = settings_with_priorities();
            settings.priorities.push(PriorityLevel {
                id: "urgent".to_string(),
                label: "Urgent".to_string(),
                color: "#ff0000".to_string(),
                rank: 0, // more severe than "high" (rank 1)
            });
            let urgent_task = task_with_priority("urgent");

            let tier =
                evaluate_status_tier(&as_refs(&rules), &[&urgent_task], &settings, 0, today());

            assert_eq!(tier, StatusTier::Critical);
        }

        #[test]
        fn min_priority_does_not_match_a_less_severe_level() {
            let mut rules = rules_all_unset();
            rules[1].min_priority = Some("high".to_string());
            let low_priority_task = task_with_priority("low");
            let settings = settings_with_priorities();

            let tier = evaluate_status_tier(
                &as_refs(&rules),
                &[&low_priority_task],
                &settings,
                0,
                today(),
            );

            assert_eq!(tier, StatusTier::Great);
        }

        #[test]
        fn min_priority_referencing_an_unknown_id_never_matches() {
            let mut rules = rules_all_unset();
            rules[1].min_priority = Some("does-not-exist".to_string());
            let any_task = task_with_priority("high");
            let settings = settings_with_priorities();

            let tier = evaluate_status_tier(&as_refs(&rules), &[&any_task], &settings, 0, today());

            assert_eq!(tier, StatusTier::Great);
        }

        #[test]
        fn estimated_time_left_exceeds_minutes_matches_when_strictly_greater() {
            let mut rules = rules_all_unset();
            rules[0].estimated_time_left_exceeds_minutes = Some(100);
            let settings = Settings::default();

            let tier = evaluate_status_tier(&as_refs(&rules), &[], &settings, 101, today());

            assert_eq!(tier, StatusTier::Severe);
        }

        #[test]
        fn estimated_time_left_exceeds_minutes_does_not_match_when_exactly_equal() {
            let mut rules = rules_all_unset();
            rules[0].estimated_time_left_exceeds_minutes = Some(100);
            let settings = Settings::default();

            let tier = evaluate_status_tier(&as_refs(&rules), &[], &settings, 100, today());

            assert_eq!(tier, StatusTier::Great);
        }

        #[test]
        fn estimated_time_left_exceeds_minutes_does_not_match_when_lower() {
            let mut rules = rules_all_unset();
            rules[0].estimated_time_left_exceeds_minutes = Some(100);
            let settings = Settings::default();

            let tier = evaluate_status_tier(&as_refs(&rules), &[], &settings, 50, today());

            assert_eq!(tier, StatusTier::Great);
        }

        #[test]
        fn an_unparsable_due_date_is_skipped_rather_than_matching() {
            let mut rules = rules_all_unset();
            rules[0].due_within_days = Some(7);
            let malformed = task_due("not-a-date");
            let settings = Settings::default();

            let tier = evaluate_status_tier(&as_refs(&rules), &[&malformed], &settings, 0, today());

            assert_eq!(tier, StatusTier::Great);
        }

        #[test]
        fn a_task_with_no_due_date_never_matches_due_within_days() {
            let mut rules = rules_all_unset();
            rules[0].due_within_days = Some(7);
            let no_due_date = Task::new("Demo".to_string());
            let settings = Settings::default();

            let tier =
                evaluate_status_tier(&as_refs(&rules), &[&no_due_date], &settings, 0, today());

            assert_eq!(tier, StatusTier::Great);
        }

        #[test]
        fn empty_task_list_never_matches_due_within_days_or_min_priority() {
            let mut rules = rules_all_unset();
            rules[0].due_within_days = Some(7);
            rules[1].min_priority = Some("high".to_string());
            let settings = settings_with_priorities();

            let tier = evaluate_status_tier(&as_refs(&rules), &[], &settings, 0, today());

            assert_eq!(tier, StatusTier::Great);
        }

        #[test]
        fn negative_due_within_days_only_matches_already_overdue_tasks() {
            let mut rules = rules_all_unset();
            rules[0].due_within_days = Some(-1);
            let due_today = task_due("2026-06-24");
            let due_yesterday = task_due("2026-06-23");
            let settings = Settings::default();

            let tier_today =
                evaluate_status_tier(&as_refs(&rules), &[&due_today], &settings, 0, today());
            let tier_yesterday =
                evaluate_status_tier(&as_refs(&rules), &[&due_yesterday], &settings, 0, today());

            assert_eq!(tier_today, StatusTier::Great);
            assert_eq!(tier_yesterday, StatusTier::Severe);
        }

        #[test]
        fn on_track_is_the_least_severe_real_tier_and_matches_last() {
            let mut rules = rules_all_unset();
            rules[3].due_within_days = Some(7);
            let due_in_a_week = task_due("2026-07-01");
            let settings = Settings::default();

            let tier =
                evaluate_status_tier(&as_refs(&rules), &[&due_in_a_week], &settings, 0, today());

            assert_eq!(tier, StatusTier::OnTrack);
        }
    }
}
