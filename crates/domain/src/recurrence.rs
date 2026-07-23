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
//! Two of the fields are read somewhere else, because they are not arithmetic:
//!
//! * A lunar rule counts its months and years on the calendar in
//!   [`crate::lunar`], which is computed from the sky rather than tabulated. The
//!   other frequencies are the same in both calendars — a day is a day and a
//!   week is a week — so only "monthly" and "yearly" change meaning.
//! * A working day is the contract's own words "holiday-aware rather than Monday
//!   to Friday". [`is_working_day`] is the single place that reads
//!   [`crate::holidays`], and every frequency that counts working days follows
//!   from it.
//!
//! What is deliberately not here: *generating* instances — creating the next
//! task, marking a list row as repeating — is the caller's job. This module only
//! answers "when".
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
use crate::holidays;
use crate::lunar::{self, LunarDate, LunarMonth};

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
    /// A lunar rule reaching a year [`crate::lunar`] does not answer for.
    /// Guessing at it would put the occurrences a whole month out, which is
    /// worse than not answering.
    LunarOutOfRange,
}

impl std::fmt::Display for RecurrenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnreadableDate => {
                formatter.write_str("a repeat rule is counted from dates written as YYYY-MM-DD")
            }
            Self::UnusableRule(error) => write!(formatter, "{error}"),
            Self::LunarOutOfRange => write!(
                formatter,
                "the lunar calendar is only known from {} to {}",
                lunar::FIRST_YEAR,
                lunar::LAST_YEAR
            ),
        }
    }
}

impl std::error::Error for RecurrenceError {}

/// One occurrence of a repeat rule.
///
/// A date and one thing the caller has to be told about it. The engine answers
/// for years the published holiday arrangement does not reach by reading them as
/// Monday to Friday (see [`is_working_day`]), which is the right trade only if
/// whoever shows the answer can say so; a bare date cannot be shown with that
/// caveat because nothing in it carries the caveat. Here it travels with the
/// date, and a caller cannot read one without meeting the other.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Occurrence {
    /// The civil date it falls on, `YYYY-MM-DD`.
    pub date: String,
    /// Whether the answer counted working days through a year no published
    /// arrangement covers, and read those days as Monday to Friday.
    ///
    /// Only a working-day rule can set it — no other frequency reads the
    /// arrangement at all. When it is set the date is still the engine's best
    /// answer, but it is arithmetic rather than data: whoever shows it should
    /// say which years the app is speaking for, and
    /// [`crate::holidays::coverage`] is where those years come from.
    pub assumed_monday_to_friday: bool,
}

/// When the rule happens next, strictly after `after`.
///
/// `anchor` is the occurrence the rule is counted from — the task's own date.
/// It is always an occurrence itself, even on a day the pattern would not name
/// (a working-day rule anchored on a Saturday), which is how RFC 5545 reads
/// `DTSTART` and therefore how the exported file expands.
///
/// `Ok(None)` means there is no next occurrence: the series ran past the rule's
/// end date, or it has already produced as many occurrences as the rule counts,
/// or the rule names a position the calendar does not bring round again — "the
/// 31st, every February", and its lunar counterpart, a leap month that does not
/// come back before the calendar runs out.
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
) -> Result<Option<Occurrence>, RecurrenceError> {
    let anchor = parse_date(anchor).ok_or(RecurrenceError::UnreadableDate)?;
    let after = parse_date(after).ok_or(RecurrenceError::UnreadableDate)?;

    Ok(next_date(rule, anchor, after)?.map(|date| Occurrence {
        assumed_monday_to_friday: counted_past_the_published_years(rule, anchor, date),
        date: format_iso_date(date),
    }))
}

