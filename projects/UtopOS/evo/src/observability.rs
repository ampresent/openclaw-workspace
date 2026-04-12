use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use crate::error::AppError;
use crate::cmd::{run_cmd, run_cmd_with_timeout};

/// Structured log entry parsed from journald
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub unit: String,
    pub priority: String,   // emerg, alert, crit, err, warning, notice, info, debug
    pub message: String,
    pub pid: Option<u64>,
    pub hostname: Option<String>,
    pub facility: Option<String>,
}

/// Log query parameters
#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub unit: Option<String>,
    pub since: Option<String>,      // e.g., "1h", "2026-04-12", "yesterday"
    pub until: Option<String>,
    pub priority: Option<String>,   // filter by priority level
    pub limit: Option<usize>,
    pub search: Option<String>,     // grep filter
}

/// Log aggregation response
#[derive(Debug, Serialize)]
pub struct LogAggregateResponse {
    pub entries: Vec<LogEntry>,
    pub total: usize,
    pub time_range: TimeRange,
    pub units_summary: std::collections::HashMap<String, usize>,
    pub priority_summary: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Serialize)]
pub struct TimeRange {
    pub oldest: Option<String>,
    pub newest: Option<String>,
}

/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub name: String,
    pub description: String,
    pub condition: AlertCondition,
    pub duration_minutes: u64,  // how long condition must be true
    pub severity: String,       // critical, warning, info
    pub enabled: bool,
    pub notify_channels: Vec<String>, // "webhook", "log", "email"
    pub cooldown_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCondition {
    pub metric: String,         // "disk_percent", "memory_percent", "cpu_percent", "service_down"
    pub operator: String,       // "gt", "lt", "eq", "ne"
    pub threshold: f64,
    pub target: Option<String>, // disk path, service name, etc.
}

/// Active alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveAlert {
    pub rule_name: String,
    pub triggered_at: String,
    pub current_value: f64,
    pub threshold: f64,
    pub message: String,
    pub acknowledged: bool,
}

/// Alert state stored on disk
#[derive(Debug, Serialize, Deserialize)]
pub struct AlertState {
    pub rules: Vec<AlertRule>,
    pub active_alerts: Vec<ActiveAlert>,
    pub last_check: Option<String>,
    pub alert_history: Vec<AlertHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertHistoryEntry {
    pub rule_name: String,
    pub triggered_at: String,
    pub resolved_at: Option<String>,
    pub message: String,
}

/// Prometheus metrics endpoint response
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub prometheus_text: String,
}

/// Grafana integration config
#[derive(Debug, Serialize, Deserialize)]
pub struct GrafanaConfig {
    pub url: Option<String>,
    pub api_key_env: Option<String>,
    pub dashboards: Vec<DashboardConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub name: String,
    pub uid: String,
    pub panels: Vec<String>,
}

/// Loki integration config
#[derive(Debug, Serialize, Deserialize)]
pub struct LokiConfig {
    pub url: Option<String>,    // e.g., "http://localhost:3100"
    pub push_enabled: bool,
    pub labels: std::collections::HashMap<String, String>,
}

/// Full observability config
#[derive(Debug, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub grafana: Option<GrafanaConfig>,
    pub loki: Option<LokiConfig>,
    pub prometheus: Option<PrometheusConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrometheusConfig {
    pub exporter_port: u16,     // default 9100
    pub scrape_interval: String, // "15s"
}

// ============================================================
// Handlers
// ============================================================

/// POST /api/observability/logs — query structured logs
pub async fn query_logs(
    State(_state): State<Arc<AppState>>,
    Json(query): Json<LogQuery>,
) -> Result<Json<LogAggregateResponse>, AppError> {
    let entries = fetch_journald_logs(&query).await?;

    let mut units_summary: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut priority_summary: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for entry in &entries {
        *units_summary.entry(entry.unit.clone()).or_default() += 1;
        *priority_summary.entry(entry.priority.clone()).or_default() += 1;
    }

    let oldest = entries.first().map(|e| e.timestamp.clone());
    let newest = entries.last().map(|e| e.timestamp.clone());

    Ok(Json(LogAggregateResponse {
        total: entries.len(),
        entries,
        time_range: TimeRange { oldest, newest },
        units_summary,
        priority_summary,
    }))
}

