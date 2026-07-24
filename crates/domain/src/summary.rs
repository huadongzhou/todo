//! Deciding whether the daily morning summary should go out now, and what it
//! should say — the pure half of 晨间摘要 (提醒通知/06).
//!
//! Pure rules only, per the layering in AGENTS.md: no Tauri, no clock, no
//! database. The scheduler reads the user's summary preference and the stored
//! todos, hands them here with the current instant, and gets back either nothing
//! or one summary to raise — its text already built from a live count of what is
//! due today and what is overdue.
//!
//! The summary is a *timed aggregate*, unlike the reminders and renags beside it:
//! it does not fire off a task's own moment but once a day, at a time the user
//! sets, carrying a count rather than any one task. Three things follow, and they
//! are the whole of what this module decides:
//!
//! * **once a day.** The scheduler polls every 30s, so the summary must fire the
//!   first poll at or after its time and no more that day. Like a renag it rides
//!   01's `reminder_deliveries` with a synthetic key — `summary:<YYYY-MM-DD>` on
//!   the local day — so a poll that has already sent today reads its own record
//!   and stays quiet, and a restart reads back what already went out. The key is
//!   the local date, so it resets across days on its own and a machine that was
//!   off for a week catches up with today's summary only, never a backlog.
//! * **only what is worth saying.** With nothing due today and nothing overdue
//!   (and no 我的一天 items) the summary is silence, not an empty "all clear" —
//!   a digest that fires every morning with nothing in it is noise (DESIGN 原则
//!   1). The counts drive the body: a zero segment is dropped, so a morning with
//!   overdue work but nothing due today reads "逾期 2 条。", not "今日到期 0 条".
//! * **quiet-agnostic here.** This does not know about the do-not-disturb window;
//!   it answers "is the summary due, and what does it say" from the counts as
//!   they stand *now*. The scheduler rides the same quiet gate the reminders do
//!   (提醒通知/05): a summary whose time falls inside the window is held — not
//!   recorded — so a later poll after the window ends re-derives it here from
//!   fresh counts and it goes out then, reflecting the flush moment rather than
//!   the held one. Holding is the scheduler's job; recomputing on every poll is
//!   what makes the flush counts current for free.
//!
//! The 我的一天 (today's list) segment is the summary's third count, but its
//! source — 视图与统计/06 — has not landed, so [`my_day_count`] returns `None`
//! and the segment is left out rather than faked. When that lands it fills the
//! seam with the day's picked-for-today count and the segment joins the body with
//! no other change here.

use std::collections::HashSet;

use todo_contracts::{Todo, TodoStatus};

use crate::civil::{civil_from_days, days_from_civil, format_iso_date, parse_date};
use crate::quiet::parse_hm;

const SECONDS_PER_DAY: i64 = 86_400;

/// The delivery-key namespace for the daily summary, so its synthetic key can
/// never be mistaken for a real `reminder_at` instant (01) or a `renag:` key (04)
/// in the shared `reminder_deliveries` table, or the other way round.
const SUMMARY_KEY_PREFIX: &str = "summary:";

/// The task id the summary's delivery is recorded under. It belongs to no task —
/// the summary is a standalone daily notification — so it rides a fixed synthetic
/// id, which the delivery key's `(todo_id, reminder_at)` pair needs. Real task
/// ids are UUIDs and real `reminder_at`s are ISO instants, so neither half of the
/// pair can collide with a stored todo's.
const SUMMARY_TODO_ID: &str = "morning-summary";

/// The three counts the summary reports, as they stand at the poll it is built.
///
/// `my_day` is `Option` on purpose: its source (视图与统计/06) has not landed, so
/// it is `None` today and its segment is left out — a missing source is an omitted
/// segment, not a zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SummaryCounts {
    /// Open, un-archived tasks whose due date is today's local calendar day.
    pub due_today: i64,
    /// Open, un-archived tasks whose due date is a local calendar day before
    /// today's.
    pub overdue: i64,
    /// The 我的一天 (today's list) count, or `None` until 视图与统计/06 supplies
    /// its source.
    pub my_day: Option<i64>,
}

