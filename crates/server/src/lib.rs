use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

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
    ContractValidationError, HealthResponse, SyncCursor, SyncRequest, SyncResponse, TodoSyncChange,
};
use todo_domain::next_sync_cursor;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Default)]
pub struct SyncService {
    state: Mutex<SyncState>,
}

#[derive(Default)]
struct SyncState {
    operation_ids: HashSet<String>,
    changes: Vec<TodoSyncChange>,
    cursor: SyncCursor,
}

impl SyncService {
    pub fn sync(&self, request: SyncRequest) -> Result<SyncResponse, ServerError> {
        request.validate().map_err(ServerError::InvalidRequest)?;

        let mut state = self
            .state
            .lock()
            .map_err(|_| ServerError::StateUnavailable)?;
        let mut acknowledged_operation_ids = Vec::with_capacity(request.operations.len());

        for operation in request.operations {
            if state.operation_ids.contains(&operation.operation_id) {
                acknowledged_operation_ids.push(operation.operation_id);
                continue;
            }

            state.cursor = next_sync_cursor(state.cursor).ok_or(ServerError::CursorExhausted)?;
            state.operation_ids.insert(operation.operation_id.clone());
            acknowledged_operation_ids.push(operation.operation_id.clone());
            let revision = state.cursor;
            state
                .changes
                .push(TodoSyncChange::from_operation(operation, revision));
        }

        let changes = state
            .changes
            .iter()
            .filter(|change| change.revision > request.cursor)
            .cloned()
            .collect();

        Ok(SyncResponse {
            acknowledged_operation_ids,
            changes,
            next_cursor: state.cursor,
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
    StateUnavailable,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::InvalidRequest(error) => (StatusCode::BAD_REQUEST, error.to_string()),
            Self::CursorExhausted | Self::StateUnavailable => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "sync state is unavailable".to_owned(),
            ),
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
    use todo_contracts::{SyncOperationKind, TodoPatch, TodoSyncOperation};
    use tower::ServiceExt;

    use super::*;

    fn request(operation_id: &str, cursor: SyncCursor) -> SyncRequest {
        SyncRequest {
            device_id: "desktop-1".to_owned(),
            cursor,
            operations: vec![TodoSyncOperation {
                operation_id: operation_id.to_owned(),
                todo_id: "todo-1".to_owned(),
                kind: SyncOperationKind::Upsert,
                occurred_at: "2026-07-16T00:00:00Z".to_owned(),
                patch: Some(TodoPatch {
                    title: Some("Ship Axum".to_owned()),
                    ..TodoPatch::default()
                }),
            }],
        }
    }

    #[test]
    fn sync_acknowledges_operations_and_advances_the_cursor() {
        let service = SyncService::default();
        let response = service
            .sync(request("operation-1", 0))
            .expect("sync succeeds");

        assert_eq!(response.acknowledged_operation_ids, ["operation-1"]);
        assert_eq!(response.next_cursor, 1);
        assert_eq!(response.changes.len(), 1);
    }

    #[test]
    fn replayed_operations_do_not_advance_the_cursor() {
        let service = SyncService::default();
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
        let app = create_router(Arc::new(SyncService::default()), None);
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
}