/// GET /api/observability/metrics — Prometheus-compatible metrics
pub async fn metrics_endpoint(
    State(_state): State<Arc<AppState>>,
) -> Result<String, AppError> {
    let mut metrics = String::new();

    // System metrics
    metrics.push_str("# HELP UtopOS_up Whether UtopOS agent is running\n");
    metrics.push_str("# TYPE UtopOS_up gauge\n");
    metrics.push_str("UtopOS_up 1\n\n");

    // CPU usage
    if let Ok(cpu) = get_cpu_usage().await {
        metrics.push_str("# HELP UtopOS_cpu_usage_percent CPU usage percentage\n");
        metrics.push_str("# TYPE UtopOS_cpu_usage_percent gauge\n");
        metrics.push_str(&format!("UtopOS_cpu_usage_percent {cpu:.1}\n\n"));
    }

    // Memory
    if let Ok((total, used, avail)) = get_memory_info().await {
        metrics.push_str("# HELP UtopOS_memory_total_bytes Total memory in bytes\n");
        metrics.push_str("# TYPE UtopOS_memory_total_bytes gauge\n");
        metrics.push_str(&format!("UtopOS_memory_total_bytes {total}\n\n"));

        metrics.push_str("# HELP UtopOS_memory_used_bytes Used memory in bytes\n");
        metrics.push_str("# TYPE UtopOS_memory_used_bytes gauge\n");
        metrics.push_str(&format!("UtopOS_memory_used_bytes {used}\n\n"));

        let pct = if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 };
        metrics.push_str("# HELP UtopOS_memory_usage_percent Memory usage percentage\n");
        metrics.push_str("# TYPE UtopOS_memory_usage_percent gauge\n");
        metrics.push_str(&format!("UtopOS_memory_usage_percent {pct:.1}\n\n"));
    }

    // Disk usage for common mount points
    if let Ok(disks) = get_disk_usage().await {
        metrics.push_str("# HELP UtopOS_disk_usage_percent Disk usage percentage\n");
        metrics.push_str("# TYPE UtopOS_disk_usage_percent gauge\n");
        for (mount, pct) in &disks {
            let label = mount.replace('/', "_").trim_start_matches('_').to_string();
            let label = if label.is_empty() { "root".into() } else { label };
            metrics.push_str(&format!("UtopOS_disk_usage_percent{{mount=\"{mount}\"}} {pct:.1}\n"));
        }
        metrics.push('\n');
    }

    // NixOS generation
    if let Ok(gen) = get_current_generation().await {
        metrics.push_str("# HELP UtopOS_generation Current NixOS generation number\n");
        metrics.push_str("# TYPE UtopOS_generation gauge\n");
        metrics.push_str(&format!("UtopOS_generation {gen}\n\n"));
    }

    // Failed services count
    if let Ok(count) = get_failed_services_count().await {
        metrics.push_str("# HELP UtopOS_failed_services Number of failed systemd units\n");
        metrics.push_str("# TYPE UtopOS_failed_services gauge\n");
        metrics.push_str(&format!("UtopOS_failed_services {count}\n\n"));
    }

    Ok(metrics)
}

