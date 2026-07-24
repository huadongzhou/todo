//! Habit progress — how long a daily or weekly recurring task has been kept up,
//! and which recent days it could still be checked in on.
//!
//! Pure rules, per AGENTS.md: the schedule is read only through
//! [`crate::recurrence`], so there is one understanding of when a habit is due;
//! this module lays the set of checked-in dates over that schedule and never
//! re-derives it. No clock is read — both the streak and the make-up days follow
//! from the task's current due date (its next, not-yet-past occurrence) and the
//! recorded check-ins alone, so the same inputs give the same answer on every
//! device.
//!
//! A habit is a daily or weekly recurring task (任务管理/09). Its single row
//! advances on each check-in rather than leaving a completed sibling behind, so
//! the streak cannot be a stored counter that drifts from the truth: it is
//! counted afresh from the check-in set every time it is asked for.

use std::collections::BTreeSet;

use todo_contracts::{RecurrenceFrequency, RecurrenceRule};

use crate::civil::{add_days, format_iso_date, parse_date, CivilDate};
use crate::recurrence::{occurrences_between, RecurrenceError};

/// How many recent make-up days to offer at once. A short list of the days just
/// missed, not a calendar to fill in: a fuller history is the heat-map's job
/// (视图与统计 7.4), and offering more than a few would be a place to catch up
/// rather than a nudge back onto the streak.
const MAKEUP_LIMIT: usize = 3;

/// A habit's streak and the recent days it could still be checked in on.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HabitProgress {
    /// Scheduled days checked in a row, counting back from the last one that
    /// fell due. Zero when the most recent past due day was missed and not made
    /// up. The current, not-yet-past occurrence is never counted, so leaving
    /// today (or this week) unchecked is not yet a break — only a day that has
    /// gone by unchecked is.
    pub streak: u32,
    /// Past scheduled dates in the recent window not yet checked in, most recent
    /// first, at most [`MAKEUP_LIMIT`]. Empty when nothing recent was missed.
    pub makeup: Vec<String>,
}

/// The streak and recent make-up days for a habit.
///
/// `due_date` is the task's current due date — its next occurrence, on or after
/// today once the day-start pass has advanced it — and `check_ins` the scheduled
/// dates already checked, both local `YYYY-MM-DD`. Only a daily or weekly rule is
/// a habit; any other frequency answers with an empty progress, because a streak
/// of "months in a row" is not what a habit view means.
pub fn habit_progress(
    rule: &RecurrenceRule,
    due_date: &str,
    check_ins: &[String],
) -> Result<HabitProgress, RecurrenceError> {
    let Some(window_days) = makeup_window_days(rule) else {
        return Ok(HabitProgress::default());
    };

    let due = parse_date(due_date).ok_or(RecurrenceError::UnreadableDate)?;
    // `count` is measured from the task's original anchor, which a habit no
    // longer carries once its single row has advanced. The streak and make-up
    // walks re-anchor on a recent occurrence, so the count is dropped here and
    // the recorded check-ins are the only thing that bounds the past; everything
    // else about the rule — its interval, its named days, its calendar — is what
    // decides which days are on the grid, unchanged.
    let schedule = RecurrenceRule {
        count: None,
        ..rule.clone()
    };
    let checked: BTreeSet<&str> = check_ins.iter().map(String::as_str).collect();

    let streak = streak_count(&schedule, due, check_ins, &checked)?;
    let makeup = makeup_days(&schedule, due, window_days, &checked)?;

    Ok(HabitProgress { streak, makeup })
}

/// How far back a make-up window reaches, in whole days, or `None` for a rule
/// that is not a habit.
///
/// The span is a whole number of the rule's own periods, so its start is itself
/// an on-grid occurrence and can anchor the engine's forward walk. It is wide
/// enough to hold the [`MAKEUP_LIMIT`] most recent misses at that interval — a
/// daily habit looks back about a week of its own occurrences, a weekly one about
/// three weeks — while the cap, not the window, decides how many finally show.
fn makeup_window_days(rule: &RecurrenceRule) -> Option<i64> {
    let interval = i64::from(rule.interval);
    match rule.frequency {
        RecurrenceFrequency::Daily => Some(8 * interval),
        RecurrenceFrequency::Weekly => Some(4 * 7 * interval),
        _ => None,
    }
}

/// The earliest checked-in date, read on the civil calendar. A value that does
/// not parse — one a build with another idea of the shape might have written —
/// is skipped rather than allowed to sink the whole count.
fn earliest_check_in(check_ins: &[String]) -> Option<CivilDate> {
    check_ins.iter().filter_map(|value| parse_date(value)).min()
}

