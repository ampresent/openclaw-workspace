pub mod config;
pub mod cmd;
pub mod error;
pub mod auth;
pub mod dashboard;
pub mod audit;
pub mod healer;
pub mod flake;
pub mod cluster;
pub mod marketplace;
pub mod deps;
pub mod timeline;
pub mod advisor;
pub mod metrics;
pub mod nix_eval;
pub mod i18n;
pub mod security;
pub mod config_builder;
pub mod capacity;
pub mod gitops;
pub mod plugin;
pub mod health_score;
pub mod compat;
pub mod stream;
pub mod predict;
pub mod composer;
pub mod doctor;
pub mod dna;
pub mod theater;
pub mod chain;
pub mod collab;
pub mod bench;
pub mod topology;
pub mod timetravel;
pub mod chaos;
pub mod patterns;

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

    // Start config file stream watcher
    stream::start_file_watcher();
    tracing::info!("Config file stream watcher started");

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
        // Experimental v2: Multi-Cluster Orchestrator
        .route("/cluster/deploy", post(cluster::handle_deploy))
        .route("/cluster/status", get(cluster::handle_status))
        .route("/cluster/nodes", post(cluster::handle_add_node))
        .route("/cluster/nodes", get(cluster::handle_remove_node))
        // Experimental v2: Marketplace Browser
        .route("/marketplace/search", get(marketplace::handle_search))
        .route("/marketplace/info", get(marketplace::handle_info))
        // Experimental v2: Config Dependency Graph
        .route("/deps/graph", get(deps::handle_graph))
        .route("/deps/graph/analyze", post(deps::handle_analyze))
        // Experimental v2: Generation Timeline
        .route("/timeline", get(timeline::handle_timeline))
        .route("/timeline/compare", get(timeline::handle_compare))
        // Experimental v2: Smart Rollback Advisor
        .route("/advisor/recommend", post(advisor::handle_recommend))
        .route("/advisor/status", get(advisor::handle_status))
        // Experimental v3: Nix Expression Interpreter
        .route("/nix/eval", get(nix_eval::handle_eval))
        .route("/nix/check", get(nix_eval::handle_check))
        // Experimental v3: Multi-Language Support
        .route("/i18n/translate", get(i18n::handle_translate))
        .route("/i18n/languages", get(i18n::handle_languages))
        // Experimental v3: Security Scanner
        .route("/security/scan", get(security::handle_scan))
        .route("/security/score", get(security::handle_score))
        // Experimental v3: Interactive Config Builder (WebSocket)
        .route("/config-builder/ws", get(config_builder::handle_ws))
        // Experimental v3: Capacity Planning
        .route("/capacity/forecast", get(capacity::handle_forecast))
        // Experimental v3: GitOps Bridge
        .route("/gitops/status", get(gitops::handle_status))
        .route("/gitops/configure", post(gitops::handle_configure))
        .route("/gitops/deploy", post(gitops::handle_deploy))
        .route("/gitops/webhook", post(gitops::handle_webhook))
        // Experimental v3: Plugin System
        .route("/plugins", get(plugin::handle_list))
        .route("/plugins/health", get(plugin::handle_health))
        // Experimental v4: AI-Powered Nix Doctor
        .route("/doctor/diagnose", post(doctor::handle_diagnose))
        .route("/doctor/knowledge", get(doctor::handle_knowledge))
        // Experimental v4: Service Orchestration Composer
        .route("/compose", post(composer::handle_compose))
        .route("/compose/status", get(composer::handle_status))
        // Experimental v4: Predictive Failure Detection
        .route("/predict/alerts", get(predict::handle_alerts))
        // Experimental v4: Cross-Distro Compatibility
        .route("/compat/translate", post(compat::handle_translate))
        // Experimental v4: System Health Score
        .route("/health/score", get(health_score::handle_score))
        // Experimental v4: Config Streaming (WebSocket)
        .route("/stream/config", get(stream::handle_ws))
        // Experimental v5: Nix Config DNA — Genetic Optimization
        .route("/dna/evolve", post(dna::handle_evolve))
        .route("/dna/population", get(dna::handle_population))
        // Experimental v5: Config Theater — Replay & Undo
        .route("/theater/record", post(theater::handle_record))
        .route("/theater/replay", get(theater::handle_replay))
        .route("/theater/undo", post(theater::handle_undo))
        .route("/theater/branch", post(theater::handle_branch))
        .route("/theater/branches", get(theater::handle_branches))
        // Experimental v5: Config Blockchain
        .route("/chain/verify", get(chain::handle_verify))
        .route("/chain/history", get(chain::handle_history))
        .route("/chain/add", post(chain::handle_add_block))
        // Experimental v5: Collaborative Config Editing (WebSocket)
        .route("/collab/ws", get(collab::handle_ws))
        // Experimental v5: Config Benchmarking Suite
        .route("/bench/run", post(bench::handle_run))
        .route("/bench/results", get(bench::handle_results))
        .route("/bench/compare", get(bench::handle_compare))
        // Experimental v5: NixOS Topology Map
        .route("/topology", get(topology::handle_topology))
        .route("/topology/services", get(topology::handle_services))
        .route("/topology/connections", get(topology::handle_connections))
        // Experimental v6: Time-Travel Debugging
        .route("/timetravel/snapshot", post(timetravel::handle_snapshot))
        .route("/timetravel/snapshots", get(timetravel::handle_list))
        .route("/timetravel/diff", get(timetravel::handle_diff))
        .route("/timetravel/replay", get(timetravel::handle_replay))
        // Experimental v6: Chaos Engineering
        .route("/chaos/scenarios", get(chaos::handle_scenarios))
        .route("/chaos/run", post(chaos::handle_run))
        .route("/chaos/start", post(chaos::handle_start))
        .route("/chaos/status", get(chaos::handle_chaos_status))
        .route("/chaos/report", get(chaos::handle_report))
        // Experimental v6: Nix Config Pattern Library
        .route("/patterns", get(patterns::handle_list))
        .route("/patterns/:id", get(patterns::handle_get))
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

    // Static files
    let static_routes = Router::new()
        .route("/dashboard", get(serve_dashboard_html))
        .route("/deps", get(serve_deps_html))
        .route("/timeline", get(serve_timeline_html))
        .route("/security", get(serve_security_html))
        .route("/builder", get(serve_builder_html))
        .route("/doctor", get(serve_doctor_html))
        .route("/composer", get(serve_composer_html))
        .route("/health", get(serve_health_html))
        .route("/topology", get(serve_topology_html))
        .route("/chaos", get(serve_chaos_html))
        .with_state(state.clone());

    let app = Router::new()
        .nest("/api", api_routes)
        .merge(static_routes)
        .route("/health", get(|| async { "ok" }))
        .route("/metrics", get(metrics::handle_metrics))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    tracing::info!("nix-evo-agent listening on {addr}");
    tracing::info!("Dashboard: http://{addr}/dashboard");
    tracing::info!("Security:  http://{addr}/security");
    tracing::info!("Builder:   http://{addr}/builder");
    tracing::info!("Deps:      http://{addr}/deps");
    tracing::info!("Timeline:  http://{addr}/timeline");
    tracing::info!("Doctor:    http://{addr}/doctor");
    tracing::info!("Composer:  http://{addr}/composer");
    tracing::info!("Health:    http://{addr}/health");
    tracing::info!("Topology:  http://{addr}/topology");
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

