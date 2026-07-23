//! Turning tasks into an iCalendar document (RFC 5545).
//!
//! Pure rules only, per the layering in AGENTS.md: no clock, no file system and
//! no Tauri. The one thing the caller must supply is the moment the export
//! happens, because `DTSTAMP` is a required property of every event and reading
//! a clock is not a rule.
//!
//! Two shapes carry every task onto a calendar. A due date is a whole day, so it
//! becomes a `VALUE=DATE` event with no time zone at all; a timed block or a
//! reminder is an instant, so it is written in UTC (`...Z`). Writing instants in
//! UTC is what lets the file stay a single `VCALENDAR` with no `VTIMEZONE`
//! component — the one part of RFC 5545 that calendar applications disagree
//! about most.

use todo_contracts::{RecurrenceCalendar, RecurrenceFrequency, RecurrenceRule, Todo, Weekday};

/// Identifies the product that wrote the file (RFC 5545 3.7.3), in the
/// `-//owner//product//language` form the specification asks for.
const PRODUCT_ID: &str = "-//Just Do//Todo//EN";
/// Right-hand side of every `UID`. RFC 5545 3.8.4.7 wants a globally unique
/// value; a todo id is a UUID, and the suffix keeps it from colliding with an
/// unrelated calendar that happens to use the same id.
const UID_DOMAIN: &str = "just-do.local";
/// RFC 5545 3.1: a content line is folded so that no line is longer than 75
/// octets, the line break excluded.
const MAX_LINE_OCTETS: usize = 75;
const CRLF: &str = "\r\n";
const SECONDS_PER_DAY: i64 = 86_400;

/// A finished calendar file, and how many tasks made it in.
///
/// The count is what the caller reports to the user, and it is deliberately the
/// number of events written rather than the number of tasks handed in: a task
/// with no date at all cannot be placed on a calendar and is skipped.
#[derive(Clone, Debug)]
pub struct Calendar {
    pub content: String,
    pub event_count: usize,
    /// How many of those events carry a repeat rule RFC 5545 cannot say, and
    /// therefore appear on the calendar once instead of repeating (see
    /// [`recurrence_property`]). The caller reports it because a user who set a
    /// lunar birthday would otherwise have no way to learn that the exported
    /// file does not repeat it.
    pub unrepeatable_recurrences: usize,
}

/// Writes the tasks that carry a date as one `VCALENDAR` document.
///
/// `generated_at_unix` is the export moment in seconds since the Unix epoch; it
/// becomes the `DTSTAMP` of every event.
pub fn build_calendar(todos: &[Todo], generated_at_unix: i64) -> Calendar {
    let stamp = format_instant(generated_at_unix);
    let mut content = String::new();

    write_line(&mut content, "BEGIN:VCALENDAR");
    write_line(&mut content, "VERSION:2.0");
    write_line(&mut content, &format!("PRODID:{PRODUCT_ID}"));
    write_line(&mut content, "CALSCALE:GREGORIAN");

    let mut event_count = 0;
    let mut unrepeatable_recurrences = 0;
    for todo in todos {
        let Some(window) = event_window(todo) else {
            continue;
        };
        event_count += 1;

        write_line(&mut content, "BEGIN:VEVENT");
        write_line(
            &mut content,
            &format!("UID:{}@{UID_DOMAIN}", escape_text(&todo.id)),
        );
        write_line(&mut content, &format!("DTSTAMP:{stamp}"));
        write_line(
            &mut content,
            &format!("SUMMARY:{}", escape_text(&todo.title)),
        );
        write_line(&mut content, &time_property("DTSTART", &window.start));
        if let Some(end) = &window.end {
            write_line(&mut content, &time_property("DTEND", end));
        }
        if let Some(rule) = &todo.recurrence {
            match recurrence_property(rule, &window) {
                Some(property) => write_line(&mut content, &property),
                None => unrepeatable_recurrences += 1,
            }
        }
        if let Some(notes) = todo.notes.as_deref().filter(|notes| !notes.trim().is_empty()) {
            write_line(&mut content, &format!("DESCRIPTION:{}", escape_text(notes)));
        }
        write_line(&mut content, "END:VEVENT");
    }

    write_line(&mut content, "END:VCALENDAR");

    Calendar {
        content,
        event_count,
        unrepeatable_recurrences,
    }
}

/// Names the exported file after the moment it was written, in the basic format
/// (`20260723T101500Z`) — the extended one contains colons, which Windows
/// refuses in a file name.
///
/// The stamp only reaches the second, so two exports inside the same second want
/// the same name. `attempt` is how many names the caller has already found
/// taken; every attempt after the first adds a counter (`todo-…-2.ics`), which
/// is what makes "a second export never silently overwrites the first" true
/// rather than merely likely. The caller is expected to create the file
/// exclusively and come back with the next attempt when the name is in use.
pub fn export_file_name(generated_at_unix: i64, attempt: u32) -> String {
    let stamp = format_instant(generated_at_unix);
    match attempt {
        0 => format!("todo-{stamp}.ics"),
        taken => format!("todo-{stamp}-{}.ics", taken + 1),
    }
}

