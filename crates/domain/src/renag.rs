//! Deciding when an overdue task should be nagged again — the pure half of the
//! native reminder scheduler's renag pass.
//!
//! Pure rules only, per the layering in AGENTS.md: no Tauri, no clock, no
//! database. It runs beside [`crate::reminder::due_reminders`] rather than inside
//! it, because the two answer different questions from different anchors:
//!
//! * `due_reminders` is keyed on `reminder_at` — the one instant the user asked
//!   to be reminded at, raised once;
//! * this is keyed on `due_date` — a task that has passed its deadline and is
//!   still open, nagged again and again on a schedule until it is done.
//!
//! The nag times are a decaying backoff from the moment the task turns overdue:
//! `+1h → +3h → +7h → +15h` (i.e. one, two, four, eight hours apart), then once
//! every 24 hours from there, with no hard stop. The front is dense because a
//! deadline just missed is when a nudge is most likely to work; the tail settles
//! to once a day so a task overdue for a week is not nagged hourly. Only
//! completing the task, archiving it, moving its deadline into the future, or
//! turning the switch off stops it — the scheduler re-derives every one of those
//! from the stored rows each poll, so nothing has to tell this that a task left
//! the overdue set.
//!
//! Each nag is a delivery of its own, not the reminder fired again: the moment it
//! lands is a synthetic `reminder_at` key (`renag:<unix>`), which the scheduler
//! records in the same `reminder_deliveries` table 提醒通知/01 uses. So renag
//! *reuses* that "one key, one delivery" model rather than fighting it — each nag
//! is its own key raised once, and the keys live in a namespace (`renag:` prefix)
//! the real `reminder_at` instants can never collide with, so the two passes read
//! the one delivery set without stepping on each other.

use std::collections::HashSet;

use todo_contracts::{Todo, TodoStatus};

use crate::civil::{days_from_civil, parse_date};
use crate::dependency::dependency_lock;

const SECONDS_PER_DAY: i64 = 86_400;
const HOUR: i64 = 3_600;

/// The delivery-key namespace for a renag, so a synthetic nag key can never be
/// mistaken for a real `reminder_at` instant in the shared `reminder_deliveries`
/// table (提醒通知/01), and the other way round.
const RENAG_KEY_PREFIX: &str = "renag:";

/// Seconds after the overdue anchor at which each front-loaded nag lands, as a
/// cumulative offset: one hour, then three, then seven, then fifteen (spacings of
/// 1h, 2h, 4h, 8h). After the last of these the cap takes over.
const RENAG_STEP_SECONDS: [i64; 4] = [HOUR, 3 * HOUR, 7 * HOUR, 15 * HOUR];

/// The steady spacing once the front-loaded steps are spent: one nag a day, which
/// is "persistent" without being a pestering — a task overdue for a month is one
/// notification a day, not one an hour.
const RENAG_DAILY_SECONDS: i64 = 24 * HOUR;

/// One overdue task that should be nagged now.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenagDue {
    /// The task the nag belongs to.
    pub todo_id: String,
    /// The task's title, shown as the notification's headline.
    pub title: String,
    /// The synthetic delivery key for this particular nag (`renag:<unix>`). The
    /// caller records it in `reminder_deliveries` exactly as the reminder pass
    /// records a real `reminder_at`, so a nag that already went out is not raised
    /// again and a restart reads back what it already sent.
    pub reminder_at: String,
    /// How many whole local calendar days the task is past its due date, for the
    /// notification body. At least 1 whenever a nag is raised.
    pub days_overdue: i64,
}

/// The Unix instant a renag key stands for, or `None` when the value is not a
/// renag key this crate wrote.
///
/// A nag's delivery key is `renag:<unix>` (see [`RenagDue::reminder_at`]), the
/// instant the nag came due. 勿扰时段 (提醒通知/05) has to place that instant
/// against the quiet window, so it reads it back through this rather than split
/// the string a second way that could drift from how the key is built.
pub fn renag_occurrence_unix(reminder_at: &str) -> Option<i64> {
    reminder_at.strip_prefix(RENAG_KEY_PREFIX)?.parse().ok()
}

