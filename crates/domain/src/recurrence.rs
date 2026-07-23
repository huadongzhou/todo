//! What a repeat rule means, and when it happens next.
//!
//! Pure rules only, per the layering in AGENTS.md: no clock, no storage and no
//! Tauri. Every question is asked relative to two dates the caller supplies, so
//! the same rule gives the same answer on every device — which is the whole
//! point of keeping the interpretation here rather than in each end.
//!
//! The engine speaks civil dates, the way the contract stores them
//! (`YYYY-MM-DD`). A repeat says *which days* a task happens on; the time of day
//! belongs to the task and travels with the instance that is generated from it,
//! so nothing here needs a time zone and no daylight-saving change can move an
//! occurrence to another date.
//!
//! What is deliberately not here:
//!
//! * *Generating* instances — creating the next task, marking a list row as
//!   repeating — is the caller's job. This module only answers "when".
//! * The lunar calendar. A lunar rule is refused rather than answered in the
//!   wrong calendar, exactly as the calendar export drops the `RRULE` it cannot
//!   say honestly.
//! * Public holidays. A working day is the contract's own words "holiday-aware
//!   rather than Monday to Friday"; until the holiday data exists, [`is_working_day`]
//!   is Monday to Friday and is the single place that will read it.
//!
//! The reading of every field agrees with the `RRULE` mapping in [`crate::ics`],
//! so that what the app shows and what an exported file expands to are the same
//! series: the anchor is always the first occurrence (RFC 5545 treats `DTSTART`
//! that way), weeks begin on Monday, a position the calendar does not have is
//! skipped rather than moved, and an unreadable end date is no end date. The one
//! place they cannot agree is a rule carrying both an end date and a count:
//! RFC 5545 forbids writing both, so the export keeps the end date, while the
//! contract states both as stopping conditions and this engine therefore stops
//! at whichever comes first.

use todo_contracts::{
    ContractValidationError, RecurrenceCalendar, RecurrenceFrequency, RecurrenceRule, Weekday,
};

use crate::civil::{
    add_days, add_months, days_in_month, format_iso_date, parse_date, weekday_index, CivilDate,
};

/// How many positions a rule may name that the calendar does not have before the
/// search gives up.
///
/// "The 31st, every February" names one such position for ever, and a rule can
/// be stored in that shape — `validate` accepts a month day of 31 without
/// knowing which months it will land in. Nothing a user means names more than a
/// handful in a row (the 29th of February skips seven years at a century), so
/// this is far above any real rule and still finite.
const MAX_MISSING_POSITIONS: u32 = 400;

/// Why a repeat rule could not be answered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecurrenceError {
    /// A date handed in was not `YYYY-MM-DD`, or named a day that does not
    /// exist.
    UnreadableDate,
    /// The rule itself is not a usable combination; the contract says which.
    UnusableRule(ContractValidationError),
    /// A lunar rule. Answering it in the Gregorian calendar would put the
    /// occurrences on the wrong days, which is worse than not answering.
    UnsupportedCalendar,
}

impl std::fmt::Display for RecurrenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnreadableDate => {
                formatter.write_str("a repeat rule is counted from dates written as YYYY-MM-DD")
            }
            Self::UnusableRule(error) => write!(formatter, "{error}"),
            Self::UnsupportedCalendar => {
                formatter.write_str("the lunar calendar is not supported yet")
            }
        }
    }
}

impl std::error::Error for RecurrenceError {}