/// Whether the walk to `date` read any day of a year the published arrangement
/// does not cover.
///
/// Only a working-day rule reads the arrangement, and it reads every day between
/// the anchor and the answer, so the two ends decide it. The table covers a run
/// of years without holes, which is what lets a pair of years stand for the
/// whole span.
fn counted_past_the_published_years(
    rule: &RecurrenceRule,
    anchor: CivilDate,
    date: CivilDate,
) -> bool {
    if !matches!(rule.frequency, RecurrenceFrequency::Workday) {
        return false;
    }

    match holidays::coverage() {
        Some((first, last)) => anchor.year < first || date.year > last,
        None => true,
    }
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

    // An end date that cannot be read is no end date, as in the export: nothing
    // validates the field, and refusing the whole rule over it would stop a
    // repeat the user can still see and correct.
    let until = rule.until.as_deref().and_then(parse_date);
    let weekdays = weekly_days(rule, anchor);

    // A rule counted on the lunar calendar is answered from its anchor read on
    // that calendar, so an anchor outside the years it knows is refused here,
    // before the walk. Afterwards the only way the walk can meet that refusal is
    // by running off the far end, which is a different thing and answered
    // differently below. Read once and carried in the cursor, so the walk's
    // lunar positions need not convert the civil anchor again at every step.
    let anchor_in_lunar = if counts_in_lunar_months(rule) {
        Some(lunar_anchor(anchor)?)
    } else {
        None
    };

    let mut cursor = Cursor {
        lunar_anchor: anchor_in_lunar,
        workday: None,
        lunar_month: None,
    };
    let mut current = anchor;
    let mut index: u32 = 1;
    let mut position: u64 = 0;
    let mut missing: u32 = 0;

    loop {
        if until.is_some_and(|end| current > end) {
            return Ok(None);
        }
        if current > after {
            return Ok(Some(current));
        }
        // The occurrence in hand is the last one the rule counts, so there is no
        // next one to go looking for. Asking anyway is not merely wasted work: a
        // lunar rule would walk off the end of its calendar and report that
        // instead of the count it had already spent.
        if rule.count.is_some_and(|count| index >= count) {
            return Ok(None);
        }

        let next = loop {
            let taken = position;
            position += 1;
            match cursor.candidate(rule, anchor, &weekdays, taken) {
                Ok(Some(date)) if date > current => break date,
                // A position the calendar does not have (the 31st of a 30-day
                // month). RFC 5545 skips it rather than moving it to the 30th,
                // and it is not an occurrence, so it must not consume a count.
                Ok(None) => {
                    missing += 1;
                    if missing > MAX_MISSING_POSITIONS {
                        return Ok(None);
                    }
                }
                // A position at or before the one in hand: the days of the
                // anchor's own week that fall before the anchor.
                Ok(Some(_)) => {}
                // The walk has reached the end of the lunar calendar. Positions
                // only run forward, so nothing beyond this one can be earlier,
                // and whether that is a refusal depends on what was being asked:
                //
                // * a rule that ends before the calendar does has simply ended —
                //   the lunar year past the end begins in a civil year past
                //   `until`, so no occurrence inside the window is being hidden;
                // * a rule whose every position since its last occurrence was
                //   one the calendar does not have (a leap month that never
                //   comes round again) is the lunar reading of "the 31st, every
                //   February", which ends the search rather than failing it;
                // * otherwise the next occurrence is a real day that this module
                //   cannot compute, and that is the one worth refusing.
                Err(error) => {
                    let rule_ends_first = until.is_some_and(|end| end.year <= lunar::LAST_YEAR);
                    if rule_ends_first || missing > 0 {
                        return Ok(None);
                    }
                    return Err(error);
                }
            }
        };

        missing = 0;
        current = next;
        index += 1;
    }
}

/// The walk's running state, so the two frequencies that reach a position by
/// counting from the anchor — a working-day rule counting working days, a lunar
/// monthly rule counting lunar months — step to the next position from the last
/// instead of counting the whole way from the anchor at every position.
///
/// [`next_date`] asks for positions 0, 1, 2, … in order and never looks back, so
/// "the position before this one" is always the last thing the cursor produced.
/// The frequencies that are a formula on the position keep nothing here.
struct Cursor {
    /// The anchor read on the lunar calendar, computed once before the walk and
    /// carried so every lunar position reads it from here rather than converting
    /// the civil anchor again. `None` unless the rule counts in lunar months.
    lunar_anchor: Option<LunarDate>,
    /// The civil date of the last working-day position; the next is `interval`
    /// working days on from it.
    workday: Option<CivilDate>,
    /// The last lunar month a monthly rule produced; the next is `interval` lunar
    /// months on from it.
    lunar_month: Option<LunarMonth>,
}

