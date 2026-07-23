mod sync_store;

use std::sync::Arc;

use axum::{
    extract::State,
    http::{header::CONTENT_TYPE, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use todo_contracts::{
    ContractValidationError, HealthResponse, SyncRequest, SyncResponse, TodoSyncChange,
};
use todo_domain::next_sync_cursor;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub use sync_store::{SyncStore, SyncStoreError};

/// The op-based sync protocol, on top of the persisted log.
///
/// The service owns no state of its own: the operation ids already seen and the
/// revision last handed out are read back out of the log on every request, so
/// the answers a restarted process gives are the answers the previous one would
/// have given.
pub struct SyncService {
    store: SyncStore,
}

impl SyncService {
    pub fn new(store: SyncStore) -> Self {
        Self { store }
    }

    pub fn sync(&self, request: SyncRequest) -> Result<SyncResponse, ServerError> {
        request.validate().map_err(ServerError::InvalidRequest)?;

        let SyncRequest {
            cursor, operations, ..
        } = request;

        // Reading the current revision, appending and reading back what the
        // device is owed happen in one transaction: two requests numbering
        // operations from the same starting point would collide on a revision.
        let (acknowledged_operation_ids, changes, next_cursor) =
            self.store.with_log(move |log| {
                let mut revision = log.latest_revision()?;
                let mut acknowledged_operation_ids = Vec::with_capacity(operations.len());

                for operation in operations {
                    if log.contains_operation(&operation.operation_id)? {
                        acknowledged_operation_ids.push(operation.operation_id);
                        continue;
                    }

                    revision = next_sync_cursor(revision).ok_or(ServerError::CursorExhausted)?;
                    acknowledged_operation_ids.push(operation.operation_id.clone());
                    log.append(&TodoSyncChange::from_operation(operation, revision))?;
                }

                let changes = log.changes_since(cursor)?;
                Ok::<_, ServerError>((acknowledged_operation_ids, changes, revision))
            })?;

        Ok(SyncResponse {
            acknowledged_operation_ids,
            changes,
            next_cursor,
            server_time: now_rfc3339(),
        })
    }
}

#[derive(Clone)]
struct AppState {
    sync_service: Arc<SyncService>,
}

pub fn create_router(sync_service: Arc<SyncService>, cors_origin: Option<HeaderValue>) -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([CONTENT_TYPE]);
    let cors = match cors_origin {
        Some(origin) => cors.allow_origin(origin),
        None => cors,
    };

    Router::new()
        .route("/health", get(health))
        .route("/v1/sync", post(sync))
        .with_state(AppState { sync_service })
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_owned(),
        service: "todo-api".to_owned(),
        time: now_rfc3339(),
    })
}

async fn sync(
    State(state): State<AppState>,
    Json(request): Json<SyncRequest>,
) -> Result<Json<SyncResponse>, ServerError> {
    state.sync_service.sync(request).map(Json)
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("the RFC 3339 formatter is always valid")
}

#[derive(Debug)]
pub enum ServerError {
    CursorExhausted,
    InvalidRequest(ContractValidationError),
    Storage(SyncStoreError),
}

impl From<SyncStoreError> for ServerError {
    fn from(error: SyncStoreError) -> Self {
        Self::Storage(error)
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::InvalidRequest(error) => (StatusCode::BAD_REQUEST, error.to_string()),
            // The device keeps the operations it could not get acknowledged, so
            // the reason stays in the log rather than going out on the wire.
            error @ (Self::CursorExhausted | Self::Storage(_)) => {
                tracing::error!(?error, "the sync log could not serve the request");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "sync state is unavailable".to_owned(),
                )
            }
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use todo_contracts::{SyncCursor, SyncOperationKind, TodoPatch, TodoStatus, TodoSyncOperation};
    use tower::ServiceExt;

    use super::*;

    fn service() -> SyncService {
        SyncService::new(SyncStore::in_memory().expect("open an in-memory log"))
    }

    fn upsert(operation_id: &str, patch: TodoPatch) -> TodoSyncOperation {
        TodoSyncOperation {
            operation_id: operation_id.to_owned(),
            todo_id: "todo-1".to_owned(),
            kind: SyncOperationKind::Upsert,
            occurred_at: "2026-07-16T00:00:00Z".to_owned(),
            patch: Some(patch),
        }
    }

