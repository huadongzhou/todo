use std::{env, net::IpAddr, path::PathBuf, sync::Arc};

use axum::http::HeaderValue;
use todo_server::{create_router, SyncService, SyncStore};

/// Where the sync log lives when `DATABASE_PATH` says nothing. A single file
/// next to the process, so self-hosting is "run the binary".
const DEFAULT_DATABASE_PATH: &str = "todo-server.db";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "todo_server=info,tower_http=info".into()),
        )
        .init();

    let host = env::var("HOST")
        .unwrap_or_else(|_| "127.0.0.1".to_owned())
        .parse::<IpAddr>()?;
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_owned())
        .parse::<u16>()?;
    let cors_origin = env::var("CORS_ORIGIN")
        .ok()
        .map(|origin| HeaderValue::from_str(&origin))
        .transpose()?;
    // Opening the log is the one startup step with no useful degraded mode: a
    // server that cannot record operations would acknowledge them into nothing.
    let database_path = env::var("DATABASE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_DATABASE_PATH));
    let store = SyncStore::open(&database_path)?;
    tracing::info!(path = %database_path.display(), "sync log opened");

    let listener = tokio::net::TcpListener::bind((host, port)).await?;
    let address = listener.local_addr()?;

    tracing::info!(%address, "todo Axum server listening");
    axum::serve(
        listener,
        create_router(Arc::new(SyncService::new(store)), cors_origin),
    )
    .await?;

    Ok(())
}
