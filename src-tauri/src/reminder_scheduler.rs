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

use tauri::Manager;
use tauri_plugin_notification::NotificationExt;
use todo_domain::reminder::{due_reminders, DueReminder};

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
        poll_once(&app, now_unix_seconds());
    }
}

/// One pass: read the schedule, raise what is due, record what went out.
///
/// Every failure is logged and swallowed — a database that is momentarily busy or
/// a toast that will not show must not take the thread down, or one bad poll would
/// end reminders for the rest of the run. The next tick tries again.
fn poll_once(app: &tauri::AppHandle, now_unix: i64) {
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

    for reminder in due_reminders(&todos, &delivered, now_unix, OVERDUE_GRACE_SECONDS) {
        let (title, body) = notification_text(&reminder);
        if let Err(error) = app.notification().builder().title(title).body(body).show() {
            log::warn!(
                "Reminder for todo {} could not be shown: {error}",
                reminder.todo_id
            );
        }
        // Recorded whether or not the toast showed. The attempt has been made, and
        // re-raising it every tick because a show failed would turn one dropped
        // toast into a stream of them. A pair that fails to record here is retried
        // next tick — at worst one repeat, which "a reminder is never lost" prefers
        // to a reminder silently dropped.
        if let Err(error) = db.record_reminder_delivery(&reminder.todo_id, &reminder.reminder_at) {
            log::warn!(
                "Reminder delivery for todo {} could not be recorded: {error}",
                reminder.todo_id
            );
        }
    }
}

/// The toast a due reminder becomes — the native twin of the webview's old text.
///
/// An overdue reminder is marked in its title so the user can tell a reminder they
/// missed while the app was closed from one that has only just come due; the body
/// names the due date when there is one, matching the running app's wording so the
/// two are not two different notifications for the one task.
fn notification_text(reminder: &DueReminder) -> (String, String) {
    let title = if reminder.overdue {
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
        // late so the user can tell it apart from one that just came due.
        let (title, body) = notification_text(&reminder(true, Some("2026-07-24")));
        assert!(title.contains("逾期"), "an overdue reminder must say so: {title}");
        assert!(title.contains("买牛奶"));
        assert_eq!(body, "截止日：2026-07-24");
    }

    #[test]
    fn an_on_time_reminder_is_just_the_task() {
        let (title, body) = notification_text(&reminder(false, None));
        assert_eq!(title, "买牛奶");
        assert!(!title.contains("逾期"));
        assert_eq!(body, "提醒时间到了");
    }
}
