use axum::{
    extract::{ws::WebSocket, ws::WebSocketUpgrade, Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::{interval, Duration};

use crate::cmd::{run_cmd, AppStateRef};
use crate::error::AppError;

#[derive(Deserialize)]
pub struct DashboardQuery {
    pub interval_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: String,
    pub cpu_usage_pct: f64,
    pub memory: MemoryInfo,
    pub disk: Vec<DiskInfo>,
    pub services: Vec<ServiceStatus>,
    pub load_avg: (f64, f64, f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_kb: u64,
    pub used_kb: u64,
    pub available_kb: u64,
    pub usage_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub mount: String,
    pub used_pct: f64,
    pub total_gb: f64,
    pub used_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub active: String,
    pub sub: String,
}

async fn collect_metrics() -> Result<SystemMetrics, AppError> {
    let cpu_pct = read_cpu_usage().await.unwrap_or(0.0);
    let mem = read_memory_info().await.unwrap_or(MemoryInfo {
        total_kb: 0, used_kb: 0, available_kb: 0, usage_pct: 0.0,
    });
    let disk = read_disk_info().await.unwrap_or_default();
    let load_avg = read_load_avg().await.unwrap_or((0.0, 0.0, 0.0));
    let services = read_service_statuses().await.unwrap_or_default();
    let timestamp = chrono_now();

    Ok(SystemMetrics {
        timestamp, cpu_usage_pct: cpu_pct, memory: mem, disk, services, load_avg,
    })
}

async fn read_cpu_usage() -> Result<f64, AppError> {
    let stat1 = tokio::fs::read_to_string("/proc/stat").await?;
    let line1 = stat1.lines().find(|l| l.starts_with("cpu ")).unwrap_or("cpu 0 0 0 0");
    let vals1: Vec<u64> = line1.split_whitespace().skip(1).filter_map(|s| s.parse().ok()).collect();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let stat2 = tokio::fs::read_to_string("/proc/stat").await?;
    let line2 = stat2.lines().find(|l| l.starts_with("cpu ")).unwrap_or("cpu 0 0 0 0");
    let vals2: Vec<u64> = line2.split_whitespace().skip(1).filter_map(|s| s.parse().ok()).collect();
    if vals1.len() < 4 || vals2.len() < 4 { return Ok(0.0); }
    let total1: u64 = vals1.iter().sum();
    let total2: u64 = vals2.iter().sum();
    let idle_delta = vals2[3].saturating_sub(vals1[3]) as f64;
    let total_delta = total2.saturating_sub(total1) as f64;
    if total_delta == 0.0 { return Ok(0.0); }
    Ok(((total_delta - idle_delta) / total_delta * 100.0).clamp(0.0, 100.0))
}

async fn read_memory_info() -> Result<MemoryInfo, AppError> {
    let content = tokio::fs::read_to_string("/proc/meminfo").await?;
    let mut total = 0u64;
    let mut available = 0u64;
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 { continue; }
        let val: u64 = parts[1].parse().unwrap_or(0);
        match parts[0].trim_end_matches(':') {
            "MemTotal" => total = val,
            "MemAvailable" => available = val,
            _ => {}
        }
    }
    let used = total.saturating_sub(available);
    let usage_pct = if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 };
    Ok(MemoryInfo { total_kb: total, used_kb: used, available_kb: available, usage_pct: (usage_pct * 100.0).round() / 100.0 })
}

async fn read_disk_info() -> Result<Vec<DiskInfo>, AppError> {
    let output = run_cmd("df", &["-BG", "--output=target,size,used,pcent", "/"]).await?;
    let mut disks = Vec::new();
    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 { continue; }
        disks.push(DiskInfo {
            mount: parts[0].to_string(),
            total_gb: parts[1].trim_end_matches('G').parse().unwrap_or(0.0),
            used_gb: parts[2].trim_end_matches('G').parse().unwrap_or(0.0),
            used_pct: parts[3].trim_end_matches('%').parse().unwrap_or(0.0),
        });
    }
    Ok(disks)
}

async fn read_load_avg() -> Result<(f64, f64, f64), AppError> {
    let content = tokio::fs::read_to_string("/proc/loadavg").await?;
    let p: Vec<&str> = content.split_whitespace().collect();
    if p.len() < 3 { return Ok((0.0, 0.0, 0.0)); }
    Ok((p[0].parse().unwrap_or(0.0), p[1].parse().unwrap_or(0.0), p[2].parse().unwrap_or(0.0)))
}

async fn read_service_statuses() -> Result<Vec<ServiceStatus>, AppError> {
    let svcs = ["nginx.service", "sshd.service", "nixos-rebuild.service", "firewall.service", "network-setup.service"];
    let mut out = Vec::new();
    for svc in &svcs {
        let active = run_cmd("systemctl", &["is-active", "--no-pager", svc]).await.unwrap_or("unknown".into()).trim().to_string();
        let sub = run_cmd("systemctl", &["show", "--property=SubState", "--value", svc]).await.unwrap_or("unknown".into()).trim().to_string();
        out.push(ServiceStatus { name: svc.to_string(), active, sub });
    }
    Ok(out)
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    format!("{}", now.as_secs())
}

pub async fn handle(ws: WebSocketUpgrade, Query(params): Query<DashboardQuery>, State(_state): AppStateRef) -> impl IntoResponse {
    let interval_secs = params.interval_secs.unwrap_or(3).clamp(1, 60);
    ws.on_upgrade(move |socket| handle_socket(socket, interval_secs))
}

async fn handle_socket(socket: WebSocket, interval_secs: u64) {
    let (mut sender, mut receiver) = socket.split();
    let mut ticker = interval(Duration::from_secs(interval_secs));
    let send_task = tokio::spawn(async move {
        loop {
            ticker.tick().await;
            match collect_metrics().await {
                Ok(metrics) => {
                    if let Ok(msg) = serde_json::to_string(&metrics) {
                        if sender.send(axum::extract::ws::Message::Text(msg.into())).await.is_err() { break; }
                    }
                }
                Err(e) => {
                    let err_msg = json!({"error": e.to_string(), "timestamp": chrono_now()});
                    let _ = sender.send(axum::extract::ws::Message::Text(serde_json::to_string(&err_msg).unwrap().into())).await;
                }
            }
        }
    });
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if matches!(msg, axum::extract::ws::Message::Close(_)) { break; }
        }
    });
    tokio::select! { _ = send_task => {}, _ = recv_task => {}, }
    tracing::info!("Dashboard WebSocket client disconnected");
}