/// When an event starts or ends, in the two value types RFC 5545 offers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EventTime {
    /// A whole day, written as `VALUE=DATE`. It carries no time zone by design:
    /// a due date is the same day wherever the device is.
    AllDay(CivilDate),
    /// An instant, in seconds since the Unix epoch, always written as UTC.
    Instant(i64),
}

/// The span an event occupies. `end` is absent when the task says nothing about
/// how long it takes — RFC 5545 3.6.1 allows `DTEND` to be omitted, and
/// inventing a duration would put a length on the calendar the user never gave.
struct EventWindow {
    start: EventTime,
    end: Option<EventTime>,
    /// The zone the start was written in, in seconds east of UTC. It is kept
    /// because a repeat rule ends on a *local* date (see the contract) while
    /// `UNTIL` on a timed event must be a UTC instant, so the two can only be
    /// lined up if the zone is known. A whole-day event and a value that named
    /// no zone both leave it at 0 — the first does not need it, and for the
    /// second UTC is the only reading available.
    zone_offset_seconds: i64,
}

/// Decides where a task lands on the calendar, or that it cannot land at all.
///
/// The order is the order of specificity: an explicit timed block beats the due
/// date, and a reminder is used only when the task has no date of its own. A
/// start date is deliberately not a candidate — it says when a task may begin,
/// not that anything is happening that day.
fn event_window(todo: &Todo) -> Option<EventWindow> {
    if let Some(start) = todo.starts_at.as_deref().and_then(parse_instant) {
        let end = todo
            .ends_at
            .as_deref()
            .and_then(parse_instant)
            // An end at or before the start is not a span any calendar can
            // show, so the event keeps its start and drops the end.
            .filter(|end| end.unix_seconds > start.unix_seconds)
            .map(|end| EventTime::Instant(end.unix_seconds));
        return Some(EventWindow {
            start: EventTime::Instant(start.unix_seconds),
            end,
            zone_offset_seconds: start.offset_seconds,
        });
    }

    if let Some(date) = todo.due_date.as_deref().and_then(parse_date) {
        // For `VALUE=DATE`, RFC 5545 3.8.2.2 makes `DTEND` non-inclusive, so a
        // one-day event ends on the following day.
        return Some(EventWindow {
            start: EventTime::AllDay(date),
            end: Some(EventTime::AllDay(add_days(date, 1))),
            zone_offset_seconds: 0,
        });
    }

    todo.reminder_at
        .as_deref()
        .and_then(parse_instant)
        .map(|instant| EventWindow {
            start: EventTime::Instant(instant.unix_seconds),
            end: None,
            zone_offset_seconds: instant.offset_seconds,
        })
}

fn time_property(name: &str, time: &EventTime) -> String {
    match time {
        EventTime::AllDay(date) => format!("{name};VALUE=DATE:{}", format_date(*date)),
        EventTime::Instant(instant) => format!("{name}:{}", format_instant(*instant)),
    }
}

/// Maps a repeat rule onto an `RRULE`, or reports that RFC 5545 cannot say it.
///
/// Two rules have no honest translation and therefore produce no `RRULE` at all
/// — the event still exports, it simply appears once:
///
/// * a lunar rule counts months in a calendar `RRULE` does not know. Emitting
///   the Gregorian rule of the same shape would put the occurrences on the wrong
///   days, which is worse than exporting a single event.
/// * "every N working days" with N > 1. A working day is holiday-aware (see the
///   contract), and `FREQ=WEEKLY;INTERVAL=N` means "every N weeks", a different
///   thing entirely. At N = 1 the two do coincide on Monday–Friday, which is the
///   closest a standard rule gets, and that case is emitted.
fn recurrence_property(rule: &RecurrenceRule, window: &EventWindow) -> Option<String> {
    if matches!(rule.calendar, RecurrenceCalendar::Lunar) {
        return None;
    }

    let (frequency, by_parts) = match rule.frequency {
        RecurrenceFrequency::Daily => ("DAILY", Vec::new()),
        RecurrenceFrequency::Weekly => {
            let mut parts = Vec::new();
            // An empty weekday list means "the same weekday as the due date",
            // which is exactly what `RRULE` does when `BYDAY` is left out.
            if !rule.weekdays.is_empty() {
                let days: Vec<&str> = rule.weekdays.iter().map(byday_code).collect();
                parts.push(format!("BYDAY={}", days.join(",")));
            }
            ("WEEKLY", parts)
        }
        RecurrenceFrequency::Monthly => {
            let mut parts = Vec::new();
            if let Some(day) = rule.month_day {
                parts.push(format!("BYMONTHDAY={day}"));
            } else if rule.on_last_day {
                // -1 counts back from the end of the month, which is the one
                // way to say "the last day" for months of different lengths.
                parts.push("BYMONTHDAY=-1".to_owned());
            }
            ("MONTHLY", parts)
        }
        RecurrenceFrequency::Yearly => ("YEARLY", Vec::new()),
        RecurrenceFrequency::Workday => {
            if rule.interval != 1 {
                return None;
            }
            ("WEEKLY", vec!["BYDAY=MO,TU,WE,TH,FR".to_owned()])
        }
    };

    let mut parts = vec![format!("FREQ={frequency}")];
    // INTERVAL defaults to 1 (RFC 5545 3.3.10), so saying it adds nothing.
    if rule.interval > 1 {
        parts.push(format!("INTERVAL={}", rule.interval));
    }
    parts.extend(by_parts);

    // UNTIL and COUNT must not both appear, and an end date is the more
    // faithful of the two when a rule carries both.
    if let Some(date) = rule.until.as_deref().and_then(parse_date) {
        parts.push(format!("UNTIL={}", until_value(date, window)));
    } else if let Some(count) = rule.count {
        parts.push(format!("COUNT={count}"));
    }

    Some(format!("RRULE:{}", parts.join(";")))
}

