//! The operator's view of the server, and the alerts that come out of it.
//!
//! Everything an alert needs is already recorded elsewhere — `metrics` keeps
//! how requests have gone and what the log could not read, `todo_domain` holds
//! the rules that say when those numbers are worth acting on. What is left here
//! is putting the two together, remembering since when each concern has been
//! open, and saying so out loud when one opens or clears.
//!
//! Saying so out loud matters for a server nobody is watching: a self-hosted
//! deployment has no monitoring system attached by default, so the two things it
//! can do by itself are answer `/v1/health` (with a status an uptime probe can
//! act on) and put a line in its own log the moment something changes. Anything
//! beyond that — mail, a webhook, a hosted monitor — is a choice about a service
//! to buy or run, not something to decide here.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use time::OffsetDateTime;
use todo_contracts::{HealthAlert, HealthReport, LogHealth};
use todo_domain::{evaluate_health, overall_status, HealthThresholds, LOG_UNREACHABLE};

use crate::sync_store::LogProbe;
use crate::{rfc3339, SyncService};

/// How often the server looks at itself when nobody is asking.
///
/// A minute: an operator watching a fix land does not wait long for the alert
/// to clear, and a log line a minute is not a log an alert can hide in.
pub const DEFAULT_INTERVAL_SECONDS: u64 = 60;

/// Reads what the server has recorded about itself and answers what state it is
/// in.
pub struct HealthService {
    sync: Arc<SyncService>,
    thresholds: HealthThresholds,
    started: Instant,
    /// Which concerns are open and since when, so an operator can tell a fresh
    /// problem from one that has been open all day.
    open: Mutex<BTreeMap<&'static str, OffsetDateTime>>,
}

impl HealthService {
    pub fn new(sync: Arc<SyncService>) -> Self {
        Self::with_thresholds(sync, HealthThresholds::default())
    }

    pub fn with_thresholds(sync: Arc<SyncService>, thresholds: HealthThresholds) -> Self {
        Self {
            sync,
            thresholds,
            started: Instant::now(),
            open: Mutex::new(BTreeMap::new()),
        }
    }

    /// The state of the server, with anything that opened or cleared since the
    /// last look said out loud.
    pub fn report(&self) -> HealthReport {
        let store = self.sync.store();
        let probe = store.probe();
        let observations = store.observations();

        let log = LogHealth {
            reachable: matches!(probe, LogProbe::Ready { .. }),
            busy: matches!(probe, LogProbe::Busy),
            latest_revision: match &probe {
                LogProbe::Ready { latest_revision } => Some(*latest_revision),
                _ => None,
            },
            unreadable_rows: observations.unreadable_rows(),
            partly_read_rows: observations.partly_read_rows(),
        };
        let sync = self.sync.metrics().snapshot(observations.slowest_wait());

        if let LogProbe::Failed(error) = &probe {
            // Why it did not answer is known only here, and only worth saying
            // the first time: the alert below repeats for as long as it lasts.
            if !self.already_open(LOG_UNREACHABLE) {
                tracing::error!(%error, "the sync log did not answer a health check");
            }
        }

        let concerns = evaluate_health(&sync, &log, &self.thresholds);
        let status = overall_status(&concerns);
        let alerts = self.reconcile(&concerns);

        HealthReport {
            status,
            service: "todo-api".to_owned(),
            time: rfc3339(OffsetDateTime::now_utc()),
            uptime_seconds: self.started.elapsed().as_secs(),
            sync,
            log,
            alerts,
        }
    }

    fn already_open(&self, name: &str) -> bool {
        self.open_alerts().contains_key(name)
    }

