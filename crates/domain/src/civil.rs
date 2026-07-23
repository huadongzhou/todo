//! Dates on the civil calendar, and the arithmetic every rule that counts days
//! needs.
//!
//! It sits below both the calendar export and the repeat rules because the two
//! have to agree on what "the last day of the month" and "the same weekday"
//! mean: a second copy of this arithmetic would be a second set of answers,
//! waiting to disagree with the first. Nothing here reads a clock or a time
//! zone — a civil date is a date on a wall calendar, which is exactly what a due
//! date and a repeat rule are stored as.
//!
//! The two conversions are Howard Hinnant's civil-calendar algorithms, written
//! out rather than taken from a date crate because `todo-domain` depends on the
//! contracts alone (AGENTS.md).

/// A date on the proleptic Gregorian calendar.
///
/// The ordering is the field order, which is chronological for a date written
/// year first — comparing two dates is what tells a rule whether it has passed
/// its end.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CivilDate {
    pub(crate) year: i64,
    pub(crate) month: u32,
    pub(crate) day: u32,
}

/// Parses `YYYY-MM-DD`, the shape a due date is stored in.
///
/// Validity is checked by converting to a day number and back: a date that
/// survives the round trip is one that exists, which rules out 31 February
/// without a table of month lengths.
///
/// Every slice below goes through [`str::get`], never `&value[a..b]`: these
/// fields are free-form `String`s in the contract, nothing validates them, and a
/// value such as `"2026-07-日期"` would otherwise cut a multi-byte character in
/// half and panic instead of returning `None` the way this function promises.
pub(crate) fn parse_date(value: &str) -> Option<CivilDate> {
    let bytes = value.as_bytes();
    if bytes.len() < 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }

    let date = CivilDate {
        year: parse_number(value.get(0..4)?)? as i64,
        month: parse_number(value.get(5..7)?)?,
        day: parse_number(value.get(8..10)?)?,
    };

    if civil_from_days(days_from_civil(date)) == date {
        Some(date)
    } else {
        None
    }
}

/// Writes `YYYY-MM-DD`, the shape the contract stores a date in — the inverse of
/// [`parse_date`], so a date this crate produces can be read back by the same
/// code that read the user's.
pub(crate) fn format_iso_date(date: CivilDate) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

/// Reads a run of ASCII digits. Anything else — a sign, a space, a letter — is
/// not a number here, so the caller can treat the whole value as unusable.
pub(crate) fn parse_number(value: &str) -> Option<u32> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

pub(crate) fn add_days(date: CivilDate, days: i64) -> CivilDate {
    civil_from_days(days_from_civil(date) + days)
}

/// The month `months` after `(year, month)`, carrying into the year.
///
/// A month is counted as a position rather than added to a date because the day
/// does not survive the step: the month after 31 January has no 31st, and it is
/// the caller that decides whether that means "the 28th" or "not this month".
pub(crate) fn add_months(year: i64, month: u32, months: i64) -> (i64, u32) {
    let position = year * 12 + i64::from(month) - 1 + months;
    (position.div_euclid(12), (position.rem_euclid(12) + 1) as u32)
}

/// How many days a month holds, leap years included.
///
/// Taken as the distance to the first of the next month rather than from a table
/// of lengths, so the leap rule lives in exactly one place — the day-number
/// conversion below.
pub(crate) fn days_in_month(year: i64, month: u32) -> u32 {
    let (next_year, next_month) = add_months(year, month, 1);
    let first = CivilDate {
        year,
        month,
        day: 1,
    };
    let next_first = CivilDate {
        year: next_year,
        month: next_month,
        day: 1,
    };
    (days_from_civil(next_first) - days_from_civil(first)) as u32
}

/// Which day of the week a date falls on, counting Monday as 0.
///
/// Monday first because that is the order the contract lists `Weekday` in, and
/// the week `RRULE` counts from by default (RFC 5545 3.3.10, `WKST=MO`) — a
/// repeat rule and its exported form must agree on where a week starts or "every
/// other week" lands on different days in the app and in the calendar file.
pub(crate) fn weekday_index(date: CivilDate) -> i64 {
    // 1970-01-01 is day 0 and was a Thursday, three days into a week that begins
    // on Monday.
    (days_from_civil(date) + 3).rem_euclid(7)
}

/// Days since 1970-01-01, by Howard Hinnant's civil-calendar algorithm.
pub(crate) fn days_from_civil(date: CivilDate) -> i64 {
    let year = if date.month <= 2 {
        date.year - 1
    } else {
        date.year
    };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_position = i64::from((date.month + 9) % 12);
    let day_of_year = (153 * month_position + 2) / 5 + i64::from(date.day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Inverse of `days_from_civil`.
pub(crate) fn civil_from_days(days: i64) -> CivilDate {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_position = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_position + 2) / 5 + 1) as u32;
    let month = (if month_position < 10 {
        month_position + 3
    } else {
        month_position - 9
    }) as u32;

    CivilDate {
        year: if month <= 2 { year + 1 } else { year },
        month,
        day,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i64, month: u32, day: u32) -> CivilDate {
        CivilDate { year, month, day }
    }

    #[test]
    fn civil_dates_round_trip_across_the_awkward_boundaries() {
        for (year, month, day) in [
            (1970, 1, 1),
            (2000, 2, 29),
            (2026, 12, 31),
            (2100, 3, 1),
            (1969, 12, 31),
        ] {
            let date = date(year, month, day);
            assert_eq!(civil_from_days(days_from_civil(date)), date);
        }

        assert_eq!(days_from_civil(date(1970, 1, 1)), 0);
        assert_eq!(add_days(date(2026, 12, 31), 1), date(2027, 1, 1));
        // A day that does not exist must not round trip, or the parser would
        // accept 31 February.
        assert_ne!(
            civil_from_days(days_from_civil(date(2026, 2, 31))),
            date(2026, 2, 31)
        );
    }

    #[test]
    fn a_stored_date_survives_being_read_and_written_again() {
        assert_eq!(parse_date("2026-07-23"), Some(date(2026, 7, 23)));
        assert_eq!(format_iso_date(date(2026, 7, 23)), "2026-07-23");
        assert_eq!(format_iso_date(date(2028, 2, 29)), "2028-02-29");
        assert_eq!(parse_date("2026-02-31"), None);
        assert_eq!(parse_date("2026-7-23"), None);
        assert_eq!(parse_date("2026-07-日期"), None);
    }

    #[test]
    fn months_carry_into_the_year_in_both_directions() {
        assert_eq!(add_months(2026, 12, 1), (2027, 1));
        assert_eq!(add_months(2026, 1, -1), (2025, 12));
        assert_eq!(add_months(2026, 7, 0), (2026, 7));
        assert_eq!(add_months(2026, 7, 18), (2028, 1));
        assert_eq!(add_months(2026, 3, -15), (2024, 12));
    }

    #[test]
    fn month_lengths_follow_the_leap_rule_rather_than_a_table() {
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2028, 2), 29);
        // The century rule both ways: 2000 is a leap year, 2100 is not.
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 12), 31);
    }

    #[test]
    fn the_week_begins_on_monday() {
        // 2026-07-20 is a Monday, so the week runs to Sunday the 26th.
        for (day, index) in (20..=26).zip(0..7) {
            assert_eq!(weekday_index(date(2026, 7, day)), index, "2026-07-{day}");
        }
    }
}
