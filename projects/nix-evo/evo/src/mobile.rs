use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::AppError;

// ─── Mobile-Optimized Types ──────────────────────────────────────────────

/// Ultra-compact status response for mobile clients.
/// Field names are single characters to minimize bandwidth.
#[derive(Debug, Serialize)]
pub struct MobileStatus {
    /// h: hostname
    pub h: String,
    /// s: overall status (o=ok, w=warning, c=critical)
    pub s: String,
    /// u: uptime in seconds
    pub u: u64,
    /// m: memory used %
    pub m: f64,
    /// d: disk used % (max across all mounts)
    pub d: f64,
    /// l: load average (1min)
    pub l: f64,
    /// f: failed services count
    pub f: usize,
    /// fs: failed service names (max 5)
    pub fs: Vec<String>,
    /// ts: timestamp (unix epoch)
    pub ts: u64,
    /// v: response version
    pub v: u8,
}

/// Compact service summary for mobile.
#[derive(Debug, Serialize)]
pub struct MobileService {
    pub n: String,  // name
    pub s: String,  // status: a=active, i=inactive, f=failed
}

/// Compact alert for push notifications.
#[derive(Debug, Clone, Serialize)]
pub struct MobileAlert {
    pub id: String,
    pub lv: String,    // level: i=info, w=warning, c=critical
    pub msg: String,   // short message (<140 chars)
    pub ts: u64,
    pub ack: bool,     // acknowledged
}

// ─── Alert Store ─────────────────────────────────────────────────────────

pub struct MobileAlertStore {
    alerts: RwLock<Vec<MobileAlert>>,
    subscribers: RwLock<Vec<String>>, // push tokens (simulated)
}

impl MobileAlertStore {
    pub fn new() -> Self {
        Self {
            alerts: RwLock::new(Vec::new()),
            subscribers: RwLock::new(Vec::new()),
        }
    }

    pub async fn add_alert(&self, level: &str, message: &str) {
        let alert = MobileAlert {
            id: format!("alert-{}", chrono_now()),
            lv: level.to_string(),
            msg: message.chars().take(140).collect(),
            ts: chrono_now_u64(),
            ack: false,
        };
        let mut alerts = self.alerts.write().await;
        alerts.push(alert);
        // Keep last 100
        if alerts.len() > 100 {
            let excess = alerts.len() - 100;
            alerts.drain(0..excess);
        }
    }

    pub async fn get_alerts(&self, unack_only: bool) -> Vec<MobileAlert> {
        let alerts = self.alerts.read().await;
        if unack_only {
            alerts.iter().filter(|a| !a.ack).cloned().collect()
        } else {
            alerts.clone()
        }
    }

    pub async fn acknowledge(&self, id: &str) -> bool {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == id) {
            alert.ack = true;
            true
        } else {
            false
        }
    }

    pub async fn subscribe(&self, token: String) {
        let mut subs = self.subscribers.write().await;
        if !subs.contains(&token) {
            subs.push(token);
        }
    }
}

// ─── Offline Sync State ─────────────────────────────────────────────────

/// Offline-first sync protocol for mobile clients.
#[derive(Debug, Serialize)]
pub struct OfflineSyncState {
    /// Sync token — changes if server state changes
    pub token: String,
    /// Server timestamp
    pub ts: u64,
    /// Changed resources since last sync
    pub changes: Vec<SyncChange>,
}

#[derive(Debug, Serialize)]
pub struct SyncChange {
    pub resource: String,    // "services", "alerts", "disk"
    pub action: String,      // "update", "delete"
    pub data: serde_json::Value,
}

// ─── Compact Status Builder ──────────────────────────────────────────────