/// When the rule happens next, strictly after `after`.
///
/// `anchor` is the occurrence the rule is counted from — the task's own date.
/// It is always an occurrence itself, even on a day the pattern would not name
/// (a working-day rule anchored on a Saturday), which is how RFC 5545 reads
/// `DTSTART` and therefore how the exported file expands.
///
/// `Ok(None)` means the series has ended: it ran past the rule's end date, or it
/// has already produced as many occurrences as the rule counts.
///
/// Both dates are `YYYY-MM-DD`. Asking from before the anchor answers with the
/// anchor, so a caller can walk the whole series by feeding each answer back in
/// as the next `after`. A caller that walks it that way should keep passing the
/// *original* anchor: `count` and the end date are counted from the anchor, and
/// moving it forward would restart the count.
pub fn next_occurrence(
    rule: &RecurrenceRule,
    anchor: &str,
    after: &str,
) -> Result<Option<String>, RecurrenceError> {
    let anchor = parse_date(anchor).ok_or(RecurrenceError::UnreadableDate)?;
    let after = parse_date(after).ok_or(RecurrenceError::UnreadableDate)?;

    Ok(next_date(rule, anchor, after)?.map(format_iso_date))
}

/// The body of [`next_occurrence`], in dates rather than strings.
///
/// The walk is deliberately one occurrence at a time rather than a formula per
/// frequency: the two things that make a formula wrong — a position the calendar
/// does not have, and a count that must not include it — are the same in every
/// frequency, and they are handled once here instead of five times below.
fn next_date(
    rule: &RecurrenceRule,
    anchor: CivilDate,
    after: CivilDate,
) -> Result<Option<CivilDate>, RecurrenceError> {
    // An interval of zero would make the walk stand still, so the contract's own
    // check is the guard rather than a second opinion about the same rule.
    rule.validate().map_err(RecurrenceError::UnusableRule)?;
    if matches!(rule.calendar, RecurrenceCalendar::Lunar) {
        return Err(RecurrenceError::UnsupportedCalendar);
    }

    // An end date that cannot be read is no end date, as in the export: nothing
    // validates the field, and refusing the whole rule over it would stop a
    // repeat the user can still see and correct.
    let until = rule.until.as_deref().and_then(parse_date);
    let weekdays = weekly_days(rule, anchor);

    let mut current = anchor;
    let mut index: u32 = 1;
    let mut position: u64 = 0;
    let mut missing: u32 = 0;

    loop {
        if until.is_some_and(|end| current > end) {
            return Ok(None);
        }
        if rule.count.is_some_and(|count| index > count) {
            return Ok(None);
        }
        if current > after {
            return Ok(Some(current));
        }

        let next = loop {
            let taken = position;
            position += 1;
            match candidate(rule, anchor, &weekdays, taken) {
                Some(date) if date > current => break date,
                // A position the calendar does not have (the 31st of a 30-day
                // month). RFC 5545 skips it rather than moving it to the 30th,
                // and it is not an occurrence, so it must not consume a count.
                None => {
                    missing += 1;
                    if missing > MAX_MISSING_POSITIONS {
                        return Ok(None);
                    }
                }
                // A position at or before the one in hand: the days of the
                // anchor's own week that fall before the anchor.
                Some(_) => {}
            }
        };

        missing = 0;
        current = next;
        index += 1;
    }
}

/// The date at `position` of the rule's own counting, or `None` when that
/// position names a day the calendar does not have.
///
/// Position 0 is where the rule starts counting, which is the anchor for every
/// frequency except a monthly or weekly rule that names days of its own — there
/// it is the first named day of the anchor's own month or week, which may fall
/// before the anchor. The caller filters those out; generating them keeps the
/// positions evenly spaced, which is what makes an interval mean the same thing
/// in every frequency.
fn candidate(
    rule: &RecurrenceRule,
    anchor: CivilDate,
    weekdays: &[i64],
    position: u64,
) -> Option<CivilDate> {
    let interval = i64::from(rule.interval);
    let position = position as i64;

    match rule.frequency {
        RecurrenceFrequency::Daily => Some(add_days(anchor, position * interval)),
        RecurrenceFrequency::Weekly => {
            // The positions run through the named days of one week before moving
            // on, so the interval counts weeks and not occurrences: "every other
            // week on Monday and Friday" must skip a whole week, not one day.
            let named = weekdays.len() as i64;
            let monday = add_days(anchor, -weekday_index(anchor));
            let week = position / named;
            let day = weekdays[(position % named) as usize];
            Some(add_days(monday, week * interval * 7 + day))
        }
        RecurrenceFrequency::Monthly => {
            let (year, month) = add_months(anchor.year, anchor.month, position * interval);
            let day = if rule.on_last_day {
                days_in_month(year, month)
            } else {
                // No day named means the anchor's own day of the month, the same
                // reading the export leaves to `DTSTART`.
                rule.month_day.unwrap_or(anchor.day)
            };
            day_of(year, month, day)
        }
        RecurrenceFrequency::Yearly => {
            let (year, month) = add_months(anchor.year, anchor.month, position * interval * 12);
            day_of(year, month, anchor.day)
        }
        // Counted in working days, not in days: "every second working day" from a
        // Thursday is the following Monday.
        RecurrenceFrequency::Workday => Some(working_day_after(anchor, position * interval)),
    }
}