impl Cursor {
    /// The date at `position` of the rule's own counting, or `None` when that
    /// position names a day the calendar does not have.
    ///
    /// Position 0 is where the rule starts counting, which is the anchor for every
    /// frequency except a monthly or weekly rule that names days of its own — there
    /// it is the first named day of the anchor's own month or week, which may fall
    /// before the anchor. The caller filters those out; generating them keeps the
    /// positions evenly spaced, which is what makes an interval mean the same thing
    /// in every frequency.
    ///
    /// `Ok(None)` says "this position exists but the calendar has no such day",
    /// which the caller steps past. `Err` says "the calendar does not reach this
    /// position at all", which it cannot: the caller decides from what it was asking
    /// whether that ends the series or refuses it.
    ///
    /// The three frequencies that are a formula on the position compute it here and
    /// keep no state; the two that count step from the last position the cursor
    /// holds — [`Cursor::workday_step`] and [`Cursor::lunar_month_step`].
    fn candidate(
        &mut self,
        rule: &RecurrenceRule,
        anchor: CivilDate,
        weekdays: &[i64],
        position: u64,
    ) -> Result<Option<CivilDate>, RecurrenceError> {
        let interval = i64::from(rule.interval);
        let position = position as i64;
        let steps = position * interval;
        // The calendar decides what a month and a year are, and nothing else: a day
        // and a week are the same length in both, so the other three frequencies
        // never ask.
        let lunar = matches!(rule.calendar, RecurrenceCalendar::Lunar);

        let date = match rule.frequency {
            RecurrenceFrequency::Daily => Some(add_days(anchor, steps)),
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
            RecurrenceFrequency::Monthly if lunar => return self.lunar_month_step(rule, interval),
            RecurrenceFrequency::Yearly if lunar => return self.lunar_year_at(steps),
            RecurrenceFrequency::Monthly => {
                let (year, month) = add_months(anchor.year, anchor.month, steps);
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
                let (year, month) = add_months(anchor.year, anchor.month, steps * 12);
                day_of(year, month, anchor.day)
            }
            // Counted in working days, not in days: "every second working day" from a
            // Thursday is the following Monday.
            RecurrenceFrequency::Workday => Some(self.workday_step(anchor, interval)),
        };

        Ok(date)
    }

    /// The next working-day position: the anchor at position 0, then `interval`
    /// working days on from the last one answered.
    ///
    /// [`working_day_after`] composes — advancing `a` working days and then `b`
    /// lands where advancing `a + b` does, because each is a plain forward walk —
    /// so stepping `interval` from the previous answer is the very date
    /// `working_day_after(anchor, position * interval)` gives, reached in `interval`
    /// steps rather than in `position * interval`. That is what turns a far anchor
    /// from a per-position recount into a constant step.
    fn workday_step(&mut self, anchor: CivilDate, interval: i64) -> CivilDate {
        let date = match self.workday {
            Some(previous) => working_day_after(previous, interval),
            None => anchor,
        };
        self.workday = Some(date);
        date
    }

    /// A monthly rule counted in lunar months, stepped `interval` months on from
    /// the last rather than counted from the anchor every time.
    ///
    /// Every field is read the way the Gregorian branch reads it, on the calendar
    /// the rule names: `on_last_day` is the twenty-ninth or the thirtieth as that
    /// month happens to run, no day named means the anchor's own day, and a day the
    /// month is too short for is skipped rather than pulled back — the thirtieth of
    /// a 29-day month is this calendar's 31 February. Leap months are months:
    /// "every month" lands in one when it comes round.
    ///
    /// The month is kept whether or not the day exists in it, because a month too
    /// short for the day is a skipped position and not the end of the walk — the
    /// next position still steps from this month. [`lunar::month_after`] composes
    /// over the supported range the same way [`working_day_after`] does.
    fn lunar_month_step(
        &mut self,
        rule: &RecurrenceRule,
        interval: i64,
    ) -> Result<Option<CivilDate>, RecurrenceError> {
        let anchor = self.lunar_anchor();
        let month = match self.lunar_month {
            Some(previous) => lunar::month_after(month_start(previous), interval),
            None => lunar::month_after(anchor, 0),
        }
        .ok_or(RecurrenceError::LunarOutOfRange)?;
        self.lunar_month = Some(month);

        let day = if rule.on_last_day {
            month.length
        } else {
            rule.month_day.unwrap_or(anchor.day)
        };
        Ok(lunar::day_in(&month, day))
    }