/// Scheduled days checked in a row, counting back from the day before `due`.
///
/// `due` is the next occurrence, so the days strictly before it are the ones that
/// have already fallen due; the streak is the unbroken run of them, from the most
/// recent backwards, that appears in the check-in set. The oldest checked-in day
/// is itself an occurrence and a valid re-anchor for a daily or weekly rule, so
/// the engine enumerates the grid forward from it and the run is read off the
/// tail. Bounded by the check-in history, which is itself bounded.
fn streak_count(
    schedule: &RecurrenceRule,
    due: CivilDate,
    check_ins: &[String],
    checked: &BTreeSet<&str>,
) -> Result<u32, RecurrenceError> {
    let Some(oldest) = earliest_check_in(check_ins) else {
        return Ok(0);
    };
    let last_due = add_days(due, -1);
    if last_due < oldest {
        return Ok(0);
    }

    let grid = occurrences_between(
        schedule,
        &format_iso_date(oldest),
        &format_iso_date(oldest),
        &format_iso_date(last_due),
    )?;

    let mut streak = 0;
    for occurrence in grid.iter().rev() {
        if checked.contains(occurrence.date.as_str()) {
            streak += 1;
        } else {
            break;
        }
    }
    Ok(streak)
}

/// The recent scheduled days that have fallen due but were not checked in, most
/// recent first, capped at [`MAKEUP_LIMIT`].
///
/// The window ends the day before `due` — the current occurrence is checked
/// through the row itself, never made up — and opens `window_days` earlier. Its
/// start is a whole number of periods before `due`, so it is an on-grid
/// occurrence the engine can enumerate the window forward from.
fn makeup_days(
    schedule: &RecurrenceRule,
    due: CivilDate,
    window_days: i64,
    checked: &BTreeSet<&str>,
) -> Result<Vec<String>, RecurrenceError> {
    let window_start = add_days(due, -window_days);
    let last_due = add_days(due, -1);
    if last_due < window_start {
        return Ok(Vec::new());
    }

    let grid = occurrences_between(
        schedule,
        &format_iso_date(window_start),
        &format_iso_date(window_start),
        &format_iso_date(last_due),
    )?;

    Ok(grid
        .into_iter()
        .rev()
        .filter(|occurrence| !checked.contains(occurrence.date.as_str()))
        .take(MAKEUP_LIMIT)
        .map(|occurrence| occurrence.date)
        .collect())
}

#[cfg(test)]
mod tests {
    use todo_contracts::{RecurrenceCalendar, Weekday};

    use super::*;

    fn daily() -> RecurrenceRule {
        RecurrenceRule {
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            weekdays: Vec::new(),
            month_day: None,
            on_last_day: false,
            calendar: RecurrenceCalendar::Gregorian,
            until: None,
            count: None,
        }
    }