/// The day of the month, or nothing when the month is too short for it.
fn day_of(year: i64, month: u32, day: u32) -> Option<CivilDate> {
    (day <= days_in_month(year, month)).then_some(CivilDate { year, month, day })
}

/// The weekdays a weekly rule lands on, as Monday-based indices in week order.
///
/// An empty list means "the same weekday as the task's own date" (contract), so
/// the two cases are collapsed here and the rest of the engine only knows one of
/// them. Sorting is what makes the positions run forward through the week.
fn weekly_days(rule: &RecurrenceRule, anchor: CivilDate) -> Vec<i64> {
    if !matches!(rule.frequency, RecurrenceFrequency::Weekly) || rule.weekdays.is_empty() {
        return vec![weekday_index(anchor)];
    }

    let mut days: Vec<i64> = rule.weekdays.iter().map(weekday_number).collect();
    days.sort_unstable();
    days
}

fn weekday_number(weekday: &Weekday) -> i64 {
    match weekday {
        Weekday::Monday => 0,
        Weekday::Tuesday => 1,
        Weekday::Wednesday => 2,
        Weekday::Thursday => 3,
        Weekday::Friday => 4,
        Weekday::Saturday => 5,
        Weekday::Sunday => 6,
    }
}

/// The `count`-th working day after `date`; `date` itself at zero.
fn working_day_after(date: CivilDate, count: i64) -> CivilDate {
    let mut date = date;
    for _ in 0..count {
        date = add_days(date, 1);
        while !is_working_day(date) {
            date = add_days(date, 1);
        }
    }
    date
}