    /// A yearly rule counted in lunar years — the lunar birthday.
    ///
    /// The occurrence keeps the anchor's month, its day, and whether that month was
    /// a leap one. A year without that leap month is skipped, exactly as a common
    /// year is skipped by a rule anchored on 29 February: most years have no leap
    /// fourth month, and moving the occurrence into the ordinary fourth month would
    /// be a different date from the one the user chose.
    ///
    /// A direct reading of the target year — no walk — so unlike the two stepped
    /// frequencies it needs only the anchor the cursor already holds. A year past
    /// the end of the calendar is `Err`, which the caller reads by what it was
    /// asking: some leap months do not come round again before the calendar ends,
    /// and a rule anchored in one ends its series the way "the 31st, every February"
    /// does, in [`next_date`], rather than reporting the end of the calendar.
    fn lunar_year_at(&self, steps: i64) -> Result<Option<CivilDate>, RecurrenceError> {
        let anchor = self.lunar_anchor();
        let year = anchor.year + steps;
        if !lunar::covers_year(year) {
            return Err(RecurrenceError::LunarOutOfRange);
        }

        // Inside the supported years a missing month is a real absence rather than
        // missing data, so it is a skipped position and not an error.
        Ok(lunar::month_of_year(year, anchor.month, anchor.leap)
            .and_then(|month| lunar::day_in(&month, anchor.day)))
    }

    /// The anchor read on the lunar calendar. Present whenever a lunar-counting
    /// method runs, because [`next_date`] reads and refuses it before the walk.
    fn lunar_anchor(&self) -> LunarDate {
        self.lunar_anchor
            .expect("a lunar rule reads its anchor before the walk begins")
    }
}

/// A lunar month as a date the month counting can locate it by; only the year,
/// month and leap are read, so the day is a placeholder.
fn month_start(month: LunarMonth) -> LunarDate {
    LunarDate {
        year: month.year,
        month: month.month,
        leap: month.leap,
        day: 1,
    }
}

/// The day of the month, or nothing when the month is too short for it.
fn day_of(year: i64, month: u32, day: u32) -> Option<CivilDate> {
    (day <= days_in_month(year, month)).then_some(CivilDate { year, month, day })
}

/// The anchor read on the lunar calendar, which is where a lunar rule counts
/// from.
fn lunar_anchor(anchor: CivilDate) -> Result<LunarDate, RecurrenceError> {
    lunar::from_civil(anchor).ok_or(RecurrenceError::LunarOutOfRange)
}

