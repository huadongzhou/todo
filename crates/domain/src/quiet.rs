//! The do-not-disturb window and how it reshapes a poll's deliveries — the pure
//! half of 勿扰时段 (提醒通知/05).
//!
//! Pure rules only, per the layering in AGENTS.md: no Tauri, no clock, no
//! database. The scheduler reads the user's quiet window from settings, works out
//! which reminders and renags have come due (`crate::reminder`, `crate::renag`),
//! and hands their instants here with the current instant; it gets back a plan
//! that says which to raise one toast each, which to fold into a single
//! "the quiet hours are over" summary, and which to hold for a later poll.
//!
//! Nothing here is a queue. A reminder held back during the window is simply one
//! the scheduler did not record as delivered — so a later poll finds it due and
//! undelivered all over again, which is exactly the "暂存队列" the delivery table
//! already gives for free (提醒通知/01). The one thing this module decides is,
//! for the poll happening now:
//!
//! * inside the window → nothing goes out, nothing is recorded (the batch waits);
//! * outside it → the reminders whose own moment fell inside a quiet window are
//!   the held-back batch and are folded into one summary when there are two or
//!   more of them, while the rest go out one at a time exactly as they would with
//!   no quiet hours set;
//! * no valid window at all → everything goes out one at a time (fail-open), so a
//!   window left blank or set start-equals-end never silently swallows a reminder,
//!   which the母任务 "a reminder is never lost" requires.

/// Seconds in a day, for turning a Unix instant into a local minute-of-day.
const SECONDS_PER_DAY: i64 = 86_400;

/// A do-not-disturb window, as two minutes of the local day.
///
/// Held as minutes rather than the `"HH:mm"` strings the settings store keeps so
/// the comparison is plain integer arithmetic. The start is inclusive and the end
/// exclusive (see [`QuietWindow::contains`]); a window that crosses midnight
/// (`start > end`, the common 22:00–08:00 case) is the union of the evening and
/// the small hours.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuietWindow {
    start_min: i64,
    end_min: i64,
}

impl QuietWindow {
    /// Builds a window from the two `"HH:mm"` strings the settings store holds,
    /// or `None` when either will not parse or the two are equal.
    ///
    /// An equal start and end is an empty window — it names no span of time — so
    /// it silences nothing: returning `None` here is what makes the scheduler
    /// fail open on a misconfigured window rather than treat "22:00 to 22:00" as
    /// the whole day. A value the `type="time"` input never writes (an out-of-range
    /// hour, a missing colon) is `None` for the same reason.
    pub fn parse(start: &str, end: &str) -> Option<QuietWindow> {
        let start_min = parse_hm(start)?;
        let end_min = parse_hm(end)?;
        if start_min == end_min {
            return None;
        }
        Some(QuietWindow { start_min, end_min })
    }

    /// Whether a local minute-of-day falls inside the window.
    ///
    /// Start inclusive, end exclusive: at exactly the start minute the device is
    /// silent, and at exactly the end minute it is not — so a 22:00–08:00 window
    /// is silent from 22:00 on the dot and the first poll at 08:00 is already the
    /// morning, the one that flushes the night's batch. When the window crosses
    /// midnight (`start > end`) the inside is everything from the start to
    /// midnight plus everything from midnight to the end.
    pub fn contains(&self, minute_of_day: i64) -> bool {
        if self.start_min < self.end_min {
            minute_of_day >= self.start_min && minute_of_day < self.end_min
        } else {
            minute_of_day >= self.start_min || minute_of_day < self.end_min
        }
    }
}

/// The local minute-of-day a Unix instant falls on, for a device `zone_offset_seconds`
/// east of UTC.
///
/// The window is a wall-clock span, so both the current instant and each
/// reminder's instant are read against it in local minutes. The offset is the
/// same seam 提醒通知/04's renag anchor and 提醒通知/07 use: passing 0 reads the
/// minute in UTC — late, never early, for the primary east-of-UTC market — and
/// 07 supplies the device's real offset with no change to the rule.
pub fn local_minute_of_day(unix: i64, zone_offset_seconds: i64) -> i64 {
    (unix + zone_offset_seconds).rem_euclid(SECONDS_PER_DAY) / 60
}

/// What a poll should do with the reminders that have come due, once the quiet
/// window is taken into account.
///
/// The two lists carry indices into the caller's own `due_instants` slice, in
/// ascending order, so the caller can map each back to the reminder or renag it
/// came from. An index in neither list is held: the caller raises nothing and
/// records nothing for it, leaving it for a later poll.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct QuietPlan {
    /// Items to raise one notification each, with their own text — the ordinary
    /// path, and the whole of it when quiet hours are off.
    pub deliver: Vec<usize>,
    /// Items to fold into a single summary notification. Empty, or two or more:
    /// a lone held reminder is delivered with its own text (it moves to
    /// `deliver`) rather than summarised, because a one-line summary of one thing
    /// says less than the thing itself. When non-empty the caller shows one
    /// summary toast and then records every one of these, so the batch is not
    /// raised again next poll.
    pub summarize: Vec<usize>,
}