    /// The open alerts. A panic while holding this lock costs an alert's
    /// "since", which is not worth failing a health check over.
    fn open_alerts(&self) -> std::sync::MutexGuard<'_, BTreeMap<&'static str, OffsetDateTime>> {
        self.open
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Brings the open set in line with what is wrong now, logging each change,
    /// and answers the alerts as they should be reported.
    fn reconcile(&self, concerns: &[todo_domain::HealthConcern]) -> Vec<HealthAlert> {
        let now = OffsetDateTime::now_utc();
        let mut open = self.open_alerts();

        open.retain(|name, since| {
            if concerns.iter().any(|concern| concern.name == *name) {
                return true;
            }
            tracing::info!(
                alert = name,
                since = %rfc3339(*since),
                "health alert cleared"
            );
            false
        });

        let mut alerts: Vec<HealthAlert> = concerns
            .iter()
            .map(|concern| {
                let since = *open.entry(concern.name).or_insert_with(|| {
                    tracing::error!(
                        alert = concern.name,
                        severity = ?concern.severity,
                        detail = %concern.detail,
                        "health alert opened"
                    );
                    now
                });
                HealthAlert {
                    name: concern.name.to_owned(),
                    severity: concern.severity,
                    since: rfc3339(since),
                    detail: concern.detail.clone(),
                }
            })
            .collect();

        // Oldest first: the one that has been open longest is usually the one
        // the others followed from.
        alerts.sort_by(|left, right| left.since.cmp(&right.since));
        alerts
    }
}

/// Looks at the server every `interval` for the life of the process.
///
/// A thread rather than a Tokio task, for the same reason the snapshot schedule
/// is one: the look takes locks that request handling also takes, and it must
/// keep happening when every worker is busy — a server too loaded to answer is
/// exactly the one whose alerts need to come out.
///
/// Nothing is logged when nothing changed. The transitions are the output.
pub fn spawn_watch(health: Arc<HealthService>, interval: Duration) -> thread::JoinHandle<()> {
    thread::spawn(move || loop {
        let report = health.report();
        tracing::debug!(
            status = ?report.status,
            requests = report.sync.requests,
            failures = report.sync.failures,
            slowest_millis = report.sync.slowest_millis,
            "health checked"
        );
        thread::sleep(interval);
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use todo_contracts::{HealthStatus, SyncRequest, TodoPatch, TodoSyncOperation};

    use super::*;
    use crate::SyncStore;

    fn health() -> HealthService {
        HealthService::new(Arc::new(SyncService::new(
            SyncStore::in_memory().expect("open an in-memory log"),
        )))
    }

    fn upsert(operation_id: &str, title: &str) -> SyncRequest {
        SyncRequest {
            device_id: "desktop-1".to_owned(),
            cursor: 0,
            operations: vec![TodoSyncOperation {
                operation_id: operation_id.to_owned(),
                todo_id: "todo-1".to_owned(),
                kind: todo_contracts::SyncOperationKind::Upsert,
                occurred_at: "2026-07-23T00:00:00Z".to_owned(),
                patch: Some(TodoPatch {
                    title: Some(title.to_owned()),
                    ..TodoPatch::default()
                }),
            }],
        }
    }

    #[test]
    fn a_server_that_has_served_says_so() {
        let health = health();
        health
            .sync
            .sync(upsert("operation-1", "write it down"))
            .expect("sync succeeds");

        let report = health.report();

        assert_eq!(report.status, HealthStatus::Ok);
        assert!(report.alerts.is_empty());
        assert!(report.log.reachable);
        assert_eq!(report.log.latest_revision, Some(1));
        assert_eq!(report.sync.requests, 1);
        assert_eq!(report.sync.failures, 0);
        assert_eq!(report.service, "todo-api");
    }

    #[test]
    fn a_request_the_contract_refused_is_not_the_servers_failure() {
        let health = health();
        for index in 0..4 {
            let mut bad = upsert(&format!("operation-{index}"), "   ");
            bad.operations[0].patch = Some(TodoPatch {
                title: Some("   ".to_owned()),
                ..TodoPatch::default()
            });
            assert!(health.sync.sync(bad).is_err());
        }

        let report = health.report();

        assert_eq!(report.sync.rejections, 4);
        assert_eq!(report.sync.failures, 0);
        assert_eq!(report.status, HealthStatus::Ok);
    }

    #[test]
    fn rows_that_cannot_be_read_are_reported_as_worth_looking_at() {
        // What puts them there is covered where they are read; what matters
        // here is that they reach the report and raise something.
        let health = health();
        health
            .sync
            .store()
            .observations()
            .note_unreadable(7, "revision 7: title is not readable".to_owned());

        let report = health.report();

        assert_eq!(report.log.unreadable_rows.rows, 1);
        assert_eq!(report.log.unreadable_rows.new_in_window, 1);
        assert!(report
            .log
            .unreadable_rows
            .newest
            .as_deref()
            .expect("the newest fault is named")
            .contains("title"));
        assert_eq!(report.status, HealthStatus::Degraded);
        assert_eq!(
            report
                .alerts
                .iter()
                .map(|alert| alert.name.as_str())
                .collect::<Vec<_>>(),
            vec![todo_domain::UNREADABLE_ROWS_GROWING]
        );
        assert!(report.alerts[0].detail.contains("revision 7"));
    }

    #[test]
    fn an_alert_keeps_the_time_it_opened_and_clears_when_the_fault_stops() {
        // The window is what makes an alert able to clear at all, so a short
        // one is what makes that testable.
        let sync = Arc::new(SyncService::with_window(
            SyncStore::in_memory().expect("open an in-memory log"),
            Duration::from_millis(80),
        ));
        let health = HealthService::new(Arc::clone(&sync));
        sync.store()
            .observations()
            .note_unreadable(7, "revision 7: title is not readable".to_owned());

        let opened = health.report();
        assert_eq!(opened.alerts.len(), 1);
        let since = opened.alerts[0].since.clone();

        let again = health.report();
        assert_eq!(
            again.alerts[0].since, since,
            "an alert that is still open keeps the moment it opened"
        );

        std::thread::sleep(Duration::from_millis(160));
        let settled = health.report();

        assert!(settled.alerts.is_empty(), "{:?}", settled.alerts);
        assert_eq!(settled.status, HealthStatus::Ok);
        assert_eq!(
            settled.log.unreadable_rows.rows, 1,
            "the row is still counted after the alert clears"
        );
    }
}
