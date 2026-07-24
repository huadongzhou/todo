//! The native reminder scheduler: the desktop half of "a reminder fires even
//! while the app is only in the tray, and a reminder missed while it was closed
//! is raised again — marked overdue — when it starts".
//!
//! The webview used to own reminder timing with `setTimeout`, which only runs
//! while the window is alive: close to the tray or quit, and every pending
//! reminder went silent. This moves the timing into a background thread that
//! polls the local database, so the webview no longer has to be open at all.
//!
//! Which reminders are due is a pure rule (`todo_domain::reminder::due_reminders`)
//! so `cargo test` can pin it down; this module owns the parts that cannot be a
//! pure function — the thread, the clock, the toast, and the record that a
//! reminder went out. The database is read through the same [`TodoDb`] the IPC
//! commands use (its `Mutex` serialises the two), so there is no second
//! connection and no reminder is ever raised from more than one place.
//!
//! Desktop only: the module is compiled just for desktop targets, so mobile — a
//! platform whose process the OS suspends, and whose local notifications are their
//! own scheduling work (TODO.md 2.3) — is left for that波次 to plug in here.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::Local;
use tauri::Manager;
use tauri_plugin_notification::NotificationExt;
use todo_domain::ics::instant_unix_seconds;
use todo_domain::quiet::{local_minute_of_day, plan_quiet_delivery};
use todo_domain::reminder::{due_reminders, DueReminder};
use todo_domain::renag::{due_renags, renag_occurrence_unix, RenagDue};
use todo_domain::summary::due_morning_summary;

use crate::reminder_prefs;
use crate::todo_db::TodoDb;

/// How often the schedule is re-read from the database.
///
/// A reminder is raised within this of its moment while the app runs, which is
/// close enough for a task reminder and keeps the poll to a handful of small reads
/// a minute. It is not the to-the-millisecond precision the old `setTimeout` had,
/// and a reminder does not need it.
const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// How far past its moment a reminder may be raised before it is called overdue.
///
/// Wider than one poll, so a reminder the running app raises a tick late still
/// reads as on time; a reminder missed while the app was closed is minutes or
/// hours late and reads as overdue, which is the mark the restart catch-up is
/// required to carry.
const OVERDUE_GRACE_SECONDS: i64 = 60;

/// The device's current offset from UTC, in seconds east — the local reads the
/// scheduler makes each poll (the overdue anchor 提醒通知/04, the quiet window and
/// its held-batch classification 提醒通知/05, and the morning summary's local day
/// and time 提醒通知/06) all weigh a wall-clock boundary, so they need to know
/// where the device sits (提醒通知/07).
///
/// Read fresh every poll, and from the OS rather than a preference the webview
/// wrote: the app can sit in the tray with no webview running while the machine
/// travels across a zone or crosses a daylight-saving boundary, so any cached
/// offset would go stale and fire a reminder at the wrong wall-clock time. The
/// domain rules already take a `zone_offset_seconds`; this is the seam that was a
/// fixed `0` until now, so those rules are unchanged — they are simply handed the
/// real offset.
///
/// `chrono::Local` re-reads the OS zone on each call, so the value is the one in
/// effect at this instant. A platform that cannot resolve its zone falls back to
/// UTC inside chrono, which reads late — never early — for the primary
/// east-of-UTC market, the same tolerance the fixed `0` had before it. The sign
/// matches the domain's convention directly (`local_minus_utc` is seconds east of
/// UTC, e.g. +28800 for UTC+8), so it is fed through with no adjustment.
fn current_zone_offset_seconds() -> i64 {
    i64::from(Local::now().offset().local_minus_utc())
}

/// Starts the background scheduler.
///
/// Best effort, per the AGENTS.md client rule that a native failure degrades
/// rather than aborts: a thread that will not spawn is logged and the app runs on
/// with reminders limited to the running window, instead of failing to launch.
pub fn spawn(app: &tauri::AppHandle) {
    // Ask for notification permission once, from the side that now raises
    // reminders (the webview no longer does). Best effort: a platform that refuses
    // or a user who declines degrades to no toast, exactly as the webview path did.
    if let Err(error) = app.notification().request_permission() {
        log::warn!("Notification permission could not be requested: {error}");
    }

    let handle = app.clone();
    if let Err(error) = std::thread::Builder::new()
        .name("reminder-scheduler".to_owned())
        .spawn(move || run(handle))
    {
        log::warn!(
            "Reminder scheduler thread not started; reminders are limited to the running window: \
             {error}"
        );
    }
}

