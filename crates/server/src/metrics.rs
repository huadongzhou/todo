//! What the server keeps about how it has been doing.
//!
//! Two things are recorded, because two different questions get asked about a
//! sync server that has gone wrong. How are requests going — how many, how many
//! the log refused, how long the slowest took and how much of that was spent
//! waiting for the single connection. And what has the log been unable to read
//! — which rows, since when, and whether more of them keep turning up.
//!
//! Both are kept over a rolling window as well as for the whole run. The window
//! is what lets an alert clear on its own once the fault stops; the totals are
//! what is left for whoever has to clean up afterwards.

use std::collections::{BTreeSet, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use time::OffsetDateTime;
use todo_contracts::{RowFaults, SyncHealth};

use crate::rfc3339;

/// How far back the windowed numbers reach.
///
/// Two minutes: long enough that one slow request does not raise an alarm,
/// short enough that an operator who has just fixed something sees the alert
/// clear while still watching.
pub const WINDOW: Duration = Duration::from_secs(120);

/// How many slots the window is cut into.
///
/// Slots are how the window forgets: each holds the requests that landed in one
/// slice of it, and a slice whose turn comes round again is emptied rather than
/// added to. Thirty of them means the window is accurate to a thirtieth of its
/// length, which is finer than any threshold here cares about.
const SLOTS: usize = 30;

/// How a request ended, from the server's point of view.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestOutcome {
    Served,
    /// The contract refused what the device sent. Not the server's failure, and
    /// counted apart so one misbehaving device cannot declare the server down.
    Rejected,
    /// The log could not serve the request.
    Failed,
}

#[derive(Clone, Copy, Default)]
struct RequestSlot {
    /// Which slice of time this slot holds, or `None` while it holds nothing.
    id: Option<u64>,
    requests: u64,
    failures: u64,
    rejections: u64,
    micros: u64,
    micros_max: u64,
}

struct RequestState {
    slots: [RequestSlot; SLOTS],
    requests_total: u64,
    failures_total: u64,
}

/// How sync requests have been going.
pub struct RequestMetrics {
    window: Duration,
    started: Instant,
    state: Mutex<RequestState>,
}

impl Default for RequestMetrics {
    fn default() -> Self {
        Self::new(WINDOW)
    }
}