impl SummaryCounts {
    /// Whether there is nothing worth a summary: no task due today, none overdue,
    /// and no 我的一天 items (or no source for them). The scheduler sends nothing
    /// when this holds, so an empty morning is silence rather than an "all clear"
    /// that repeats every day.
    fn is_empty(&self) -> bool {
        self.due_today == 0 && self.overdue == 0 && self.my_day.unwrap_or(0) == 0
    }
}

/// The user's morning-summary preference: whether it is on, and the `"HH:mm"`
/// local time it should push at.
///
/// The time is kept as the raw string the settings store holds so the one strict
/// `"HH:mm"` reading in the domain ([`crate::quiet::parse_hm`]) parses it too,
/// rather than a second parser here drifting from the one the quiet window uses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorningSummaryPrefs {
    pub enabled: bool,
    pub time: String,
}

/// One morning summary that should be raised now.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorningSummary {
    /// The synthetic task id the delivery is recorded under (`morning-summary`).
    pub todo_id: String,
    /// The synthetic delivery key for today's summary (`summary:<YYYY-MM-DD>`),
    /// recorded in `reminder_deliveries` exactly as a reminder or renag records
    /// its key, so the summary is raised once a day and a restart does not repeat
    /// it.
    pub reminder_at: String,
    /// The instant the summary was scheduled for (today's local date at the push
    /// time), for placing it against the quiet window the same way a reminder's
    /// or renag's instant is placed.
    pub scheduled_unix: i64,
    /// The notification headline.
    pub title: String,
    /// The count body — the non-zero segments joined with ` · `, ending in a
    /// full stop.
    pub body: String,
}

/// Counts the tasks that feed the summary, as they stand at `now_unix`.
///
/// The reading matches the list's own due-date badges rather than the stricter
/// gate a renag applies (提醒通知/06 规格 §5): a task is counted whether or not it
/// is locked behind a prerequisite, because "what is on my plate today" is what
/// the user sees in the list (`src/lib/dueDate.ts`), which does not hide a locked
/// task's badge. `zone_offset_seconds` is where the device sits, in seconds east
/// of UTC: a due date is a local calendar day, so today's day and each task's day
/// are read through the same offset the renag anchor and quiet window use (0 for
/// now, the device's real offset from 提醒通知/07).
pub fn summary_counts(todos: &[Todo], now_unix: i64, zone_offset_seconds: i64) -> SummaryCounts {
    let today = (now_unix + zone_offset_seconds).div_euclid(SECONDS_PER_DAY);

    let mut due_today = 0;
    let mut overdue = 0;
    for todo in todos {
        // Completed or archived: off the list the user sees, so off the count.
        if !matches!(todo.status, TodoStatus::Open) || todo.archived_at.is_some() {
            continue;
        }
        // A task with no due date, or one this build cannot read, is neither due
        // today nor overdue — it has no deadline to weigh against the day.
        let Some(due_date) = todo.due_date.as_deref().and_then(parse_date) else {
            continue;
        };
        let due_day = days_from_civil(due_date);
        if due_day == today {
            due_today += 1;
        } else if due_day < today {
            overdue += 1;
        }
    }

    SummaryCounts {
        due_today,
        overdue,
        my_day: my_day_count(),
    }
}

/// The 我的一天 (today's list) count — the summary's third segment.
///
/// `None` until 视图与统计/06 (今日清单) lands: its "picked for today" list does
/// not exist yet, so the segment is omitted rather than faked with a stand-in
/// that would mislead once the real list is there. When it lands, that task fills
/// this seam with the day's count (its verification already carries "今日清单数据
/// 可被晨间摘要读取"), and the third segment joins [`format_summary_body`] with no
/// other change — a count read, no new field and no UI.
fn my_day_count() -> Option<i64> {
    None
}

