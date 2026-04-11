pub mod config;
pub mod cmd;
pub mod error;
pub mod auth;
pub mod dashboard;
pub mod audit;
pub mod healer;
pub mod flake;

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
    let has_token = config.api_token.is_some();

    let state = Arc::new(AppState { config });

    // Start self-healer background task
    healer::start_background_task();
    tracing::info!("Self-healer background task started");

    // API routes (require auth if token is set)
    let api_routes = Router::new()
        // Original v0.1 endpoints
        .route("/snapshot", get(system_snapshot::handle))
        .route("/logs", get(service_logs::handle))
        .route("/config", get(config_read::handle))
        .route("/package", get(package_info::handle))
        .route("/generations", get(generation_diff::handle))
        .route("/config/validate", post(config_validate::handle))
        .route("/config/apply", post(config_apply::handle))
        .route("/rollback", post(rollback::handle))
        // Experimental: WebSocket dashboard
        .route("/dashboard/ws", get(dashboard::handle))
        // Experimental: Audit trail
        .route("/audit", get(audit::handle_query))
        .route("/audit/stats", get(audit::handle_stats))
        // Experimental: Self-healer status
        .route("/healer/status", get(healer::handle_status))
        // Experimental: Flake converter
        .route("/flake/convert", post(flake::handle_convert))
        .with_state(state.clone());

    // Apply auth middleware to API routes if token is configured
    let api_routes = if has_token {
        api_routes.layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ))
    } else {
        api_routes
    };

    // Static files for dashboard
    let static_routes = Router::new()
        .route("/dashboard", get(serve_dashboard_html))
        .with_state(state.clone());

    let app = Router::new()
        .nest("/api", api_routes)
        .merge(static_routes)
        .route("/health", get(|| async { "ok" }))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    tracing::info!("nix-evo-agent listening on {addr}");
    tracing::info!("Dashboard available at http://{addr}/dashboard");
    if has_token {
        tracing::info!("API token authentication enabled");
    } else {
        tracing::warn!("No API token configured — all requests allowed");
    }

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Serve the dashboard HTML page
async fn serve_dashboard_html() -> axum::response::Html<String> {
    let html = include_str!("../static/dashboard.html");
    axum::response::Html(html.to_string())
}
