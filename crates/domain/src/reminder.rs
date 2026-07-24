//! Deciding which reminders have come due — the pure half of the native reminder
//! scheduler.
//!
//! Pure rules only, per the layering in AGENTS.md: no Tauri, no clock, no
//! database. The scheduler thread reads the stored todos and the set of reminders
//! it has already delivered, hands them here with the current instant, and gets
//! back the reminders to raise now. Keeping the decision here rather than in the
//! thread is what lets `cargo test` cover the rules the product turns on:
//!
//! * a reminder that has arrived is raised, and only once — the delivery set is
//!   what a later poll, or a restart, reads to know it already went out;
//! * a reminder missed while the app was closed is raised again and marked
//!   overdue, which is the mark the restart catch-up has to carry;
//! * a task that is completed, archived, or still locked behind a prerequisite
//!   raises nothing — the same gate the running app applied before it ever armed a
//!   reminder (任务管理/12), re-derived here from the stored rows so the native
//!   path needs no help from the webview to honour it.

use std::collections::HashSet;

use todo_contracts::{Todo, TodoStatus};

use crate::dependency::dependency_lock;
use crate::ics::instant_unix_seconds;

/// One reminder that has come due and should be raised now.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DueReminder {
    /// The task the reminder belongs to.
    pub todo_id: String,
    /// The task's title, shown as the notification's headline.
    pub title: String,
    /// The task's due date (`YYYY-MM-DD`) when it has one, so the notification can
    /// name it exactly as the running app's toast does.
    pub due_date: Option<String>,
    /// The instant that came due, kept verbatim so the caller records it as the
    /// delivery key — the same string it read off the row, so the key it writes is
    /// the key a later poll looks up.
    pub reminder_at: String,
    /// Whether the moment had slipped far enough past before it was raised to be
    /// worth telling the user it is late — a reminder missed while the app was
    /// closed rather than one the running app caught promptly.
    pub overdue: bool,
}