/// Polls forever, one pass per [`POLL_INTERVAL`]. The first pass is a tick in, not
/// at launch, which lets the webview's own startup (draining parked writes, the
/// first sync) settle before the schedule is first read; a reminder that has been
/// waiting a tick longer is no worse for it.
fn run(app: tauri::AppHandle) {
    loop {
        std::thread::sleep(POLL_INTERVAL);
        poll_once(&app, now_unix_seconds(), current_zone_offset_seconds());
    }
}

/// One pass: read the schedule, raise what is due, record what went out.
///
/// Every failure is logged and swallowed — a database that is momentarily busy or
/// a toast that will not show must not take the thread down, or one bad poll would
/// end reminders for the rest of the run. The next tick tries again.
///
/// The due reminders and, when the user asked for them, the overdue renags
/// (提醒通知/04) are gathered into one batch and passed through the quiet-window
/// plan (提醒通知/05): inside the window nothing goes out, the night's held-back
/// reminders are folded into one summary when the window ends, and with quiet
/// hours off the plan is a plain "one toast each" — today's behaviour untouched.
fn poll_once(app: &tauri::AppHandle, now_unix: i64, zone_offset_seconds: i64) {
    let db = app.state::<TodoDb>();

    let todos = match db.list() {
        Ok(todos) => todos,
        Err(error) => {
            log::warn!("Reminder poll could not read todos: {error}");
            return;
        }
    };
    let delivered = match db.delivered_reminders() {
        Ok(delivered) => delivered,
        Err(error) => {
            log::warn!("Reminder poll could not read delivered reminders: {error}");
            return;
        }
    };

    // The due batch: reminders always; overdue renags only when the switch is on,
    // read fresh each poll so turning it off stops the next tick from nagging.
    let mut items: Vec<DueItem> =
        due_reminders(&todos, &delivered, now_unix, OVERDUE_GRACE_SECONDS)
            .into_iter()
            .map(DueItem::Reminder)
            .collect();
    if reminder_prefs::renag_overdue_enabled(app) {
        items.extend(
            due_renags(&todos, &delivered, now_unix, zone_offset_seconds)
                .into_iter()
                .map(DueItem::Renag),
        );
    }

    // The quiet window is read only when the switch is on, so with it off the plan
    // is `deliver` = every item and `summarize` empty — the pre-05 send path. A
    // switch left on but a window that will not parse reads as `None` too, and the
    // plan fails open on it: a reminder is delivered rather than silently swallowed.
    let window = if reminder_prefs::quiet_hours_enabled(app) {
        reminder_prefs::quiet_hours_window(app)
    } else {
        None
    };
    let instants: Vec<i64> = items.iter().map(|item| item.instant_unix(now_unix)).collect();
    let plan = plan_quiet_delivery(&instants, window, now_unix, zone_offset_seconds);

    // One toast each for the ordinary deliveries — reminders on time or overdue,
    // renags, and any lone reminder flushed after the window.
    //
    // A reminder whose own moment fell inside a quiet window was held on purpose,
    // so when it is flushed after the window it is a 补推, not a miss: it carries
    // no `【逾期】` mark, the same as the summary path already drops it for a
    // held batch of two or more (提醒通知/05, 提醒通知/07 §5). The check reads the
    // moment against the window through the very functions the plan classified it
    // with, so the lone-flushed reminder and the summarised batch agree on what
    // counts as "held". A fresh daytime reminder the app missed while it was
    // closed is not in the window and keeps its overdue mark.
    for &index in &plan.deliver {
        let item = &items[index];
        let quiet_flushed = window.is_some_and(|quiet| {
            quiet.contains(local_minute_of_day(instants[index], zone_offset_seconds))
        });
        let (title, body) = item.notification_text(quiet_flushed);
        show(app, &title, &body, item.todo_id());
        record(&db, item);
    }

    // A single summary for a held-back batch of two or more, then every one of them
    // is recorded, so the batch is not raised again next poll. Recording all of
    // them behind one toast is what makes "each reminder is delivered exactly once"
    // hold for the summary path (提醒通知/05 §2).
    if !plan.summarize.is_empty() {
        let (title, body) = summary_notification_text(plan.summarize.len());
        show(app, &title, &body, "quiet-hours summary");
        for &index in &plan.summarize {
            record(&db, &items[index]);
        }
    }

    // 提醒通知/06 morning summary: a separate daily toast — today's due and overdue
    // counts — read fresh each poll, so a restart or an edit is reflected at once.
    // It rides the same `reminder_deliveries` table with a `summary:<date>` key,
    // one per local day, so it is raised once a day and a restart does not repeat
    // it. Built only when the switch is on; off, the poll never touches it.
    if let Some(summary) = due_morning_summary(
        &todos,
        &delivered,
        &reminder_prefs::morning_summary_prefs(app),
        now_unix,
        zone_offset_seconds,
    ) {
        // Ride the same quiet gate the reminders do, but on its own one-item plan:
        // inside the window nothing goes out and nothing is recorded (a later poll
        // re-derives it from fresh counts, so a summary held through the night
        // reflects the flush moment); outside it the summary is raised on its own.
        // A one-item plan can never summarise, so the morning summary is never
        // folded into the quiet-hours re-push — the two stay two notifications
        // (提醒通知/06 §4).
        let plan = plan_quiet_delivery(&[summary.scheduled_unix], window, now_unix, zone_offset_seconds);
        if !plan.deliver.is_empty() {
            show(app, &summary.title, &summary.body, "morning summary");
            if let Err(error) =
                db.record_reminder_delivery(&summary.todo_id, &summary.reminder_at)
            {
                log::warn!("Morning summary delivery could not be recorded: {error}");
            }
        }
    }
}