/// Whether the rule counts its positions on the lunar calendar.
///
/// Only "monthly" and "yearly" do. The calendar decides what a month and a year
/// are and nothing else, so a daily or weekly rule marked lunar is answered
/// without ever opening it — and is not refused for an anchor outside its years.
fn counts_in_lunar_months(rule: &RecurrenceRule) -> bool {
    matches!(rule.calendar, RecurrenceCalendar::Lunar)
        && matches!(
            rule.frequency,
            RecurrenceFrequency::Monthly | RecurrenceFrequency::Yearly
        )
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
/// The published arrangement decides it for the years [`crate::holidays`]
/// covers: a weekend day the notice turns into a working day is one, and a day
/// off it names is not, whatever weekday that falls on.
///
/// Beyond those years there is no arrangement to read, and the answer falls back
/// to Monday to Friday. That is a deliberate choice between two wrong answers:
/// refusing would stop every "every working day" task in the app on the first of
/// January until the release carrying the next notice, while falling back is
/// wrong only about the handful of days that notice moves.
///
/// The choice only holds while the fallback is not silent, so it is not left to
/// a caller to remember: an answer that had to fall back says so in
/// [`Occurrence::assumed_monday_to_friday`], and
/// [`crate::holidays::coverage`] gives the years to name when it does.
fn is_working_day(date: CivilDate) -> bool {
    holidays::working_day(date).unwrap_or_else(|| weekday_index(date) < 5)
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
                Some(occurrence) => {
                    cursor.clone_from(&occurrence.date);
                    dates.push(occurrence.date);
                }
                None => break,
            }
        }

        dates
    }

    /// One answer, as the date alone — for the tests that ask a single question
    /// and do not care what else the answer carries.
    fn next_date_only(
        rule: &RecurrenceRule,
        anchor: &str,
        after: &str,
    ) -> Result<Option<String>, RecurrenceError> {
        Ok(next_occurrence(rule, anchor, after)?.map(|occurrence| occurrence.date))
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
            next_date_only(&rule(RecurrenceFrequency::Daily), "2026-07-23", "2026-01-01"),
            Ok(Some("2026-07-23".to_owned()))
        );
    }

    #[test]
    fn asking_from_far_ahead_skips_straight_to_the_day_after() {
        let mut mwf = rule(RecurrenceFrequency::Weekly);
        mwf.weekdays = vec![Weekday::Monday, Weekday::Wednesday, Weekday::Friday];

        assert_eq!(
            next_date_only(&mwf, "2026-07-23", "2027-03-02"),
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

    /// The same rule, counted on the lunar calendar.
    fn lunar_rule(frequency: RecurrenceFrequency) -> RecurrenceRule {
        RecurrenceRule {
            calendar: RecurrenceCalendar::Lunar,
            ..rule(frequency)
        }
    }

    /// How a date reads on the lunar calendar, as month, leap and day.
    fn reading(date: &str) -> (u32, bool, u32) {
        let lunar = lunar::from_civil(parse_date(date).expect("a readable date"))
            .expect("a date inside the supported years");
        (lunar.month, lunar.leap, lunar.day)
    }

    #[test]
    fn a_lunar_yearly_rule_keeps_its_lunar_date_and_moves_on_the_civil_one() {
        // The first day of the first month — the one lunar date everybody
        // knows the civil dates of. A civil yearly rule would answer
        // 2027-02-17, which is nine days into the second month.
        assert_eq!(
            series(&lunar_rule(RecurrenceFrequency::Yearly), "2026-02-17", 5),
            [
                "2026-02-17",
                "2027-02-06",
                "2028-01-26",
                "2029-02-13",
                "2030-02-03"
            ]
        );
    }

    #[test]
    fn a_lunar_yearly_rule_on_a_leap_month_waits_for_the_year_that_has_one() {
        // 2023 holds a leap second month; most years do not, so the rule skips
        // them rather than landing in the ordinary second month, which is a
        // different date from the one the user picked. The same reading a
        // civil rule anchored on 29 February gets.
        let leap = lunar::month_of_year(2023, 2, true).expect("2023 holds a leap second month");
        let anchor = format_iso_date(lunar::day_in(&leap, 1).expect("its first day"));

        let dates = series(&lunar_rule(RecurrenceFrequency::Yearly), &anchor, 2);
        assert_eq!(dates[0], anchor);
        assert_eq!(reading(&dates[1]), (2, true, 1));

        let years_apart = parse_date(&dates[1]).expect("a readable date").year - 2023;
        assert!(
            years_apart > 10,
            "a leap month is not a yearly event, but this one came back after {years_apart} years"
        );
    }

    #[test]
    fn a_lunar_monthly_rule_lands_on_the_same_day_of_each_lunar_month() {
        // Anchored on the first day of the first month of 2026, which is
        // 2026-02-17. The civil dates are 29 or 30 days apart in no pattern —
        // that irregularity is the whole reason this cannot be a civil rule.
        let dates = series(&lunar_rule(RecurrenceFrequency::Monthly), "2026-02-17", 4);
        let readings: Vec<(u32, bool, u32)> = dates.iter().map(|date| reading(date)).collect();

        assert_eq!(
            readings,
            [(1, false, 1), (2, false, 1), (3, false, 1), (4, false, 1)]
        );
    }

    #[test]
    fn a_lunar_monthly_rule_counts_a_leap_month_as_a_month() {
        // The month after the second month of 2023 is its leap second month,
        // not the third: "every month" means every month the calendar has.
        let second = lunar::month_of_year(2023, 2, false).expect("a second month");
        let anchor = format_iso_date(lunar::day_in(&second, 1).expect("its first day"));

        let dates = series(&lunar_rule(RecurrenceFrequency::Monthly), &anchor, 3);
        let readings: Vec<(u32, bool, u32)> = dates.iter().map(|date| reading(date)).collect();

        assert_eq!(readings, [(2, false, 1), (2, true, 1), (3, false, 1)]);
    }

    #[test]
    fn a_lunar_month_too_short_for_the_day_is_skipped_rather_than_moved() {
        // The thirtieth of a 29-day month is this calendar's 31 February. The
        // months that have one are not evenly spaced, so some steps are one
        // month and others are several.
        let mut thirtieth = lunar_rule(RecurrenceFrequency::Monthly);
        thirtieth.month_day = Some(30);

        let dates = series(&thirtieth, "2026-02-17", 5);
        assert!(
            dates.iter().skip(1).all(|date| reading(date).2 == 30),
            "{dates:?}"
        );
        assert!(
            dates.windows(2).any(|pair| {
                let gap = |date: &str| {
                    crate::civil::days_from_civil(parse_date(date).expect("a readable date"))
                };
                gap(&pair[1]) - gap(&pair[0]) > 31
            }),
            "a month without a thirtieth should have been passed over: {dates:?}"
        );
    }

    #[test]
    fn the_last_day_of_a_lunar_month_is_its_own_length() {
        let mut month_end = lunar_rule(RecurrenceFrequency::Monthly);
        month_end.on_last_day = true;

        let dates = series(&month_end, "2026-02-17", 4);
        for date in dates.iter().skip(1) {
            let lunar_date =
                lunar::from_civil(parse_date(date).expect("a readable date")).expect("in range");
            let month = lunar::month_of_year(lunar_date.year, lunar_date.month, lunar_date.leap)
                .expect("its own month");
            assert_eq!(lunar_date.day, month.length, "{date}");
        }
    }

    #[test]
    fn a_day_and_a_week_are_the_same_in_both_calendars() {
        // Only "monthly" and "yearly" mean something different on the lunar
        // calendar, so a daily or weekly rule marked lunar answers exactly as
        // the Gregorian one does rather than being refused.
        assert_eq!(
            series(&lunar_rule(RecurrenceFrequency::Daily), "2026-07-23", 3),
            series(&rule(RecurrenceFrequency::Daily), "2026-07-23", 3)
        );
        assert_eq!(
            series(&lunar_rule(RecurrenceFrequency::Weekly), "2026-07-23", 3),
            series(&rule(RecurrenceFrequency::Weekly), "2026-07-23", 3)
        );
    }

    #[test]
    fn a_lunar_rule_outside_the_years_the_calendar_is_known_for_is_refused() {
        // Not answered in the wrong calendar and not quietly ended: a series
        // that has run out of data says so.
        let yearly = lunar_rule(RecurrenceFrequency::Yearly);

        assert_eq!(
            next_date_only(&yearly, "2101-02-17", "2101-02-17"),
            Err(RecurrenceError::LunarOutOfRange)
        );
        assert_eq!(
            next_date_only(&yearly, "2098-02-17", "2100-12-31"),
            Err(RecurrenceError::LunarOutOfRange)
        );
        // Inside the range the same rule answers.
        assert!(next_date_only(&yearly, "2098-02-17", "2098-02-17").is_ok());
    }

    #[test]
    fn an_anchor_the_lunar_calendar_does_not_reach_is_refused_before_anything_else() {
        // The low end of the range, which an end date must not turn into an
        // empty series: the anchor cannot be read at all, so there is nothing to
        // say the series has ended.
        let mut until_2000 = lunar_rule(RecurrenceFrequency::Yearly);
        until_2000.until = Some("2000-01-01".to_owned());

        assert_eq!(
            next_date_only(&until_2000, "1928-06-01", "1928-06-01"),
            Err(RecurrenceError::LunarOutOfRange)
        );

        // A daily rule never opens the lunar calendar, so the same anchor is no
        // trouble there.
        assert_eq!(
            next_date_only(
                &lunar_rule(RecurrenceFrequency::Daily),
                "1928-06-01",
                "1928-06-01"
            ),
            Ok(Some("1928-06-02".to_owned()))
        );
    }

    #[test]
    fn a_lunar_series_that_ends_before_the_calendar_does_ends_rather_than_fails() {
        // Both of these ask for an occurrence the rule could not have returned
        // anyway — one is past its end date, the other past its count — and
        // reporting the end of the calendar instead of the end of the rule is
        // the engine answering a question nobody asked. The Gregorian rules of
        // the same shape answer `Ok(None)`.
        let mut until_the_last_year = lunar_rule(RecurrenceFrequency::Yearly);
        until_the_last_year.until = Some("2100-12-31".to_owned());

        assert_eq!(
            next_date_only(&until_the_last_year, "2099-02-01", "2100-12-30"),
            Ok(None)
        );

        let mut once = lunar_rule(RecurrenceFrequency::Yearly);
        once.count = Some(1);

        assert_eq!(next_date_only(&once, "2100-02-09", "2100-02-09"), Ok(None));

        // The refusal is still there for the rule that carries neither: this one
        // really is asking for a year the calendar does not reach.
        assert_eq!(
            next_date_only(
                &lunar_rule(RecurrenceFrequency::Yearly),
                "2100-02-09",
                "2100-02-09"
            ),
            Err(RecurrenceError::LunarOutOfRange)
        );
    }

    #[test]
    fn a_lunar_position_that_never_comes_round_again_ends_the_search() {
        // A leap tenth month last happened in 1984 and does not happen again
        // before the calendar ends, so a yearly rule anchored in one has the
        // anchor and nothing more. That is the same answer the Gregorian branch
        // gives "the 31st, every February" — and not a report that the calendar
        // stops in 2100, which is true but is not what went wrong.
        let leap = lunar::month_of_year(1984, 10, true).expect("1984 holds a leap tenth month");
        let anchor = format_iso_date(lunar::day_in(&leap, 1).expect("its first day"));
        let yearly = lunar_rule(RecurrenceFrequency::Yearly);

        assert_eq!(
            series(&yearly, &anchor, 5),
            [anchor.clone()],
            "only the anchor, which is an occurrence by definition"
        );
        assert_eq!(next_date_only(&yearly, &anchor, &anchor), Ok(None));
    }

    #[test]
    fn an_answer_counted_past_the_published_years_says_so() {
        let workday = rule(RecurrenceFrequency::Workday);

        let inside = next_occurrence(&workday, "2026-07-23", "2026-07-23")
            .expect("a rule the engine can read")
            .expect("an occurrence");
        assert_eq!(inside.date, "2026-07-24");
        assert!(!inside.assumed_monday_to_friday);

        // The first day past the table. 2027-01-01 is a Friday and no notice
        // has been published saying it is new year's day, so the engine counts
        // it as a working day; the date is still the best answer there is, but
        // it is arithmetic rather than data and the answer carries that.
        let beyond = next_occurrence(&workday, "2026-12-31", "2026-12-31")
            .expect("a rule the engine can read")
            .expect("an occurrence");
        assert_eq!(beyond.date, "2027-01-01");
        assert!(beyond.assumed_monday_to_friday);
        assert_eq!(
            holidays::coverage(),
            Some((2024, 2026)),
            "the years to name when an answer says so"
        );

        // No other frequency reads the arrangement, so no other frequency can
        // have fallen back to the week.
        let daily = next_occurrence(&rule(RecurrenceFrequency::Daily), "2030-01-01", "2030-01-01")
            .expect("a rule the engine can read")
            .expect("an occurrence");
        assert!(!daily.assumed_monday_to_friday);
    }

    #[test]
    fn a_rule_the_contract_refuses_is_refused_here_too() {
        // An interval of zero would leave the walk standing still.
        let mut standing_still = rule(RecurrenceFrequency::Daily);
        standing_still.interval = 0;

        assert_eq!(
            next_date_only(&standing_still, "2026-07-23", "2026-07-23"),
            Err(RecurrenceError::UnusableRule(
                ContractValidationError::InvalidRecurrence
            ))
        );

        let mut repeated_day = rule(RecurrenceFrequency::Weekly);
        repeated_day.weekdays = vec![Weekday::Monday, Weekday::Monday];

        assert_eq!(
            next_date_only(&repeated_day, "2026-07-23", "2026-07-23"),
            Err(RecurrenceError::UnusableRule(
                ContractValidationError::DuplicateEntry
            ))
        );
    }

    #[test]
    fn a_date_the_engine_cannot_read_is_refused() {
        let daily = rule(RecurrenceFrequency::Daily);

        assert_eq!(
            next_date_only(&daily, "2026-02-31", "2026-07-23"),
            Err(RecurrenceError::UnreadableDate)
        );
        assert_eq!(
            next_date_only(&daily, "2026-07-23", "今天"),
            Err(RecurrenceError::UnreadableDate)
        );
        assert_eq!(
            next_date_only(&daily, "2026-07-23T10:15:00Z", "2026-07-23"),
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
    fn a_working_day_rule_works_the_saturdays_the_notice_names() {
        // 2026-02-14 is a Saturday everybody works, to pay for the nine days
        // off that follow it: the rule lands on it, then steps over the whole
        // spring festival to the Tuesday after it.
        assert_eq!(
            series(&rule(RecurrenceFrequency::Workday), "2026-02-13", 4),
            ["2026-02-13", "2026-02-14", "2026-02-24", "2026-02-25"]
        );

        // The same again in May, where the make-up Saturday stands alone: the
        // Sunday after it is still a Sunday.
        assert_eq!(
            series(&rule(RecurrenceFrequency::Workday), "2026-05-08", 3),
            ["2026-05-08", "2026-05-09", "2026-05-11"]
        );
    }

    #[test]
    fn a_working_day_rule_steps_over_a_holiday_that_falls_on_a_weekday() {
        // 2026-04-06 is a Monday off for the qingming festival, so the working
        // day after Friday the 3rd is the Tuesday.
        assert_eq!(
            series(&rule(RecurrenceFrequency::Workday), "2026-04-03", 3),
            ["2026-04-03", "2026-04-07", "2026-04-08"]
        );

        // And the seven days of the national holiday, which swallow a whole
        // working week.
        assert_eq!(
            series(&rule(RecurrenceFrequency::Workday), "2026-09-30", 2),
            ["2026-09-30", "2026-10-08"]
        );
    }

    #[test]
    fn every_n_working_days_counts_the_arrangement_too() {
        let mut every_other = rule(RecurrenceFrequency::Workday);
        every_other.interval = 2;

        // From the Thursday before the spring festival: Friday the 13th is one
        // working day on and the make-up Saturday is two, then the next two
        // working days are the 24th and the 25th.
        assert_eq!(
            series(&every_other, "2026-02-12", 3),
            ["2026-02-12", "2026-02-14", "2026-02-25"]
        );
    }

    #[test]
    fn beyond_the_published_years_a_working_day_is_monday_to_friday_again() {
        // No arrangement has been published for 2027, so the engine answers
        // from the week alone: the first of October is a Friday and the rule
        // does not know it is a holiday. Such an answer says so — see
        // `an_answer_counted_past_the_published_years_says_so`.
        assert_eq!(holidays::coverage(), Some((2024, 2026)));
        assert_eq!(
            series(&rule(RecurrenceFrequency::Workday), "2027-10-01", 2),
            ["2027-10-01", "2027-10-04"]
        );
    }

    #[test]
    fn a_long_working_day_walk_holds_its_footing_across_many_positions() {
        // One `next_occurrence` call now carries a cursor across all its
        // positions instead of recounting from the anchor at each; a long walk
        // is where a dropped or repeated step would show. Past 2026 no
        // arrangement is published, so a working day is Monday to Friday: every
        // gap is one day inside a week or three across a weekend, and never a
        // weekend day itself.
        let dates = series(&rule(RecurrenceFrequency::Workday), "2027-01-01", 300);
        assert_eq!(dates.len(), 300);
        assert_eq!(dates[0], "2027-01-01");
        for pair in dates.windows(2) {
            let earlier = parse_date(&pair[0]).expect("a readable date");
            let later = parse_date(&pair[1]).expect("a readable date");
            let gap = crate::civil::days_from_civil(later) - crate::civil::days_from_civil(earlier);
            assert!(gap == 1 || gap == 3, "{pair:?} are {gap} days apart");
            assert!(
                crate::civil::weekday_index(later) < 5,
                "{} is a weekend",
                pair[1]
            );
        }
    }

    #[test]
    fn a_long_lunar_monthly_walk_stays_on_its_day_across_many_positions() {
        // The lunar monthly cursor steps one month on from the last rather than
        // recounting the whole way from the anchor; over a long walk every
        // landing must still read back as the first day of its lunar month, and
        // the civil gaps must be the 29 or 30 days a lunar month runs — a lost or
        // doubled month would break one or the other.
        let dates = series(&lunar_rule(RecurrenceFrequency::Monthly), "2026-02-17", 120);
        assert_eq!(dates.len(), 120);
        for date in &dates {
            assert_eq!(reading(date).2, 1, "{date} is not the first of its lunar month");
        }
        for pair in dates.windows(2) {
            let earlier = parse_date(&pair[0]).expect("a readable date");
            let later = parse_date(&pair[1]).expect("a readable date");
            let gap = crate::civil::days_from_civil(later) - crate::civil::days_from_civil(earlier);
            assert!(gap == 29 || gap == 30, "{pair:?} are {gap} days apart");
        }
    }

    #[test]
    fn the_error_says_which_rule_it_refused() {
        assert_eq!(
            RecurrenceError::UnusableRule(ContractValidationError::InvalidRecurrence).to_string(),
            "the recurrence rule is not a usable combination"
        );
        assert_eq!(
            RecurrenceError::LunarOutOfRange.to_string(),
            "the lunar calendar is only known from 1929 to 2100"
        );
    }
}
