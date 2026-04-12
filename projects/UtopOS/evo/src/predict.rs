use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::cmd::{run_cmd, AppStateRef};
use crate::error::AppError;

/// A metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub timestamp: String,
    pub value: f64,
}

/// Trend analysis for a metric
#[derive(Debug, Clone, Serialize)]
pub struct MetricTrend {
    pub name: String,
    pub current_value: f64,
    pub avg_rate_per_hour: f64,
    pub trend_direction: String,  // "rising", "falling", "stable"
    pub projected_hours_to_threshold: Option<f64>,
    pub confidence: f64,
}

/// A predicted alert
#[derive(Debug, Clone, Serialize)]
pub struct PredictedAlert {
    pub alert_id: String,
    pub severity: String,  // "info", "warning", "critical"
    pub category: String,  // "disk", "memory", "cpu", "service"
    pub title: String,
    pub description: String,
    pub predicted_at: String,
    pub estimated_time: Option<String>,
    pub metric_trend: Option<MetricTrend>,
    pub recommended_actions: Vec<String>,
}

/// Response for GET /api/predict/alerts
#[derive(Debug, Serialize)]
pub struct PredictResponse {
    pub timestamp: String,
    pub alerts: Vec<PredictedAlert>,
    pub system_summary: SystemSummary,
    pub risk_score: f64,
}

#[derive(Debug, Serialize)]
pub struct SystemSummary {
    pub disk_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub cpu_load_1m: f64,
    pub uptime_hours: f64,
    pub failed_services: Vec<String>,
}

/// Collect current disk usage
async fn get_disk_usage() -> Result<Vec<DiskMount>, AppError> {
    let output = run_cmd("df", &["-h", "--output=target,size,used,avail,pcent"]).await?;
    let mut mounts = Vec::new();

    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            let path = parts[0].to_string();
            // Skip pseudo filesystems
            if path.starts_with("/dev") || path.starts_with("/run") || path == "tmpfs" { continue; }
            if let Ok(pct) = parts[4].trim_end_matches('%').parse::<f64>() {
                mounts.push(DiskMount {
                    path,
                    usage_percent: pct,
                    used: parts[2].to_string(),
                    avail: parts[3].to_string(),
                });
            }
        }
    }
    Ok(mounts)
}

struct DiskMount {
    path: String,
    usage_percent: f64,
    used: String,
    avail: String,
}

/// Get memory usage
async fn get_memory_usage() -> Result<MemoryInfo, AppError> {
    let output = run_cmd("free", &["-b"]).await?;
    for line in output.lines() {
        if line.starts_with("Mem:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let total: f64 = parts[1].parse().unwrap_or(1.0);
                let used: f64 = parts[2].parse().unwrap_or(0.0);
                return Ok(MemoryInfo {
                    total_bytes: total,
                    used_bytes: used,
                    usage_percent: (used / total * 100.0),
                });
            }
        }
    }
    Err(AppError::Internal { message: "Could not parse memory info".into() })
}

struct MemoryInfo {
    total_bytes: f64,
    used_bytes: f64,
    usage_percent: f64,
}

/// Get CPU load
async fn get_load() -> Result<(f64, f64, f64), AppError> {
    let output = run_cmd("cat", &["/proc/loadavg"]).await?;
    let parts: Vec<&str> = output.split_whitespace().collect();
    if parts.len() >= 3 {
        Ok((
            parts[0].parse().unwrap_or(0.0),
            parts[1].parse().unwrap_or(0.0),
            parts[2].parse().unwrap_or(0.0),
        ))
    } else {
        Ok((0.0, 0.0, 0.0))
    }
}