    fn request(operation_id: &str, cursor: SyncCursor) -> SyncRequest {
        SyncRequest {
            device_id: "desktop-1".to_owned(),
            cursor,
            operations: vec![upsert(
                operation_id,
                TodoPatch {
                    title: Some("Ship Axum".to_owned()),
                    ..TodoPatch::default()
                },
            )],
        }
    }

    /// A file path no other test shares, cleaned up with its WAL sidecars.
    struct TempDatabase {
        path: std::path::PathBuf,
    }

    impl TempDatabase {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("todo-server-{name}-{}.sqlite3", std::process::id()));
            let database = Self { path };
            database.remove();
            database
        }

        fn remove(&self) {
            for suffix in ["", "-wal", "-shm"] {
                let mut path = self.path.clone().into_os_string();
                path.push(suffix);
                let _ = std::fs::remove_file(path);
            }
        }

        /// Stands in for a server process: a service over the same file, with
        /// nothing carried over from the last one but the file itself.
        fn restart(&self) -> SyncService {
            SyncService::new(SyncStore::open(&self.path).expect("open the sync log"))
        }
    }

    impl Drop for TempDatabase {
        fn drop(&mut self) {
            self.remove();
        }
    }

    #[test]
    fn sync_acknowledges_operations_and_advances_the_cursor() {
        let service = service();
        let response = service
            .sync(request("operation-1", 0))
            .expect("sync succeeds");

        assert_eq!(response.acknowledged_operation_ids, ["operation-1"]);
        assert_eq!(response.next_cursor, 1);
        assert_eq!(response.changes.len(), 1);
    }

    #[test]
    fn replayed_operations_do_not_advance_the_cursor() {
        let service = service();
        service
            .sync(request("operation-1", 0))
            .expect("first sync succeeds");
        let response = service
            .sync(request("operation-1", 1))
            .expect("replay succeeds");

        assert_eq!(response.next_cursor, 1);
        assert!(response.changes.is_empty());
    }

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let app = create_router(Arc::new(service()), None);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("health response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn the_log_a_restarted_server_serves_is_the_one_it_stopped_with() {
        let database = TempDatabase::new("restart");

        let before = database
            .restart()
            .sync(request("operation-1", 0))
            .expect("first sync succeeds");
        assert_eq!(before.next_cursor, 1);

        // A device that has pulled nothing yet asks the restarted process for
        // everything: the answer has to be the one the stopped process would
        // have given, log and cursor alike.
        let after = database
            .restart()
            .sync(SyncRequest {
                device_id: "desktop-2".to_owned(),
                cursor: 0,
                operations: Vec::new(),
            })
            .expect("sync after the restart succeeds");

        assert_eq!(after.next_cursor, 1);
        assert_eq!(after.changes.len(), 1);
        assert_eq!(after.changes[0].operation_id, "operation-1");
        assert_eq!(after.changes[0].revision, 1);
        assert_eq!(
            after.changes[0]
                .patch
                .as_ref()
                .and_then(|patch| patch.title.as_deref()),
            Some("Ship Axum")
        );
    }

    #[test]
    fn an_offline_backlog_pushed_after_a_restart_is_recorded_once() {
        let database = TempDatabase::new("backlog");

        // The device was offline for three edits and pushes them in one go.
        let backlog = SyncRequest {
            device_id: "desktop-1".to_owned(),
            cursor: 0,
            operations: vec![
                upsert(
                    "operation-1",
                    TodoPatch {
                        title: Some("first".to_owned()),
                        ..TodoPatch::default()
                    },
                ),
                upsert(
                    "operation-2",
                    TodoPatch {
                        title: Some("second".to_owned()),
                        ..TodoPatch::default()
                    },
                ),
                TodoSyncOperation {
                    operation_id: "operation-3".to_owned(),
                    todo_id: "todo-2".to_owned(),
                    kind: SyncOperationKind::Delete,
                    occurred_at: "2026-07-16T00:00:03Z".to_owned(),
                    patch: None,
                },
            ],
        };

        let pushed = database
            .restart()
            .sync(backlog.clone())
            .expect("the backlog is accepted");
        assert_eq!(pushed.acknowledged_operation_ids.len(), 3);
        assert_eq!(pushed.next_cursor, 3);

        // The response never reached the device (or the process died right
        // after committing), so the same batch is pushed again to a restarted
        // server. It must be acknowledged without being recorded twice.
        let replayed = database
            .restart()
            .sync(backlog)
            .expect("the replayed backlog is accepted");

        assert_eq!(
            replayed.acknowledged_operation_ids,
            ["operation-1", "operation-2", "operation-3"]
        );
        assert_eq!(replayed.next_cursor, 3);
        assert_eq!(replayed.changes.len(), 3);
        assert!(matches!(
            replayed.changes[2].kind,
            SyncOperationKind::Delete
        ));
        assert!(replayed.changes[2].patch.is_none());
    }

    #[test]
    fn field_level_patches_reach_a_restarted_server_intact() {
        let database = TempDatabase::new("fields");

        // Two devices touch the same todo, each carrying only the fields it
        // changed. Server-wins field-level merging happens on the device, and
        // it can only be right if the log hands back the field sets it was
        // given: a field the operation did not carry must still be absent.
        database
            .restart()
            .sync(SyncRequest {
                device_id: "desktop-1".to_owned(),
                cursor: 0,
                operations: vec![
                    upsert(
                        "operation-1",
                        TodoPatch {
                            title: Some("renamed on the desktop".to_owned()),
                            ..TodoPatch::default()
                        },
                    ),
                    upsert(
                        "operation-2",
                        TodoPatch {
                            status: Some(TodoStatus::Completed),
                            completed_at: Some("2026-07-16T09:00:00Z".to_owned()),
                            ..TodoPatch::default()
                        },
                    ),
                ],
            })
            .expect("both operations are accepted");

        let pulled = database
            .restart()
            .sync(SyncRequest {
                device_id: "phone-1".to_owned(),
                cursor: 0,
                operations: Vec::new(),
            })
            .expect("the other device pulls");

        let first = pulled.changes[0].patch.as_ref().expect("a rename patch");
        assert_eq!(first.title.as_deref(), Some("renamed on the desktop"));
        assert!(first.status.is_none());
        assert!(first.completed_at.is_none());

        let second = pulled.changes[1]
            .patch
            .as_ref()
            .expect("a completion patch");
        assert!(matches!(second.status, Some(TodoStatus::Completed)));
        assert_eq!(second.completed_at.as_deref(), Some("2026-07-16T09:00:00Z"));
        assert!(second.title.is_none());
        // Revision order is what the device applies them in, so the later
        // operation must still come last.
        assert!(pulled.changes[0].revision < pulled.changes[1].revision);
    }

    #[test]
    fn a_rejected_request_leaves_nothing_behind() {
        let database = TempDatabase::new("rejected");
        let service = database.restart();
        service
            .sync(request("operation-1", 0))
            .expect("the good operation is accepted");

        // One unacceptable operation in the batch fails the whole request, so
        // the acceptable one next to it must not be half-recorded either.
        let outcome = service.sync(SyncRequest {
            device_id: "desktop-1".to_owned(),
            cursor: 0,
            operations: vec![
                upsert(
                    "operation-2",
                    TodoPatch {
                        title: Some("fine".to_owned()),
                        ..TodoPatch::default()
                    },
                ),
                upsert(
                    "operation-3",
                    TodoPatch {
                        title: Some("   ".to_owned()),
                        ..TodoPatch::default()
                    },
                ),
            ],
        });
        assert!(matches!(outcome, Err(ServerError::InvalidRequest(_))));

        let after = database
            .restart()
            .sync(SyncRequest {
                device_id: "desktop-1".to_owned(),
                cursor: 0,
                operations: Vec::new(),
            })
            .expect("sync after the rejection succeeds");
        assert_eq!(after.next_cursor, 1);
        assert_eq!(after.changes.len(), 1);
        assert_eq!(after.changes[0].operation_id, "operation-1");
    }
}