/// GET /api/observability/alerts — list alert rules and active alerts
pub async fn list_alerts(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<AlertState>, AppError> {
    let state = load_alert_state().unwrap_or_else(|| AlertState {
        rules: default_alert_rules(),
        active_alerts: vec![],
        last_check: None,
        alert_history: vec![],
    });
    Ok(Json(state))
}

/// POST /api/observability/alerts/check — run alert evaluation
pub async fn check_alerts(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<ActiveAlert>>, AppError> {
    let mut state = load_alert_state().unwrap_or_else(|| AlertState {
        rules: default_alert_rules(),
        active_alerts: vec![],
        last_check: None,
        alert_history: vec![],
    });

    let mut new_alerts = Vec::new();

    for rule in &state.rules {
        if !rule.enabled {
            continue;
        }

        let value = evaluate_condition(&rule.condition).await?;

        let triggered = match rule.condition.operator.as_str() {
            "gt" => value > rule.condition.threshold,
            "lt" => value < rule.condition.threshold,
            "eq" => (value - rule.condition.threshold).abs() < f64::EPSILON,
            "ne" => (value - rule.condition.threshold).abs() >= f64::EPSILON,
            "gte" => value >= rule.condition.threshold,
            "lte" => value <= rule.condition.threshold,
            _ => false,
        };

        if triggered {
            let alert = ActiveAlert {
                rule_name: rule.name.clone(),
                triggered_at: chrono_now(),
                current_value: value,
                threshold: rule.condition.threshold,
                message: format!(
                    "⚠️ {} — {} {} {} (当前值: {:.1})",
                    rule.name, rule.condition.metric, rule.condition.operator, rule.condition.threshold, value
                ),
                acknowledged: false,
            };
            new_alerts.push(alert.clone());

            // Log the alert
            tracing::warn!("Alert triggered: {} — value={:.1}, threshold={:.1}",
                rule.name, value, rule.condition.threshold);
        }
    }

    state.active_alerts = new_alerts.clone();
    state.last_check = Some(chrono_now());
    save_alert_state(&state)?;

    Ok(Json(new_alerts))
}

/// POST /api/observability/alerts/rules — add/update alert rule
pub async fn upsert_alert_rule(
    State(_state): State<Arc<AppState>>,
    Json(rule): Json<AlertRule>,
) -> Result<Json<AlertRule>, AppError> {
    let mut state = load_alert_state().unwrap_or_else(|| AlertState {
        rules: vec![],
        active_alerts: vec![],
        last_check: None,
        alert_history: vec![],
    });

    // Replace existing or add new
    if let Some(existing) = state.rules.iter_mut().find(|r| r.name == rule.name) {
        *existing = rule.clone();
    } else {
        state.rules.push(rule.clone());
    }

    save_alert_state(&state)?;
    Ok(Json(rule))
}

/// GET /api/observability/config — get integration configs
pub async fn get_observability_config(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ObservabilityConfig>, AppError> {
    let config_path = "/etc/UtopOS/observability.json";
    let config = std::fs::read_to_string(config_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(ObservabilityConfig {
            grafana: None,
            loki: None,
            prometheus: Some(PrometheusConfig {
                exporter_port: 9100,
                scrape_interval: "15s".into(),
            }),
        });
    Ok(Json(config))
}

// ============================================================
// Helpers
// ============================================================

/// Fetch logs from journald and parse into structured entries
async fn fetch_journald_logs(query: &LogQuery) -> Result<Vec<LogEntry>, AppError> {
    let mut args = vec!["--output=json", "--no-pager"];

    if let Some(unit) = &query.unit {
        args.push("-u");
        args.push(unit.as_str());
    }

    let since = query.since.as_deref().unwrap_or("-1h");
    args.push("--since");
    args.push(since);

    if let Some(until) = &query.until {
        args.push("--until");
        args.push(until.as_str());
    }

    let limit = query.limit.unwrap_or(100);
    let limit_str = limit.to_string();
    args.push("-n");
    args.push(&limit_str);

    let output = run_cmd_with_timeout("journalctl", &args, 30).await?;

    let mut entries: Vec<LogEntry> = output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let val: serde_json::Value = serde_json::from_str(line).ok()?;
            Some(LogEntry {
                timestamp: val.get("__REALTIME_TIMESTAMP")
                    .and_then(|t| t.as_str())
                    .and_then(|t| t.parse::<u64>().ok())
                    .map(|ts| {
                        // Convert microseconds since epoch to ISO string
                        let secs = ts / 1_000_000;
                        format!("{secs}")
                    })
                    .unwrap_or_default(),
                unit: val.get("_SYSTEMD_UNIT")
                    .and_then(|u| u.as_str())
                    .unwrap_or(val.get("SYSLOG_IDENTIFIER").and_then(|u| u.as_str()).unwrap_or("unknown"))
                    .to_string(),
                priority: val.get("PRIORITY")
                    .and_then(|p| p.as_str())
                    .and_then(|p| p.parse::<u8>().ok())
                    .map(priority_to_string)
                    .unwrap_or_else(|| "info".into()),
                message: val.get("MESSAGE")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string(),
                pid: val.get("_PID").and_then(|p| p.as_str()).and_then(|p| p.parse().ok()),
                hostname: val.get("_HOSTNAME").and_then(|h| h.as_str()).map(|s| s.to_string()),
                facility: val.get("SYSLOG_FACILITY").and_then(|f| f.as_str()).map(|s| s.to_string()),
            })
        })
        .collect();

    // Filter by search term
    if let Some(search) = &query.search {
        let search_lower = search.to_lowercase();
        entries.retain(|e| e.message.to_lowercase().contains(&search_lower));
    }

    // Filter by priority
    if let Some(min_priority) = &query.priority {
        let min_level = priority_to_level(min_priority);
        entries.retain(|e| priority_to_level(&e.priority) <= min_level);
    }

    Ok(entries)
}

fn priority_to_string(p: u8) -> String {
    match p {
        0 => "emerg".into(),
        1 => "alert".into(),
        2 => "crit".into(),
        3 => "err".into(),
        4 => "warning".into(),
        5 => "notice".into(),
        6 => "info".into(),
        7 => "debug".into(),
        _ => "unknown".into(),
    }
}

fn priority_to_level(p: &str) -> u8 {
    match p {
        "emerg" => 0,
        "alert" => 1,
        "crit" => 2,
        "err" => 3,
        "error" => 3,
        "warning" | "warn" => 4,
        "notice" => 5,
        "info" => 6,
        "debug" => 7,
        _ => 6,
    }
}

async fn get_cpu_usage() -> Result<f64, AppError> {
    let output = run_cmd("top", &["-bn1"]).await?;
    for line in output.lines() {
        if line.contains("%Cpu") || line.contains("Cpu(s)") {
            // Parse: %Cpu(s):  2.0 us,  0.7 sy,  0.0 ni, 97.0 id, ...
            if let Some(idle) = line.split("id,").next() {
                if let Some(num) = idle.split_whitespace().last() {
                    if let Ok(idle_pct) = num.parse::<f64>() {
                        return Ok(100.0 - idle_pct);
                    }
                }
            }
        }
    }
    Ok(0.0)
}

async fn get_memory_info() -> Result<(u64, u64, u64), AppError> {
    let output = run_cmd("free", &["-b"]).await?;
    for line in output.lines() {
        if line.starts_with("Mem:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let total: u64 = parts[1].parse().unwrap_or(0);
                let used: u64 = parts[2].parse().unwrap_or(0);
                let avail: u64 = parts.get(6).and_then(|s| s.parse().ok())
                    .unwrap_or(parts[3].parse().unwrap_or(0));
                return Ok((total, used, avail));
            }
        }
    }
    Ok((0, 0, 0))
}

async fn get_disk_usage() -> Result<Vec<(String, f64)>, AppError> {
    let output = run_cmd("df", &["-h", "--output=target,pcent"]).await?;
    let mut disks = Vec::new();
    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let mount = parts[0].to_string();
            let pct_str = parts[1].trim_end_matches('%');
            if let Ok(pct) = pct_str.parse::<f64>() {
                disks.push((mount, pct));
            }
        }
    }
    Ok(disks)
}