impl RequestMetrics {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            started: Instant::now(),
            state: Mutex::new(RequestState {
                slots: [RequestSlot::default(); SLOTS],
                requests_total: 0,
                failures_total: 0,
            }),
        }
    }

    /// Records one finished request.
    ///
    /// `elapsed` is the whole service call — the wait for the connection and
    /// the durable commit included — because that is what the device waited
    /// for.
    pub fn record(&self, outcome: RequestOutcome, elapsed: Duration) {
        let id = slot_id(self.started, self.window, Instant::now());
        let micros = duration_micros(elapsed);

        let Ok(mut state) = self.state.lock() else {
            // Losing a measurement is not worth failing a request that
            // otherwise succeeded.
            return;
        };

        state.requests_total += 1;
        if outcome == RequestOutcome::Failed {
            state.failures_total += 1;
        }

        let slot = &mut state.slots[(id as usize) % SLOTS];
        if slot.id != Some(id) {
            *slot = RequestSlot {
                id: Some(id),
                ..RequestSlot::default()
            };
        }
        slot.requests += 1;
        match outcome {
            RequestOutcome::Served => {}
            RequestOutcome::Rejected => slot.rejections += 1,
            RequestOutcome::Failed => slot.failures += 1,
        }
        slot.micros += micros;
        slot.micros_max = slot.micros_max.max(micros);
    }

    /// The numbers as they stand, with `lock_wait_slowest` folded in from the
    /// log's own observations — the two are measured in different places and
    /// read as one.
    pub fn snapshot(&self, lock_wait_slowest: Duration) -> SyncHealth {
        let oldest = oldest_slot_id(self.started, self.window, Instant::now());
        let Ok(state) = self.state.lock() else {
            return empty_sync_health(self.window);
        };

        let mut requests = 0;
        let mut failures = 0;
        let mut rejections = 0;
        let mut micros = 0;
        let mut micros_max = 0;
        for slot in state.slots.iter() {
            if !slot.id.is_some_and(|id| id >= oldest) {
                continue;
            }
            requests += slot.requests;
            failures += slot.failures;
            rejections += slot.rejections;
            micros += slot.micros;
            micros_max = micros_max.max(slot.micros_max);
        }

        SyncHealth {
            window_seconds: self.window.as_secs(),
            requests,
            failures,
            rejections,
            average_millis: match requests {
                0 => 0.0,
                requests => millis(micros / requests),
            },
            slowest_millis: millis(micros_max),
            lock_wait_slowest_millis: duration_millis(lock_wait_slowest),
            requests_total: state.requests_total,
            failures_total: state.failures_total,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct WaitSlot {
    id: Option<u64>,
    micros_max: u64,
}

/// Distinct rows the log could not read as they were meant to be read, and how
/// long requests have been waiting for the log's one connection.
///
/// Both belong to the store rather than to the service: they are what reading
/// and locking the file cost, observed where it happens.
pub struct LogObservations {
    window: Duration,
    started: Instant,
    waits: Mutex<[WaitSlot; SLOTS]>,
    unreadable: Mutex<RowFaultLog>,
    partly_read: Mutex<RowFaultLog>,
}

impl Default for LogObservations {
    fn default() -> Self {
        Self::new(WINDOW)
    }
}

impl LogObservations {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            started: Instant::now(),
            waits: Mutex::new([WaitSlot::default(); SLOTS]),
            unreadable: Mutex::new(RowFaultLog::default()),
            partly_read: Mutex::new(RowFaultLog::default()),
        }
    }

    /// Records how long a request waited before it had the connection.
    pub fn note_wait(&self, waited: Duration) {
        let id = slot_id(self.started, self.window, Instant::now());
        let micros = duration_micros(waited);

        let Ok(mut waits) = self.waits.lock() else {
            return;
        };
        let slot = &mut waits[(id as usize) % SLOTS];
        if slot.id != Some(id) {
            *slot = WaitSlot {
                id: Some(id),
                micros_max: 0,
            };
        }
        slot.micros_max = slot.micros_max.max(micros);
    }

    /// Notes a row that could not be read at all, answering whether this is the
    /// first time this process has met it.
    ///
    /// The answer is what a caller says out loud: a row that cannot be read
    /// will be met again on every pull from a cursor below it, and a warning
    /// per read is noise that hides the one thing worth knowing, which is that
    /// another row has gone bad.
    pub fn note_unreadable(&self, revision: i64, detail: String) -> bool {
        self.note(&self.unreadable, revision, detail)
    }

    /// Notes a row served without the fields this build has no name for.
    pub fn note_partly_read(&self, revision: i64, detail: String) -> bool {
        self.note(&self.partly_read, revision, detail)
    }

    fn note(&self, log: &Mutex<RowFaultLog>, revision: i64, detail: String) -> bool {
        let Ok(mut log) = log.lock() else {
            return false;
        };
        log.note(revision, detail, self.window)
    }

    /// The longest any request in the window waited for the connection.
    pub fn slowest_wait(&self) -> Duration {
        let oldest = oldest_slot_id(self.started, self.window, Instant::now());
        let Ok(waits) = self.waits.lock() else {
            return Duration::ZERO;
        };
        let micros = waits
            .iter()
            .filter(|slot| slot.id.is_some_and(|id| id >= oldest))
            .map(|slot| slot.micros_max)
            .max()
            .unwrap_or_default();
        Duration::from_micros(micros)
    }

    pub fn unreadable_rows(&self) -> RowFaults {
        summarise(&self.unreadable, self.window)
    }

    pub fn partly_read_rows(&self) -> RowFaults {
        summarise(&self.partly_read, self.window)
    }
}

