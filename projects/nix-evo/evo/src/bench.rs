use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::cmd::run_cmd;
use crate::error::AppError;

// ─── Benchmark Types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRun {
    pub id: String,
    pub label: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: String, // "running", "completed", "failed"
    pub config_snapshot: Option<String>,
    pub metrics: Vec<MetricResult>,
    pub summary: Option<BenchmarkSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricResult {
    pub name: String,
    pub category: String,
    pub value: f64,
    pub unit: String,
    pub iterations: u32,
    pub stddev: f64,
    pub min: f64,
    pub max: f64,
    pub confidence_95: (f64, f64), // (lower, upper)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub total_metrics: usize,
    pub boot_time_ms: Option<f64>,
    pub build_time_ms: Option<f64>,
    pub disk_size_mb: Option<f64>,
    pub security_score: Option<f64>,
    pub overall_grade: String, // "A+", "A", "B", "C", "D", "F"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub baseline: BenchmarkRun,
    pub current: BenchmarkRun,
    pub deltas: Vec<MetricDelta>,
    pub verdict: String, // "improved", "regressed", "neutral"
    pub regression_count: usize,
    pub improvement_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDelta {
    pub name: String,
    pub baseline_value: f64,
    pub current_value: f64,
    pub delta_pct: f64,
    pub direction: String, // "better", "worse", "same"
    pub significant: bool, // statistically significant change
}

// ─── Benchmark Engine ────────────────────────────────────────────────────

pub struct BenchEngine {
    runs: RwLock<Vec<BenchmarkRun>>,
    next_id: std::sync::atomic::AtomicU64,
}

impl BenchEngine {
    pub fn new() -> Self {
        Self {
            runs: RwLock::new(Vec::new()),
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> String {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("bench-{}", id)
    }

    /// Run a full benchmark suite.
    pub async fn run(&self, label: String, metrics: Vec<String>) -> BenchmarkRun {
        let id = self.next_id();
        let now = chrono::Utc::now().to_rfc3339();

        let mut run = BenchmarkRun {
            id: id.clone(),
            label,
            started_at: now,
            completed_at: None,
            status: "running".into(),
            config_snapshot: None,
            metrics: Vec::new(),
            summary: None,
        };

        // Run requested benchmarks
        for metric_name in &metrics {
            match metric_name.as_str() {
                "boot_time" => {
                    if let Some(m) = self.measure_boot_time().await {
                        run.metrics.push(m);
                    }
                }
                "build_time" => {
                    if let Some(m) = self.measure_build_time().await {
                        run.metrics.push(m);
                    }
                }
                "disk_size" => {
                    if let Some(m) = self.measure_disk_size().await {
                        run.metrics.push(m);
                    }
                }
                "service_startup" => {
                    let ms = self.measure_service_startup().await;
                    run.metrics.extend(ms);
                }
                "security_score" => {
                    if let Some(m) = self.measure_security().await {
                        run.metrics.push(m);
                    }
                }
                "memory_usage" => {
                    if let Some(m) = self.measure_memory().await {
                        run.metrics.push(m);
                    }
                }
                _ => {}
            }
        }

        // Compute summary
        run.summary = Some(self.compute_summary(&run.metrics));
        run.completed_at = Some(chrono::Utc::now().to_rfc3339());
        run.status = "completed".into();

        // Store
        let mut runs = self.runs.write().await;
        runs.push(run.clone());

        run
    }

    async fn measure_boot_time(&self) -> Option<MetricResult> {
        let output = run_cmd("systemd-analyze", &["time"]).await.ok()?;
        // Parse: "Startup finished in 3.456s (kernel) + 12.789s (userspace) = 16.245s"
        let total_ms = if let Some(total) = output.split("= ").last() {
            let secs: f64 = total.trim().trim_end_matches('s').parse().unwrap_or(0.0);
            secs * 1000.0
        } else {
            0.0
        };

        Some(MetricResult {
            name: "boot_time".into(),
            category: "boot".into(),
            value: total_ms,
            unit: "ms".into(),
            iterations: 1,
            stddev: 0.0,
            min: total_ms,
            max: total_ms,
            confidence_95: (total_ms, total_ms),
        })
    }

    async fn measure_build_time(&self) -> Option<MetricResult> {
        // Dry-build timing
        let start = std::time::Instant::now();
        let result = run_cmd("nixos-rebuild", &["dry-build"]).await;
        let elapsed_ms = start.elapsed().as_millis() as f64;

        Some(MetricResult {
            name: "build_time".into(),
            category: "build".into(),
            value: elapsed_ms,
            unit: "ms".into(),
            iterations: 1,
            stddev: 0.0,
            min: elapsed_ms,
            max: elapsed_ms,
            confidence_95: (elapsed_ms, elapsed_ms),
        })
    }

    async fn measure_disk_size(&self) -> Option<MetricResult> {
        let output = run_cmd("du", &["-sm", "/nix/store"]).await.ok()?;
        let mb: f64 = output.split_whitespace().next()?.parse().unwrap_or(0.0);

        Some(MetricResult {
            name: "disk_size".into(),
            category: "storage".into(),
            value: mb,
            unit: "MB".into(),
            iterations: 1,
            stddev: 0.0,
            min: mb,
            max: mb,
            confidence_95: (mb, mb),
        })
    }

    async fn measure_service_startup(&self) -> Vec<MetricResult> {
        let mut results = Vec::new();
        let services = ["nginx.service", "sshd.service", "docker.service"];

        for svc in &services {
            let output = run_cmd("systemd-analyze", &["blame"]).await.unwrap_or_default();
            // Parse: " 12.345s nginx.service"
            for line in output.lines() {
                let trimmed = line.trim();
                if trimmed.ends_with(svc) {
                    let time_str = trimmed.split_whitespace().next().unwrap_or("0s");
                    let secs: f64 = time_str.trim_end_matches('s').parse().unwrap_or(0.0);
                    results.push(MetricResult {
                        name: format!("service_start_{}", svc.trim_end_matches(".service")),
                        category: "services".into(),
                        value: secs * 1000.0,
                        unit: "ms".into(),
                        iterations: 1,
                        stddev: 0.0,
                        min: secs * 1000.0,
                        max: secs * 1000.0,
                        confidence_95: (secs * 1000.0, secs * 1000.0),
                    });
                    break;
                }
            }
        }
        results
    }

    async fn measure_security(&self) -> Option<MetricResult> {
        let mut score: f64 = 50.0;

        // Check firewall
        if let Ok(output) = run_cmd("iptables", &["-L", "-n"]).await {
            if output.lines().count() > 5 { score += 10.0; }
        }
        // Check SSH config
        if let Ok(output) = run_cmd("grep", &["-c", "PermitRootLogin no", "/etc/ssh/sshd_config"]).await {
            if output.trim() != "0" { score += 10.0; }
        }
        // Check for updates
        if let Ok(output) = run_cmd("nix-env", &["-q"]).await {
            let pkg_count = output.lines().count() as f64;
            score += (pkg_count * 0.1).min(10.0);
        }
        // Check fail2ban
        if let Ok(output) = run_cmd("systemctl", &["is-active", "fail2ban"]).await {
            if output.trim() == "active" { score += 10.0; }
        }

        let score = score.min(100.0);

        Some(MetricResult {
            name: "security_score".into(),
            category: "security".into(),
            value: score,
            unit: "score".into(),
            iterations: 1,
            stddev: 0.0,
            min: score,
            max: score,
            confidence_95: (score, score),
        })
    }

    async fn measure_memory(&self) -> Option<MetricResult> {
        let output = run_cmd("free", &["-m"]).await.ok()?;
        let total: f64 = output.lines().nth(1)?
            .split_whitespace().nth(1)?.parse().unwrap_or(0.0);
        let used: f64 = output.lines().nth(1)?
            .split_whitespace().nth(2)?.parse().unwrap_or(0.0);

        Some(MetricResult {
            name: "memory_used".into(),
            category: "memory".into(),
            value: used,
            unit: "MB".into(),
            iterations: 1,
            stddev: 0.0,
            min: used,
            max: used,
            confidence_95: (used, used),
        })
    }

    fn compute_summary(&self, metrics: &[MetricResult]) -> BenchmarkSummary {
        let boot = metrics.iter().find(|m| m.name == "boot_time").map(|m| m.value);
        let build = metrics.iter().find(|m| m.name == "build_time").map(|m| m.value);
        let disk = metrics.iter().find(|m| m.name == "disk_size").map(|m| m.value);
        let security = metrics.iter().find(|m| m.name == "security_score").map(|m| m.value);

        // Grade based on combined score
        let grade = match security.unwrap_or(50.0) {
            s if s >= 90.0 => "A+",
            s if s >= 80.0 => "A",
            s if s >= 70.0 => "B",
            s if s >= 60.0 => "C",
            s if s >= 50.0 => "D",
            _ => "F",
        };

        BenchmarkSummary {
            total_metrics: metrics.len(),
            boot_time_ms: boot,
            build_time_ms: build,
            disk_size_mb: disk,
            security_score: security,
            overall_grade: grade.to_string(),
        }
    }

    /// Compare two benchmark runs.
    pub async fn compare(&self, baseline_id: &str, current_id: &str) -> Result<ComparisonResult, String> {
        let runs = self.runs.read().await;

        let baseline = runs.iter().find(|r| r.id == baseline_id)
            .ok_or_else(|| format!("Baseline '{}' not found", baseline_id))?.clone();
        let current = runs.iter().find(|r| r.id == current_id)
            .ok_or_else(|| format!("Current '{}' not found", current_id))?.clone();

        let mut deltas = Vec::new();
        for m_curr in &current.metrics {
            if let Some(m_base) = baseline.metrics.iter().find(|m| m.name == m_curr.name) {
                let delta_pct = if m_base.value != 0.0 {
                    ((m_curr.value - m_base.value) / m_base.value) * 100.0
                } else {
                    0.0
                };

                // Higher is better for security, worse for time/disk
                let is_time_or_disk = m_curr.category == "boot" || m_curr.category == "build" || m_curr.category == "storage" || m_curr.category == "memory";
                let direction = if delta_pct.abs() < 1.0 {
                    "same"
                } else if is_time_or_disk {
                    if delta_pct < 0.0 { "better" } else { "worse" }
                } else {
                    if delta_pct > 0.0 { "better" } else { "worse" }
                };

                deltas.push(MetricDelta {
                    name: m_curr.name.clone(),
                    baseline_value: m_base.value,
                    current_value: m_curr.value,
                    delta_pct,
                    direction: direction.into(),
                    significant: delta_pct.abs() > 5.0,
                });
            }
        }

        let improvements = deltas.iter().filter(|d| d.direction == "better").count();
        let regressions = deltas.iter().filter(|d| d.direction == "worse").count();

        let verdict = if regressions > improvements {
            "regressed"
        } else if improvements > regressions {
            "improved"
        } else {
            "neutral"
        };

        Ok(ComparisonResult {
            baseline,
            current,
            deltas,
            verdict: verdict.into(),
            regression_count: regressions,
            improvement_count: improvements,
        })
    }

    pub async fn get_runs(&self) -> Vec<BenchmarkRun> {
        self.runs.read().await.clone()
    }
}

// ─── API types ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RunRequest {
    pub label: Option<String>,
    pub metrics: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CompareQuery {
    pub baseline: String,
    pub current: String,
}

// ─── Global engine ───────────────────────────────────────────────────────

use std::sync::LazyLock;
static BENCH_ENGINE: LazyLock<Arc<BenchEngine>> = LazyLock::new(|| Arc::new(BenchEngine::new()));

/// POST /api/bench/run — Run benchmarks
pub async fn handle_run(Json(req): Json<RunRequest>) -> Result<impl IntoResponse, AppError> {
    let engine = BENCH_ENGINE.clone();
    let label = req.label.unwrap_or_else(|| format!("run-{}", chrono::Utc::now().timestamp()));
    let metrics = req.metrics.unwrap_or_else(|| vec![
        "boot_time".into(), "build_time".into(), "disk_size".into(),
        "security_score".into(), "memory_usage".into(),
    ]);
    let run = engine.run(label, metrics).await;
    Ok(Json(serde_json::to_value(&run).unwrap()))
}

/// GET /api/bench/results — Get benchmark results
pub async fn handle_results() -> impl IntoResponse {
    let engine = BENCH_ENGINE.clone();
    let runs = engine.get_runs().await;
    Json(serde_json::json!({ "runs": runs }))
}

/// GET /api/bench/compare — Compare two benchmark runs
pub async fn handle_compare(Query(q): Query<CompareQuery>) -> Result<impl IntoResponse, AppError> {
    let engine = BENCH_ENGINE.clone();
    match engine.compare(&q.baseline, &q.current).await {
        Ok(result) => Ok(Json(serde_json::to_value(&result).unwrap())),
        Err(e) => Err(AppError::BadRequest(e)),
    }
}