/// Serve the deps graph HTML page
async fn serve_deps_html() -> axum::response::Html<String> {
    let html = include_str!("../static/deps.html");
    axum::response::Html(html.to_string())
}

/// Serve the timeline HTML page
async fn serve_timeline_html() -> axum::response::Html<String> {
    let html = include_str!("../static/timeline.html");
    axum::response::Html(html.to_string())
}

/// Serve the security scanner HTML page
async fn serve_security_html() -> axum::response::Html<String> {
    let html = include_str!("../static/security.html");
    axum::response::Html(html.to_string())
}

/// Serve the config builder HTML page
async fn serve_builder_html() -> axum::response::Html<String> {
    let html = include_str!("../static/builder.html");
    axum::response::Html(html.to_string())
}

/// Serve the doctor HTML page
async fn serve_doctor_html() -> axum::response::Html<String> {
    let html = include_str!("../static/doctor.html");
    axum::response::Html(html.to_string())
}

/// Serve the composer HTML page
async fn serve_composer_html() -> axum::response::Html<String> {
    let html = include_str!("../static/composer.html");
    axum::response::Html(html.to_string())
}

/// Serve the health score HTML page
async fn serve_health_html() -> axum::response::Html<String> {
    let html = include_str!("../static/health.html");
    axum::response::Html(html.to_string())
}
/// Serve the topology HTML page
async fn serve_topology_html() -> axum::response::Html<String> {
    let html = include_str!("../static/topology.html");
    axum::response::Html(html.to_string())
}

/// Serve the chaos engineering HTML page
async fn serve_chaos_html() -> axum::response::Html<String> {
    let html = include_str!("../static/chaos.html");
    axum::response::Html(html.to_string())
}