/// A reminder or an overdue renag that has come due this poll, unified so the
/// quiet-hours plan weighs the two in one batch and can fold them into a single
/// summary together (提醒通知/05 §2).
enum DueItem {
    Reminder(DueReminder),
    Renag(RenagDue),
}

impl DueItem {
    /// The instant the item came due, for placing it against the quiet window.
    ///
    /// A reminder carries its ISO `reminder_at`; a renag carries a `renag:<unix>`
    /// key. A value neither reader can parse falls back to `now`, which reads as
    /// outside the window whenever the poll itself is — so a stored value gone bad
    /// is delivered, never held back and lost. In practice both always parse: the
    /// reminder pass has already dropped an unreadable `reminder_at`, and a renag
    /// key is one this build wrote.
    fn instant_unix(&self, now_unix: i64) -> i64 {
        let parsed = match self {
            DueItem::Reminder(reminder) => instant_unix_seconds(&reminder.reminder_at),
            DueItem::Renag(renag) => renag_occurrence_unix(&renag.reminder_at),
        };
        parsed.unwrap_or(now_unix)
    }

    /// The `(todo_id, reminder_at)` delivery key recorded once the item is raised,
    /// so a later poll or a restart does not raise it again.
    fn delivery_key(&self) -> (&str, &str) {
        match self {
            DueItem::Reminder(reminder) => (&reminder.todo_id, &reminder.reminder_at),
            DueItem::Renag(renag) => (&renag.todo_id, &renag.reminder_at),
        }
    }

    /// The item's own notification text — the deadline reminder's, or the renag's.
    ///
    /// `quiet_flushed` is whether the item's own moment fell inside a quiet window,
    /// so a reminder held there and flushed after it drops its `【逾期】` mark. It
    /// is meaningful only for a reminder: a renag's wording ("逾期未完成 · …") is
    /// its own kind of notification, not the late-catch-up mark, so it is unchanged.
    fn notification_text(&self, quiet_flushed: bool) -> (String, String) {
        match self {
            DueItem::Reminder(reminder) => notification_text(reminder, quiet_flushed),
            DueItem::Renag(renag) => renag_notification_text(renag),
        }
    }