    fn dates(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn a_run_of_checked_days_up_to_yesterday_is_the_streak() {
        // Due today (2026-07-23), the three days before it checked: today is the
        // current occurrence and not counted, so the streak is those three.
        let progress = habit_progress(
            &daily(),
            "2026-07-23",
            &dates(&["2026-07-20", "2026-07-21", "2026-07-22"]),
        )
        .expect("a habit the engine can read");
        assert_eq!(progress.streak, 3);
    }

    #[test]
    fn leaving_today_unchecked_is_not_yet_a_break() {
        // The current due day carries no weight either way: unchecked today, the
        // streak still stands at what was kept up through yesterday.
        let progress = habit_progress(&daily(), "2026-07-23", &dates(&["2026-07-22"]))
            .expect("a habit the engine can read");
        assert_eq!(progress.streak, 1);
        // And today is never offered as a make-up day.
        assert!(!progress.makeup.contains(&"2026-07-23".to_owned()));
    }

    #[test]
    fn a_missed_day_that_has_gone_by_breaks_the_streak() {
        // The day-start pass has moved the due date to 2026-07-24, so 07-23 has
        // fallen due unchecked: the streak is broken and 07-23 is the newest day
        // to make up.
        let progress = habit_progress(
            &daily(),
            "2026-07-24",
            &dates(&["2026-07-20", "2026-07-21", "2026-07-22"]),
        )
        .expect("a habit the engine can read");
        assert_eq!(progress.streak, 0);
        assert_eq!(progress.makeup.first().map(String::as_str), Some("2026-07-23"));
    }

    #[test]
    fn make_up_offers_the_recent_misses_newest_first_and_capped() {
        // Only yesterday checked: the days before it are misses, and the three
        // most recent are offered, newest first.
        let progress = habit_progress(&daily(), "2026-07-23", &dates(&["2026-07-22"]))
            .expect("a habit the engine can read");
        assert_eq!(progress.streak, 1);
        assert_eq!(
            progress.makeup,
            dates(&["2026-07-21", "2026-07-20", "2026-07-19"])
        );
    }

    #[test]
    fn a_kept_up_habit_offers_nothing_to_make_up() {
        // Every day the window reaches checked: no misses, so no buttons — the
        // common case, where only the streak pill shows.
        let progress = habit_progress(
            &daily(),
            "2026-07-23",
            &dates(&[
                "2026-07-15",
                "2026-07-16",
                "2026-07-17",
                "2026-07-18",
                "2026-07-19",
                "2026-07-20",
                "2026-07-21",
                "2026-07-22",
            ]),
        )
        .expect("a habit the engine can read");
        assert!(progress.makeup.is_empty());
        assert_eq!(progress.streak, 8);
    }

    #[test]
    fn a_daily_interval_counts_on_its_own_grid() {
        // Every other day: only the on-grid days before the due date count, and
        // the make-up window steps in twos too.
        let mut every_other = daily();
        every_other.interval = 2;

        let progress =
            habit_progress(&every_other, "2026-07-23", &dates(&["2026-07-19", "2026-07-21"]))
                .expect("a habit the engine can read");
        assert_eq!(progress.streak, 2);
        assert_eq!(progress.makeup.first().map(String::as_str), Some("2026-07-17"));
    }

    #[test]
    fn a_weekly_streak_counts_the_weeks_it_was_kept() {
        // Weekly with no named day keeps the due date's weekday (Thursday). Two
        // past Thursdays checked, the next due 2026-07-30: a two-week streak.
        let weekly = RecurrenceRule {
            frequency: RecurrenceFrequency::Weekly,
            ..daily()
        };
        let progress =
            habit_progress(&weekly, "2026-07-30", &dates(&["2026-07-16", "2026-07-23"]))
                .expect("a habit the engine can read");
        assert_eq!(progress.streak, 2);
        // The earlier Thursdays in the window are the misses to offer.
        assert_eq!(progress.makeup.first().map(String::as_str), Some("2026-07-09"));
    }

    #[test]
    fn a_weekly_rule_that_names_days_makes_up_each_of_them() {
        // Monday/Wednesday/Friday, next due Monday 2026-08-03. The Friday and
        // Wednesday just gone are unchecked, so both are recent misses.
        let mut mwf = RecurrenceRule {
            frequency: RecurrenceFrequency::Weekly,
            ..daily()
        };
        mwf.weekdays = vec![Weekday::Monday, Weekday::Wednesday, Weekday::Friday];

        let progress = habit_progress(&mwf, "2026-08-03", &dates(&["2026-07-27"]))
            .expect("a habit the engine can read");
        // 2026-07-31 (Fri) and 2026-07-29 (Wed) are the newest missed named days.
        assert_eq!(
            progress.makeup.first().map(String::as_str),
            Some("2026-07-31")
        );
        assert!(progress.makeup.contains(&"2026-07-29".to_owned()));
    }

    #[test]
    fn no_check_ins_means_no_streak_but_still_offers_recent_misses() {
        // A habit that has never been checked: streak zero, yet the last few
        // scheduled days are there to catch up on.
        let progress =
            habit_progress(&daily(), "2026-07-23", &[]).expect("a habit the engine can read");
        assert_eq!(progress.streak, 0);
        assert_eq!(
            progress.makeup,
            dates(&["2026-07-22", "2026-07-21", "2026-07-20"])
        );
    }

    #[test]
    fn a_monthly_rule_is_not_a_habit() {
        // Only daily and weekly are habits; a monthly rule answers empty rather
        // than counting "months in a row".
        let monthly = RecurrenceRule {
            frequency: RecurrenceFrequency::Monthly,
            ..daily()
        };
        let progress = habit_progress(&monthly, "2026-08-10", &dates(&["2026-06-10", "2026-07-10"]))
            .expect("the engine reads the rule");
        assert_eq!(progress, HabitProgress::default());
    }

    #[test]
    fn an_unreadable_due_date_is_refused() {
        assert!(habit_progress(&daily(), "not-a-date", &[]).is_err());
    }
}