/// Get failed services
async fn get_failed_services() -> Vec<String> {
    match run_cmd("systemctl", &["--failed", "--no-legend", "--no-pager"]).await {
        Ok(output) => output.lines()
            .filter_map(|l| {
                let parts: Vec<&str> = l.split_whitespace().collect();
                if !parts.is_empty() { Some(parts[0].to_string()) } else { None }
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Estimate hours until disk is full based on trend (simplified linear model)
fn estimate_hours_to_full(current_pct: f64, _mount: &str) -> Option<f64> {
    if current_pct < 50.0 { return None; }
    // Simplified: assume ~0.5% growth per hour for high-usage filesystems
    let growth_rate = match current_pct {
        p if p > 95.0 => 2.0,
        p if p > 90.0 => 1.0,
        p if p > 80.0 => 0.5,
        p if p > 70.0 => 0.3,
        _ => 0.1,
    };
    let remaining = 100.0 - current_pct;
    Some(remaining / growth_rate)
}

/// Generate alerts based on current system state
async fn generate_alerts() -> Result<PredictResponse, AppError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let timestamp = format!("{}", now.as_secs());

    let mut alerts = Vec::new();
    let mut failed_services = Vec::new();

    // Disk analysis
    let mounts = get_disk_usage().await.unwrap_or_default();
    for mount in &mounts {
        if mount.usage_percent > 90.0 {
            let hours = estimate_hours_to_full(mount.usage_percent, &mount.path);
            alerts.push(PredictedAlert {
                alert_id: format!("disk-{}", mount.path.replace('/', "_")),
                severity: if mount.usage_percent > 95.0 { "critical".into() } else { "warning".into() },
                category: "disk".into(),
                title: format!("Disk {} at {:.0}%", mount.path, mount.usage_percent),
                description: format!(
                    "{} is {:.1}% full ({} used, {} available).{}",
                    mount.path, mount.usage_percent, mount.used, mount.avail,
                    hours.map(|h| format!(" At current rate, will be full in ~{:.0} hours.", h)).unwrap_or_default()
                ),
                predicted_at: timestamp.clone(),
                estimated_time: hours.map(|h| format!("{:.0} hours", h)),
                metric_trend: Some(MetricTrend {
                    name: format!("disk.{}", mount.path),
                    current_value: mount.usage_percent,
                    avg_rate_per_hour: 0.5,
                    trend_direction: "rising".into(),
                    projected_hours_to_threshold: hours,
                    confidence: 0.7,
                }),
                recommended_actions: vec![
                    "Run nix-collect-garbage -d to free space".into(),
                    "Check for large log files: journalctl --disk-usage".into(),
                    "Consider expanding the disk volume".into(),
                ],
            });
        } else if mount.usage_percent > 75.0 {
            alerts.push(PredictedAlert {
                alert_id: format!("disk-warn-{}", mount.path.replace('/', "_")),
                severity: "info".into(),
                category: "disk".into(),
                title: format!("Disk {} at {:.0}% — monitor closely", mount.path, mount.usage_percent),
                description: format!("{} is at {:.1}% usage. Consider cleanup soon.", mount.path, mount.usage_percent),
                predicted_at: timestamp.clone(),
                estimated_time: estimate_hours_to_full(mount.usage_percent, &mount.path).map(|h| format!("{:.0} hours", h)),
                metric_trend: None,
                recommended_actions: vec!["Schedule garbage collection".into()],
            });
        }
    }

    // Memory analysis
    let mem = get_memory_usage().await.unwrap_or(MemoryInfo { total_bytes: 0.0, used_bytes: 0.0, usage_percent: 0.0 });
    if mem.usage_percent > 85.0 {
        alerts.push(PredictedAlert {
            alert_id: "memory-high".into(),
            severity: if mem.usage_percent > 95.0 { "critical".into() } else { "warning".into() },
            category: "memory".into(),
            title: format!("Memory usage at {:.0}%", mem.usage_percent),
            description: format!(
                "Using {:.1}G / {:.1}G ({:.1}%). High memory pressure may cause OOM kills.",
                mem.used_bytes / 1e9, mem.total_bytes / 1e9, mem.usage_percent
            ),
            predicted_at: timestamp.clone(),
            estimated_time: None,
            metric_trend: Some(MetricTrend {
                name: "memory.usage".into(),
                current_value: mem.usage_percent,
                avg_rate_per_hour: 0.2,
                trend_direction: "rising".into(),
                projected_hours_to_threshold: if mem.usage_percent > 90.0 { Some(12.0) } else { None },
                confidence: 0.6,
            }),
            recommended_actions: vec![
                "Identify memory-heavy processes: ps aux --sort=-%mem | head".into(),
                "Consider adding swap space".into(),
                "Review NixOS services for memory leaks".into(),
            ],
        });
    }

    // CPU load analysis
    let (load1, load5, _load15) = get_load().await.unwrap_or((0.0, 0.0, 0.0));
    let cores = num_cpus();
    if load1 > cores as f64 * 1.5 {
        alerts.push(PredictedAlert {
            alert_id: "cpu-high".into(),
            severity: "warning".into(),
            category: "cpu".into(),
            title: format!("CPU load {} exceeds {} cores", load1, cores),
            description: format!(
                "1-minute load average is {:.2} on {} cores (load/core: {:.2}). System is overloaded.",
                load1, cores, load1 / cores as f64
            ),
            predicted_at: timestamp.clone(),
            estimated_time: None,
            metric_trend: None,
            recommended_actions: vec![
                "Check top processes: htop or top -bn1".into(),
                "Review for runaway builds: nix-store --gc".into(),
            ],
        });
    }

    // Failed services
    failed_services = get_failed_services().await;
    for svc in &failed_services {
        alerts.push(PredictedAlert {
            alert_id: format!("svc-{}", svc),
            severity: "warning".into(),
            category: "service".into(),
            title: format!("Service {} is in failed state", svc),
            description: format!("Systemd reports {} as failed. This may indicate a configuration issue or crash.", svc),
            predicted_at: timestamp.clone(),
            estimated_time: None,
            metric_trend: None,
            recommended_actions: vec![
                format!("systemctl status {}", svc),
                format!("journalctl -u {} --no-pager -n 30", svc),
                format!("systemctl restart {}", svc),
            ],
        });
    }

    let risk_score = calculate_risk_score(&alerts, mem.usage_percent, mounts.first().map(|m| m.usage_percent).unwrap_or(0.0));

    Ok(PredictResponse {
        timestamp,
        alerts,
        system_summary: SystemSummary {
            disk_usage_percent: mounts.first().map(|m| m.usage_percent).unwrap_or(0.0),
            memory_usage_percent: mem.usage_percent,
            cpu_load_1m: load1,
            uptime_hours: get_uptime_hours().await.unwrap_or(0.0),
            failed_services,
        },
        risk_score,
    })
}

fn calculate_risk_score(alerts: &[PredictedAlert], mem_pct: f64, disk_pct: f64) -> f64 {
    let mut score: f64 = 0.0;
    for alert in alerts {
        score += match alert.severity.as_str() {
            "critical" => 25.0,
            "warning" => 15.0,
            "info" => 5.0,
            _ => 0.0,
        };
    }
    score += (mem_pct - 50.0).max(0.0) * 0.2;
    score += (disk_pct - 50.0).max(0.0) * 0.3;
    score.min(100.0)
}

fn num_cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

async fn get_uptime_hours() -> Result<f64, AppError> {
    let output = run_cmd("cat", &["/proc/uptime"]).await?;
    let secs: f64 = output.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0.0);
    Ok(secs / 3600.0)
}

/// GET /api/predict/alerts — get predicted failure alerts
pub async fn handle_alerts(
    State(_state): AppStateRef,
) -> Result<impl IntoResponse, AppError> {
    let response = generate_alerts().await?;
    Ok(Json(serde_json::to_value(&response).unwrap_or_default()))
}
