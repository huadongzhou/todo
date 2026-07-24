mod civil;
pub mod dependency;
pub mod habit;
pub mod holidays;
pub mod ics;
mod lunar;
pub mod quiet;
pub mod recurrence;
pub mod reminder;
pub mod renag;

pub use todo_contracts::SyncCursor;

use todo_contracts::{HealthStatus, LogHealth, SyncHealth};

pub fn next_sync_cursor(current: SyncCursor) -> Option<SyncCursor> {
    current.checked_add(1)
}

/// When the numbers a server reports about itself become worth saying out loud.
///
/// The thresholds live here, next to the rule that reads them, rather than in
/// the handler that serves the report: what counts as unwell is a judgement
/// about the service, not about HTTP.
#[derive(Clone, Copy, Debug)]
pub struct HealthThresholds {
    /// How few requests in the window make the failure ratio meaningless. A
    /// self-hosted server serves a handful of devices, so this is not a sample
    /// size — it is "more than one request went wrong".
    pub min_requests: u64,
    /// The share of requests the log refused that counts as an outage.
    pub failure_ratio: f64,
    /// How long the slowest request in the window may take before it is worth
    /// a look.
    ///
    /// A second is a number nothing healthy reaches: measured on a release
    /// build, one request at a time takes under a millisecond, and sixty-four
    /// devices syncing at once push the slowest to about 120 ms. It fires for
    /// a disk that has gone bad or a queue far past anything a self-hosted
    /// server is meant to carry, not for a busy afternoon.
    pub slowest_millis: f64,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            min_requests: 3,
            // Half of a small number of requests failing is not a blip: the
            // failures counted here are the log refusing to serve, and a log
            // that refuses one request in two is not serving.
            failure_ratio: 0.5,
            slowest_millis: 1_000.0,
        }
    }
}

/// One thing worth an operator's attention, and how much attention.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthConcern {
    /// A stable name, so a log line or a dashboard can be keyed on it.
    pub name: &'static str,
    pub severity: HealthStatus,
    pub detail: String,
}

pub const LOG_UNREACHABLE: &str = "sync_log_unreachable";
pub const SYNC_FAILURE_RATE: &str = "sync_failure_rate";
pub const SYNC_SLOW: &str = "sync_slow";
pub const UNREADABLE_ROWS_GROWING: &str = "unreadable_rows_growing";
pub const PARTLY_READ_ROWS_GROWING: &str = "partly_read_rows_growing";

/// Reads a server's own numbers and says what is worth acting on.
///
/// Every rule asks about the window rather than the whole run, because an alert
/// that cannot clear is one an operator learns to ignore: a fault that stopped
/// drops out of the window on its own, while the totals stay in the report for
/// whoever has to clean up after it.
pub fn evaluate_health(
    sync: &SyncHealth,
    log: &LogHealth,
    thresholds: &HealthThresholds,
) -> Vec<HealthConcern> {
    let mut concerns = Vec::new();

    // Busy is not unwell. The look at the log does not wait for the single
    // connection, so a server in the middle of serving answers "busy", and
    // treating that as unreachable would raise an alarm at exactly the moment
    // the server is doing its job.
    if !log.reachable && !log.busy {
        concerns.push(HealthConcern {
            name: LOG_UNREACHABLE,
            severity: HealthStatus::Unhealthy,
            detail: "the sync log did not answer when it was last looked at".to_owned(),
        });
    }

    if sync.requests >= thresholds.min_requests {
        let ratio = sync.failures as f64 / sync.requests as f64;
        if ratio >= thresholds.failure_ratio {
            concerns.push(HealthConcern {
                name: SYNC_FAILURE_RATE,
                severity: HealthStatus::Unhealthy,
                detail: format!(
                    "{} of the last {} sync requests could not be served ({:.0}%)",
                    sync.failures,
                    sync.requests,
                    ratio * 100.0
                ),
            });
        }
    }

    if sync.slowest_millis >= thresholds.slowest_millis {
        concerns.push(HealthConcern {
            name: SYNC_SLOW,
            severity: HealthStatus::Degraded,
            detail: format!(
                "the slowest sync request took {:.0} ms, of which {:.0} ms was spent waiting for \
                 the log",
                sync.slowest_millis, sync.lock_wait_slowest_millis
            ),
        });
    }

    if log.unreadable_rows.new_in_window > 0 {
        concerns.push(HealthConcern {
            name: UNREADABLE_ROWS_GROWING,
            severity: HealthStatus::Degraded,
            detail: format!(
                "{} more log rows became unreadable ({} in all); devices are not receiving them{}",
                log.unreadable_rows.new_in_window,
                log.unreadable_rows.rows,
                detail_of(&log.unreadable_rows.newest),
            ),
        });
    }

    if log.partly_read_rows.new_in_window > 0 {
        concerns.push(HealthConcern {
            name: PARTLY_READ_ROWS_GROWING,
            severity: HealthStatus::Degraded,
            detail: format!(
                "{} more log rows were served without fields this build cannot name ({} in all); \
                 a newer build wrote them{}",
                log.partly_read_rows.new_in_window,
                log.partly_read_rows.rows,
                detail_of(&log.partly_read_rows.newest),
            ),
        });
    }

    concerns
}