/// Splits the due items into deliver-now, fold-into-summary, and hold, applying
/// the quiet window.
///
/// `due_instants` are the Unix instants of the reminders and renags that have
/// come due and not yet been delivered, in the caller's order. `window` is the
/// configured window, or `None` when quiet hours are off or the window will not
/// parse — in which case nothing is ever held (fail-open). `now_unix` is the
/// instant of this poll.
///
/// Inside the window right now, every due item is held: the plan is empty, so the
/// poll raises and records nothing and the batch waits. Outside it, an item whose
/// own moment fell inside a quiet window is part of the held-back batch — folded
/// into the summary when the batch runs to two or more — while an item whose
/// moment was never in a window (an ordinary daytime reminder) goes out on its
/// own, so turning quiet hours on does not start batching reminders that have
/// nothing to do with the night.
pub fn plan_quiet_delivery(
    due_instants: &[i64],
    window: Option<QuietWindow>,
    now_unix: i64,
    zone_offset_seconds: i64,
) -> QuietPlan {
    let Some(window) = window else {
        // Fail-open: no window, so every due item is delivered on its own, exactly
        // as it would be with no quiet hours configured at all.
        return QuietPlan {
            deliver: (0..due_instants.len()).collect(),
            summarize: Vec::new(),
        };
    };

    if window.contains(local_minute_of_day(now_unix, zone_offset_seconds)) {
        // Inside the window: hold everything. Both reminders and renags are gated
        // by this one check, and a held item is left unrecorded so a later poll
        // finds it again.
        return QuietPlan::default();
    }

    // Outside the window. An item whose moment fell inside a quiet window is
    // held-back; the rest are fresh and go out one at a time.
    let mut held = Vec::new();
    let mut fresh = Vec::new();
    for (index, &instant) in due_instants.iter().enumerate() {
        if window.contains(local_minute_of_day(instant, zone_offset_seconds)) {
            held.push(index);
        } else {
            fresh.push(index);
        }
    }

    if held.len() >= 2 {
        QuietPlan {
            deliver: fresh,
            summarize: held,
        }
    } else {
        // Zero or one held: a single flushed reminder reads better as itself than
        // as a summary of one, so it joins the fresh deliveries.
        fresh.extend(held);
        fresh.sort_unstable();
        QuietPlan {
            deliver: fresh,
            summarize: Vec::new(),
        }
    }
}

/// Parses `"HH:mm"` into a minute of the day (0–1439), or `None` for anything the
/// `type="time"` input would not produce.
///
/// Strict: exactly `HH:mm`, two digits each side of the colon, hours 0–23 and
/// minutes 0–59. A settings file edited by hand into `"9:00"` or `"24:00"` reads
/// as `None`, which the caller turns into a fail-open (no window) rather than a
/// wrong window.
fn parse_hm(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' {
        return None;
    }
    let hours = parse_two_digits(value.get(0..2)?)?;
    let minutes = parse_two_digits(value.get(3..5)?)?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(hours * 60 + minutes)
}

