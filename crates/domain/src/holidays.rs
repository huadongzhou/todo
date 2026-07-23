//! Which days the published holiday arrangement calls days off, and which
//! weekend days it calls working days.
//!
//! A working day in mainland China is not Monday to Friday. The State Council
//! publishes one notice a year («国务院办公厅关于〈年份〉部分节假日安排的通知»)
//! that names the runs of days off and, to pay for the long ones, names weekend
//! days everybody works instead. A rule that repeats "every working day" is
//! wrong on both kinds of day unless it reads that notice.
//!
//! # The data, and updating it
//!
//! [`ARRANGEMENTS`] below *is* the data: one entry per year, written the way the
//! notice reads it — each holiday as the day it starts on and how many days it
//! lasts, each make-up working day as itself. Adding a year is adding one entry
//! and nothing else; no code changes with it, and there is no file to ship
//! alongside the binary, no download and no service to call. The notice for the
//! next year comes out around November, so the update lands in the release after
//! it — which is what the task calls "随版本更新".
//!
//! The table is compiled in rather than read from a file because that is what
//! makes it the same on every platform this app runs on, including the mobile
//! ones where there is no obvious place to put a data file, and because a table
//! that cannot be lost cannot be half-loaded.
//!
//! # Years the table does not cover
//!
//! [`coverage`] says how far it reaches. Beyond that there is no arrangement to
//! read — nobody has published one yet — and [`working_day`] answers `None`.
//! What the caller does with that is the caller's decision; the repeat engine
//! falls back to Monday to Friday rather than refusing to answer, because the
//! alternative is that every "every working day" task in the app stops working
//! on 1 January until the next release, which is a far larger harm than being
//! wrong about the handful of days a year the notice moves.
//!
//! That trade is only worth making while the fallback is visible, so the engine
//! does not leave it to be asked about: an occurrence that had to fall back
//! carries [`crate::recurrence::Occurrence::assumed_monday_to_friday`], and
//! `coverage` is where the years to show alongside it come from.

use crate::civil::{days_from_civil, weekday_index, CivilDate};

/// One year's arrangement, exactly as its notice states it.
struct Arrangement {
    year: i64,
    /// Each run of days off: the month and day it starts on, and how many days
    /// it lasts. A run may cross into the next month, as the spring festival of
    /// 2025 does.
    holidays: &'static [(u32, u32, u32)],
    /// Each weekend day the notice turns into a working day.
    workdays: &'static [(u32, u32)],
}

/// The published arrangements, newest last.
///
/// Sources, one notice per entry:
///
/// * 2024 — 国办发明电〔2023〕7号
/// * 2025 — 国办发明电〔2024〕12号
/// * 2026 — 国办发明电〔2025〕7号
const ARRANGEMENTS: &[Arrangement] = &[
    Arrangement {
        year: 2024,
        holidays: &[
            (1, 1, 1),   // 元旦：1月1日放假
            (2, 10, 8),  // 春节：2月10日至17日
            (4, 4, 3),   // 清明节：4月4日至6日
            (5, 1, 5),   // 劳动节：5月1日至5日
            (6, 10, 1),  // 端午节：6月10日放假
            (9, 15, 3),  // 中秋节：9月15日至17日
            (10, 1, 7),  // 国庆节：10月1日至7日
        ],
        workdays: &[(2, 4), (2, 18), (4, 7), (4, 28), (5, 11), (9, 14), (9, 29), (10, 12)],
    },
    Arrangement {
        year: 2025,
        holidays: &[
            (1, 1, 1),   // 元旦：1月1日放假
            (1, 28, 8),  // 春节：1月28日（除夕）至2月4日
            (4, 4, 3),   // 清明节：4月4日至6日
            (5, 1, 5),   // 劳动节：5月1日至5日
            (5, 31, 3),  // 端午节：5月31日至6月2日
            (10, 1, 8),  // 国庆节、中秋节：10月1日至8日
        ],
        workdays: &[(1, 26), (2, 8), (4, 27), (9, 28), (10, 11)],
    },
    Arrangement {
        year: 2026,
        holidays: &[
            (1, 1, 3),   // 元旦：1月1日至3日
            (2, 15, 9),  // 春节：2月15日至23日
            (4, 4, 3),   // 清明节：4月4日至6日
            (5, 1, 5),   // 劳动节：5月1日至5日
            (6, 19, 3),  // 端午节：6月19日至21日
            (9, 25, 3),  // 中秋节：9月25日至27日
            (10, 1, 7),  // 国庆节：10月1日至7日
        ],
        workdays: &[(1, 4), (2, 14), (2, 28), (5, 9), (9, 20), (10, 10)],
    },
];