/// The summary to raise at `now_unix`, or `None` when none is due.
///
/// Returns `None` when the summary is off, when its push time has not arrived yet
/// today, when today's summary already went out (its `summary:<date>` key is in
/// `delivered`), or when there is nothing worth reporting (every count zero). It
/// is quiet-agnostic: a summary whose time falls in the quiet window is still
/// returned here, and the scheduler holds it on the same gate the reminders ride
/// — the held poll records nothing, so a later poll after the window ends calls
/// this again and gets a summary built from the counts as they stand then.
///
/// `delivered` is the shared set of `(todo_id, reminder_at)` pairs already
/// raised. `zone_offset_seconds` reads the local day and time-of-day, the same
/// seam 提醒通知/07 completes.
pub fn due_morning_summary(
    todos: &[Todo],
    delivered: &HashSet<(String, String)>,
    prefs: &MorningSummaryPrefs,
    now_unix: i64,
    zone_offset_seconds: i64,
) -> Option<MorningSummary> {
    if !prefs.enabled {
        return None;
    }
    // A time the strict reader cannot make sense of (a hand-edited settings file)
    // leaves the summary un-schedulable, so it stays silent rather than firing at
    // some guessed hour.
    let scheduled_minute = parse_hm(&prefs.time)?;

    let now_local = now_unix + zone_offset_seconds;
    let local_day = now_local.div_euclid(SECONDS_PER_DAY);
    let now_minute_of_day = now_local.rem_euclid(SECONDS_PER_DAY) / 60;
    if now_minute_of_day < scheduled_minute {
        // The push time has not come round yet today.
        return None;
    }

    let date = format_iso_date(civil_from_days(local_day));
    let reminder_at = format!("{SUMMARY_KEY_PREFIX}{date}");
    if delivered.contains(&(SUMMARY_TODO_ID.to_owned(), reminder_at.clone())) {
        // Today's summary already went out — the whole point of the key.
        return None;
    }

    let counts = summary_counts(todos, now_unix, zone_offset_seconds);
    if counts.is_empty() {
        // Nothing due, nothing overdue, no my-day items: silence, not an empty
        // "all clear" that would repeat every morning.
        return None;
    }

    let scheduled_unix = local_day * SECONDS_PER_DAY + scheduled_minute * 60 - zone_offset_seconds;
    Some(MorningSummary {
        todo_id: SUMMARY_TODO_ID.to_owned(),
        reminder_at,
        scheduled_unix,
        title: "今日待办概览".to_owned(),
        body: format_summary_body(&counts),
    })
}