/// Reads a two-character run of ASCII digits. Anything else — a sign, a space, a
/// letter, a multi-byte character — is not a time here, so the caller treats the
/// whole value as unusable.
fn parse_two_digits(value: &str) -> Option<i64> {
    if value.len() != 2 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-07-24T00:00:00Z, a UTC local midnight, so a minute-of-day maps to that
    // many minutes past this instant.
    const MIDNIGHT: i64 = 1_784_851_200;

    /// A Unix instant at `hour:minute` UTC on the midnight day above.
    fn at(hour: i64, minute: i64) -> i64 {
        MIDNIGHT + (hour * 60 + minute) * 60
    }

    fn night() -> QuietWindow {
        QuietWindow::parse("22:00", "08:00").expect("a valid cross-midnight window")
    }

    #[test]
    fn a_window_parses_from_two_clock_strings() {
        let window = night();
        assert_eq!(window.start_min, 22 * 60);
        assert_eq!(window.end_min, 8 * 60);
    }

    #[test]
    fn an_empty_or_unreadable_window_is_none_so_the_caller_fails_open() {
        // Start equal to end names no span of time.
        assert_eq!(QuietWindow::parse("08:00", "08:00"), None);
        // Shapes the time input never writes.
        assert_eq!(QuietWindow::parse("9:00", "08:00"), None);
        assert_eq!(QuietWindow::parse("24:00", "08:00"), None);
        assert_eq!(QuietWindow::parse("22:60", "08:00"), None);
        assert_eq!(QuietWindow::parse("", "08:00"), None);
        assert_eq!(QuietWindow::parse("22-00", "08:00"), None);
        // A multi-byte value must return None, not panic on a byte slice.
        assert_eq!(QuietWindow::parse("22:0点", "08:00"), None);
    }

    #[test]
    fn a_cross_midnight_window_is_inclusive_of_its_start_and_exclusive_of_its_end() {
        let window = night();
        // Exactly 22:00 is already silent; a minute before is not.
        assert!(window.contains(22 * 60));
        assert!(!window.contains(22 * 60 - 1));
        // Across midnight the small hours are silent.
        assert!(window.contains(0));
        assert!(window.contains(2 * 60));
        assert!(window.contains(8 * 60 - 1));
        // Exactly 08:00 is the morning — the poll that ends the silence.
        assert!(!window.contains(8 * 60));
        assert!(!window.contains(12 * 60));
    }

    #[test]
    fn a_same_day_window_silences_only_the_span_between_its_ends() {
        let window = QuietWindow::parse("13:00", "14:00").expect("a valid same-day window");
        assert!(!window.contains(12 * 60 + 59));
        assert!(window.contains(13 * 60));
        assert!(window.contains(13 * 60 + 30));
        assert!(!window.contains(14 * 60));
        // Midnight is outside a same-day window, unlike a cross-midnight one.
        assert!(!window.contains(0));
    }

    #[test]
    fn a_minute_of_day_reads_through_the_zone_offset() {
        // Midnight UTC is minute 0 for a UTC device.
        assert_eq!(local_minute_of_day(MIDNIGHT, 0), 0);
        assert_eq!(local_minute_of_day(at(22, 30), 0), 22 * 60 + 30);
        // Eight hours east, midnight UTC is 08:00 local.
        assert_eq!(local_minute_of_day(MIDNIGHT, 8 * 3_600), 8 * 60);
        // A minute west of UTC, midnight UTC is a minute before local midnight —
        // the last minute of the previous day (23:59), not a negative one.
        assert_eq!(local_minute_of_day(MIDNIGHT, -60), 23 * 60 + 59);
    }

    #[test]
    fn no_window_delivers_everything_on_its_own() {
        // Fail-open: quiet hours off (or a window that would not parse) leaves the
        // ordinary per-reminder path untouched.
        let due = [at(9, 0), at(15, 0), at(23, 0)];
        let plan = plan_quiet_delivery(&due, None, at(23, 30), 0);
        assert_eq!(plan.deliver, vec![0, 1, 2]);
        assert!(plan.summarize.is_empty());
    }

    #[test]
    fn inside_the_window_nothing_goes_out() {
        // Two reminders came due in the small hours; the poll is at 02:00, inside
        // the window, so both are held — raised and recorded nowhere.
        let due = [at(1, 0), at(1, 30)];
        let plan = plan_quiet_delivery(&due, Some(night()), at(2, 0), 0);
        assert_eq!(plan, QuietPlan::default());
    }

    #[test]
    fn the_first_poll_after_the_window_folds_the_night_into_one_summary() {
        // 窗口结束补推合并: three reminders came due across the night and were held;
        // the first poll at 08:00 flushes the whole batch as a single summary and
        // records every one of them.
        let due = [at(22, 30), at(1, 0), at(7, 0)];
        let plan = plan_quiet_delivery(&due, Some(night()), at(8, 0), 0);
        assert!(plan.deliver.is_empty());
        assert_eq!(plan.summarize, vec![0, 1, 2]);
    }

    #[test]
    fn a_single_held_reminder_is_delivered_as_itself_not_a_summary_of_one() {
        // Exactly one reminder was held overnight: at 08:00 it goes out with its
        // own text, no summary.
        let due = [at(3, 0)];
        let plan = plan_quiet_delivery(&due, Some(night()), at(8, 0), 0);
        assert_eq!(plan.deliver, vec![0]);
        assert!(plan.summarize.is_empty());
    }

    #[test]
    fn a_daytime_reminder_outside_the_window_is_never_batched() {
        // Quiet hours are on, but two reminders come due at 15:00 — outside the
        // window. Their moments were never silent, so they go out one at a time
        // and there is no "quiet hours are over" summary in the middle of the day.
        let due = [at(15, 0), at(15, 0)];
        let plan = plan_quiet_delivery(&due, Some(night()), at(15, 1), 0);
        assert_eq!(plan.deliver, vec![0, 1]);
        assert!(plan.summarize.is_empty());
    }

    #[test]
    fn the_flush_poll_keeps_held_and_fresh_apart() {
        // At the flush poll a reminder that only just came due (09:00, outside the
        // window) rides alongside the held night batch: the fresh one goes out on
        // its own, the two held ones become the summary.
        let due = [at(23, 0), at(9, 0), at(2, 0)];
        let plan = plan_quiet_delivery(&due, Some(night()), at(9, 1), 0);
        assert_eq!(plan.deliver, vec![1]);
        assert_eq!(plan.summarize, vec![0, 2]);
    }

    #[test]
    fn an_empty_window_holds_nothing() {
        // A start-equals-end window parses to None, so even the small hours deliver
        // — the母任务 "never lost" guard against a misconfigured window.
        let due = [at(1, 0), at(2, 0)];
        let plan = plan_quiet_delivery(&due, QuietWindow::parse("08:00", "08:00"), at(2, 0), 0);
        assert_eq!(plan.deliver, vec![0, 1]);
        assert!(plan.summarize.is_empty());
    }
}
