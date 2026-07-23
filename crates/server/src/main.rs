use std::{
    env,
    net::IpAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use axum::http::HeaderValue;
use todo_server::{
    backup::{
        self, BackupPolicy, DEFAULT_BACKUP_DIRECTORY, DEFAULT_INTERVAL_SECONDS, DEFAULT_KEEP,
    },
    create_router,
    health::{self, HealthService, DEFAULT_INTERVAL_SECONDS as DEFAULT_HEALTH_INTERVAL_SECONDS},
    SyncService, SyncStore,
};

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

    // Opening the log is the one startup step with no useful degraded mode: a
    // server that cannot record operations would acknowledge them into nothing.
    let database_path = env::var("DATABASE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_DATABASE_PATH));

    // `todo-server restore <snapshot>` puts a snapshot in place and exits, so
    // an operator runs it with the server stopped — which is also what keeps
    // the restore clear of the one-connection rule the running server rests on.
    let mut arguments = env::args().skip(1);
    if let Some(command) = arguments.next() {
        if command != "restore" {
            return Err(format!(
                "unknown command {command:?}; usage: todo-server [restore <snapshot>]"
            )
            .into());
        }
        let snapshot = arguments
            .next()
            .ok_or("usage: todo-server restore <snapshot>")?;
        let restored = backup::restore(Path::new(&snapshot), &database_path)?;
        let preserved = restored
            .preserved
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "nothing was in place".to_owned());
        tracing::info!(
            snapshot = %snapshot,
            path = %database_path.display(),
            latest_revision = restored.latest_revision,
            preserved = %preserved,
            "sync log restored from a snapshot"
        );
        // Two things an operator would otherwise only find out about later, and
        // one of them by losing changes. A restored log carries on numbering
        // from where the snapshot stopped, so revisions past it are handed out
        // a second time — to a different operation. A device whose cursor is
        // already past that point asks for "everything after" a number the log
        // has not reached again and is told there is nothing, for good.
        tracing::warn!(
            latest_revision = restored.latest_revision,
            "devices must be reset to cursor 0 and pull the log again: revisions past this one \
             were handed out before and will be handed out again"
        );
        tracing::warn!(
            preserved = %preserved,
            "what was in place is kept under this name and nothing removes it later; clearing it \
             once the restore is confirmed is the operator's call"
        );
        return Ok(());
    }

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
    // A mistyped backup setting is a mistake to hear about at startup, not one
    // to paper over with the default: the whole point of the schedule is that
    // it runs the way the operator asked.
    let backup_directory = env::var("BACKUP_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_BACKUP_DIRECTORY));
    let backup_interval_seconds = env::var("BACKUP_INTERVAL_SECONDS")
        .ok()
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(DEFAULT_INTERVAL_SECONDS);
    let backup_keep = env::var("BACKUP_KEEP")
        .ok()
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(DEFAULT_KEEP);
    let health_interval_seconds = env::var("HEALTH_INTERVAL_SECONDS")
        .ok()
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(DEFAULT_HEALTH_INTERVAL_SECONDS);

    let store = SyncStore::open(&database_path)?;
    tracing::info!(path = %database_path.display(), "sync log opened");

    let sync_service = Arc::new(SyncService::new(store));

    if backup_interval_seconds == 0 {
        // Explicitly asked for, so it is said out loud rather than left to be
        // discovered when a snapshot is needed and there is none.
        tracing::warn!("BACKUP_INTERVAL_SECONDS is 0: no snapshots will be taken");
    } else {
        backup::spawn_scheduler(
            Arc::clone(&sync_service),
            BackupPolicy {
                directory: backup_directory.clone(),
                interval: Duration::from_secs(backup_interval_seconds),
                keep: backup_keep,
            },
        );
        tracing::info!(
            directory = %backup_directory.display(),
            interval_seconds = backup_interval_seconds,
            keep = backup_keep,
            "sync log snapshot schedule started"
        );
    }

    // The watch is what makes an alert reach a self-hosted server's operator
    // without anything else installed: the log line comes out whether or not
    // `/v1/health` is ever asked.
    let health_service = Arc::new(HealthService::new(Arc::clone(&sync_service)));
    if health_interval_seconds == 0 {
        tracing::warn!(
            "HEALTH_INTERVAL_SECONDS is 0: alerts will only be noticed by whoever asks /v1/health"
        );
    } else {
        health::spawn_watch(
            Arc::clone(&health_service),
            Duration::from_secs(health_interval_seconds),
        );
        tracing::info!(
            interval_seconds = health_interval_seconds,
            "health watch started"
        );
    }

    let listener = tokio::net::TcpListener::bind((host, port)).await?;
    let address = listener.local_addr()?;

    tracing::info!(%address, "todo Axum server listening");
    axum::serve(
        listener,
        create_router(sync_service, health_service, cors_origin),
    )
    .await?;

    Ok(())
}