/// The count body: each non-zero segment as `<标签> <N> 条`, joined with ` · `
/// and ended with a full stop.
///
/// A zero segment is dropped — the summary surfaces what needs attention, so a
/// morning with overdue work but nothing due today reads "逾期 2 条。" rather than
/// carrying a "今日到期 0 条". Only ever called with a non-empty count, so the
/// joined body is never blank.
fn format_summary_body(counts: &SummaryCounts) -> String {
    let mut segments = Vec::new();
    if counts.due_today > 0 {
        segments.push(format!("今日到期 {} 条", counts.due_today));
    }
    if counts.overdue > 0 {
        segments.push(format!("逾期 {} 条", counts.overdue));
    }
    if let Some(my_day) = counts.my_day {
        if my_day > 0 {
            segments.push(format!("我的一天 {my_day} 条"));
        }
    }
    format!("{}。", segments.join(" · "))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::quiet::{plan_quiet_delivery, QuietWindow};

    /// A device on UTC — the offset the scheduler passes today.
    const UTC: i64 = 0;

    /// 2026-07-24T00:00:00Z, a UTC local midnight, so a minute-of-day maps to that
    /// many minutes past this instant.
    const MIDNIGHT: i64 = 1_784_851_200;

    /// A Unix instant at `hour:minute` UTC on the midnight day above.
    fn at(hour: i64, minute: i64) -> i64 {
        MIDNIGHT + (hour * 60 + minute) * 60
    }

    fn todo(id: &str) -> Todo {
        Todo {
            id: id.to_owned(),
            title: format!("task {id}"),
            status: TodoStatus::Open,
            created_at: "2026-07-01T00:00:00Z".to_owned(),
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
            check_ins: Vec::new(),
        }
    }

    fn due(id: &str, due_date: &str) -> Todo {
        Todo {
            due_date: Some(due_date.to_owned()),
            ..todo(id)
        }
    }

    fn no_deliveries() -> HashSet<(String, String)> {
        HashSet::new()
    }

    fn on(time: &str) -> MorningSummaryPrefs {
        MorningSummaryPrefs {
            enabled: true,
            time: time.to_owned(),
        }
    }

    /// Today, for the fixtures: 2026-07-24. A task due on this date is "due today",
    /// one due earlier is "overdue".
    fn one_due_today_and_one_overdue() -> Vec<Todo> {
        vec![due("a", "2026-07-24"), due("b", "2026-07-20")]
    }

    #[test]
    fn the_summary_fires_at_its_time_with_a_body_of_the_non_zero_counts() {
        let todos = one_due_today_and_one_overdue();
        // A minute before 09:00: not yet.
        assert!(
            due_morning_summary(&todos, &no_deliveries(), &on("09:00"), at(8, 59), UTC).is_none()
        );
        // On the dot it fires, and the body counts both segments.
        let summary = due_morning_summary(&todos, &no_deliveries(), &on("09:00"), at(9, 0), UTC)
            .expect("the summary is due at 09:00");
        assert_eq!(summary.title, "今日待办概览");
        assert_eq!(summary.body, "今日到期 1 条 · 逾期 1 条。");
        assert_eq!(summary.reminder_at, "summary:2026-07-24");
        assert_eq!(summary.todo_id, "morning-summary");
    }

    #[test]
    fn a_summary_already_sent_today_is_not_sent_again() {
        let todos = one_due_today_and_one_overdue();
        let first = due_morning_summary(&todos, &no_deliveries(), &on("09:00"), at(9, 0), UTC)
            .expect("the first summary of the day");

        // The same key recorded: a later poll the same day raises nothing.
        let delivered: HashSet<(String, String)> =
            [(first.todo_id.clone(), first.reminder_at.clone())]
                .into_iter()
                .collect();
        assert!(
            due_morning_summary(&todos, &delivered, &on("09:00"), at(9, 30), UTC).is_none(),
            "the day's summary is once and done"
        );
        assert!(
            due_morning_summary(&todos, &delivered, &on("09:00"), at(18, 0), UTC).is_none(),
            "still nothing later the same day"
        );
    }

    #[test]
    fn the_next_local_day_is_a_fresh_summary() {
        // The key is the local date, so a new day is a key the previous day's
        // delivery does not cover — and the count is re-read for the new day, so
        // yesterday's "due today" task is today's "overdue".
        let todos = vec![due("a", "2026-07-24")];
        let yesterday =
            due_morning_summary(&todos, &no_deliveries(), &on("09:00"), at(9, 0), UTC).unwrap();
        let delivered: HashSet<(String, String)> =
            [(yesterday.todo_id.clone(), yesterday.reminder_at.clone())]
                .into_iter()
                .collect();

        // A day on, at 09:00 on the 25th: a fresh key, and the task that was due
        // on the 24th now counts as overdue.
        let today = due_morning_summary(&todos, &delivered, &on("09:00"), at(33, 0), UTC)
            .expect("a new day's summary is due");
        assert_eq!(today.reminder_at, "summary:2026-07-25");
        assert_eq!(today.body, "逾期 1 条。");
    }

    #[test]
    fn an_all_zero_morning_sends_nothing() {
        // Nothing due today, nothing overdue: no summary, so an empty morning is
        // silence rather than a daily "all clear".
        let clear = vec![due("a", "2026-08-01"), todo("b")];
        assert!(
            due_morning_summary(&clear, &no_deliveries(), &on("09:00"), at(9, 0), UTC).is_none()
        );
    }

    #[test]
    fn the_switch_being_off_sends_nothing() {
        let todos = one_due_today_and_one_overdue();
        let off = MorningSummaryPrefs {
            enabled: false,
            time: "09:00".to_owned(),
        };
        assert!(due_morning_summary(&todos, &no_deliveries(), &off, at(9, 0), UTC).is_none());
    }

    #[test]
    fn an_unreadable_time_leaves_the_summary_unscheduled() {
        // A hand-edited settings file: the strict "HH:mm" reader returns nothing,
        // so the summary stays silent rather than firing at a guessed hour.
        let todos = one_due_today_and_one_overdue();
        for bad in ["9:00", "24:00", "", "0900"] {
            assert!(
                due_morning_summary(&todos, &no_deliveries(), &on(bad), at(23, 0), UTC).is_none(),
                "an unreadable time ({bad}) schedules nothing"
            );
        }
    }

    #[test]
    fn a_summary_scheduled_inside_the_quiet_window_is_held_until_it_ends() {
        // The correctness requirement from 提醒通知/05: a summary set to 07:00 with
        // a 22:00–08:00 quiet window must not break "no notification inside the
        // window". The summary is quiet-agnostic here — it returns due — and the
        // scheduler rides the same quiet plan the reminders do. A one-item plan
        // never summarises, so the summary is never folded into the quiet re-push.
        let window = QuietWindow::parse("22:00", "08:00");
        let held = vec![due("a", "2026-07-24")];

        // 07:30, inside the window: due here, but the plan holds it (delivers
        // nothing, records nothing), so a later poll re-derives it.
        let summary = due_morning_summary(&held, &no_deliveries(), &on("07:00"), at(7, 30), UTC)
            .expect("the summary is due at 07:30");
        let held_plan = plan_quiet_delivery(&[summary.scheduled_unix], window, at(7, 30), UTC);
        assert!(
            held_plan.deliver.is_empty() && held_plan.summarize.is_empty(),
            "inside the window the summary is held, not raised"
        );

        // 08:00, the window has ended: the summary is delivered on its own (never
        // summarised), and its body reflects the flush moment's counts — a second
        // task has since gone overdue, so the flushed summary counts it.
        let flushed = vec![due("a", "2026-07-24"), due("b", "2026-07-20")];
        let at_flush = due_morning_summary(&flushed, &no_deliveries(), &on("07:00"), at(8, 0), UTC)
            .expect("the summary is still due at the flush");
        let flush_plan = plan_quiet_delivery(&[at_flush.scheduled_unix], window, at(8, 0), UTC);
        assert_eq!(flush_plan.deliver, vec![0]);
        assert!(flush_plan.summarize.is_empty(), "one item is never a summary");
        // Re-computed at the flush: both the still-due and the newly-overdue task.
        assert_eq!(at_flush.body, "今日到期 1 条 · 逾期 1 条。");
    }

    #[test]
    fn the_counts_match_the_lists_badges_not_the_renag_gate() {
        // Completed, archived, and no-due-date tasks are off the count, and a
        // task locked behind a prerequisite is still counted — the summary answers
        // "what is on my plate today", which is what the list's badges show, not
        // the stricter set a renag would nag.
        let mut completed = due("done", "2026-07-24");
        completed.status = TodoStatus::Completed;
        completed.completed_at = Some("2026-07-24T08:00:00Z".to_owned());
        let mut archived = due("filed", "2026-07-20");
        archived.archived_at = Some("2026-07-24T00:00:00Z".to_owned());
        let mut locked = due("blocked", "2026-07-24");
        locked.depends_on = vec!["gate".to_owned()];

        let todos = vec![
            due("today", "2026-07-24"),
            due("late", "2026-07-19"),
            locked,
            completed,
            archived,
            todo("no-deadline"),
        ];
        let counts = summary_counts(&todos, at(9, 0), UTC);
        // "today" and the locked task are both due today; "late" is overdue; the
        // completed, archived and no-due-date tasks count for nothing.
        assert_eq!(counts.due_today, 2);
        assert_eq!(counts.overdue, 1);
        assert_eq!(counts.my_day, None);
    }

    #[test]
    fn the_body_drops_zero_segments_and_omits_the_absent_my_day() {
        // Overdue work but nothing due today: the zero segment is dropped.
        let overdue_only = SummaryCounts {
            due_today: 0,
            overdue: 2,
            my_day: None,
        };
        assert_eq!(format_summary_body(&overdue_only), "逾期 2 条。");

        // Due today but nothing overdue, and my_day still without a source.
        let due_only = SummaryCounts {
            due_today: 3,
            overdue: 0,
            my_day: None,
        };
        assert_eq!(format_summary_body(&due_only), "今日到期 3 条。");
    }

    #[test]
    fn the_my_day_segment_joins_the_body_once_its_source_lands() {
        // The seam 视图与统计/06 fills: a Some my-day count adds the third segment,
        // with no other change to the body's shape.
        let all_three = SummaryCounts {
            due_today: 3,
            overdue: 2,
            my_day: Some(5),
        };
        assert_eq!(format_summary_body(&all_three), "今日到期 3 条 · 逾期 2 条 · 我的一天 5 条。");
        // A zero my-day count is dropped like any other zero segment.
        let empty_my_day = SummaryCounts {
            due_today: 1,
            overdue: 0,
            my_day: Some(0),
        };
        assert_eq!(format_summary_body(&empty_my_day), "今日到期 1 条。");
    }
}
