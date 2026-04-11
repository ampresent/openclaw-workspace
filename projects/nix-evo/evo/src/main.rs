pub mod config;
pub mod cmd;
pub mod error;
pub mod auth;
pub mod conda;
pub mod conda_diag;
pub mod hybrid;
pub mod conda_lock;
pub mod venv_bridge;
pub mod env_sync;
pub mod env_test;
pub mod resolver;
pub mod build_cache;

use axum::{
    routing::{get, post, delete},
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
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("nix_evo_agent=info,tower_http=info")),
        )
        .init();

    let config = Config::from_args();
    let addr = config.bind_addr();
    let has_token = config.api_token.is_some();

    let state = Arc::new(AppState { config });

    // Core NixOS API routes
    let api_routes = Router::new()
        .route("/snapshot", get(system_snapshot::handle))
        .route("/logs", get(service_logs::handle))
        .route("/config", get(config_read::handle))
        .route("/package", get(package_info::handle))
        .route("/generations", get(generation_diff::handle))
        .route("/config/validate", post(config_validate::handle))
        .route("/config/apply", post(config_apply::handle))
        .route("/rollback", post(rollback::handle))
        // Conda environment management routes
        .route("/conda/envs", get(conda_handlers::list_envs_handler))
        .route("/conda/packages", get(conda_handlers::list_packages_handler))
        .route("/conda/create", post(conda_handlers::create_env_handler))
        .route("/conda/install", post(conda_handlers::install_handler))
        .route("/conda/remove", post(conda_handlers::remove_handler))
        .route("/conda/export", get(conda_handlers::export_handler))
        .route("/conda/create-from-yml", post(conda_handlers::create_from_yml_handler))
        .route("/conda/envs/:name", delete(conda_handlers::remove_env_handler))
        // Conda diagnostics routes
        .route("/conda/diag", get(conda_diag::diag_handler))
        .route("/conda/drift", get(conda_diag::drift_handler))
        // Hybrid NixOS+conda view
        .route("/hybrid/snapshot", get(hybrid::snapshot_handler))
        // Python virtual environment bridge
        .route("/python/envs", get(venv_bridge::list_python_envs_handler))
        // Environment sync engine
        .route("/env/sync", post(env_sync::sync_handler))
        .route("/env/export-all", post(env_sync::export_all_handler))
        // Environment testing framework
        .route("/env/test", post(env_test::test_handler))
        .route("/env/test/auto", post(env_test::auto_test_handler))
        // Package dependency resolver
        .route("/resolve/package/:name", get(resolver::resolve_handler))
        .route("/resolve/batch", post(resolver::batch_resolve_handler))
        // Build cache manager
        .route("/cache/status", get(build_cache::cache_status_handler))
        .route("/cache/clean", post(build_cache::cache_clean_handler))
        .route("/cache/mirror", post(build_cache::mirror_setup_handler))


        .route("/conda/lock", post(conda_lock::lock_handler))
        .with_state(state.clone());

    let api_routes = if has_token {
        api_routes.layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ))
    } else {
        api_routes
    };

    let app = Router::new()
        // Conda health dashboard
        .route("/dashboard/conda", get(|| async { axum::response::Html(include_str!("../static/conda-dashboard.html")) }))

        .nest("/api", api_routes)
        .route("/health", get(|| async { "ok" }))
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