    /// The task id, for the log line when a raise fails.
    fn todo_id(&self) -> &str {
        self.delivery_key().0
    }
}

/// Raises one toast, logging and swallowing a failure so a notification that will
/// not show does not take the poll — or the thread — down with it.
fn show(app: &tauri::AppHandle, title: &str, body: &str, context: &str) {
    if let Err(error) = app.notification().builder().title(title).body(body).show() {
        log::warn!("Notification for {context} could not be shown: {error}");
    }
}

/// Records a delivery whether or not its toast showed. The attempt has been made,
/// and re-raising it every tick because a show failed would turn one dropped toast
/// into a stream of them. A key that fails to record is retried next tick — at
/// worst one repeat, which "a reminder is never lost" prefers to one dropped.
fn record(db: &TodoDb, item: &DueItem) {
    let (todo_id, reminder_at) = item.delivery_key();
    if let Err(error) = db.record_reminder_delivery(todo_id, reminder_at) {
        log::warn!("Reminder delivery for todo {todo_id} could not be recorded: {error}");
    }
}

/// The toast a due reminder becomes — the native twin of the webview's old text.
///
/// An overdue reminder is marked in its title so the user can tell a reminder they
/// missed while the app was closed from one that has only just come due; the body
/// names the due date when there is one, matching the running app's wording so the
/// two are not two different notifications for the one task.
///
/// The mark is dropped when `quiet_flushed` — the reminder's own moment fell in a
/// quiet window, so its late delivery is a deliberate 补推 rather than a miss, and
/// marking it 逾期 would misread the user's own quiet hours as the app running
/// behind (提醒通知/07 §5). This is the same reasoning the summary path applies to
/// a held batch of two or more; doing it here for the lone flushed reminder keeps
/// the two补推 paths consistent.
fn notification_text(reminder: &DueReminder, quiet_flushed: bool) -> (String, String) {
    let title = if reminder.overdue && !quiet_flushed {
        format!("【逾期】{}", reminder.title)
    } else {
        reminder.title.clone()
    };
    let body = match reminder.due_date.as_deref() {
        Some(due_date) => format!("截止日：{due_date}"),
        None => "提醒时间到了".to_owned(),
    };
    (title, body)
}

/// The toast an overdue renag becomes (提醒通知/04).
///
/// Worded to read as its own kind of notification, not the deadline reminder
/// again and not 01's `【逾期】` catch-up (which means "this reminder went out
/// late"): a renag means "the deadline passed and the task is still open, here is
/// another nudge". The body names how long it has been overdue and says plainly
/// how to make it stop — completing the task — which is the one line that keeps a
/// repeat notification from feeling like it can never be silenced.
fn renag_notification_text(renag: &RenagDue) -> (String, String) {
    let title = format!("逾期未完成 · {}", renag.title);
    let body = format!("已逾期 {} 天，完成后不再提醒。", renag.days_overdue);
    (title, body)
}

/// The one toast a held-back batch becomes when the quiet window ends with two or
/// more reminders waiting (提醒通知/05 §2).
///
/// It names the count, not each task: a count is safe against any title and any
/// number, and the app's own reminder bar takes over the per-task detail the
/// moment the user opens the window. It carries no `【逾期】` mark either — 01's
/// mark means "this reminder went out late by accident", and a reminder the user's
/// own quiet hours held back was late on purpose, so the mark would misread it.
fn summary_notification_text(count: usize) -> (String, String) {
    (
        "勿扰时段已结束".to_owned(),
        format!("勿扰期间有 {count} 条提醒待查看。"),
    )
}

