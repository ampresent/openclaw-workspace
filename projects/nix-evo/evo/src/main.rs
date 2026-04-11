pub mod config;
pub mod cmd;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use config::Config;
use cmd::*;

/// Shared application state passed to all handlers
pub struct AppState {
    pub config: Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("nix_evo_agent=info,tower_http=info")),
        )
        .init();

    // CLI args
    let config = Config::from_args();
    let addr = config.bind_addr();

    let state = Arc::new(AppState { config });

    let app = Router::new()
        // Diagnostic endpoints
        .route("/api/snapshot", get(system_snapshot::handle))
        .route("/api/logs", get(service_logs::handle))
        .route("/api/config", get(config_read::handle))
        .route("/api/package", get(package_info::handle))
        .route("/api/generations", get(generation_diff::handle))
        // Action endpoints
        .route("/api/config/validate", post(config_validate::handle))
        .route("/api/config/apply", post(config_apply::handle))
        .route("/api/rollback", post(rollback::handle))
        // Health
        .route("/health", get(|| async { "ok" }))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("nix-evo-agent listening on {addr}");

    axum::serve(listener, app).await?;
    Ok(())
}