async fn get_current_generation() -> Result<u64, AppError> {
    let output = run_cmd("nixos-rebuild", &["list-generations"]).await?;
    // Find the current (highest) generation
    let max_gen = output
        .lines()
        .filter_map(|line| {
            line.split_whitespace().next().and_then(|s| s.parse::<u64>().ok())
        })
        .max()
        .unwrap_or(0);
    Ok(max_gen)
}

async fn get_failed_services_count() -> Result<u64, AppError> {
    let output = run_cmd_with_timeout("systemctl", &["--failed", "--no-legend"], 10).await?;
    let count = output.lines().filter(|l| !l.trim().is_empty()).count() as u64;
    Ok(count)
}

/// Evaluate a single alert condition and return the current metric value
async fn evaluate_condition(condition: &AlertCondition) -> Result<f64, AppError> {
    match condition.metric.as_str() {
        "disk_percent" => {
            let target = condition.target.as_deref().unwrap_or("/");
            let disks = get_disk_usage().await?;
            Ok(disks.iter()
                .find(|(m, _)| m == target)
                .map(|(_, p)| *p)
                .unwrap_or(0.0))
        }
        "memory_percent" => {
            let (total, used, _) = get_memory_info().await?;
            if total > 0 {
                Ok((used as f64 / total as f64) * 100.0)
            } else {
                Ok(0.0)
            }
        }
        "cpu_percent" => get_cpu_usage().await,
        "service_down" => {
            let target = condition.target.as_deref().unwrap_or("");
            let output = run_cmd_with_timeout("systemctl", &["is-active", target], 5).await?;
            Ok(if output.trim() == "active" { 0.0 } else { 1.0 })
        }
        "failed_services" => {
            let count = get_failed_services_count().await?;
            Ok(count as f64)
        }
        _ => Err(AppError::Validation {
            field: "condition.metric".into(),
            message: format!("未知的监控指标: {}", condition.metric),
        }),
    }
}