/// The worst of what is open, or `Ok` when nothing is.
pub fn overall_status(concerns: &[HealthConcern]) -> HealthStatus {
    if concerns
        .iter()
        .any(|concern| concern.severity == HealthStatus::Unhealthy)
    {
        return HealthStatus::Unhealthy;
    }
    if concerns.is_empty() {
        return HealthStatus::Ok;
    }
    HealthStatus::Degraded
}

fn detail_of(newest: &Option<String>) -> String {
    match newest {
        Some(newest) => format!("; newest: {newest}"),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use todo_contracts::RowFaults;

    use super::*;

    #[test]
    fn cursor_increments_without_wrapping() {
        assert_eq!(next_sync_cursor(41), Some(42));
        assert_eq!(next_sync_cursor(SyncCursor::MAX), None);
    }

    fn healthy_sync() -> SyncHealth {
        SyncHealth {
            window_seconds: 120,
            requests: 10,
            failures: 0,
            rejections: 0,
            average_millis: 2.0,
            slowest_millis: 8.0,
            lock_wait_slowest_millis: 1.0,
            requests_total: 100,
            failures_total: 0,
        }
    }

    fn no_faults() -> RowFaults {
        RowFaults {
            rows: 0,
            reads: 0,
            new_in_window: 0,
            first_seen_at: None,
            newest_seen_at: None,
            newest: None,
        }
    }

    fn healthy_log() -> LogHealth {
        LogHealth {
            reachable: true,
            busy: false,
            latest_revision: Some(41),
            unreadable_rows: no_faults(),
            partly_read_rows: no_faults(),
        }
    }

    fn names(concerns: &[HealthConcern]) -> Vec<&'static str> {
        concerns.iter().map(|concern| concern.name).collect()
    }

    #[test]
    fn a_server_doing_its_job_has_nothing_to_report() {
        let concerns = evaluate_health(
            &healthy_sync(),
            &healthy_log(),
            &HealthThresholds::default(),
        );

        assert!(concerns.is_empty());
        assert_eq!(overall_status(&concerns), HealthStatus::Ok);
    }

    #[test]
    fn a_log_that_is_merely_busy_is_not_an_alarm() {
        // The look does not wait for the connection, so a server in the middle
        // of serving reports itself busy. Alerting on that would fire hardest
        // exactly when the server is working.
        let log = LogHealth {
            reachable: false,
            busy: true,
            latest_revision: None,
            ..healthy_log()
        };

        let concerns = evaluate_health(&healthy_sync(), &log, &HealthThresholds::default());

        assert!(concerns.is_empty());
    }

    #[test]
    fn a_log_that_does_not_answer_is_unhealthy() {
        let log = LogHealth {
            reachable: false,
            busy: false,
            latest_revision: None,
            ..healthy_log()
        };

        let concerns = evaluate_health(&healthy_sync(), &log, &HealthThresholds::default());

        assert_eq!(names(&concerns), vec![LOG_UNREACHABLE]);
        assert_eq!(overall_status(&concerns), HealthStatus::Unhealthy);
    }

    #[test]
    fn one_failure_out_of_one_request_is_not_an_outage() {
        let sync = SyncHealth {
            requests: 1,
            failures: 1,
            ..healthy_sync()
        };

        let concerns = evaluate_health(&sync, &healthy_log(), &HealthThresholds::default());

        assert!(concerns.is_empty(), "{concerns:?}");
    }

    #[test]
    fn a_log_refusing_half_the_requests_is_unhealthy() {
        let sync = SyncHealth {
            requests: 8,
            failures: 4,
            ..healthy_sync()
        };

        let concerns = evaluate_health(&sync, &healthy_log(), &HealthThresholds::default());

        assert_eq!(names(&concerns), vec![SYNC_FAILURE_RATE]);
        assert!(concerns[0].detail.contains("4 of the last 8"));
        assert_eq!(overall_status(&concerns), HealthStatus::Unhealthy);
    }

    #[test]
    fn requests_a_device_sent_wrong_are_not_the_servers_failures() {
        // Rejections are the contract doing its job. Counting them as failures
        // would let one misbehaving device declare the server down.
        let sync = SyncHealth {
            requests: 8,
            failures: 0,
            rejections: 8,
            ..healthy_sync()
        };

        let concerns = evaluate_health(&sync, &healthy_log(), &HealthThresholds::default());

        assert!(concerns.is_empty(), "{concerns:?}");
    }

    #[test]
    fn a_slow_request_is_worth_a_look_but_not_an_outage() {
        let sync = SyncHealth {
            slowest_millis: 1_500.0,
            lock_wait_slowest_millis: 1_400.0,
            ..healthy_sync()
        };

        let concerns = evaluate_health(&sync, &healthy_log(), &HealthThresholds::default());

        assert_eq!(names(&concerns), vec![SYNC_SLOW]);
        assert_eq!(overall_status(&concerns), HealthStatus::Degraded);
        assert!(concerns[0].detail.contains("1400 ms"));
    }

    #[test]
    fn bad_rows_are_reported_while_they_are_still_turning_up() {
        let growing = LogHealth {
            unreadable_rows: RowFaults {
                rows: 3,
                reads: 12,
                new_in_window: 2,
                newest: Some("revision 9: title is not readable".to_owned()),
                ..no_faults()
            },
            ..healthy_log()
        };

        let concerns = evaluate_health(&healthy_sync(), &growing, &HealthThresholds::default());

        assert_eq!(names(&concerns), vec![UNREADABLE_ROWS_GROWING]);
        assert!(concerns[0].detail.contains("2 more"));
        assert!(concerns[0].detail.contains("3 in all"));
        assert!(concerns[0].detail.contains("revision 9"));
    }

    #[test]
    fn bad_rows_that_stopped_turning_up_clear_while_the_count_stays() {
        // The rows are still there and still counted, but nothing new has been
        // found: an alert that cannot clear is one nobody reads.
        let settled = LogHealth {
            unreadable_rows: RowFaults {
                rows: 3,
                reads: 40,
                new_in_window: 0,
                ..no_faults()
            },
            ..healthy_log()
        };

        let concerns = evaluate_health(&healthy_sync(), &settled, &HealthThresholds::default());

        assert!(concerns.is_empty());
    }

    #[test]
    fn rows_served_without_fields_this_build_cannot_name_are_reported_separately() {
        let rolled_back = LogHealth {
            partly_read_rows: RowFaults {
                rows: 5,
                reads: 5,
                new_in_window: 5,
                newest: Some("revision 12: dropped subtasks.0.note".to_owned()),
                ..no_faults()
            },
            ..healthy_log()
        };

        let concerns = evaluate_health(&healthy_sync(), &rolled_back, &HealthThresholds::default());

        assert_eq!(names(&concerns), vec![PARTLY_READ_ROWS_GROWING]);
        assert!(concerns[0].detail.contains("a newer build wrote them"));
    }

    #[test]
    fn the_worst_open_concern_is_the_one_reported() {
        let sync = SyncHealth {
            requests: 8,
            failures: 8,
            slowest_millis: 2_000.0,
            ..healthy_sync()
        };

        let concerns = evaluate_health(&sync, &healthy_log(), &HealthThresholds::default());

        assert_eq!(names(&concerns), vec![SYNC_FAILURE_RATE, SYNC_SLOW]);
        assert_eq!(overall_status(&concerns), HealthStatus::Unhealthy);
    }
}