/// The overdue tasks that should be nagged at `now_unix`, one nag each.
///
/// `delivered` is the shared set of `(todo_id, reminder_at)` pairs already
/// raised — reminders and renags both — so a nag whose synthetic key is in it is
/// skipped, which is what keeps one nag from firing twice and lets a restart pick
/// up where it left off. `zone_offset_seconds` is where the device sits, in
/// seconds east of UTC: a due date is a local calendar day, so the instant it
/// turns overdue is local midnight of the following day, which is that many
/// seconds from the UTC day boundary. Supplying it is how the anchor matches the
/// view layer's own "overdue" reading; a caller that has no offset yet passes 0
/// and reads the boundary in UTC (提醒通知/07 supplies the real one).
///
/// A task raises at most one nag per call — the most recent one due — so an app
/// that was closed across several overdue days catches up with a single nag on
/// the next poll rather than a burst of every one it missed.
pub fn due_renags(
    todos: &[Todo],
    delivered: &HashSet<(String, String)>,
    now_unix: i64,
    zone_offset_seconds: i64,
) -> Vec<RenagDue> {
    // The graph the dependency lock is read against, built once for the pass, the
    // same way `due_reminders` builds it.
    let known: HashSet<String> = todos.iter().map(|todo| todo.id.clone()).collect();
    let completed: HashSet<String> = todos
        .iter()
        .filter(|todo| matches!(todo.status, TodoStatus::Completed))
        .map(|todo| todo.id.clone())
        .collect();

    let mut due = Vec::new();
    for todo in todos {
        // Completed or archived: off the overdue set entirely, the same gate the
        // reminder pass and the running app apply.
        if !matches!(todo.status, TodoStatus::Open) || todo.archived_at.is_some() {
            continue;
        }
        // Renag is anchored on the deadline, so a task without a due date — even
        // one with a reminder — has nothing to be overdue against (提醒通知/04
        // 规格 6.6). A due date this build cannot read is skipped rather than
        // treated as the epoch, matching the reminder pass.
        let Some(due_date) = todo.due_date.as_deref().and_then(parse_date) else {
            continue;
        };

        // Local midnight of the day after the due date, carried to a UTC instant:
        // the task is overdue once the local day has advanced past its due date,
        // exactly the view layer's `isOverdue`.
        let anchor = (days_from_civil(due_date) + 1) * SECONDS_PER_DAY - zone_offset_seconds;
        if now_unix < anchor {
            // Not overdue yet — a future deadline waits for a later poll.
            continue;
        }
        // Still waiting on a prerequisite: a locked task nags nothing until it
        // opens, the same hold the reminder pass applies (任务管理/12). Once it
        // unlocks a later poll catches it up with the current nag.
        if dependency_lock(&todo.depends_on, &known, &completed).locked {
            continue;
        }

        // The most recent nag due at `now`. `None` means the task is overdue but
        // the first nag (an hour in) has not arrived, so the deadline-time
        // reminder and the first nag never land on top of each other.
        let Some(offset) = latest_renag_offset(now_unix - anchor) else {
            continue;
        };
        let occurrence_unix = anchor + offset;
        let key = format!("{RENAG_KEY_PREFIX}{occurrence_unix}");
        // Already raised: this nag went out on an earlier poll (or before a
        // restart). The next one is still in the future.
        if delivered.contains(&(todo.id.clone(), key.clone())) {
            continue;
        }

        let now_local_day = (now_unix + zone_offset_seconds).div_euclid(SECONDS_PER_DAY);
        due.push(RenagDue {
            todo_id: todo.id.clone(),
            title: todo.title.clone(),
            reminder_at: key,
            days_overdue: now_local_day - days_from_civil(due_date),
        });
    }
    due
}

