//! The Chinese lunisolar calendar, for the repeat rules that are counted in it.
//!
//! A lunar birthday is the case this exists for: "the eighth day of the fourth
//! month, every year" falls on a different civil date each time, and no amount
//! of arithmetic on the civil calendar produces it.
//!
//! # How the calendar is decided
//!
//! The calendar is not a convention that can be looked up in a rule book — it is
//! read off the sky, by three rules:
//!
//! 1. A month begins on the civil day the new moon falls on, in China Standard
//!    Time. That makes months 29 or 30 days long, in no fixed pattern.
//! 2. The month containing the December solstice is the eleventh month. The run
//!    of months from one such month to the next is a *sui*, and holds 12 months
//!    in an ordinary year and 13 in a leap one.
//! 3. In a sui of 13, the leap month is the first one containing no *major solar
//!    term* (中气 — the sun passing a multiple of 30° of longitude). It takes the
//!    number of the month before it and is marked as the leap.
//!
//! So the calendar follows from two astronomical quantities: the instants of the
//! new moons, and the sun's longitude. Both are computed here (Jean Meeus,
//! *Astronomical Algorithms*, chapters 49 and 25) rather than tabulated, which
//! is what makes this module need no yearly data update — unlike the public
//! holidays in [`crate::holidays`], the lunar calendar is not announced by
//! anyone, it is derived.
//!
//! # What it is good for, and where it stops
//!
//! The series below place a new moon within about half a minute of the full
//! theory, and the sun's longitude within about 0.01° (a quarter of an hour of
//! time). That is far more than a date needs — except for the one case that
//! decides a date anyway: an event falling within a minute of midnight in China,
//! where the computed instant and the true one can name different days. Such a
//! case is rare and, when it happens, it is the same case the published almanacs
//! disagree about.
//!
//! Two of them are in the range, and they are named here rather than left to be
//! found: the new moon of 2057-09-28 falls 2.6 seconds before midnight in China
//! and the one of 2097-08-07 twenty seconds after it, both well inside the half
//! minute this module can be out by. Every other new moon in the range clears
//! midnight by two minutes or more. A day's error at one of those two would move
//! a month start without disturbing anything the structural check looks at — the
//! months would still be 29 or 30 days and still run end to end — so
//! `the_new_moons_that_all_but_touch_midnight_are_named_rather_than_left_to_be_found`
//! is what would catch a change to the series above.
//!
//! The range is [`FIRST_YEAR`] to [`LAST_YEAR`]. Outside it the answer is `None`
//! rather than a guess: a lunar date computed outside the window these series
//! are fitted to can be wrong by a whole month, which — unlike being wrong by a
//! minute — is not a rounding error. Every caller turns that `None` into a
//! refusal the user can see.
//!
//! It begins at 1929 because that is when China began keeping standard time. The
//! calendar before it was reckoned for Beijing local mean time, UTC+7:45:40,
//! fourteen minutes behind the zone this module uses — and a new moon or a solar
//! term falling inside those fourteen minutes belongs to a different day, which
//! moves every day of the month it starts. The first day of 1916 is the case
//! that shows it: the new moon is at 16:05:18 UT on 3 February, which is 23:51
//! that evening in Beijing and 00:05 the next morning in the zone, so the year
//! begins on the 3rd by the reckoning of its own time and on the 4th by this
//! one. Answering for 1900 to 1928 would mean carrying both conventions and the
//! changeover between them, for years nothing in this app is anchored in; those
//! years are refused for the same reason 2101 is.

use crate::civil::{civil_from_days, days_from_civil, CivilDate};

/// The first lunar year this module answers for.
///
/// 1929, not 1900: it is the year China took up the standard time
/// [`CHINA_OFFSET`] states, and the calendar before it was reckoned fourteen
/// minutes earlier. See the module documentation.
pub(crate) const FIRST_YEAR: i64 = 1929;
/// The last lunar year this module answers for.
pub(crate) const LAST_YEAR: i64 = 2100;

/// Days in a mean lunation — the unit `k` counts new moons in.
const MEAN_LUNATION: f64 = 29.530_588_861;