/// The reminders that have come due at `now_unix` and have not been delivered.
///
/// `delivered` is the set of `(todo_id, reminder_at)` pairs already raised; a
/// reminder whose pair is in it is skipped, which is what keeps one instant from
/// firing twice and what lets a restart rebuild the schedule without re-raising
/// anything already sent. `overdue_grace_seconds` is how far past its moment a
/// reminder may be raised before it counts as late: raised within it (the app was
/// running and the poll caught the moment promptly) it is on time; raised well
/// after (the app was closed when the moment passed) it is overdue.
///
/// `now_unix` is seconds since the Unix epoch; a reminder is due when its instant
/// is at or before it.
pub fn due_reminders(
    todos: &[Todo],
    delivered: &HashSet<(String, String)>,
    now_unix: i64,
    overdue_grace_seconds: i64,
) -> Vec<DueReminder> {
    // The graph the lock is read against: every id this device holds, and the
    // subset that is done. Built once for the whole pass rather than per todo.
    let known: HashSet<String> = todos.iter().map(|todo| todo.id.clone()).collect();
    let completed: HashSet<String> = todos
        .iter()
        .filter(|todo| matches!(todo.status, TodoStatus::Completed))
        .map(|todo| todo.id.clone())
        .collect();

    let mut due = Vec::new();
    for todo in todos {
        // A completed or archived task raises nothing: it is off the list the
        // running app would have scheduled from.
        if !matches!(todo.status, TodoStatus::Open) || todo.archived_at.is_some() {
            continue;
        }
        let Some(reminder_at) = todo.reminder_at.as_deref() else {
            continue;
        };
        // Already raised: the whole point of the delivery set.
        if delivered.contains(&(todo.id.clone(), reminder_at.to_owned())) {
            continue;
        }
        // A value no writer here produces cannot be timed; skipping it matches the
        // running app dropping an unreadable reminder rather than firing it at the
        // epoch.
        let Some(reminder_unix) = instant_unix_seconds(reminder_at) else {
            continue;
        };
        // Not yet: a future instant waits for a later poll.
        if reminder_unix > now_unix {
            continue;
        }
        // Still waiting on a prerequisite: the task holds no reminder until it
        // unlocks (任务管理/12, 规格 d). Once its prerequisites are done a later
        // poll raises it — past-due, and marked overdue if its instant slipped by
        // while it waited, so a reminder whose moment passed under the lock catches
        // up the moment it opens.
        if dependency_lock(&todo.depends_on, &known, &completed).locked {
            continue;
        }
        due.push(DueReminder {
            todo_id: todo.id.clone(),
            title: todo.title.clone(),
            due_date: todo.due_date.clone(),
            reminder_at: reminder_at.to_owned(),
            overdue: now_unix - reminder_unix > overdue_grace_seconds,
        });
    }
    due
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRACE: i64 = 60;
    // 2026-07-24T09:00:00Z as seconds since the epoch, the instant the fixtures
    // below are reminded at.
    const NINE_AM: i64 = 1_784_883_600;

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

    fn reminded(id: &str, reminder_at: &str) -> Todo {
        Todo {
            reminder_at: Some(reminder_at.to_owned()),
            ..todo(id)
        }
    }

    fn no_deliveries() -> HashSet<(String, String)> {
        HashSet::new()
    }

    fn ids(due: &[DueReminder]) -> Vec<&str> {
        due.iter().map(|reminder| reminder.todo_id.as_str()).collect()
    }

    #[test]
    fn a_reminder_that_has_arrived_is_raised_once_it_is_due() {
        let todos = vec![reminded("a", "2026-07-24T09:00:00Z")];
        // A minute before nine: nothing is due yet.
        assert!(due_reminders(&todos, &no_deliveries(), NINE_AM - 60, GRACE).is_empty());
        // On the dot: it is due.
        assert_eq!(ids(&due_reminders(&todos, &no_deliveries(), NINE_AM, GRACE)), vec!["a"]);
    }

    #[test]
    fn a_reminder_already_delivered_is_not_raised_again() {
        let todos = vec![reminded("a", "2026-07-24T09:00:00Z")];
        let delivered: HashSet<(String, String)> =
            [("a".to_owned(), "2026-07-24T09:00:00Z".to_owned())]
                .into_iter()
                .collect();
        // Long past due, but it already went out: the delivery set is what makes a
        // restart rebuild the schedule without re-raising it.
        assert!(due_reminders(&todos, &delivered, NINE_AM + 86_400, GRACE).is_empty());
    }

    #[test]
    fn a_reminder_edited_to_a_new_time_is_raised_again() {
        // The delivery key is the pair, so re-arming the same task at a new instant
        // is a new key the old delivery does not cover.
        let delivered: HashSet<(String, String)> =
            [("a".to_owned(), "2026-07-24T09:00:00Z".to_owned())]
                .into_iter()
                .collect();
        let todos = vec![reminded("a", "2026-07-24T18:00:00Z")];
        let six_pm = NINE_AM + 9 * 3_600;
        assert_eq!(ids(&due_reminders(&todos, &delivered, six_pm, GRACE)), vec!["a"]);
    }

    #[test]
    fn a_reminder_within_the_grace_is_on_time_and_past_it_is_overdue() {
        let todos = vec![reminded("a", "2026-07-24T09:00:00Z")];
        // Raised half a minute late — the running app's poll caught it: on time.
        let prompt = due_reminders(&todos, &no_deliveries(), NINE_AM + 30, GRACE);
        assert_eq!(prompt.len(), 1);
        assert!(!prompt[0].overdue);
        // Raised an hour late — the app was closed when nine came round: overdue.
        let late = due_reminders(&todos, &no_deliveries(), NINE_AM + 3_600, GRACE);
        assert_eq!(late.len(), 1);
        assert!(late[0].overdue);
    }

    #[test]
    fn a_completed_or_archived_task_raises_nothing() {
        let mut done = reminded("a", "2026-07-24T09:00:00Z");
        done.status = TodoStatus::Completed;
        done.completed_at = Some("2026-07-24T08:00:00Z".to_owned());
        let mut archived = reminded("b", "2026-07-24T09:00:00Z");
        archived.archived_at = Some("2026-07-20T00:00:00Z".to_owned());

        let due = due_reminders(&[done, archived], &no_deliveries(), NINE_AM + 3_600, GRACE);
        assert!(due.is_empty(), "{:?}", ids(&due));
    }

    #[test]
    fn a_task_with_no_reminder_or_an_unreadable_one_raises_nothing() {
        let plain = todo("a");
        let unreadable = reminded("b", "sometime on tuesday");
        let due = due_reminders(&[plain, unreadable], &no_deliveries(), NINE_AM + 3_600, GRACE);
        assert!(due.is_empty(), "{:?}", ids(&due));
    }

    #[test]
    fn a_locked_task_holds_its_reminder_until_the_prerequisite_is_done() {
        let mut blocked = reminded("a", "2026-07-24T09:00:00Z");
        blocked.depends_on = vec!["gate".to_owned()];

        // While the prerequisite is open, the blocked task raises nothing (only the
        // prerequisite's own reminder, an hour earlier, is due).
        let gate_open = reminded("gate", "2026-07-24T08:00:00Z");
        let locked = due_reminders(
            &[gate_open, blocked.clone()],
            &no_deliveries(),
            NINE_AM + 3_600,
            GRACE,
        );
        assert_eq!(ids(&locked), vec!["gate"]);

        // Complete the prerequisite: the blocked task unlocks, and its past-due
        // reminder catches up — marked overdue because its moment slipped by while
        // it waited under the lock.
        let mut gate_done = reminded("gate", "2026-07-24T08:00:00Z");
        gate_done.status = TodoStatus::Completed;
        gate_done.completed_at = Some("2026-07-24T08:30:00Z".to_owned());
        let unlocked = due_reminders(
            &[gate_done, blocked],
            &no_deliveries(),
            NINE_AM + 3_600,
            GRACE,
        );
        // Only the still-open blocked task is left to raise (the completed
        // prerequisite raises nothing), and it is overdue.
        assert_eq!(ids(&unlocked), vec!["a"]);
        assert!(unlocked[0].overdue);
    }

    #[test]
    fn consecutive_recurring_instances_each_raise_their_own_reminder() {
        // The core of 周期提醒: a repeat fires on every instance, across a day
        // boundary. The scheduler knows nothing about repeat rules — the store
        // generates the next instance (任务管理/08 `buildNextInstance`) as a fresh
        // row with a new id and every date shifted onto the next occurrence — so all
        // that has to hold here is that each instance's own `(todo_id, reminder_at)`
        // is a delivery key of its own. The instants are written the way the store
        // writes them (`shiftInstant` → `toISOString`, with the `.000Z` fraction),
        // so this also pins that `instant_unix_seconds` reads that exact shape.
        let first_at = "2026-07-24T09:00:00.000Z";
        let instance_one = reminded("instance-1", first_at);

        // Instance one comes due at nine and is raised; the app records it.
        assert_eq!(
            ids(&due_reminders(&[instance_one.clone()], &no_deliveries(), NINE_AM, GRACE)),
            vec!["instance-1"]
        );

        // Completing it files the row done and drops a fresh instance in its place —
        // new id, reminder a day on (`buildNextInstance` shifts every date by the
        // days the due date moved). A day later that successor is due; the completed
        // predecessor, already delivered and now off the open list, raises nothing.
        let mut predecessor = instance_one;
        predecessor.status = TodoStatus::Completed;
        predecessor.completed_at = Some("2026-07-24T09:03:00Z".to_owned());
        let successor = reminded("instance-2", "2026-07-25T09:00:00.000Z");
        let delivered: HashSet<(String, String)> =
            [("instance-1".to_owned(), first_at.to_owned())]
                .into_iter()
                .collect();

        let next_day =
            due_reminders(&[predecessor, successor], &delivered, NINE_AM + 86_400, GRACE);
        assert_eq!(ids(&next_day), vec!["instance-2"]);
        // On its own day the successor is on time, not a missed catch-up.
        assert!(!next_day[0].overdue);
    }

    #[test]
    fn a_recurring_row_advanced_in_place_re_arms_its_reminder_for_the_next_occurrence() {
        // The other generation path (任务管理/09 habit check-in, and the overdue
        // catch-up `advanceOverdueRecurring`): the row keeps its id and its date
        // moves to the next occurrence, the reminder moving with it. So the same id
        // carries a new `reminder_at` — a delivery key the previous occurrence's does
        // not cover, which is why a habit reminds on every occurrence rather than
        // only the first.
        let habit_id = "habit";
        let first_at = "2026-07-24T09:00:00.000Z";
        let next_at = "2026-07-25T09:00:00.000Z";

        // The first occurrence is raised and recorded.
        assert_eq!(
            ids(&due_reminders(&[reminded(habit_id, first_at)], &no_deliveries(), NINE_AM, GRACE)),
            vec![habit_id]
        );
        let after_first: HashSet<(String, String)> =
            [(habit_id.to_owned(), first_at.to_owned())].into_iter().collect();

        // Checked in for the day, the row advances in place to the next day at the
        // same wall-clock time. The old key is spent, but the new day's key is not,
        // so the next occurrence is raised rather than swallowed as a repeat.
        let next_day =
            due_reminders(&[reminded(habit_id, next_at)], &after_first, NINE_AM + 86_400, GRACE);
        assert_eq!(ids(&next_day), vec![habit_id]);

        // Once that occurrence is itself recorded it does not fire twice — the same
        // guard that stops any single instant repeating, so each occurrence raises
        // exactly one reminder.
        let after_second: HashSet<(String, String)> = [
            (habit_id.to_owned(), first_at.to_owned()),
            (habit_id.to_owned(), next_at.to_owned()),
        ]
        .into_iter()
        .collect();
        assert!(due_reminders(
            &[reminded(habit_id, next_at)],
            &after_second,
            NINE_AM + 86_400,
            GRACE
        )
        .is_empty());
    }
}