/// `UNTIL` must use the same value type as `DTSTART` (RFC 5545 3.3.10): a date
/// for a whole-day event, a UTC instant for a timed one.
///
/// The stored end date is a *local* date and inclusive (see the contract), so
/// the instant that keeps the last occurrence in is the last second of that day
/// **in the event's own zone**, carried back to UTC. Taking the last second of
/// the UTC day instead drops one occurrence for an evening event west of UTC —
/// a daily 20:00 at UTC−8 happens at 04:00Z the next day, which is already past
/// `…T235959Z` of the end date. Where the stored value named no zone the offset
/// is 0 and this is the same value as before; that is not a guess, it is the
/// only reading such a value has.
fn until_value(date: CivilDate, window: &EventWindow) -> String {
    match window.start {
        EventTime::AllDay(_) => format_date(date),
        EventTime::Instant(_) => format_instant(
            days_from_civil(date) * SECONDS_PER_DAY + SECONDS_PER_DAY - 1
                - window.zone_offset_seconds,
        ),
    }
}

fn byday_code(weekday: &Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "MO",
        Weekday::Tuesday => "TU",
        Weekday::Wednesday => "WE",
        Weekday::Thursday => "TH",
        Weekday::Friday => "FR",
        Weekday::Saturday => "SA",
        Weekday::Sunday => "SU",
    }
}

/// Escapes a TEXT value (RFC 5545 3.3.11). A colon needs no escape inside TEXT,
/// which is why a title may contain one.
fn escape_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            ';' => escaped.push_str("\\;"),
            ',' => escaped.push_str("\\,"),
            '\n' => escaped.push_str("\\n"),
            // A CRLF inside a note becomes the single escaped newline above.
            '\r' => {}
            _ => escaped.push(character),
        }
    }
    escaped
}

/// Appends one content line, folded per RFC 5545 3.1.
///
/// The limit counts octets, not characters, so a title in Chinese folds at the
/// right place; folding happens on character boundaries so a multi-byte
/// character is never cut in half. Continuation lines begin with a single space,
/// which counts toward the 75.
fn write_line(out: &mut String, line: &str) {
    let mut used = 0;
    for character in line.chars() {
        let width = character.len_utf8();
        if used + width > MAX_LINE_OCTETS {
            out.push_str(CRLF);
            out.push(' ');
            used = 1;
        }
        out.push(character);
        used += width;
    }
    out.push_str(CRLF);
}

/// A date on the proleptic Gregorian calendar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CivilDate {
    year: i64,
    month: u32,
    day: u32,
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
fn parse_date(value: &str) -> Option<CivilDate> {
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

/// An instant that remembers the zone it was written in.
///
/// The zone is kept, rather than thrown away once the value is in UTC, because a
/// repeat rule ends on a local date and `UNTIL` has to be an instant; see
/// [`until_value`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ZonedInstant {
    /// Seconds since the Unix epoch.
    unix_seconds: i64,
    /// Seconds east of UTC, as the value spelled it.
    offset_seconds: i64,
}

/// Parses an ISO 8601 instant into seconds since the Unix epoch and its zone.
///
/// Accepts what this application actually produces and receives: a date and time
/// to the second, an optional fraction, and either `Z` or a `±HH:MM` / `±HHMM`
/// offset. A value with no offset at all is read as UTC — every writer in the
/// repository appends one, so this only decides what to do with a value no
/// writer here produces, and reading it as UTC keeps the task on the calendar
/// instead of dropping it.
///
/// As in [`parse_date`], every slice goes through [`str::get`] so that an
/// arbitrary `String` — `"2026-07-23T10:15:00日"` included — returns `None`
/// instead of panicking on a character boundary.
fn parse_instant(value: &str) -> Option<ZonedInstant> {
    let bytes = value.as_bytes();
    if bytes.len() < 19 || (bytes[10] != b'T' && bytes[10] != b't') {
        return None;
    }
    if bytes[13] != b':' || bytes[16] != b':' {
        return None;
    }

    let date = parse_date(value.get(0..10)?)?;
    let hour = parse_number(value.get(11..13)?)?;
    let minute = parse_number(value.get(14..16)?)?;
    let second = parse_number(value.get(17..19)?)?;
    // 60 is the leap second RFC 3339 allows; it is folded into the minute
    // rather than refused, because refusing would drop the task.
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }

    let mut rest = value.get(19..)?;
    if let Some(fraction) = rest.strip_prefix('.') {
        let digits = fraction
            .bytes()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digits == 0 {
            return None;
        }
        rest = fraction.get(digits..)?;
    }

    let offset_seconds = parse_offset(rest)?;
    let day_seconds =
        i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second.min(59));
    Some(ZonedInstant {
        unix_seconds: days_from_civil(date) * SECONDS_PER_DAY + day_seconds - offset_seconds,
        offset_seconds,
    })
}