/// The rows one kind of fault has been met on.
///
/// `rows` holds a revision per distinct row rather than a count, because
/// counting them once each is the whole point — the same row is read again on
/// every pull from a cursor below it. A log where that set grows large is a log
/// where nothing can be read, which is a problem several sizes larger than the
/// memory it takes to say so.
#[derive(Default)]
struct RowFaultLog {
    rows: BTreeSet<i64>,
    reads: u64,
    first_seen_at: Option<OffsetDateTime>,
    newest_seen_at: Option<OffsetDateTime>,
    newest: Option<String>,
    /// When each distinct row was first met, kept only for the window.
    recent: VecDeque<Instant>,
}

impl RowFaultLog {
    fn note(&mut self, revision: i64, detail: String, window: Duration) -> bool {
        let now = Instant::now();
        self.forget_older_than(now, window);
        self.reads += 1;

        if !self.rows.insert(revision) {
            return false;
        }

        let moment = OffsetDateTime::now_utc();
        self.first_seen_at.get_or_insert(moment);
        self.newest_seen_at = Some(moment);
        self.newest = Some(detail);
        self.recent.push_back(now);
        true
    }

    fn forget_older_than(&mut self, now: Instant, window: Duration) {
        while self
            .recent
            .front()
            .is_some_and(|first| now.duration_since(*first) > window)
        {
            self.recent.pop_front();
        }
    }

    fn summarise(&mut self, window: Duration) -> RowFaults {
        self.forget_older_than(Instant::now(), window);
        RowFaults {
            rows: self.rows.len() as u64,
            reads: self.reads,
            new_in_window: self.recent.len() as u64,
            first_seen_at: self.first_seen_at.map(rfc3339),
            newest_seen_at: self.newest_seen_at.map(rfc3339),
            newest: self.newest.clone(),
        }
    }
}

fn summarise(log: &Mutex<RowFaultLog>, window: Duration) -> RowFaults {
    match log.lock() {
        Ok(mut log) => log.summarise(window),
        Err(_) => RowFaults {
            rows: 0,
            reads: 0,
            new_in_window: 0,
            first_seen_at: None,
            newest_seen_at: None,
            newest: None,
        },
    }
}

/// Which slice of the window `at` falls in, counted from the start of the run
/// so the number only ever grows.
fn slot_id(started: Instant, window: Duration, at: Instant) -> u64 {
    let slot = (window.as_nanos() / SLOTS as u128).max(1);
    (at.duration_since(started).as_nanos() / slot) as u64
}

/// The oldest slice still inside the window.
fn oldest_slot_id(started: Instant, window: Duration, at: Instant) -> u64 {
    slot_id(started, window, at).saturating_sub(SLOTS as u64 - 1)
}

fn duration_micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn duration_millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn millis(micros: u64) -> f64 {
    micros as f64 / 1_000.0
}

