use std::{env, net::IpAddr, sync::Arc};

use axum::http::HeaderValue;
use todo_server::{create_router, SyncService};

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
    let listener = tokio::net::TcpListener::bind((host, port)).await?;
    let address = listener.local_addr()?;

    tracing::info!(%address, "todo Axum server listening");
    axum::serve(
        listener,
        create_router(Arc::new(SyncService::default()), cors_origin),
    )
    .await?;

    Ok(())
}