/// Julian Day of 1970-01-01 00:00 UT, the day [`crate::civil`] counts from.
const EPOCH_JD: f64 = 2_440_587.5;

/// China Standard Time as a fraction of a day.
///
/// The calendar is defined there and nowhere else: a month begins on the day the
/// new moon falls on *in China*, whatever zone the device asking is in. A rule
/// whose dates changed when the user travelled would not be the same rule.
///
/// This is the zone the country has kept since 1929, which is why
/// [`FIRST_YEAR`] is 1929 and not earlier: applying it to the years the calendar
/// was reckoned in Beijing local mean time would put some of their months a day
/// out, silently and inside the range this module says it answers for.
const CHINA_OFFSET: f64 = 8.0 / 24.0;

/// A date on the lunar calendar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LunarDate {
    pub(crate) year: i64,
    /// 1 to 12. A leap month carries the number of the month it follows and is
    /// told apart by `leap`.
    pub(crate) month: u32,
    pub(crate) leap: bool,
    /// 1 to 29 or 30, depending on the month.
    pub(crate) day: u32,
}

/// One month of the lunar calendar, and where it sits on the civil one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LunarMonth {
    pub(crate) year: i64,
    pub(crate) month: u32,
    pub(crate) leap: bool,
    /// Days since 1970-01-01 of the first day of the month.
    start: i64,
    /// 29 or 30.
    pub(crate) length: u32,
}

/// Whether a lunar year is one this module answers for.
pub(crate) fn covers_year(year: i64) -> bool {
    (FIRST_YEAR..=LAST_YEAR).contains(&year)
}

/// The lunar date a civil date falls on, or `None` outside the supported years.
pub(crate) fn from_civil(date: CivilDate) -> Option<LunarDate> {
    let day = days_from_civil(date);

    // A sui beginning in December of year Y runs to December of Y+1, so a civil
    // date is in the one that began this December or in the one that began last.
    let mut months = sui(date.year)?;
    if day < months[0].start {
        months = sui(date.year - 1)?;
    }

    let month = months
        .iter()
        .find(|month| day >= month.start && day < month.start + i64::from(month.length))?;

    if !covers_year(month.year) {
        return None;
    }

    Some(LunarDate {
        year: month.year,
        month: month.month,
        leap: month.leap,
        day: (day - month.start + 1) as u32,
    })
}

/// The month `steps` lunar months after the one `from` falls in.
///
/// Leap months are counted: a monthly rule lands in one when it comes round,
/// which is what "every month" means on this calendar. `None` means the walk
/// left the supported years — never that the month does not exist, since every
/// step of a lunar month does.
pub(crate) fn month_after(from: LunarDate, steps: i64) -> Option<LunarMonth> {
    let mut months = sui_holding(from.year, from.month)?;
    let mut index = months.iter().position(|month| {
        month.year == from.year && month.month == from.month && month.leap == from.leap
    })?;
    let mut remaining = steps;

    loop {
        let room = (months.len() - index - 1) as i64;
        if remaining <= room {
            let month = months[index + remaining as usize];
            return covers_year(month.year).then_some(month);
        }
        remaining -= room + 1;
        months = sui(months[0].year + 1)?;
        index = 0;
    }
}

/// The month of a lunar year, or `None` when that year does not have it.
///
/// Not having it is the ordinary case for a leap month: a rule anchored in a
/// leap fourth month names a month most years do not hold, which is skipped the
/// way 29 February is skipped on the civil calendar.
pub(crate) fn month_of_year(year: i64, month: u32, leap: bool) -> Option<LunarMonth> {
    sui_holding(year, month)?
        .into_iter()
        .find(|candidate| candidate.month == month && candidate.leap == leap && candidate.year == year)
}

/// The civil date of a day of a lunar month, or `None` when the month is too
/// short for it — the thirtieth of a 29-day month, this calendar's own version
/// of 31 February.
pub(crate) fn day_in(month: &LunarMonth, day: u32) -> Option<CivilDate> {
    (day >= 1 && day <= month.length).then(|| civil_from_days(month.start + i64::from(day) - 1))
}