fn empty_sync_health(window: Duration) -> SyncHealth {
    SyncHealth {
        window_seconds: window.as_secs(),
        requests: 0,
        failures: 0,
        rejections: 0,
        average_millis: 0.0,
        slowest_millis: 0.0,
        lock_wait_slowest_millis: 0.0,
        requests_total: 0,
        failures_total: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_counted_by_how_they_ended() {
        let metrics = RequestMetrics::default();
        metrics.record(RequestOutcome::Served, Duration::from_millis(1));
        metrics.record(RequestOutcome::Failed, Duration::from_millis(2));
        metrics.record(RequestOutcome::Rejected, Duration::from_millis(3));

        let health = metrics.snapshot(Duration::ZERO);

        assert_eq!(health.requests, 3);
        assert_eq!(health.failures, 1);
        assert_eq!(health.rejections, 1);
        assert_eq!(health.requests_total, 3);
        assert_eq!(health.failures_total, 1);
    }

    #[test]
    fn the_slowest_request_and_the_average_are_both_kept() {
        let metrics = RequestMetrics::default();
        metrics.record(RequestOutcome::Served, Duration::from_millis(10));
        metrics.record(RequestOutcome::Served, Duration::from_millis(30));

        let health = metrics.snapshot(Duration::from_millis(7));

        assert!((health.slowest_millis - 30.0).abs() < 1.0, "{health:?}");
        assert!((health.average_millis - 20.0).abs() < 1.0, "{health:?}");
        assert!(
            (health.lock_wait_slowest_millis - 7.0).abs() < 0.1,
            "{health:?}"
        );
    }

    #[test]
    fn what_happened_before_the_window_stops_being_reported() {
        // The window is what lets an alert clear on its own: a burst of
        // failures that stopped has to fall out of it, while the totals keep
        // it on the record.
        let metrics = RequestMetrics::new(Duration::from_millis(60));
        metrics.record(RequestOutcome::Failed, Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(120));

        let health = metrics.snapshot(Duration::ZERO);

        assert_eq!(health.requests, 0);
        assert_eq!(health.failures, 0);
        assert_eq!(health.requests_total, 1, "the run's total does not forget");
        assert_eq!(health.failures_total, 1);
    }

    #[test]
    fn a_row_read_again_is_not_a_second_row() {
        // The point of counting rather than warning: a row nothing can read is
        // read again on every pull from a cursor below it.
        let observations = LogObservations::default();

        assert!(observations.note_unreadable(7, "revision 7: title".to_owned()));
        assert!(!observations.note_unreadable(7, "revision 7: title".to_owned()));
        assert!(!observations.note_unreadable(7, "revision 7: title".to_owned()));
        assert!(observations.note_unreadable(9, "revision 9: kind".to_owned()));

        let faults = observations.unreadable_rows();
        assert_eq!(faults.rows, 2, "two rows, however often they are read");
        assert_eq!(faults.reads, 4);
        assert_eq!(faults.new_in_window, 2);
        assert_eq!(faults.newest.as_deref(), Some("revision 9: kind"));
        assert!(faults.first_seen_at.is_some());
        assert!(faults.newest_seen_at.is_some());
    }

    #[test]
    fn rows_that_stopped_turning_up_leave_the_window_but_not_the_count() {
        let observations = LogObservations::new(Duration::from_millis(60));
        observations.note_unreadable(7, "revision 7: title".to_owned());
        std::thread::sleep(Duration::from_millis(120));

        let faults = observations.unreadable_rows();

        assert_eq!(faults.new_in_window, 0, "nothing new has been found");
        assert_eq!(faults.rows, 1, "the row is still there and still counted");
    }

    #[test]
    fn the_two_kinds_of_bad_row_are_counted_apart() {
        // A row nobody can read and a row served without a field a newer build
        // wrote call for opposite actions, so they are never added together.
        let observations = LogObservations::default();
        observations.note_unreadable(7, "revision 7: title".to_owned());
        observations.note_partly_read(8, "revision 8: subtasks.0.note".to_owned());

        assert_eq!(observations.unreadable_rows().rows, 1);
        assert_eq!(observations.partly_read_rows().rows, 1);
        assert_eq!(
            observations.partly_read_rows().newest.as_deref(),
            Some("revision 8: subtasks.0.note")
        );
    }

    #[test]
    fn the_longest_wait_for_the_connection_is_the_one_reported() {
        let observations = LogObservations::default();
        observations.note_wait(Duration::from_millis(2));
        observations.note_wait(Duration::from_millis(40));
        observations.note_wait(Duration::from_millis(5));

        let slowest = observations.slowest_wait();

        assert!(slowest >= Duration::from_millis(39), "{slowest:?}");
        assert!(slowest <= Duration::from_millis(41), "{slowest:?}");
    }
}