/// The offset from the overdue anchor of the most recent nag due after `elapsed`
/// seconds overdue, or `None` when the first nag is still ahead.
///
/// Returning only the latest — not every nag whose time has passed — is what
/// makes a catch-up after downtime a single nag rather than a burst: the poll
/// raises the one that is current and lets the ones it slept through stay
/// unraised, since nothing reads a nag older than the latest.
fn latest_renag_offset(elapsed: i64) -> Option<i64> {
    let first = RENAG_STEP_SECONDS[0];
    if elapsed < first {
        return None;
    }

    let cap = RENAG_STEP_SECONDS[RENAG_STEP_SECONDS.len() - 1];
    if elapsed < cap + RENAG_DAILY_SECONDS {
        // Still on the front-loaded steps: the greatest one at or before `elapsed`.
        return RENAG_STEP_SECONDS
            .into_iter()
            .filter(|&step| step <= elapsed)
            .next_back();
    }
    // Past the front: one nag every 24 hours from the last front-loaded step.
    let days_past = (elapsed - cap) / RENAG_DAILY_SECONDS;
    Some(cap + days_past * RENAG_DAILY_SECONDS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A device on UTC — the offset the scheduler passes today. The tests that
    /// are about the zone name their own.
    const UTC: i64 = 0;

    fn todo(id: &str) -> Todo {
        Todo {
            id: id.to_owned(),
            title: format!("task {id}"),
            status: TodoStatus::Open,
            created_at: "2026-07-01T00:00:00Z".to_owned(),
            completed_at: None,
            archived_at: None,
            due_date: None,
            reminders: Vec::new(),
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

    fn overdue(id: &str, due_date: &str) -> Todo {
        Todo {
            due_date: Some(due_date.to_owned()),
            ..todo(id)
        }
    }

    fn no_deliveries() -> HashSet<(String, String)> {
        HashSet::new()
    }

    fn ids(due: &[RenagDue]) -> Vec<&str> {
        due.iter().map(|renag| renag.todo_id.as_str()).collect()
    }

    /// The instant a task with this due date turns overdue, in the given zone —
    /// the anchor the nags count from.
    fn anchor(due_date: &str, zone_offset_seconds: i64) -> i64 {
        (days_from_civil(parse_date(due_date).expect("a readable due date")) + 1) * SECONDS_PER_DAY
            - zone_offset_seconds
    }

    #[test]
    fn a_task_not_yet_overdue_is_never_nagged() {
        let todos = vec![overdue("a", "2026-07-24")];
        let anchor = anchor("2026-07-24", UTC);
        // A second before the day turns over: still due, not overdue.
        assert!(due_renags(&todos, &no_deliveries(), anchor - 1, UTC).is_empty());
        // Exactly at the boundary it is overdue, but the first nag is an hour off:
        // the deadline reminder and the first nag do not land together.
        assert!(due_renags(&todos, &no_deliveries(), anchor, UTC).is_empty());
        assert!(due_renags(&todos, &no_deliveries(), anchor + HOUR - 1, UTC).is_empty());
    }

    #[test]
    fn the_first_nag_lands_an_hour_after_the_deadline_day_turns_over() {
        let todos = vec![overdue("a", "2026-07-24")];
        let anchor = anchor("2026-07-24", UTC);

        let due = due_renags(&todos, &no_deliveries(), anchor + HOUR, UTC);
        assert_eq!(ids(&due), vec!["a"]);
        // Overdue since the day turned over: one whole local day past the deadline.
        assert_eq!(due[0].days_overdue, 1);
        assert!(due[0].reminder_at.starts_with(RENAG_KEY_PREFIX));
    }

    #[test]
    fn a_nag_already_delivered_is_not_raised_again() {
        let todos = vec![overdue("a", "2026-07-24")];
        let anchor = anchor("2026-07-24", UTC);
        let first = due_renags(&todos, &no_deliveries(), anchor + HOUR, UTC);
        let key = first[0].reminder_at.clone();

        // The same nag key recorded: a later poll within the same step raises
        // nothing, the whole point of the delivery set.
        let delivered: HashSet<(String, String)> = [("a".to_owned(), key)].into_iter().collect();
        assert!(due_renags(&todos, &delivered, anchor + HOUR, UTC).is_empty());
        // Still nothing halfway to the next step.
        assert!(due_renags(&todos, &delivered, anchor + 2 * HOUR, UTC).is_empty());
    }

    #[test]
    fn the_front_loaded_steps_each_raise_a_nag_of_their_own() {
        let todos = vec![overdue("a", "2026-07-24")];
        let anchor = anchor("2026-07-24", UTC);
        // The four front-loaded nags land at +1h, +3h, +7h, +15h and no sooner.
        let mut keys = Vec::new();
        for step in [HOUR, 3 * HOUR, 7 * HOUR, 15 * HOUR] {
            let due = due_renags(&todos, &no_deliveries(), anchor + step, UTC);
            assert_eq!(ids(&due), vec!["a"], "a nag is due at +{}h", step / HOUR);
            keys.push(due[0].reminder_at.clone());
        }
        // Each step is a distinct delivery, so none of them dedup against another.
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), 4, "the four front-loaded nags are four keys");
    }

    #[test]
    fn once_the_front_is_spent_it_caps_at_one_nag_a_day() {
        let todos = vec![overdue("a", "2026-07-24")];
        let anchor = anchor("2026-07-24", UTC);

        // The first daily nag is 24h after the last front-loaded step (15h) — at
        // +39h — and nothing new lands in the gap after the fourth step.
        let fourth = due_renags(&todos, &no_deliveries(), anchor + 15 * HOUR, UTC);
        let fourth_key = fourth[0].reminder_at.clone();
        let delivered: HashSet<(String, String)> =
            [("a".to_owned(), fourth_key.clone())].into_iter().collect();
        // 20 hours after the fourth step: past it, but the daily nag (at +39h) is
        // not due yet, so the fourth nag holds and raises nothing new.
        assert!(due_renags(&todos, &delivered, anchor + 35 * HOUR, UTC).is_empty());

        // At +39h the first daily nag is due, and it is a new key.
        let daily = due_renags(&todos, &delivered, anchor + 39 * HOUR, UTC);
        assert_eq!(ids(&daily), vec!["a"]);
        assert_ne!(daily[0].reminder_at, fourth_key);
        assert_eq!(daily[0].days_overdue, 2);

        // A day on from that, at +63h, the next daily nag is due — one a day, no
        // hard stop.
        let recorded: HashSet<(String, String)> = [
            ("a".to_owned(), fourth_key),
            ("a".to_owned(), daily[0].reminder_at.clone()),
        ]
        .into_iter()
        .collect();
        assert!(due_renags(&todos, &recorded, anchor + 62 * HOUR, UTC).is_empty());
        let next_day = due_renags(&todos, &recorded, anchor + 63 * HOUR, UTC);
        assert_eq!(ids(&next_day), vec!["a"]);
        assert_ne!(next_day[0].reminder_at, daily[0].reminder_at);
    }

    #[test]
    fn a_gap_across_several_overdue_days_catches_up_with_one_nag_not_a_burst() {
        // The app was closed for days while the task stayed overdue. On the next
        // poll it must raise the current nag only — not every one it slept
        // through — which is what "capped at one a day" has to mean across
        // downtime.
        let todos = vec![overdue("a", "2026-07-24")];
        let anchor = anchor("2026-07-24", UTC);

        let due = due_renags(&todos, &no_deliveries(), anchor + 100 * HOUR, UTC);
        assert_eq!(due.len(), 1, "one catch-up nag, not a backlog: {:?}", ids(&due));
        assert_eq!(ids(&due), vec!["a"]);
        // 100h overdue is into the fifth local day past the deadline.
        assert_eq!(due[0].days_overdue, 5);
    }

    #[test]
    fn completing_or_archiving_a_task_stops_the_nagging() {
        let mut done = overdue("a", "2026-07-24");
        done.status = TodoStatus::Completed;
        done.completed_at = Some("2026-07-25T09:00:00Z".to_owned());
        let mut archived = overdue("b", "2026-07-24");
        archived.archived_at = Some("2026-07-25T00:00:00Z".to_owned());

        let anchor = anchor("2026-07-24", UTC);
        let due = due_renags(&[done, archived], &no_deliveries(), anchor + 10 * HOUR, UTC);
        assert!(due.is_empty(), "{:?}", ids(&due));
    }

    #[test]
    fn a_task_without_a_readable_due_date_is_never_nagged() {
        // No due date at all — a reminder-only task has no deadline to be overdue
        // against — and a due date this build cannot read: neither nags.
        let plain = todo("a");
        let mut reminded_only = todo("b");
        reminded_only.reminders = vec![todo_contracts::Reminder {
            at: "2026-07-24T09:00:00Z".to_owned(),
            offset: 0,
        }];
        let unreadable = overdue("c", "sometime last week");

        let far_future = anchor("2026-07-24", UTC) + 1_000 * HOUR;
        let due = due_renags(
            &[plain, reminded_only, unreadable],
            &no_deliveries(),
            far_future,
            UTC,
        );
        assert!(due.is_empty(), "{:?}", ids(&due));
    }

    #[test]
    fn a_locked_task_holds_its_nagging_until_the_prerequisite_is_done() {
        let mut blocked = overdue("a", "2026-07-24");
        blocked.depends_on = vec!["gate".to_owned()];
        let anchor = anchor("2026-07-24", UTC);

        // While the prerequisite is open, the blocked task nags nothing even
        // though its deadline has passed.
        let gate_open = overdue("gate", "2026-07-24");
        let locked = due_renags(
            &[gate_open, blocked.clone()],
            &no_deliveries(),
            anchor + 10 * HOUR,
            UTC,
        );
        // Only the prerequisite itself, overdue and open, nags.
        assert_eq!(ids(&locked), vec!["gate"]);

        // Complete the prerequisite: the blocked task unlocks and its nagging
        // catches up.
        let mut gate_done = overdue("gate", "2026-07-24");
        gate_done.status = TodoStatus::Completed;
        gate_done.completed_at = Some("2026-07-25T08:00:00Z".to_owned());
        let unlocked = due_renags(
            &[gate_done, blocked],
            &no_deliveries(),
            anchor + 10 * HOUR,
            UTC,
        );
        assert_eq!(ids(&unlocked), vec!["a"]);
    }

    #[test]
    fn the_overdue_anchor_follows_the_zone_offset() {
        // The same due date turns overdue at different instants depending on where
        // the device is: local midnight of the following day. East of UTC that
        // instant is earlier in UTC; west of UTC it is later. The rule reads the
        // boundary through the offset it is handed, which is the seam 提醒通知/07
        // completes by feeding the device's real offset.
        let todos = vec![overdue("a", "2026-07-24")];
        let east = 8 * HOUR; // UTC+8
        let west = -8 * HOUR; // UTC-8

        // An instant that is an hour past the east anchor but still before the
        // west one: nagged for the eastern device, silent for the western.
        let east_first_nag = anchor("2026-07-24", east) + HOUR;
        assert_eq!(
            ids(&due_renags(&todos, &no_deliveries(), east_first_nag, east)),
            vec!["a"],
        );
        assert!(
            due_renags(&todos, &no_deliveries(), east_first_nag, west).is_empty(),
            "west of UTC the day has not turned over yet"
        );

        // The western device reaches its own first nag 16 hours later.
        let west_first_nag = anchor("2026-07-24", west) + HOUR;
        assert_eq!(west_first_nag - east_first_nag, 16 * HOUR);
        assert_eq!(
            ids(&due_renags(&todos, &no_deliveries(), west_first_nag, west)),
            vec!["a"],
        );
    }

    #[test]
    fn a_renag_key_reads_back_to_the_instant_it_was_built_from() {
        // The key 提醒通知/05 places against the quiet window is the one this pass
        // wrote, so the two agree on the nag's moment.
        let todos = vec![overdue("a", "2026-07-24")];
        let anchor = anchor("2026-07-24", UTC);
        let occurrence = anchor + HOUR;
        let due = due_renags(&todos, &no_deliveries(), occurrence, UTC);
        assert_eq!(renag_occurrence_unix(&due[0].reminder_at), Some(occurrence));
        // A real reminder instant is not a renag key.
        assert_eq!(renag_occurrence_unix("2026-07-24T09:00:00Z"), None);
        assert_eq!(renag_occurrence_unix("renag:not-a-number"), None);
    }

    #[test]
    fn days_overdue_counts_whole_local_days_past_the_deadline() {
        let todos = vec![overdue("a", "2026-07-24")];
        let anchor = anchor("2026-07-24", UTC);
        // Ten hours in, still the first day past the deadline.
        assert_eq!(
            due_renags(&todos, &no_deliveries(), anchor + 10 * HOUR, UTC)[0].days_overdue,
            1
        );
        // Thirty hours in, the local day has turned once more: two days past.
        assert_eq!(
            due_renags(&todos, &no_deliveries(), anchor + 30 * HOUR, UTC)[0].days_overdue,
            2
        );
    }
}