/// The sui holding a numbered month: months 11 and 12 belong to the one that
/// began in their own year, every other month to the one that began the year
/// before.
fn sui_holding(year: i64, month: u32) -> Option<Vec<LunarMonth>> {
    if !covers_year(year) {
        return None;
    }
    sui(if month >= 11 { year } else { year - 1 })
}

/// The months of the sui that begins with the eleventh month of `start_year`,
/// numbered and marked.
///
/// This is where the three rules in the module documentation are applied, and
/// the only place that decides what a month is called.
fn sui(start_year: i64) -> Option<Vec<LunarMonth>> {
    if !(FIRST_YEAR - 1..=LAST_YEAR).contains(&start_year) {
        return None;
    }

    let first = new_moon_on_or_before(winter_solstice_day(start_year)?);
    let last = new_moon_on_or_before(winter_solstice_day(start_year + 1)?);
    let count = last - first;
    // 12 or 13 is the whole range a sui can have; anything else means the
    // arithmetic above went wrong, and a made-up calendar is worse than none.
    if count != 12 && count != 13 {
        return None;
    }

    let starts: Vec<i64> = (0..=count).map(|step| new_moon_day(first + step)).collect();
    // The eleventh month holds the solstice by construction, so the search for
    // the month without a major term starts after it.
    let leap = if count == 13 {
        (1..count as usize).find(|&index| !holds_major_term(starts[index], starts[index + 1]))
    } else {
        None
    };
    if count == 13 && leap.is_none() {
        return None;
    }

    let mut months: Vec<LunarMonth> = Vec::with_capacity(count as usize);
    let mut number: u32 = 11;
    let mut year = start_year;

    for index in 0..count as usize {
        let length = (starts[index + 1] - starts[index]) as u32;
        if leap == Some(index) {
            // A leap month repeats the number of the month it follows and does
            // not advance the count, which is what keeps a 13-month year from
            // running to a thirteenth number.
            let previous = months.last()?;
            months.push(LunarMonth {
                year: previous.year,
                month: previous.month,
                leap: true,
                start: starts[index],
                length,
            });
            continue;
        }

        months.push(LunarMonth {
            year,
            month: number,
            leap: false,
            start: starts[index],
            length,
        });
        if number == 12 {
            // The lunar year turns between the twelfth month and the first, not
            // at the solstice the sui is measured from.
            number = 1;
            year += 1;
        } else {
            number += 1;
        }
    }

    Some(months)
}

/// Whether a major solar term falls in `[start, next)`.
///
/// Asked as "did the sun change its 30° sector between the two month starts"
/// rather than by finding the term's instant: the sun moves about 29° in a lunar
/// month and the sectors are 30° wide, so a month holds at most one, and the
/// sector at midnight of each month start is all the question needs.
fn holds_major_term(start: i64, next: i64) -> bool {
    major_term_index(start) != major_term_index(next)
}

/// Which 30° sector of the sun's longitude a civil day begins in, counted from
/// the December solstice.
fn major_term_index(day: i64) -> i64 {
    let longitude = solar_longitude(terrestrial_time(day as f64 + EPOCH_JD - CHINA_OFFSET));
    (normalize_degrees(longitude - 270.0) / 30.0).floor() as i64
}

/// The civil day, in China, of the December solstice of `year`.
fn winter_solstice_day(year: i64) -> Option<i64> {
    if !(FIRST_YEAR - 1..=LAST_YEAR + 1).contains(&year) {
        return None;
    }

    let guess = days_from_civil(CivilDate {
        year,
        month: 12,
        day: 21,
    }) as f64
        + EPOCH_JD;
    Some(china_day(universal_time(solar_term_jde(270.0, guess))))
}

/// The civil day, in China, of the `k`-th new moon.
fn new_moon_day(k: i64) -> i64 {
    china_day(universal_time(new_moon_jde(k as f64)))
}

/// The `k` of the last new moon on or before a civil day.
fn new_moon_on_or_before(day: i64) -> i64 {
    let mut k = (((day as f64 + EPOCH_JD) - 2_451_550.097_66) / MEAN_LUNATION).floor() as i64;
    while new_moon_day(k) > day {
        k -= 1;
    }
    while new_moon_day(k + 1) <= day {
        k += 1;
    }
    k
}

