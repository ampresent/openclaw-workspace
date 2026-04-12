use axum::response::IntoResponse;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;
use tokio::sync::RwLock;

/// Global metrics collector
pub struct Metrics {
    /// Total API requests
    pub api_requests_total: AtomicU64,
    /// API requests by method+path
    pub api_requests_by_path: RwLock<std::collections::HashMap<String, AtomicU64>>,
    /// Total API errors
    pub api_errors_total: AtomicU64,
    /// Response time histogram buckets (in ms)
    pub response_time_buckets: RwLock<ResponseTimeBuckets>,
    /// Generation count (updated on boot)
    pub generation_count: AtomicU64,
    /// Healer actions total
    pub healer_actions_total: AtomicU64,
    /// Healer actions by service
    pub healer_actions_by_service: RwLock<std::collections::HashMap<String, AtomicU64>>,
    /// Active WebSocket connections
    pub ws_connections_active: AtomicU64,
    /// Cluster node count
    pub cluster_nodes_total: AtomicU64,
    /// Cluster deploy total
    pub cluster_deploys_total: AtomicU64,
    /// Cluster deploy failures
    pub cluster_deploy_failures_total: AtomicU64,
    /// Audit log entries
    pub audit_entries_total: AtomicU64,
    /// Last scrape time
    pub last_scrape: RwLock<Option<Instant>>,
}

#[derive(Debug)]
pub struct ResponseTimeBuckets {
    pub le_10ms: AtomicU64,
    pub le_50ms: AtomicU64,
    pub le_100ms: AtomicU64,
    pub le_250ms: AtomicU64,
    pub le_500ms: AtomicU64,
    pub le_1000ms: AtomicU64,
    pub le_5000ms: AtomicU64,
    pub le_inf: AtomicU64,
    pub sum: AtomicU64,     // sum of all durations in ms
    pub count: AtomicU64,   // total observations
}

impl Default for ResponseTimeBuckets {
    fn default() -> Self {
        Self {
            le_10ms: AtomicU64::new(0),
            le_50ms: AtomicU64::new(0),
            le_100ms: AtomicU64::new(0),
            le_250ms: AtomicU64::new(0),
            le_500ms: AtomicU64::new(0),
            le_1000ms: AtomicU64::new(0),
            le_5000ms: AtomicU64::new(0),
            le_inf: AtomicU64::new(0),
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            api_requests_total: AtomicU64::new(0),
            api_requests_by_path: RwLock::new(std::collections::HashMap::new()),
            api_errors_total: AtomicU64::new(0),
            response_time_buckets: RwLock::new(ResponseTimeBuckets::default()),
            generation_count: AtomicU64::new(0),
            healer_actions_total: AtomicU64::new(0),
            healer_actions_by_service: RwLock::new(std::collections::HashMap::new()),
            ws_connections_active: AtomicU64::new(0),
            cluster_nodes_total: AtomicU64::new(0),
            cluster_deploys_total: AtomicU64::new(0),
            cluster_deploy_failures_total: AtomicU64::new(0),
            audit_entries_total: AtomicU64::new(0),
            last_scrape: RwLock::new(None),
        }
    }

    /// Record an API request
    pub async fn record_request(&self, path: &str, method: &str, duration_ms: u64, is_error: bool) {
        self.api_requests_total.fetch_add(1, Ordering::Relaxed);
        if is_error {
            self.api_errors_total.fetch_add(1, Ordering::Relaxed);
        }

        let key = format!("{method} {path}");
        {
            let mut by_path = self.api_requests_by_path.write().await;
            by_path.entry(key).or_insert_with(|| AtomicU64::new(0)).fetch_add(1, Ordering::Relaxed);
        }

        // Record response time
        {
            let buckets = self.response_time_buckets.write().await;
            buckets.count.fetch_add(1, Ordering::Relaxed);
            buckets.sum.fetch_add(duration_ms, Ordering::Relaxed);
            buckets.le_inf.fetch_add(1, Ordering::Relaxed);
            if duration_ms <= 10 { buckets.le_10ms.fetch_add(1, Ordering::Relaxed); }
            if duration_ms <= 50 { buckets.le_50ms.fetch_add(1, Ordering::Relaxed); }
            if duration_ms <= 100 { buckets.le_100ms.fetch_add(1, Ordering::Relaxed); }
            if duration_ms <= 250 { buckets.le_250ms.fetch_add(1, Ordering::Relaxed); }
            if duration_ms <= 500 { buckets.le_500ms.fetch_add(1, Ordering::Relaxed); }
            if duration_ms <= 1000 { buckets.le_1000ms.fetch_add(1, Ordering::Relaxed); }
            if duration_ms <= 5000 { buckets.le_5000ms.fetch_add(1, Ordering::Relaxed); }
        }
    }