async fn build_mobile_status() -> MobileStatus {
    let ts = chrono_now_u64();

    // Hostname
    let hostname = tokio::process::Command::new("hostname")
        .output()
        .await
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into());

    // Uptime
    let uptime = tokio::fs::read_to_string("/proc/uptime")
        .await
        .ok()
        .and_then(|c| c.split_whitespace().next().and_then(|s| s.parse::<f64>().ok()))
        .unwrap_or(0.0) as u64;

    // Memory
    let mem_pct = if let Ok(output) = tokio::process::Command::new("free")
        .output()
        .await
    {
        let text = String::from_utf8_lossy(&output.stdout);
        text.lines()
            .find(|l| l.starts_with("Mem:"))
            .and_then(|l| {
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() >= 3 {
                    let total: f64 = parts[1].parse().unwrap_or(1.0);
                    let used: f64 = parts[2].parse().unwrap_or(0.0);
                    Some((used / total) * 100.0)
                } else {
                    None
                }
            })
            .unwrap_or(0.0)
    } else {
        0.0
    };

    // Disk (max %)
    let disk_pct = if let Ok(output) = tokio::process::Command::new("df")
        .args(&["--output=pcent"])
        .output()
        .await
    {
        let text = String::from_utf8_lossy(&output.stdout);
        text.lines()
            .filter_map(|l| l.trim().trim_end_matches('%').parse::<f64>().ok())
            .fold(0.0f64, f64::max)
    } else {
        0.0
    };

    // Load average
    let load = tokio::fs::read_to_string("/proc/loadavg")
        .await
        .ok()
        .and_then(|c| c.split_whitespace().next().and_then(|s| s.parse::<f64>().ok()))
        .unwrap_or(0.0);

    // Failed services
    let (failed_count, failed_names) = if let Ok(output) = tokio::process::Command::new("systemctl")
        .args(&["list-units", "--state=failed", "--no-pager", "--plain", "--no-legend"])
        .output()
        .await
    {
        let text = String::from_utf8_lossy(&output.stdout);
        let names: Vec<String> = text.lines()
            .filter_map(|l| l.split_whitespace().next())
            .map(|s| s.trim_end_matches(".service").to_string())
            .take(5)
            .collect();
        (names.len(), names)
    } else {
        (0, vec![])
    };

    // Overall status
    let status = if failed_count > 2 || disk_pct > 90.0 || mem_pct > 95.0 {
        "c"
    } else if failed_count > 0 || disk_pct > 80.0 || mem_pct > 85.0 || load > 4.0 {
        "w"
    } else {
        "o"
    };

    MobileStatus {
        h: hostname,
        s: status.to_string(),
        u: uptime,
        m: (mem_pct * 10.0).round() / 10.0,
        d: (disk_pct * 10.0).round() / 10.0,
        l: (load * 100.0).round() / 100.0,
        f: failed_count,
        fs: failed_names,
        ts,
        v: 1,
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn chrono_now() -> String {
    chrono_now_u64().to_string()
}

fn chrono_now_u64() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ─── Singleton ───────────────────────────────────────────────────────────

use std::sync::OnceLock;
pub static ALERTS: OnceLock<MobileAlertStore> = OnceLock::new();

pub fn alerts() -> &'static MobileAlertStore {
    ALERTS.get_or_init(MobileAlertStore::new)
}

// ─── HTTP Handlers ───────────────────────────────────────────────────────

/// GET /api/mobile/status
pub async fn handle_status() -> Json<MobileStatus> {
    Json(build_mobile_status().await)
}

/// GET /api/mobile/alerts
#[derive(Debug, Deserialize)]
pub struct AlertQuery {
    pub unack: Option<bool>,
}

pub async fn handle_alerts(Query(q): Query<AlertQuery>) -> Json<serde_json::Value> {
    let unack_only = q.unack.unwrap_or(false);
    let alerts = alerts().get_alerts(unack_only).await;
    Json(serde_json::json!({
        "count": alerts.len(),
        "alerts": alerts,
    }))
}

/// POST /api/mobile/alerts/ack
#[derive(Debug, Deserialize)]
pub struct AckRequest {
    pub id: String,
}

pub async fn handle_acknowledge(Json(req): Json<AckRequest>) -> Json<serde_json::Value> {
    let ok = alerts().acknowledge(&req.id).await;
    Json(serde_json::json!({ "ok": ok }))
}

/// POST /api/mobile/subscribe
#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub token: String,
}

pub async fn handle_subscribe(Json(req): Json<SubscribeRequest>) -> Json<serde_json::Value> {
    alerts().subscribe(req.token).await;
    Json(serde_json::json!({ "subscribed": true }))
}

/// GET /api/mobile/sync
#[derive(Debug, Deserialize)]
pub struct SyncQuery {
    pub token: Option<String>,
}

pub async fn handle_sync(Query(_q): Query<SyncQuery>) -> Json<OfflineSyncState> {
    let status = build_mobile_status().await;
    let sync_token = format!("{:016x}", status.ts ^ (status.u as u64));

    let mut changes = Vec::new();

    // Include current status as a change
    changes.push(SyncChange {
        resource: "status".into(),
        action: "update".into(),
        data: serde_json::to_value(&status).unwrap_or_default(),
    });

    // Include alerts
    let alert_list = alerts().get_alerts(true).await;
    if !alert_list.is_empty() {
        changes.push(SyncChange {
            resource: "alerts".into(),
            action: "update".into(),
            data: serde_json::json!({ "unack_count": alert_list.len() }),
        });
    }

    Json(OfflineSyncState {
        token: sync_token,
        ts: status.ts,
        changes,
    })
}