/// Which civil day in China a Julian Day in universal time falls on.
fn china_day(jd: f64) -> i64 {
    (jd + CHINA_OFFSET + 0.5).floor() as i64 - 2_440_588
}

/// The instant a new moon is exact, in dynamical time (Meeus, chapter 49).
///
/// `k` is whole for a new moon and counts from the one of 2000 January 6. The
/// largest correction below is about ten hours, so nothing here is decoration.
fn new_moon_jde(k: f64) -> f64 {
    let t = k / 1_236.85;
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;

    let mut jde = 2_451_550.097_66 + MEAN_LUNATION * k + 0.000_154_37 * t2
        - 0.000_000_150 * t3
        + 0.000_000_000_73 * t4;

    // The eccentricity of the earth's orbit, which slowly changes and scales
    // every term the sun's anomaly appears in.
    let e = 1.0 - 0.002_516 * t - 0.000_007_4 * t2;
    let sun = (2.5534 + 29.105_356_70 * k - 0.000_001_4 * t2 - 0.000_000_11 * t3).to_radians();
    let moon = (201.5643 + 385.816_935_28 * k + 0.010_758_2 * t2 + 0.000_012_38 * t3
        - 0.000_000_058 * t4)
        .to_radians();
    let node = (160.7108 + 390.670_502_84 * k - 0.001_611_8 * t2 - 0.000_002_27 * t3
        + 0.000_000_011 * t4)
        .to_radians();
    let ascending =
        (124.7746 - 1.563_755_88 * k + 0.002_067_2 * t2 + 0.000_002_15 * t3).to_radians();

    jde += -0.407_20 * moon.sin()
        + 0.172_41 * e * sun.sin()
        + 0.016_08 * (2.0 * moon).sin()
        + 0.010_39 * (2.0 * node).sin()
        + 0.007_39 * e * (moon - sun).sin()
        - 0.005_14 * e * (moon + sun).sin()
        + 0.002_08 * e * e * (2.0 * sun).sin()
        - 0.001_11 * (moon - 2.0 * node).sin()
        - 0.000_57 * (moon + 2.0 * node).sin()
        + 0.000_56 * e * (2.0 * moon + sun).sin()
        - 0.000_42 * (3.0 * moon).sin()
        + 0.000_42 * e * (sun + 2.0 * node).sin()
        + 0.000_38 * e * (sun - 2.0 * node).sin()
        - 0.000_24 * e * (2.0 * moon - sun).sin()
        - 0.000_17 * ascending.sin()
        - 0.000_07 * (moon + 2.0 * sun).sin()
        + 0.000_04 * (2.0 * moon - 2.0 * node).sin()
        + 0.000_04 * (3.0 * sun).sin()
        + 0.000_03 * (moon + sun - 2.0 * node).sin()
        + 0.000_03 * (2.0 * moon + 2.0 * node).sin()
        - 0.000_03 * (moon + sun + 2.0 * node).sin()
        + 0.000_03 * (moon - sun + 2.0 * node).sin()
        - 0.000_02 * (moon - sun - 2.0 * node).sin()
        - 0.000_02 * (3.0 * moon + sun).sin()
        + 0.000_02 * (4.0 * moon).sin();

    // The planetary arguments: half a minute in all, which matters only because
    // half a minute is enough to carry an event across midnight.
    jde + 0.000_325 * sin_degrees(299.77 + 0.107_408 * k - 0.009_173 * t2)
        + 0.000_165 * sin_degrees(251.88 + 0.016_321 * k)
        + 0.000_164 * sin_degrees(251.83 + 26.651_886 * k)
        + 0.000_126 * sin_degrees(349.42 + 36.412_478 * k)
        + 0.000_110 * sin_degrees(84.66 + 18.206_239 * k)
        + 0.000_062 * sin_degrees(141.74 + 53.303_771 * k)
        + 0.000_060 * sin_degrees(207.14 + 2.453_732 * k)
        + 0.000_056 * sin_degrees(154.84 + 7.306_860 * k)
        + 0.000_047 * sin_degrees(34.52 + 27.261_239 * k)
        + 0.000_042 * sin_degrees(207.19 + 0.121_824 * k)
        + 0.000_040 * sin_degrees(291.34 + 1.844_379 * k)
        + 0.000_037 * sin_degrees(161.72 + 24.198_154 * k)
        + 0.000_035 * sin_degrees(239.56 + 25.513_099 * k)
        + 0.000_023 * sin_degrees(331.55 + 3.592_518 * k)
}