/// Reads the zone designator that closes an instant, in seconds east of UTC.
fn parse_offset(value: &str) -> Option<i64> {
    if value.is_empty() || value == "Z" || value == "z" {
        return Some(0);
    }

    // `strip_prefix` on a `char`, not `split_at(1)`: the first character of an
    // arbitrary value may be multi-byte, and splitting inside it would panic.
    let (sign, digits) = if let Some(digits) = value.strip_prefix('+') {
        (1, digits)
    } else if let Some(digits) = value.strip_prefix('-') {
        (-1, digits)
    } else {
        return None;
    };

    let (hours, minutes) = match digits.len() {
        5 if digits.as_bytes()[2] == b':' => (digits.get(0..2)?, digits.get(3..5)?),
        4 => (digits.get(0..2)?, digits.get(2..4)?),
        _ => return None,
    };

    let hours = parse_number(hours)?;
    let minutes = parse_number(minutes)?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(sign * (i64::from(hours) * 3_600 + i64::from(minutes) * 60))
}

/// Reads a run of ASCII digits. Anything else — a sign, a space, a letter — is
/// not a number here, so the caller can treat the whole value as unusable.
fn parse_number(value: &str) -> Option<u32> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn format_date(date: CivilDate) -> String {
    format!("{:04}{:02}{:02}", date.year, date.month, date.day)
}

fn format_instant(unix_seconds: i64) -> String {
    let days = unix_seconds.div_euclid(SECONDS_PER_DAY);
    let day_seconds = unix_seconds.rem_euclid(SECONDS_PER_DAY);
    let date = civil_from_days(days);
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        date.year,
        date.month,
        date.day,
        day_seconds / 3_600,
        (day_seconds % 3_600) / 60,
        day_seconds % 60,
    )
}

fn add_days(date: CivilDate, days: i64) -> CivilDate {
    civil_from_days(days_from_civil(date) + days)
}