    /// Record a healer action
    pub async fn record_healer_action(&self, service: &str) {
        self.healer_actions_total.fetch_add(1, Ordering::Relaxed);
        let mut by_svc = self.healer_actions_by_service.write().await;
        by_svc.entry(service.to_string()).or_insert_with(|| AtomicU64::new(0)).fetch_add(1, Ordering::Relaxed);
    }

    /// Set generation count
    pub fn set_generation_count(&self, count: u64) {
        self.generation_count.store(count, Ordering::Relaxed);
    }

    /// Render Prometheus text format
    pub async fn render_prometheus(&self) -> String {
        let mut out = String::new();

        // API requests total
        out.push_str("# HELP nix_evo_api_requests_total Total number of API requests\n");
        out.push_str("# TYPE nix_evo_api_requests_total counter\n");
        out.push_str(&format!("nix_evo_api_requests_total {}\n", self.api_requests_total.load(Ordering::Relaxed)));
        out.push('\n');

        // API requests by path
        out.push_str("# HELP nix_evo_api_requests_by_path API requests by method and path\n");
        out.push_str("# TYPE nix_evo_api_requests_by_path counter\n");
        let by_path = self.api_requests_by_path.read().await;
        for (path, count) in by_path.iter() {
            out.push_str(&format!("nix_evo_api_requests_by_path{{path=\"{}\"}} {}\n", path, count.load(Ordering::Relaxed)));
        }
        out.push('\n');
        drop(by_path);

        // API errors
        out.push_str("# HELP nix_evo_api_errors_total Total number of API errors\n");
        out.push_str("# TYPE nix_evo_api_errors_total counter\n");
        out.push_str(&format!("nix_evo_api_errors_total {}\n", self.api_errors_total.load(Ordering::Relaxed)));
        out.push('\n');

        // Response time histogram
        let buckets = self.response_time_buckets.read().await;
        out.push_str("# HELP nix_evo_api_response_duration_ms API response duration in milliseconds\n");
        out.push_str("# TYPE nix_evo_api_response_duration_ms histogram\n");
        out.push_str(&format!("nix_evo_api_response_duration_ms_bucket{{le=\"10\"}} {}\n", buckets.le_10ms.load(Ordering::Relaxed)));
        out.push_str(&format!("nix_evo_api_response_duration_ms_bucket{{le=\"50\"}} {}\n", buckets.le_50ms.load(Ordering::Relaxed)));
        out.push_str(&format!("nix_evo_api_response_duration_ms_bucket{{le=\"100\"}} {}\n", buckets.le_100ms.load(Ordering::Relaxed)));
        out.push_str(&format!("nix_evo_api_response_duration_ms_bucket{{le=\"250\"}} {}\n", buckets.le_250ms.load(Ordering::Relaxed)));
        out.push_str(&format!("nix_evo_api_response_duration_ms_bucket{{le=\"500\"}} {}\n", buckets.le_500ms.load(Ordering::Relaxed)));
        out.push_str(&format!("nix_evo_api_response_duration_ms_bucket{{le=\"1000\"}} {}\n", buckets.le_1000ms.load(Ordering::Relaxed)));
        out.push_str(&format!("nix_evo_api_response_duration_ms_bucket{{le=\"5000\"}} {}\n", buckets.le_5000ms.load(Ordering::Relaxed)));
        out.push_str(&format!("nix_evo_api_response_duration_ms_bucket{{le=\"+Inf\"}} {}\n", buckets.le_inf.load(Ordering::Relaxed)));
        out.push_str(&format!("nix_evo_api_response_duration_ms_sum {}\n", buckets.sum.load(Ordering::Relaxed)));
        out.push_str(&format!("nix_evo_api_response_duration_ms_count {}\n", buckets.count.load(Ordering::Relaxed)));
        out.push('\n');
        drop(buckets);

        // Generation count
        out.push_str("# HELP nix_evo_generation_count Current number of NixOS generations\n");
        out.push_str("# TYPE nix_evo_generation_count gauge\n");
        out.push_str(&format!("nix_evo_generation_count {}\n", self.generation_count.load(Ordering::Relaxed)));
        out.push('\n');

        // Healer actions
        out.push_str("# HELP nix_evo_healer_actions_total Total self-healing actions taken\n");
        out.push_str("# TYPE nix_evo_healer_actions_total counter\n");
        out.push_str(&format!("nix_evo_healer_actions_total {}\n", self.healer_actions_total.load(Ordering::Relaxed)));
        out.push('\n');

        let by_svc = self.healer_actions_by_service.read().await;
        if !by_svc.is_empty() {
            out.push_str("# HELP nix_evo_healer_actions_by_service Healing actions by service\n");
            out.push_str("# TYPE nix_evo_healer_actions_by_service counter\n");
            for (svc, count) in by_svc.iter() {
                out.push_str(&format!("nix_evo_healer_actions_by_service{{service=\"{}\"}} {}\n", svc, count.load(Ordering::Relaxed)));
            }
            out.push('\n');
        }
        drop(by_svc);

        // WebSocket connections
        out.push_str("# HELP nix_evo_ws_connections_active Active WebSocket connections\n");
        out.push_str("# TYPE nix_evo_ws_connections_active gauge\n");
        out.push_str(&format!("nix_evo_ws_connections_active {}\n", self.ws_connections_active.load(Ordering::Relaxed)));
        out.push('\n');

        // Cluster metrics
        out.push_str("# HELP nix_evo_cluster_nodes_total Total cluster nodes configured\n");
        out.push_str("# TYPE nix_evo_cluster_nodes_total gauge\n");
        out.push_str(&format!("nix_evo_cluster_nodes_total {}\n", self.cluster_nodes_total.load(Ordering::Relaxed)));
        out.push('\n');

        out.push_str("# HELP nix_evo_cluster_deploys_total Total cluster deploys executed\n");
        out.push_str("# TYPE nix_evo_cluster_deploys_total counter\n");
        out.push_str(&format!("nix_evo_cluster_deploys_total {}\n", self.cluster_deploys_total.load(Ordering::Relaxed)));
        out.push('\n');

        out.push_str("# HELP nix_evo_cluster_deploy_failures_total Cluster deploy failures\n");
        out.push_str("# TYPE nix_evo_cluster_deploy_failures_total counter\n");
        out.push_str(&format!("nix_evo_cluster_deploy_failures_total {}\n", self.cluster_deploy_failures_total.load(Ordering::Relaxed)));
        out.push('\n');

        // Audit entries
        out.push_str("# HELP nix_evo_audit_entries_total Total audit log entries\n");
        out.push_str("# TYPE nix_evo_audit_entries_total counter\n");
        out.push_str(&format!("nix_evo_audit_entries_total {}\n", self.audit_entries_total.load(Ordering::Relaxed)));
        out.push('\n');

        // Process info
        out.push_str("# HELP nix_evo_up Whether the agent is up (always 1)\n");
        out.push_str("# TYPE nix_evo_up gauge\n");
        out.push_str("nix_evo_up 1\n");

        out
    }
}

/// Global metrics singleton
pub static METRICS: OnceLock<Metrics> = OnceLock::new();

pub fn metrics() -> &'static Metrics {
    METRICS.get_or_init(Metrics::new)
}

// ─── HTTP Handler ──────────────────────────────────────────────────────

/// GET /metrics — Prometheus text format
pub async fn handle_metrics() -> impl IntoResponse {
    let m = metrics();
    *m.last_scrape.write().await = Some(Instant::now());
    let body = m.render_prometheus().await;
    (
        axum::http::StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}