/// Default alert rules
fn default_alert_rules() -> Vec<AlertRule> {
    vec![
        AlertRule {
            name: "disk-full".into(),
            description: "磁盘空间不足".into(),
            condition: AlertCondition {
                metric: "disk_percent".into(),
                operator: "gt".into(),
                threshold: 90.0,
                target: Some("/".into()),
            },
            duration_minutes: 5,
            severity: "critical".into(),
            enabled: true,
            notify_channels: vec!["log".into()],
            cooldown_minutes: 30,
        },
        AlertRule {
            name: "memory-high".into(),
            description: "内存使用过高".into(),
            condition: AlertCondition {
                metric: "memory_percent".into(),
                operator: "gt".into(),
                threshold: 90.0,
                target: None,
            },
            duration_minutes: 5,
            severity: "warning".into(),
            enabled: true,
            notify_channels: vec!["log".into()],
            cooldown_minutes: 15,
        },
        AlertRule {
            name: "service-failed".into(),
            description: "有服务处于失败状态".into(),
            condition: AlertCondition {
                metric: "failed_services".into(),
                operator: "gt".into(),
                threshold: 0.0,
                target: None,
            },
            duration_minutes: 1,
            severity: "critical".into(),
            enabled: true,
            notify_channels: vec!["log".into()],
            cooldown_minutes: 10,
        },
        AlertRule {
            name: "cpu-high".into(),
            description: "CPU 使用过高".into(),
            condition: AlertCondition {
                metric: "cpu_percent".into(),
                operator: "gt".into(),
                threshold: 95.0,
                target: None,
            },
            duration_minutes: 10,
            severity: "warning".into(),
            enabled: true,
            notify_channels: vec!["log".into()],
            cooldown_minutes: 15,
        },
    ]
}

fn load_alert_state() -> Option<AlertState> {
    let path = "/var/lib/UtopOS/alert-state.json";
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

fn save_alert_state(state: &AlertState) -> Result<(), AppError> {
    let dir = "/var/lib/UtopOS";
    std::fs::create_dir_all(dir).map_err(|e| AppError::IoError {
        path: dir.into(),
        message: e.to_string(),
    })?;
    let path = format!("{dir}/alert-state.json");
    let content = serde_json::to_string_pretty(state).map_err(|e| AppError::Internal {
        message: format!("序列化告警状态失败: {e}"),
    })?;
    std::fs::write(&path, content).map_err(|e| AppError::IoError {
        path,
        message: e.to_string(),
    })?;
    Ok(())
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