/// Whether a date is a working day.
///
/// Monday to Friday for now. The contract calls a working day holiday-aware, and
/// this is the one question the engine asks about a single date — when the
/// holiday and make-up-day data arrives it is read here, and every rule that
/// counts working days follows without another change.
fn is_working_day(date: CivilDate) -> bool {
    weekday_index(date) < 5
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(frequency: RecurrenceFrequency) -> RecurrenceRule {
        RecurrenceRule {
            frequency,
            interval: 1,
            weekdays: Vec::new(),
            month_day: None,
            on_last_day: false,
            calendar: RecurrenceCalendar::Gregorian,
            until: None,
            count: None,
        }
    }

    /// The first `wanted` occurrences from the anchor on, the anchor included.
    ///
    /// Walking with the answer as the next `after` is how a caller reads the
    /// series, so the tests read it the same way.
    fn series(rule: &RecurrenceRule, anchor: &str, wanted: usize) -> Vec<String> {
        let start = parse_date(anchor).expect("a readable anchor");
        let mut cursor = format_iso_date(add_days(start, -1));
        let mut dates = Vec::new();

        while dates.len() < wanted {
            match next_occurrence(rule, anchor, &cursor).expect("a rule the engine can read") {
                Some(date) => {
                    cursor.clone_from(&date);
                    dates.push(date);
                }
                None => break,
            }
        }

        dates
    }

    #[test]
    fn a_daily_rule_lands_on_every_day_from_the_anchor() {
        assert_eq!(
            series(&rule(RecurrenceFrequency::Daily), "2026-07-23", 4),
            ["2026-07-23", "2026-07-24", "2026-07-25", "2026-07-26"]
        );
    }

    #[test]
    fn a_custom_interval_counts_days_across_the_end_of_a_month() {
        let mut every_third_day = rule(RecurrenceFrequency::Daily);
        every_third_day.interval = 3;

        assert_eq!(
            series(&every_third_day, "2026-07-28", 4),
            ["2026-07-28", "2026-07-31", "2026-08-03", "2026-08-06"]
        );
    }

    #[test]
    fn a_weekly_rule_that_names_no_day_keeps_the_anchors_weekday() {
        // 2026-07-23 is a Thursday.
        assert_eq!(
            series(&rule(RecurrenceFrequency::Weekly), "2026-07-23", 3),
            ["2026-07-23", "2026-07-30", "2026-08-06"]
        );
    }

    #[test]
    fn every_other_week_skips_a_whole_week() {
        let mut fortnightly = rule(RecurrenceFrequency::Weekly);
        fortnightly.interval = 2;

        assert_eq!(
            series(&fortnightly, "2026-07-23", 3),
            ["2026-07-23", "2026-08-06", "2026-08-20"]
        );
    }

    #[test]
    fn monday_wednesday_friday_lands_on_each_of_them() {
        let mut mwf = rule(RecurrenceFrequency::Weekly);
        mwf.weekdays = vec![Weekday::Monday, Weekday::Wednesday, Weekday::Friday];

        // Anchored on a Thursday, which the rule does not name: the anchor is
        // still the first occurrence, then the named days follow. The same
        // series ical.js expands `FREQ=WEEKLY;BYDAY=MO,WE,FR` into from the same
        // start date, so the app and an exported file agree.
        assert_eq!(
            series(&mwf, "2026-07-23", 6),
            [
                "2026-07-23",
                "2026-07-24",
                "2026-07-27",
                "2026-07-29",
                "2026-07-31",
                "2026-08-03",
            ]
        );
    }

    #[test]
    fn an_interval_and_a_set_of_weekdays_count_weeks_together() {
        // The pair is the case a per-occurrence interval would get wrong: the
        // rule must skip the whole week of the 27th, not every second landing.
        let mut every_other_mwf = rule(RecurrenceFrequency::Weekly);
        every_other_mwf.interval = 2;
        every_other_mwf.weekdays = vec![Weekday::Monday, Weekday::Wednesday, Weekday::Friday];

        assert_eq!(
            series(&every_other_mwf, "2026-07-23", 6),
            [
                "2026-07-23",
                "2026-07-24",
                "2026-08-03",
                "2026-08-05",
                "2026-08-07",
                "2026-08-17",
            ]
        );
    }

    #[test]
    fn the_days_of_a_week_come_in_week_order_whatever_order_they_were_stored_in() {
        let mut written_backwards = rule(RecurrenceFrequency::Weekly);
        written_backwards.weekdays = vec![Weekday::Friday, Weekday::Monday];

        assert_eq!(
            series(&written_backwards, "2026-07-20", 4),
            ["2026-07-20", "2026-07-24", "2026-07-27", "2026-07-31"]
        );
    }

    #[test]
    fn a_monthly_rule_keeps_its_day_of_the_month() {
        let mut tenth = rule(RecurrenceFrequency::Monthly);
        tenth.month_day = Some(10);

        assert_eq!(
            series(&tenth, "2026-08-10", 3),
            ["2026-08-10", "2026-09-10", "2026-10-10"]
        );
    }

    #[test]
    fn a_monthly_rule_that_names_no_day_keeps_the_anchors_day_across_the_year_end() {
        assert_eq!(
            series(&rule(RecurrenceFrequency::Monthly), "2026-11-15", 4),
            ["2026-11-15", "2026-12-15", "2027-01-15", "2027-02-15"]
        );
    }

    #[test]
    fn a_monthly_interval_counts_months_across_the_year_end() {
        let mut every_five_months = rule(RecurrenceFrequency::Monthly);
        every_five_months.interval = 5;

        assert_eq!(
            series(&every_five_months, "2026-10-15", 3),
            ["2026-10-15", "2027-03-15", "2027-08-15"]
        );
    }

    #[test]
    fn a_month_too_short_for_the_day_is_skipped_rather_than_moved() {
        // The 31st exists in seven months; February, April, June, September and
        // November are passed over, not pulled back to their last day — which is
        // what `on_last_day` is for, and what RFC 5545 does with BYMONTHDAY=31.
        let mut thirty_first = rule(RecurrenceFrequency::Monthly);
        thirty_first.month_day = Some(31);

        assert_eq!(
            series(&thirty_first, "2026-01-31", 5),
            [
                "2026-01-31",
                "2026-03-31",
                "2026-05-31",
                "2026-07-31",
                "2026-08-31",
            ]
        );
    }

    #[test]
    fn the_last_day_of_the_month_follows_the_month_it_lands_in() {
        let mut month_end = rule(RecurrenceFrequency::Monthly);
        month_end.on_last_day = true;

        // 28, 31 and 30 in a row: the one position no fixed day number can say.
        assert_eq!(
            series(&month_end, "2026-01-31", 6),
            [
                "2026-01-31",
                "2026-02-28",
                "2026-03-31",
                "2026-04-30",
                "2026-05-31",
                "2026-06-30",
            ]
        );
    }

    #[test]
    fn the_last_day_of_february_follows_the_leap_year() {
        let mut month_end = rule(RecurrenceFrequency::Monthly);
        month_end.on_last_day = true;
        month_end.interval = 12;

        assert_eq!(
            series(&month_end, "2027-02-28", 3),
            ["2027-02-28", "2028-02-29", "2029-02-28"]
        );
    }

    #[test]
    fn the_last_day_of_the_month_is_the_last_day_whatever_the_anchor_was() {
        // Anchored on the 15th, which is nowhere near the end: the rule says
        // "the last day", so every occurrence after the anchor is one.
        let mut month_end = rule(RecurrenceFrequency::Monthly);
        month_end.on_last_day = true;

        assert_eq!(
            series(&month_end, "2026-01-15", 3),
            ["2026-01-15", "2026-01-31", "2026-02-28"]
        );
    }

    #[test]
    fn a_yearly_rule_on_a_leap_day_waits_for_the_next_leap_year() {
        assert_eq!(
            series(&rule(RecurrenceFrequency::Yearly), "2024-02-29", 3),
            ["2024-02-29", "2028-02-29", "2032-02-29"]
        );
    }

    #[test]
    fn a_yearly_rule_keeps_its_day_across_the_years() {
        let mut every_other_year = rule(RecurrenceFrequency::Yearly);
        every_other_year.interval = 2;

        assert_eq!(
            series(&every_other_year, "2026-09-01", 3),
            ["2026-09-01", "2028-09-01", "2030-09-01"]
        );
    }

    #[test]
    fn every_working_day_steps_over_the_weekend() {
        // Anchored on a Friday: the next working day is the Monday.
        assert_eq!(
            series(&rule(RecurrenceFrequency::Workday), "2026-07-24", 4),
            ["2026-07-24", "2026-07-27", "2026-07-28", "2026-07-29"]
        );
    }

    #[test]
    fn every_n_working_days_counts_in_working_days_not_in_days() {
        let mut every_other = rule(RecurrenceFrequency::Workday);
        every_other.interval = 2;

        // From Thursday: Friday is one working day on, Monday is two.
        assert_eq!(
            series(&every_other, "2026-07-23", 4),
            ["2026-07-23", "2026-07-27", "2026-07-29", "2026-07-31"]
        );
    }

    #[test]
    fn the_anchor_is_an_occurrence_even_on_a_day_the_pattern_never_names() {
        // A working-day rule anchored on a Saturday. RFC 5545 reads DTSTART the
        // same way, so an exported file expands to this series too.
        assert_eq!(
            series(&rule(RecurrenceFrequency::Workday), "2026-07-25", 3),
            ["2026-07-25", "2026-07-27", "2026-07-28"]
        );
    }

    #[test]
    fn an_end_date_keeps_its_own_day_and_stops_after_it() {
        let mut until_friday = rule(RecurrenceFrequency::Daily);
        until_friday.until = Some("2026-07-25".to_owned());

        assert_eq!(
            series(&until_friday, "2026-07-23", 10),
            ["2026-07-23", "2026-07-24", "2026-07-25"]
        );
    }

    #[test]
    fn an_end_date_before_the_anchor_ends_the_series_at_once() {
        let mut already_over = rule(RecurrenceFrequency::Daily);
        already_over.until = Some("2026-07-01".to_owned());

        assert!(series(&already_over, "2026-07-23", 5).is_empty());
    }

    #[test]
    fn a_count_includes_the_anchor() {
        let mut three_times = rule(RecurrenceFrequency::Daily);
        three_times.count = Some(3);

        assert_eq!(
            series(&three_times, "2026-07-23", 10),
            ["2026-07-23", "2026-07-24", "2026-07-25"]
        );
    }

    #[test]
    fn a_count_does_not_spend_itself_on_a_month_that_was_skipped() {
        let mut four_times = rule(RecurrenceFrequency::Monthly);
        four_times.month_day = Some(31);
        four_times.count = Some(4);

        assert_eq!(
            series(&four_times, "2026-01-31", 10),
            ["2026-01-31", "2026-03-31", "2026-05-31", "2026-07-31"]
        );
    }

    #[test]
    fn a_rule_carrying_both_ends_stops_at_whichever_comes_first() {
        // The contract states both as stopping conditions, so both hold. The
        // calendar export can only write one of the two (RFC 5545 forbids both)
        // and keeps the end date; a rule that carries both therefore repeats
        // longer in an exported file than in the app.
        let mut count_first = rule(RecurrenceFrequency::Daily);
        count_first.until = Some("2026-12-31".to_owned());
        count_first.count = Some(2);

        assert_eq!(
            series(&count_first, "2026-07-23", 10),
            ["2026-07-23", "2026-07-24"]
        );

        let mut date_first = rule(RecurrenceFrequency::Daily);
        date_first.until = Some("2026-07-24".to_owned());
        date_first.count = Some(99);

        assert_eq!(
            series(&date_first, "2026-07-23", 10),
            ["2026-07-23", "2026-07-24"]
        );
    }

    #[test]
    fn an_unreadable_end_date_is_no_end_date() {
        // The field is a free-form string that nothing validates, and the export
        // ignores one it cannot read; refusing the rule instead would stop a
        // repeat the user can still see and correct.
        let mut nonsense_end = rule(RecurrenceFrequency::Daily);
        nonsense_end.until = Some("2026-02-31".to_owned());

        assert_eq!(
            series(&nonsense_end, "2026-07-23", 2),
            ["2026-07-23", "2026-07-24"]
        );
    }

    #[test]
    fn asking_from_before_the_anchor_answers_with_the_anchor() {
        assert_eq!(
            next_occurrence(&rule(RecurrenceFrequency::Daily), "2026-07-23", "2026-01-01"),
            Ok(Some("2026-07-23".to_owned()))
        );
    }

    #[test]
    fn asking_from_far_ahead_skips_straight_to_the_day_after() {
        let mut mwf = rule(RecurrenceFrequency::Weekly);
        mwf.weekdays = vec![Weekday::Monday, Weekday::Wednesday, Weekday::Friday];

        assert_eq!(
            next_occurrence(&mwf, "2026-07-23", "2027-03-02"),
            Ok(Some("2027-03-03".to_owned()))
        );
    }

    #[test]
    fn a_day_of_the_month_no_month_ever_has_ends_the_search() {
        // The 31st of February, every February: a shape the contract accepts and
        // the calendar never satisfies. The search stops instead of running for
        // ever.
        let mut impossible = rule(RecurrenceFrequency::Monthly);
        impossible.month_day = Some(31);
        impossible.interval = 12;

        assert_eq!(
            series(&impossible, "2026-02-15", 5),
            ["2026-02-15"],
            "only the anchor, which is an occurrence by definition"
        );
    }

    #[test]
    fn a_lunar_rule_is_refused_rather_than_answered_in_the_wrong_calendar() {
        let mut lunar = rule(RecurrenceFrequency::Yearly);
        lunar.calendar = RecurrenceCalendar::Lunar;

        assert_eq!(
            next_occurrence(&lunar, "2026-09-01", "2026-09-01"),
            Err(RecurrenceError::UnsupportedCalendar)
        );
    }

    #[test]
    fn a_rule_the_contract_refuses_is_refused_here_too() {
        // An interval of zero would leave the walk standing still.
        let mut standing_still = rule(RecurrenceFrequency::Daily);
        standing_still.interval = 0;

        assert_eq!(
            next_occurrence(&standing_still, "2026-07-23", "2026-07-23"),
            Err(RecurrenceError::UnusableRule(
                ContractValidationError::InvalidRecurrence
            ))
        );

        let mut repeated_day = rule(RecurrenceFrequency::Weekly);
        repeated_day.weekdays = vec![Weekday::Monday, Weekday::Monday];

        assert_eq!(
            next_occurrence(&repeated_day, "2026-07-23", "2026-07-23"),
            Err(RecurrenceError::UnusableRule(
                ContractValidationError::DuplicateEntry
            ))
        );
    }

    #[test]
    fn a_date_the_engine_cannot_read_is_refused() {
        let daily = rule(RecurrenceFrequency::Daily);

        assert_eq!(
            next_occurrence(&daily, "2026-02-31", "2026-07-23"),
            Err(RecurrenceError::UnreadableDate)
        );
        assert_eq!(
            next_occurrence(&daily, "2026-07-23", "今天"),
            Err(RecurrenceError::UnreadableDate)
        );
        assert_eq!(
            next_occurrence(&daily, "2026-07-23T10:15:00Z", "2026-07-23"),
            Ok(Some("2026-07-24".to_owned())),
            "a date followed by a time is still a date"
        );
    }

    #[test]
    fn a_daylight_saving_change_moves_no_occurrence() {
        // A rule counts days on a wall calendar, so a day of 23 or 25 hours is
        // still one day. Both changeovers are covered: North America on
        // 2027-03-14, Europe on 2026-10-25.
        assert_eq!(
            series(&rule(RecurrenceFrequency::Daily), "2027-03-13", 3),
            ["2027-03-13", "2027-03-14", "2027-03-15"]
        );

        let mut weekly_across_the_change = rule(RecurrenceFrequency::Weekly);
        weekly_across_the_change.weekdays = vec![Weekday::Sunday];

        assert_eq!(
            series(&weekly_across_the_change, "2026-10-18", 3),
            ["2026-10-18", "2026-10-25", "2026-11-01"]
        );
    }

    #[test]
    fn the_error_says_which_rule_it_refused() {
        assert_eq!(
            RecurrenceError::UnusableRule(ContractValidationError::InvalidRecurrence).to_string(),
            "the recurrence rule is not a usable combination"
        );
        assert_eq!(
            RecurrenceError::UnsupportedCalendar.to_string(),
            "the lunar calendar is not supported yet"
        );
    }
}