/// The current instant in seconds since the Unix epoch. A clock set before 1970
/// (the one case `duration_since` fails) reads as the epoch, which only makes a
/// reminder look due — never the reverse — so nothing is lost.
fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reminder(overdue: bool, due_date: Option<&str>) -> DueReminder {
        DueReminder {
            todo_id: "a".to_owned(),
            title: "买牛奶".to_owned(),
            due_date: due_date.map(str::to_owned),
            reminder_at: "2026-07-24T09:00:00.000Z".to_owned(),
            overdue,
        }
    }

    #[test]
    fn an_overdue_reminder_is_marked_in_its_title() {
        // The restart catch-up requirement: a re-raised reminder has to say it is
        // late so the user can tell it apart from one that just came due. Not a
        // quiet-hours flush, so the mark stays.
        let (title, body) = notification_text(&reminder(true, Some("2026-07-24")), false);
        assert!(title.contains("逾期"), "an overdue reminder must say so: {title}");
        assert!(title.contains("买牛奶"));
        assert_eq!(body, "截止日：2026-07-24");
    }

    #[test]
    fn an_on_time_reminder_is_just_the_task() {
        let (title, body) = notification_text(&reminder(false, None), false);
        assert_eq!(title, "买牛奶");
        assert!(!title.contains("逾期"));
        assert_eq!(body, "提醒时间到了");
    }

    #[test]
    fn a_reminder_flushed_after_the_quiet_window_drops_its_overdue_mark() {
        // 提醒通知/07 §5: a reminder whose own moment fell inside the quiet window
        // was held on purpose, so the lone-flush path drops `【逾期】` just as the
        // summary path drops it for a held batch — the user's quiet hours are not
        // the app running late. The body is unchanged; only the mark goes.
        let (title, body) = notification_text(&reminder(true, Some("2026-07-24")), true);
        assert_eq!(title, "买牛奶", "a quiet-hours flush is a 补推, not a miss: {title}");
        assert!(!title.contains("逾期"));
        assert_eq!(body, "截止日：2026-07-24");
        // An on-time reminder never had the mark, so `quiet_flushed` changes nothing.
        let (on_time, _) = notification_text(&reminder(false, None), true);
        assert_eq!(on_time, "买牛奶");
    }

    #[test]
    fn a_quiet_hours_summary_names_the_count_and_carries_no_overdue_mark() {
        // The window ended with several reminders waiting: one toast that says how
        // many, without 01's `【逾期】` (they were held on purpose, not late).
        let (title, body) = summary_notification_text(4);
        assert_eq!(title, "勿扰时段已结束");
        assert_eq!(body, "勿扰期间有 4 条提醒待查看。");
        assert!(!title.contains("逾期"));
    }

    #[test]
    fn a_due_item_reads_back_the_instant_that_placed_it_against_the_window() {
        // A reminder's instant comes from its ISO `reminder_at`; a renag's from its
        // `renag:<unix>` key. Both are what the quiet plan compares to the window.
        let reminder = DueItem::Reminder(reminder(false, None));
        assert_eq!(reminder.instant_unix(0), 1_784_883_600); // 2026-07-24T09:00:00Z
        let renag = DueItem::Renag(RenagDue {
            todo_id: "a".to_owned(),
            title: "交周报".to_owned(),
            reminder_at: "renag:1784908800".to_owned(),
            days_overdue: 3,
        });
        assert_eq!(renag.instant_unix(0), 1_784_908_800);
        // A value neither reader can parse falls back to `now`, so it is delivered
        // rather than held.
        let broken = DueItem::Renag(RenagDue {
            todo_id: "a".to_owned(),
            title: "x".to_owned(),
            reminder_at: "renag:oops".to_owned(),
            days_overdue: 1,
        });
        assert_eq!(broken.instant_unix(42), 42);
    }

    #[test]
    fn an_overdue_renag_reads_as_its_own_notification() {
        // Distinct from the deadline reminder and from 01's `【逾期】` catch-up: it
        // names the task, says how long it is overdue, and tells the user how to
        // stop it.
        let renag = RenagDue {
            todo_id: "a".to_owned(),
            title: "交周报".to_owned(),
            reminder_at: "renag:1784908800".to_owned(),
            days_overdue: 3,
        };
        let (title, body) = renag_notification_text(&renag);
        assert_eq!(title, "逾期未完成 · 交周报");
        assert_eq!(body, "已逾期 3 天，完成后不再提醒。");
    }
}
