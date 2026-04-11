pub mod config;
pub mod cmd;
pub mod error;
pub mod auth;
pub mod ai_config;
pub mod backup;

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
    /// Monotonic request counter for tracing
    pub request_count: std::sync::atomic::AtomicU64,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            request_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn next_request_id(&self) -> u64 {
        self.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
    }
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
    let has_token = config.api_token.is_some();

    let state = Arc::new(AppState::new(config));

    // API routes (require auth if token is set)
    let api_routes = Router::new()
        .route("/snapshot", get(system_snapshot::handle))
        .route("/logs", get(service_logs::handle))
        .route("/config", get(config_read::handle))
        .route("/package", get(package_info::handle))
        .route("/generations", get(generation_diff::handle))
        .route("/config/validate", post(config_validate::handle))
        .route("/config/apply", post(config_apply::handle))
        .route("/config/diff", post(config_diff::handle))
        .route("/rollback", post(rollback::handle))
        .with_state(state.clone());

    // Backup routes
    let api_routes = api_routes
        .route("/backups", get(backup::list_backups))
        .route("/backup/create", post(backup::create_backup))
        .route("/backup/restore", post(backup::restore_backup))
        .route("/backup/rotate", post(backup::rotate_backups));

    // AI config generation routes
    let api_routes = api_routes
        .route("/config/generate", post(ai_config::handle))
        .route("/config/test", post(config_test::handle))
        .route("/config/test/cancel", post(config_test::cancel_test));

    // Apply auth middleware to API routes if token is configured
    let api_routes = if has_token {
        api_routes.layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ))
    } else {
        api_routes
    };

    let app = Router::new()
        .nest("/api", api_routes)
        .route("/health", get(cmd::health_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    tracing::info!("nix-evo-agent listening on {addr}");
    if has_token {
        tracing::info!("API token authentication enabled");
    } else {
        tracing::warn!("No API token configured — all requests allowed");
    }

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