/// The sun's apparent longitude in degrees at a dynamical-time Julian Day
/// (Meeus, chapter 25, the shorter series).
fn solar_longitude(jde: f64) -> f64 {
    let t = (jde - 2_451_545.0) / 36_525.0;
    let mean_longitude = 280.466_46 + 36_000.769_83 * t + 0.000_303_2 * t * t;
    let anomaly = (357.529_11 + 35_999.050_29 * t - 0.000_153_7 * t * t).to_radians();
    let centre = (1.914_602 - 0.004_817 * t - 0.000_014 * t * t) * anomaly.sin()
        + (0.019_993 - 0.000_101 * t) * (2.0 * anomaly).sin()
        + 0.000_289 * (3.0 * anomaly).sin();
    // From true to apparent: aberration, and the nutation in longitude taken as
    // its one dominant term.
    let ascending = 125.04 - 1_934.136 * t;
    normalize_degrees(mean_longitude + centre - 0.005_69 - 0.004_78 * sin_degrees(ascending))
}

/// When the sun's apparent longitude reaches `target`, starting from a guess
/// within a few days.
///
/// Newton's method on a function that is very nearly a straight line — the sun
/// covers 0.9856° a day — so a handful of steps settle it well under a second.
fn solar_term_jde(target: f64, guess: f64) -> f64 {
    let mut jde = guess;
    for _ in 0..10 {
        let difference = signed_degrees(solar_longitude(jde) - target);
        if difference.abs() < 1e-9 {
            break;
        }
        jde -= difference / 0.985_647_3;
    }
    jde
}

fn sin_degrees(degrees: f64) -> f64 {
    degrees.to_radians().sin()
}

/// An angle brought into `[0, 360)`.
fn normalize_degrees(degrees: f64) -> f64 {
    degrees.rem_euclid(360.0)
}

/// An angle brought into `[-180, 180)`, which is the form a difference has to
/// take before it can be read as "how far short".
fn signed_degrees(degrees: f64) -> f64 {
    normalize_degrees(degrees + 180.0) - 180.0
}

fn universal_time(jde: f64) -> f64 {
    jde - delta_t_days(jde)
}

fn terrestrial_time(jd: f64) -> f64 {
    jd + delta_t_days(jd)
}