/// The first and last year the table covers, or `None` when it is empty.
pub fn coverage() -> Option<(i64, i64)> {
    let first = ARRANGEMENTS.first()?.year;
    let last = ARRANGEMENTS.last()?.year;
    Some((first, last))
}

/// Whether a date is a working day, or `None` when no arrangement covers its
/// year.
///
/// A make-up day is checked before a holiday because the two never meet: the
/// notice cannot both give a day off and take it back, and reading the make-up
/// days first says plainly which one wins if a future notice ever did.
pub(crate) fn working_day(date: CivilDate) -> Option<bool> {
    let arrangement = ARRANGEMENTS
        .iter()
        .find(|arrangement| arrangement.year == date.year)?;

    if arrangement
        .workdays
        .iter()
        .any(|&(month, day)| month == date.month && day == date.day)
    {
        return Some(true);
    }

    if arrangement
        .holidays
        .iter()
        .any(|&run| covers(date, arrangement.year, run))
    {
        return Some(false);
    }

    Some(weekday_index(date) < 5)
}

/// Whether a run of days off holds a date.
///
/// Counted in day numbers rather than in month days so that a run crossing the
/// end of a month — "1月28日至2月4日" — needs no special case.
fn covers(date: CivilDate, year: i64, (month, day, length): (u32, u32, u32)) -> bool {
    let start = days_from_civil(CivilDate { year, month, day });
    let asked = days_from_civil(date);
    asked >= start && asked < start + i64::from(length)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i64, month: u32, day: u32) -> CivilDate {
        CivilDate { year, month, day }
    }

    #[test]
    fn the_table_says_how_far_it_reaches() {
        assert_eq!(coverage(), Some((2024, 2026)));
        assert!(working_day(date(2026, 7, 23)).is_some());
        assert_eq!(working_day(date(2027, 7, 23)), None, "not published yet");
        assert_eq!(working_day(date(2023, 7, 23)), None, "before the table");
    }

    #[test]
    fn a_year_is_written_the_way_its_notice_reads() {
        // Every run of the 2026 notice, at both of its ends, and one day inside
        // an ordinary week to show the notice only moves what it names.
        for (month, day) in [(1, 1), (1, 3), (2, 15), (2, 23), (4, 4), (4, 6)] {
            assert!(
                !working_day(date(2026, month, day)).expect("a covered year"),
                "2026-{month}-{day} is a day off"
            );
        }
        for (month, day) in [(5, 1), (5, 5), (6, 19), (6, 21), (9, 25), (10, 1), (10, 7)] {
            assert!(
                !working_day(date(2026, month, day)).expect("a covered year"),
                "2026-{month}-{day} is a day off"
            );
        }

        assert!(working_day(date(2026, 1, 5)).expect("a covered year"));
        assert!(working_day(date(2026, 5, 6)).expect("a covered year"));
        assert!(working_day(date(2026, 10, 8)).expect("a covered year"));
    }

    #[test]
    fn a_weekend_day_the_notice_names_is_a_working_day() {
        // Every make-up day of 2026, each of which is a Saturday or a Sunday.
        for (month, day) in [(1, 4), (2, 14), (2, 28), (5, 9), (9, 20), (10, 10)] {
            let date = date(2026, month, day);
            assert!(weekday_index(date) >= 5, "2026-{month}-{day} is a weekend");
            assert!(
                working_day(date).expect("a covered year"),
                "2026-{month}-{day} is worked"
            );
        }

        // And of the two years before it, where the pattern is the same.
        for (year, month, day) in [(2024, 2, 4), (2024, 10, 12), (2025, 1, 26), (2025, 10, 11)] {
            assert!(
                working_day(date(year, month, day)).expect("a covered year"),
                "{year}-{month}-{day} is worked"
            );
        }
    }

    #[test]
    fn a_weekend_the_notice_says_nothing_about_stays_a_weekend() {
        // 2026-07-25 is an ordinary Saturday in a covered year: the arrangement
        // is read, finds nothing, and the week decides.
        assert!(!working_day(date(2026, 7, 25)).expect("a covered year"));
        assert!(!working_day(date(2026, 7, 26)).expect("a covered year"));
        assert!(working_day(date(2026, 7, 24)).expect("a covered year"));
    }

    #[test]
    fn a_run_of_days_off_may_cross_the_end_of_a_month() {
        // 春节 2025: 1月28日至2月4日, eight days over two months.
        for (month, day) in [(1, 28), (1, 31), (2, 1), (2, 4)] {
            assert!(
                !working_day(date(2025, month, day)).expect("a covered year"),
                "2025-{month}-{day} is a day off"
            );
        }
        assert!(working_day(date(2025, 1, 27)).expect("a covered year"));
        assert!(working_day(date(2025, 2, 5)).expect("a covered year"));
    }
}