/// Days since 1970-01-01, by Howard Hinnant's civil-calendar algorithm. It is
/// used here rather than a date crate because the domain crate depends on the
/// contracts alone, and the two conversions below are the whole of what an ICS
/// file needs.
fn days_from_civil(date: CivilDate) -> i64 {
    let year = if date.month <= 2 {
        date.year - 1
    } else {
        date.year
    };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_position = i64::from((date.month + 9) % 12);
    let day_of_year = (153 * month_position + 2) / 5 + i64::from(date.day) - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Inverse of `days_from_civil`.
fn civil_from_days(days: i64) -> CivilDate {
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
    use todo_contracts::TodoStatus;

    use super::*;

    /// 2026-07-23T10:15:00Z, the moment every test exports at.
    const EXPORTED_AT: i64 = 1_784_801_700;

    fn todo(id: &str, title: &str) -> Todo {
        Todo {
            id: id.to_owned(),
            title: title.to_owned(),
            status: TodoStatus::Open,
            created_at: "2026-07-16T00:00:00Z".to_owned(),
            completed_at: None,
            archived_at: None,
            due_date: None,
            reminder_at: None,
            notes: None,
            start_date: None,
            starts_at: None,
            ends_at: None,
            estimated_minutes: None,
            recurrence: None,
            list_id: None,
            important: None,
            urgent: None,
            sort_order: None,
            tag_ids: Vec::new(),
            subtasks: Vec::new(),
            attachments: Vec::new(),
            depends_on: Vec::new(),
        }
    }

    fn weekly(weekdays: Vec<Weekday>) -> RecurrenceRule {
        RecurrenceRule {
            frequency: RecurrenceFrequency::Weekly,
            interval: 1,
            weekdays,
            month_day: None,
            on_last_day: false,
            calendar: RecurrenceCalendar::Gregorian,
            until: None,
            count: None,
        }
    }

    /// The unfolded content lines, which is what an assertion wants to look at.
    fn lines(content: &str) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        for raw in content.split(CRLF).filter(|line| !line.is_empty()) {
            match raw.strip_prefix(' ') {
                Some(continuation) => lines
                    .last_mut()
                    .expect("a continuation line always follows a line")
                    .push_str(continuation),
                None => lines.push(raw.to_owned()),
            }
        }
        lines
    }

    fn rrule_of(todo: Todo) -> Option<String> {
        let calendar = build_calendar(&[todo], EXPORTED_AT);
        lines(&calendar.content)
            .into_iter()
            .find(|line| line.starts_with("RRULE:"))
    }

    #[test]
    fn the_wrapper_carries_what_rfc_5545_requires() {
        let calendar = build_calendar(&[], EXPORTED_AT);
        let lines = lines(&calendar.content);

        assert_eq!(lines[0], "BEGIN:VCALENDAR");
        assert!(lines.contains(&"VERSION:2.0".to_owned()));
        assert!(lines.contains(&format!("PRODID:{PRODUCT_ID}")));
        assert!(lines.contains(&"CALSCALE:GREGORIAN".to_owned()));
        assert_eq!(lines.last().expect("a closing line"), "END:VCALENDAR");
        assert_eq!(calendar.event_count, 0);
    }

    #[test]
    fn every_line_ends_with_crlf() {
        let mut dated = todo("todo-1", "交周报");
        dated.due_date = Some("2026-07-23".to_owned());

        let calendar = build_calendar(&[dated], EXPORTED_AT);

        assert!(calendar.content.ends_with(CRLF));
        // A bare LF anywhere would break the line-break rule of RFC 5545 3.1.
        assert_eq!(
            calendar.content.matches('\n').count(),
            calendar.content.matches(CRLF).count()
        );
    }

    #[test]
    fn a_due_date_becomes_a_whole_day_event() {
        let mut dated = todo("todo-1", "交周报");
        dated.due_date = Some("2026-07-23".to_owned());

        let calendar = build_calendar(&[dated], EXPORTED_AT);
        let lines = lines(&calendar.content);

        assert_eq!(calendar.event_count, 1);
        assert!(lines.contains(&"UID:todo-1@just-do.local".to_owned()));
        assert!(lines.contains(&"DTSTAMP:20260723T101500Z".to_owned()));
        assert!(lines.contains(&"SUMMARY:交周报".to_owned()));
        assert!(lines.contains(&"DTSTART;VALUE=DATE:20260723".to_owned()));
        // Non-inclusive end: a one-day event ends on the following day.
        assert!(lines.contains(&"DTEND;VALUE=DATE:20260724".to_owned()));
    }

    #[test]
    fn a_timed_block_is_written_in_utc() {
        let mut block = todo("todo-2", "评审会");
        block.due_date = Some("2026-07-23".to_owned());
        block.starts_at = Some("2026-07-23T09:00:00.000Z".to_owned());
        block.ends_at = Some("2026-07-23T10:30:00Z".to_owned());

        let calendar = build_calendar(&[block], EXPORTED_AT);
        let lines = lines(&calendar.content);

        // The explicit block wins over the due date.
        assert!(lines.contains(&"DTSTART:20260723T090000Z".to_owned()));
        assert!(lines.contains(&"DTEND:20260723T103000Z".to_owned()));
    }

    #[test]
    fn an_offset_is_carried_back_to_utc() {
        let mut block = todo("todo-3", "站会");
        block.starts_at = Some("2026-07-23T09:00:00+08:00".to_owned());

        let calendar = build_calendar(&[block], EXPORTED_AT);

        assert!(lines(&calendar.content).contains(&"DTSTART:20260723T010000Z".to_owned()));
    }

    #[test]
    fn an_end_that_is_not_after_the_start_is_dropped() {
        let mut block = todo("todo-4", "错配的时间段");
        block.starts_at = Some("2026-07-23T09:00:00Z".to_owned());
        block.ends_at = Some("2026-07-23T09:00:00Z".to_owned());

        let calendar = build_calendar(&[block], EXPORTED_AT);

        assert!(!calendar.content.contains("DTEND"));
    }

    #[test]
    fn a_reminder_only_task_still_lands_on_the_calendar() {
        let mut reminded = todo("todo-5", "吃药");
        reminded.reminder_at = Some("2026-07-24T12:00:00Z".to_owned());

        let calendar = build_calendar(&[reminded], EXPORTED_AT);
        let lines = lines(&calendar.content);

        assert_eq!(calendar.event_count, 1);
        assert!(lines.contains(&"DTSTART:20260724T120000Z".to_owned()));
        // Nothing said how long it takes, so nothing is claimed.
        assert!(!calendar.content.contains("DTEND"));
    }

    #[test]
    fn a_task_without_any_date_is_left_out() {
        let undated = todo("todo-6", "总有一天");
        let mut unreadable = todo("todo-7", "坏日期");
        unreadable.due_date = Some("2026-02-31".to_owned());
        let mut dated = todo("todo-8", "有日期");
        dated.due_date = Some("2026-07-23".to_owned());

        let calendar = build_calendar(&[undated, unreadable, dated], EXPORTED_AT);

        assert_eq!(calendar.event_count, 1);
        assert!(calendar.content.contains("UID:todo-8@just-do.local"));
        assert!(!calendar.content.contains("todo-6"));
        assert!(!calendar.content.contains("todo-7"));
    }

    #[test]
    fn text_values_are_escaped() {
        let mut awkward = todo("todo-9", "买牛奶, 面包; 还有\\糖");
        awkward.due_date = Some("2026-07-23".to_owned());
        awkward.notes = Some("第一行\r\n第二行".to_owned());

        let calendar = build_calendar(&[awkward], EXPORTED_AT);
        let lines = lines(&calendar.content);

        assert!(lines.contains(&"SUMMARY:买牛奶\\, 面包\\; 还有\\\\糖".to_owned()));
        assert!(lines.contains(&"DESCRIPTION:第一行\\n第二行".to_owned()));
    }

    #[test]
    fn long_lines_fold_at_75_octets_without_splitting_a_character() {
        let mut long = todo("todo-10", &"日程".repeat(60));
        long.due_date = Some("2026-07-23".to_owned());

        let calendar = build_calendar(&[long], EXPORTED_AT);

        for line in calendar.content.split(CRLF) {
            assert!(line.len() <= MAX_LINE_OCTETS, "{} octets: {line}", line.len());
        }
        // Unfolding gives the title back intact, which is only true if no
        // multi-byte character was cut in half.
        assert!(lines(&calendar.content).contains(&format!("SUMMARY:{}", "日程".repeat(60))));
    }

    #[test]
    fn a_weekly_rule_names_its_days() {
        let mut repeating = todo("todo-11", "健身");
        repeating.due_date = Some("2026-07-23".to_owned());
        repeating.recurrence = Some(weekly(vec![
            Weekday::Monday,
            Weekday::Wednesday,
            Weekday::Friday,
        ]));

        assert_eq!(
            rrule_of(repeating),
            Some("RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR".to_owned())
        );
    }

    #[test]
    fn a_weekly_rule_without_days_leaves_byday_to_the_start() {
        let mut repeating = todo("todo-12", "周会");
        repeating.due_date = Some("2026-07-23".to_owned());
        let mut rule = weekly(Vec::new());
        rule.interval = 2;
        repeating.recurrence = Some(rule);

        assert_eq!(
            rrule_of(repeating),
            Some("RRULE:FREQ=WEEKLY;INTERVAL=2".to_owned())
        );
    }

    #[test]
    fn a_monthly_rule_says_which_day() {
        let mut on_the_tenth = todo("todo-13", "还款");
        on_the_tenth.due_date = Some("2026-08-10".to_owned());
        let mut rule = weekly(Vec::new());
        rule.frequency = RecurrenceFrequency::Monthly;
        rule.month_day = Some(10);
        on_the_tenth.recurrence = Some(rule);

        assert_eq!(
            rrule_of(on_the_tenth),
            Some("RRULE:FREQ=MONTHLY;BYMONTHDAY=10".to_owned())
        );
    }

    #[test]
    fn the_last_day_of_a_month_counts_back_from_the_end() {
        let mut month_end = todo("todo-14", "月结");
        month_end.due_date = Some("2026-07-31".to_owned());
        let mut rule = weekly(Vec::new());
        rule.frequency = RecurrenceFrequency::Monthly;
        rule.on_last_day = true;
        month_end.recurrence = Some(rule);

        assert_eq!(
            rrule_of(month_end),
            Some("RRULE:FREQ=MONTHLY;BYMONTHDAY=-1".to_owned())
        );
    }

    #[test]
    fn a_daily_rule_counts_its_interval_and_occurrences() {
        let mut every_third_day = todo("todo-15", "浇花");
        every_third_day.due_date = Some("2026-07-23".to_owned());
        let mut rule = weekly(Vec::new());
        rule.frequency = RecurrenceFrequency::Daily;
        rule.interval = 3;
        rule.count = Some(10);
        every_third_day.recurrence = Some(rule);

        assert_eq!(
            rrule_of(every_third_day),
            Some("RRULE:FREQ=DAILY;INTERVAL=3;COUNT=10".to_owned())
        );
    }

    #[test]
    fn a_yearly_rule_is_the_plain_one() {
        let mut birthday = todo("todo-16", "生日");
        birthday.due_date = Some("2026-09-01".to_owned());
        let mut rule = weekly(Vec::new());
        rule.frequency = RecurrenceFrequency::Yearly;
        birthday.recurrence = Some(rule);

        assert_eq!(rrule_of(birthday), Some("RRULE:FREQ=YEARLY".to_owned()));
    }

    #[test]
    fn an_end_date_wins_over_a_count_and_matches_the_start_value_type() {
        let mut all_day = todo("todo-17", "打卡");
        all_day.due_date = Some("2026-07-23".to_owned());
        let mut rule = weekly(Vec::new());
        rule.frequency = RecurrenceFrequency::Daily;
        rule.until = Some("2026-12-31".to_owned());
        rule.count = Some(5);
        all_day.recurrence = Some(rule.clone());

        assert_eq!(
            rrule_of(all_day),
            Some("RRULE:FREQ=DAILY;UNTIL=20261231".to_owned())
        );

        let mut timed = todo("todo-18", "打卡");
        timed.starts_at = Some("2026-07-23T09:00:00Z".to_owned());
        timed.recurrence = Some(rule);

        assert_eq!(
            rrule_of(timed),
            Some("RRULE:FREQ=DAILY;UNTIL=20261231T235959Z".to_owned())
        );
    }

    #[test]
    fn every_working_day_is_the_closest_standard_rule() {
        let mut workday = todo("todo-19", "日报");
        workday.due_date = Some("2026-07-23".to_owned());
        let mut rule = weekly(Vec::new());
        rule.frequency = RecurrenceFrequency::Workday;
        workday.recurrence = Some(rule);

        assert_eq!(
            rrule_of(workday),
            Some("RRULE:FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR".to_owned())
        );
    }

    #[test]
    fn rules_rfc_5545_cannot_say_export_as_a_single_event() {
        let mut lunar = todo("todo-20", "农历生日");
        lunar.due_date = Some("2026-09-01".to_owned());
        let mut lunar_rule = weekly(Vec::new());
        lunar_rule.frequency = RecurrenceFrequency::Yearly;
        lunar_rule.calendar = RecurrenceCalendar::Lunar;
        lunar.recurrence = Some(lunar_rule);

        let mut every_other_workday = todo("todo-21", "隔个工作日");
        every_other_workday.due_date = Some("2026-07-23".to_owned());
        let mut workday_rule = weekly(Vec::new());
        workday_rule.frequency = RecurrenceFrequency::Workday;
        workday_rule.interval = 2;
        every_other_workday.recurrence = Some(workday_rule);

        assert_eq!(rrule_of(lunar.clone()), None);
        assert_eq!(rrule_of(every_other_workday.clone()), None);
        // The events themselves still export.
        let calendar = build_calendar(&[lunar, every_other_workday], EXPORTED_AT);
        assert_eq!(calendar.event_count, 2);
    }

    #[test]
    fn the_file_name_is_the_moment_it_was_written() {
        assert_eq!(export_file_name(EXPORTED_AT, 0), "todo-20260723T101500Z.ics");
    }

    #[test]
    fn a_taken_file_name_gets_a_counter_rather_than_overwriting() {
        // Two exports inside the same second want the same name; the caller
        // comes back with the next attempt, and every one of them is distinct.
        let names: Vec<String> = (0..4).map(|attempt| export_file_name(EXPORTED_AT, attempt)).collect();

        assert_eq!(
            names,
            vec![
                "todo-20260723T101500Z.ics".to_owned(),
                "todo-20260723T101500Z-2.ics".to_owned(),
                "todo-20260723T101500Z-3.ics".to_owned(),
                "todo-20260723T101500Z-4.ics".to_owned(),
            ]
        );
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
            let date = CivilDate { year, month, day };
            assert_eq!(civil_from_days(days_from_civil(date)), date);
        }

        assert_eq!(days_from_civil(CivilDate { year: 1970, month: 1, day: 1 }), 0);
        assert_eq!(
            add_days(CivilDate { year: 2026, month: 12, day: 31 }, 1),
            CivilDate { year: 2027, month: 1, day: 1 }
        );
        // A day that does not exist must not round trip, or the parser would
        // accept 31 February.
        assert_ne!(
            civil_from_days(days_from_civil(CivilDate { year: 2026, month: 2, day: 31 })),
            CivilDate { year: 2026, month: 2, day: 31 }
        );
    }

    #[test]
    fn instants_are_read_in_every_shape_this_repository_writes() {
        let at = |value: &str| parse_instant(value).map(|instant| instant.unix_seconds);

        assert_eq!(at("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(at("1970-01-01T00:00:00.123Z"), Some(0));
        assert_eq!(at("1970-01-01T08:00:00+08:00"), Some(0));
        assert_eq!(at("1970-01-01T08:00:00+0800"), Some(0));
        assert_eq!(at("1969-12-31T23:00:00-01:00"), Some(0));
        assert_eq!(at("1970-01-01T00:00:00"), Some(0));
        assert_eq!(at("2026-07-23"), None);
        assert_eq!(at("not a time"), None);
        assert_eq!(at("1970-01-01T25:00:00Z"), None);

        // The zone travels with the instant, because UNTIL needs it.
        assert_eq!(
            parse_instant("1970-01-01T08:00:00+08:00"),
            Some(ZonedInstant { unix_seconds: 0, offset_seconds: 8 * 3_600 })
        );
        assert_eq!(
            parse_instant("1969-12-31T16:00:00-08:00"),
            Some(ZonedInstant { unix_seconds: 0, offset_seconds: -8 * 3_600 })
        );
        assert_eq!(
            parse_instant("1970-01-01T00:00:00Z"),
            Some(ZonedInstant { unix_seconds: 0, offset_seconds: 0 })
        );
    }

    /// The four slices that used to be taken by byte index. Every one of these
    /// values passes the ASCII guards in front of it and then lands on a
    /// character boundary the guard says nothing about, which used to abort the
    /// whole export inside a Tauri command.
    #[test]
    fn a_value_that_is_not_ascii_is_refused_rather_than_panicking() {
        // `&value[8..10]` — the day.
        assert_eq!(parse_date("2026-07-日期"), None);
        // `&value[5..7]` — the month, reached through a multi-byte year.
        assert_eq!(parse_date("二〇二六-07-23"), None);
        assert_eq!(parse_date("2026-07-2日"), None);
        // `&value[17..19]` — the seconds.
        assert_eq!(parse_instant("2026-07-23T10:15:0日"), None);
        // `&value[19..]` — what follows the seconds.
        assert_eq!(parse_instant("2026-07-23T10:15:00日"), None);
        // `split_at(1)` inside the zone designator, both lengths it accepts.
        assert_eq!(parse_instant("2026-07-23T10:15:00日本語"), None);
        assert_eq!(parse_instant("2026-07-23T10:15:00+0日"), None);
        assert_eq!(parse_instant("2026-07-23T10:15:00+00:日"), None);
        assert_eq!(parse_instant("2026-07-23T10:15:00.5日"), None);
        assert_eq!(parse_offset("日本"), None);
        assert_eq!(parse_offset("+0日"), None);
    }

    /// A blunt sweep over the shapes a stored string can take. `Todo::validate`
    /// checks none of these fields, so anything at all can arrive here through
    /// sync or an older database; the only requirement is that the export keeps
    /// running.
    #[test]
    fn no_string_at_all_can_stop_an_export() {
        let seeds = [
            "",
            "Z",
            "z",
            "日",
            "日本語のとても長い文字列です",
            "2026-07-23",
            "2026-07-23T10:15:00Z",
            "2026-07-23T10:15:00+08:00",
            "🎉🎉🎉🎉🎉🎉🎉🎉🎉🎉",
            "2026-07-23T10:15:00🎉",
            "2026-07-23T10:15:00.🎉",
            "2026-07-23T10:15:00+🎉",
            "20\u{0301}26-07-23T10:15:00Z",
            "\u{feff}2026-07-23T10:15:00Z",
            "----------",
            "-------------------",
        ];
        // Truncation is where a multi-byte character is most likely to be cut,
        // so every prefix of every seed — by byte, not by character — is tried.
        let mut values: Vec<String> = Vec::new();
        for seed in seeds {
            values.push(seed.to_owned());
            for end in 0..=seed.len() {
                if let Some(prefix) = seed.get(..end) {
                    values.push(prefix.to_owned());
                }
                // A byte-level truncation that lands inside a character cannot
                // be a `&str`, but it can be a `String` after a lossy decode,
                // which is exactly how such a value reaches the database.
                values.push(String::from_utf8_lossy(&seed.as_bytes()[..end]).into_owned());
            }
        }

        for value in &values {
            // Every field the export reads, one at a time, so a `None` on one
            // of them cannot hide a panic on another.
            let mut probe = todo("todo-fuzz", "边界");
            probe.due_date = Some(value.clone());
            probe.reminder_at = Some(value.clone());
            probe.starts_at = Some(value.clone());
            probe.ends_at = Some(value.clone());
            let mut rule = weekly(Vec::new());
            rule.until = Some(value.clone());
            probe.recurrence = Some(rule);

            let mut dated = todo("todo-fuzz-2", "边界");
            dated.due_date = Some("2026-07-23".to_owned());
            let mut dated_rule = weekly(Vec::new());
            dated_rule.until = Some(value.clone());
            dated.recurrence = Some(dated_rule);

            let calendar = build_calendar(&[probe, dated], EXPORTED_AT);
            assert!(calendar.content.ends_with(CRLF), "{value:?}");
        }
    }

    #[test]
    fn an_end_date_on_a_timed_event_covers_the_whole_local_day() {
        // UTC−8, 20:00 local: the occurrences happen at 04:00Z the next day, so
        // an UNTIL of 20261231T235959Z would drop the last one.
        let mut evening = todo("todo-22", "夜跑");
        evening.starts_at = Some("2026-07-23T20:00:00-08:00".to_owned());
        let mut rule = weekly(Vec::new());
        rule.frequency = RecurrenceFrequency::Daily;
        rule.until = Some("2026-12-31".to_owned());
        evening.recurrence = Some(rule.clone());

        // 2026-12-31T23:59:59−08:00 == 2027-01-01T07:59:59Z, which is after the
        // last occurrence (2027-01-01T04:00:00Z) and before the next one.
        assert_eq!(
            rrule_of(evening),
            Some("RRULE:FREQ=DAILY;UNTIL=20270101T075959Z".to_owned())
        );

        // East of UTC the day ends earlier, and the value follows.
        let mut morning = todo("todo-23", "晨读");
        morning.starts_at = Some("2026-07-23T08:00:00+08:00".to_owned());
        morning.recurrence = Some(rule);

        assert_eq!(
            rrule_of(morning),
            Some("RRULE:FREQ=DAILY;UNTIL=20261231T155959Z".to_owned())
        );
    }

    #[test]
    fn rules_rfc_5545_cannot_say_are_counted_for_the_caller_to_report() {
        let mut lunar = todo("todo-24", "农历生日");
        lunar.due_date = Some("2026-09-01".to_owned());
        let mut lunar_rule = weekly(Vec::new());
        lunar_rule.frequency = RecurrenceFrequency::Yearly;
        lunar_rule.calendar = RecurrenceCalendar::Lunar;
        lunar.recurrence = Some(lunar_rule);

        let mut every_other_workday = todo("todo-25", "隔个工作日");
        every_other_workday.due_date = Some("2026-07-23".to_owned());
        let mut workday_rule = weekly(Vec::new());
        workday_rule.frequency = RecurrenceFrequency::Workday;
        workday_rule.interval = 2;
        every_other_workday.recurrence = Some(workday_rule);

        let mut weekly_task = todo("todo-26", "周会");
        weekly_task.due_date = Some("2026-07-23".to_owned());
        weekly_task.recurrence = Some(weekly(vec![Weekday::Monday]));

        let mut plain = todo("todo-27", "交周报");
        plain.due_date = Some("2026-07-23".to_owned());

        let calendar = build_calendar(
            &[lunar, every_other_workday, weekly_task, plain],
            EXPORTED_AT,
        );

        assert_eq!(calendar.event_count, 4);
        // Only the two that lost their repeat rule count; a rule that mapped and
        // a task with no rule at all do not.
        assert_eq!(calendar.unrepeatable_recurrences, 2);
    }

    #[test]
    fn a_task_with_no_date_cannot_be_counted_as_a_lost_recurrence() {
        // It never became an event, so there is nothing for the user to be told
        // about — counting it would overstate the warning.
        let mut undated_lunar = todo("todo-28", "农历生日");
        let mut lunar_rule = weekly(Vec::new());
        lunar_rule.calendar = RecurrenceCalendar::Lunar;
        undated_lunar.recurrence = Some(lunar_rule);

        let calendar = build_calendar(&[undated_lunar], EXPORTED_AT);

        assert_eq!(calendar.event_count, 0);
        assert_eq!(calendar.unrepeatable_recurrences, 0);
    }
}