/// ΔT — how far ahead of universal time the dynamical time above runs, in days.
///
/// Measured values every ten years to 2020 and the conventional parabolic
/// extrapolation after it, read off by straight lines in between. The whole
/// quantity is a matter of minutes and the error of this estimate a matter of
/// seconds, which can only change an answer for an event falling within seconds
/// of midnight; leaving it out altogether would be a minute or two of error,
/// which is why it is here at all.
fn delta_t_days(jd: f64) -> f64 {
    /// Seconds of ΔT at 1900, 1910, … 2100.
    const SECONDS: [f64; 21] = [
        -2.8, 10.4, 21.2, 24.0, 24.3, 29.1, 33.2, 40.2, 50.5, 56.9, 63.8, 66.1, 69.4, 77.6, 84.8,
        93.0, 113.7, 135.0, 156.9, 179.5, 202.7,
    ];

    let year = 2000.0 + (jd - 2_451_545.0) / 365.25;
    let position = ((year - 1900.0) / 10.0).clamp(0.0, (SECONDS.len() - 1) as f64);
    let step = position.floor() as usize;
    let seconds = if step + 1 < SECONDS.len() {
        SECONDS[step] + (SECONDS[step + 1] - SECONDS[step]) * (position - step as f64)
    } else {
        SECONDS[step]
    };

    seconds / 86_400.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn civil(year: i64, month: u32, day: u32) -> CivilDate {
        CivilDate { year, month, day }
    }

    fn lunar(year: i64, month: u32, leap: bool, day: u32) -> LunarDate {
        LunarDate {
            year,
            month,
            leap,
            day,
        }
    }

    /// The civil date of the first day of the first month — new year's day on
    /// this calendar, and the one date of it everybody can check.
    fn new_year(year: i64) -> Option<CivilDate> {
        day_in(&month_of_year(year, 1, false)?, 1)
    }

    #[test]
    fn new_years_day_falls_where_the_almanacs_say_it_does() {
        // Spread over the supported range, and including every year the holiday
        // table covers so that the two data sources check each other.
        for (year, month, day) in [
            (1929, 2, 10),
            (1930, 1, 30),
            (1949, 1, 29),
            (1970, 2, 6),
            (1980, 2, 16),
            (1990, 1, 27),
            (2000, 2, 5),
            (2010, 2, 14),
            (2019, 2, 5),
            (2020, 1, 25),
            (2021, 2, 12),
            (2022, 2, 1),
            (2023, 1, 22),
            (2024, 2, 10),
            (2025, 1, 29),
            (2026, 2, 17),
            (2027, 2, 6),
            (2028, 1, 26),
            (2029, 2, 13),
            (2030, 2, 3),
            (2033, 1, 31),
            (2035, 2, 8),
        ] {
            assert_eq!(new_year(year), Some(civil(year, month, day)), "{year}");
        }

        // And across the whole range, the invariant that would catch a month
        // numbered one out anywhere in it: the first month begins between
        // 21 January and 21 February, never outside.
        for year in FIRST_YEAR..=LAST_YEAR {
            let day = new_year(year).expect("a year inside the supported range");
            assert_eq!(day.year, year, "{year}");
            let within = (day.month == 1 && day.day >= 21) || (day.month == 2 && day.day <= 21);
            assert!(within, "{year} begins on {day:?}");
        }
    }

    #[test]
    fn a_leap_month_is_found_where_it_belongs() {
        for (year, month) in [
            (2001, 4),
            (2004, 2),
            (2006, 7),
            (2009, 5),
            (2012, 4),
            (2014, 9),
            (2017, 6),
            (2020, 4),
            (2023, 2),
            (2025, 6),
            (2028, 5),
            (2031, 3),
            (2033, 11),
        ] {
            assert!(
                month_of_year(year, month, true).is_some(),
                "{year} should hold a leap month {month}"
            );
        }

        for year in [2021, 2022, 2024, 2026, 2027, 2029, 2030, 2032] {
            assert!(
                (1..=12).all(|month| month_of_year(year, month, true).is_none()),
                "{year} should hold no leap month"
            );
        }
    }

    #[test]
    fn the_days_the_holiday_notices_name_in_lunar_terms_agree() {
        // The published arrangements in `crate::holidays` state some of their
        // days in lunar terms, which makes them an independent check on this
        // module: the dragon boat festival is the fifth of the fifth month and
        // the mid-autumn festival the fifteenth of the eighth.
        for (year, civil_month, civil_day) in [(2024, 6, 10), (2025, 5, 31), (2026, 6, 19)] {
            assert_eq!(
                from_civil(civil(year, civil_month, civil_day)),
                Some(lunar(year, 5, false, 5)),
                "dragon boat {year}"
            );
        }

        for (year, civil_month, civil_day) in [(2024, 9, 17), (2025, 10, 6), (2026, 9, 25)] {
            assert_eq!(
                from_civil(civil(year, civil_month, civil_day)),
                Some(lunar(year, 8, false, 15)),
                "mid-autumn {year}"
            );
        }

        // "2月15日（农历腊月二十八）至23日（农历正月初七）", from the 2026 notice.
        assert_eq!(
            from_civil(civil(2026, 2, 15)),
            Some(lunar(2025, 12, false, 28))
        );
        assert_eq!(
            from_civil(civil(2026, 2, 23)),
            Some(lunar(2026, 1, false, 7))
        );
        // "1月28日（农历除夕）", from the 2025 notice: the last day of a twelfth
        // month that happens to be 29 days long.
        assert_eq!(
            from_civil(civil(2025, 1, 28)),
            Some(lunar(2024, 12, false, 29))
        );
    }

    #[test]
    fn a_lunar_date_and_a_civil_one_say_the_same_day_in_both_directions() {
        for (year, month, day) in [(2026, 2, 17), (2025, 8, 29), (2020, 5, 23), (2033, 12, 31)] {
            let date = civil(year, month, day);
            let lunar = from_civil(date).expect("a date inside the supported years");
            let month = month_of_year(lunar.year, lunar.month, lunar.leap).expect("its own month");
            assert_eq!(day_in(&month, lunar.day), Some(date));
        }
    }

    #[test]
    fn every_supported_year_is_a_calendar_that_holds_together() {
        // Structure is what a wrong new moon shows up in: months are 29 or 30
        // days, they run end to end without a gap or an overlap, a sui holds 12
        // or 13 of them with at most one leap, and the numbering runs 11, 12, 1
        // … 10 whatever the leap does to it.
        let mut previous_end: Option<i64> = None;

        for start_year in FIRST_YEAR - 1..=LAST_YEAR {
            let months = sui(start_year).expect("a sui inside the supported years");
            assert!(
                months.len() == 12 || months.len() == 13,
                "the sui of {start_year} holds {} months",
                months.len()
            );
            assert!(
                months.iter().filter(|month| month.leap).count() <= 1,
                "the sui of {start_year} holds more than one leap month"
            );

            let numbers: Vec<u32> = months
                .iter()
                .filter(|month| !month.leap)
                .map(|month| month.month)
                .collect();
            assert_eq!(numbers, [11, 12, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

            for month in &months {
                assert!(
                    month.length == 29 || month.length == 30,
                    "{}-{} is {} days",
                    month.year,
                    month.month,
                    month.length
                );
                if let Some(end) = previous_end {
                    assert_eq!(
                        month.start, end,
                        "{}-{} does not follow on",
                        month.year, month.month
                    );
                }
                previous_end = Some(month.start + i64::from(month.length));
            }
        }
    }

    #[test]
    fn the_range_starts_where_china_started_keeping_standard_time() {
        // 1929 is the first year the zone this module computes in is the zone
        // the calendar was reckoned in. Its first day is 10 February, and the
        // months of that lunar year are decided by a December solstice that
        // falls ten hours from midnight — far enough that the fourteen minutes
        // between the two conventions cannot move which month holds it.
        assert_eq!(new_year(FIRST_YEAR), Some(civil(1929, 2, 10)));
        assert!(new_year(FIRST_YEAR - 1).is_none());

        // 1916 is the year that showed the two conventions apart: its new moon
        // is at 23:51 in Beijing local mean time on 3 February and at 00:05 on
        // the 4th in the zone, so this module would have answered the 4th. It is
        // refused now rather than answered wrongly.
        assert!(from_civil(civil(1916, 2, 3)).is_none());
        assert!(from_civil(civil(1916, 2, 4)).is_none());

        // The first weeks of 1929 still belong to the twelfth month of 1928 and
        // are refused with it; the year answers from its first day on.
        assert!(from_civil(civil(1929, 1, 15)).is_none());
        assert_eq!(
            from_civil(civil(1929, 2, 10)),
            Some(lunar(1929, 1, false, 1))
        );
    }

    #[test]
    fn the_new_moons_that_all_but_touch_midnight_are_named_rather_than_left_to_be_found() {
        // The two events in the whole range that fall inside this module's own
        // margin of error: the new moon of 2057-09-28 is 2.6 seconds before
        // midnight in China and the one of 2097-08-07 twenty seconds after it.
        // Nothing structural can tell if either moves a day, so the months they
        // begin are pinned here — a change to the series above that shifts one
        // of them fails this and nothing else.
        assert_eq!(
            from_civil(civil(2057, 9, 28)),
            Some(lunar(2057, 9, false, 1))
        );
        assert_eq!(
            from_civil(civil(2097, 8, 7)),
            Some(lunar(2097, 7, false, 1))
        );

        // And the margin itself, so that the two above are known to be the only
        // ones: every new moon in the range is either one of these two or a good
        // two minutes clear of midnight.
        let mut inside_a_minute = Vec::new();
        for k in -1_000..=1_500 {
            let jd = universal_time(new_moon_jde(k as f64)) + CHINA_OFFSET + 0.5;
            let day = civil_from_days(jd.floor() as i64 - 2_440_588);
            if !covers_year(day.year) {
                continue;
            }
            let into_the_day = jd - jd.floor();
            if into_the_day.min(1.0 - into_the_day) * 86_400.0 < 60.0 {
                inside_a_minute.push(day);
            }
        }

        assert_eq!(
            inside_a_minute,
            [civil(2057, 9, 28), civil(2097, 8, 7)],
            "the new moons within a minute of midnight in China"
        );
    }

    #[test]
    fn a_year_outside_the_supported_range_is_refused_rather_than_guessed() {
        assert!(new_year(FIRST_YEAR).is_some());
        assert!(new_year(LAST_YEAR).is_some());
        assert!(new_year(FIRST_YEAR - 1).is_none());
        assert!(new_year(LAST_YEAR + 1).is_none());
        assert!(!covers_year(LAST_YEAR + 1));

        // The days at the very edges: the first civil days of 1929 still belong
        // to the lunar year 1928 and are refused with it.
        assert!(from_civil(civil(FIRST_YEAR, 1, 1)).is_none());
        assert!(from_civil(civil(LAST_YEAR + 1, 6, 1)).is_none());
        assert!(from_civil(civil(FIRST_YEAR, 12, 31)).is_some());
    }

    #[test]
    fn counting_months_forward_walks_through_the_leap_months() {
        // 2023 holds a leap second month, so the twelve months after its first
        // end in the twelfth, not in the first of 2024.
        let after_twelve = month_after(lunar(2023, 1, false, 1), 12).expect("inside the range");
        assert_eq!(
            (after_twelve.year, after_twelve.month, after_twelve.leap),
            (2023, 12, false)
        );

        let after_one = month_after(lunar(2023, 2, false, 1), 1).expect("inside the range");
        assert_eq!(
            (after_one.year, after_one.month, after_one.leap),
            (2023, 2, true)
        );

        // Across a sui boundary, which is where the months of one year are
        // handed over to the next.
        let across = month_after(lunar(2025, 11, false, 1), 3).expect("inside the range");
        assert_eq!((across.year, across.month, across.leap), (2026, 2, false));

        assert!(month_after(lunar(LAST_YEAR, 12, false, 1), 1).is_none());
    }

    #[test]
    fn a_day_the_month_is_too_short_for_has_no_civil_date() {
        let short = month_of_year(2024, 12, false).expect("inside the range");
        assert_eq!(short.length, 29);
        assert_eq!(day_in(&short, 29), Some(civil(2025, 1, 28)));
        assert_eq!(day_in(&short, 30), None);
        assert_eq!(day_in(&short, 0), None);
    }

    #[test]
    fn the_astronomy_lands_where_the_published_tables_do() {
        // The solstice and the new moon on their own, as a check on the series
        // rather than on the calendar built from them. Both are read in China,
        // which is why the new moon of 2000 January 6 at 18:14 UT is a day
        // later here.
        assert_eq!(
            winter_solstice_day(2024),
            Some(days_from_civil(civil(2024, 12, 21)))
        );
        assert_eq!(
            winter_solstice_day(2026),
            Some(days_from_civil(civil(2026, 12, 22)))
        );
        assert_eq!(new_moon_day(0), days_from_civil(civil(2000, 1, 7)));

        // Every solstice in the range falls on the 21st, 22nd or 23rd of
        // December in China — the 23rd only in the earliest years, before the
        // century's worth of drift the leap rule works off.
        for year in FIRST_YEAR..=LAST_YEAR {
            let day = civil_from_days(winter_solstice_day(year).expect("inside the range"));
            assert_eq!(day.month, 12, "{year}");
            assert!((21..=23).contains(&day.day), "{year} solstice on {day:?}");
        }

        // ΔT in seconds, at the two ends and in the middle.
        let seconds =
            |year: i64| delta_t_days(days_from_civil(civil(year, 1, 1)) as f64 + EPOCH_JD) * 86_400.0;
        assert!((seconds(2000) - 63.8).abs() < 0.5, "{}", seconds(2000));
        assert!(seconds(1900) < 0.0);
        assert!(seconds(2100) > 150.0);
    }
}
