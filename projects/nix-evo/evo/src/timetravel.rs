use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::AppError;

// ─── State Snapshot ──────────────────────────────────────────────────────

/// A point-in-time snapshot of the system state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub timestamp: String,
    pub epoch: u64,
    pub label: Option<String>,
    pub services: Vec<ServiceState>,
    pub config_hash: String,
    pub packages: Vec<String>,
    pub disk_usage: Vec<DiskMount>,
    pub memory: MemoryState,
    pub network: NetworkState,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceState {
    pub name: String,
    pub status: String, // "active", "inactive", "failed"
    pub pid: Option<u64>,
    pub uptime_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMount {
    pub mount: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub used_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryState {
    pub total_mb: u64,
    pub used_mb: u64,
    pub available_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkState {
    pub interfaces: Vec<InterfaceState>,
    pub open_ports: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceState {
    pub name: String,
    pub ip: Option<String>,
    pub state: String,
}

// ─── Diff between two snapshots ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotDiff {
    pub from_id: String,
    pub to_id: String,
    pub from_time: String,
    pub to_time: String,
    pub time_delta_secs: u64,
    pub services_changed: Vec<ServiceDiff>,
    pub packages_added: Vec<String>,
    pub packages_removed: Vec<String>,
    pub disk_changes: Vec<DiskDiff>,
    pub memory_delta_mb: i64,
    pub config_changed: bool,
    pub open_ports_added: Vec<u16>,
    pub open_ports_removed: Vec<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceDiff {
    pub name: String,
    pub from_status: String,
    pub to_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskDiff {
    pub mount: String,
    pub delta_gb: f64,
    pub from_pct: f64,
    pub to_pct: f64,
}

// ─── Replay entry ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ReplayFrame {
    pub snapshot_id: String,
    pub timestamp: String,
    pub label: Option<String>,
    pub service_count: usize,
    pub failed_services: Vec<String>,
    pub memory_used_pct: f64,
    pub disk_max_pct: f64,
}

// ─── Time-Travel Engine ──────────────────────────────────────────────────

pub struct TimeTravelEngine {
    snapshots: RwLock<Vec<Snapshot>>,
    max_snapshots: usize,
}

impl TimeTravelEngine {
    pub fn new() -> Self {
        Self {
            snapshots: RwLock::new(Vec::new()),
            max_snapshots: 1000,
        }
    }

    /// Capture a snapshot of the current system state.
    pub async fn capture(&self, label: Option<String>) -> Result<Snapshot, AppError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let epoch = now.as_secs();
        let timestamp = format_timestamp(epoch);
        let id = format!("snap-{}-{}", epoch, nanoid());

        let services = capture_services().await;
        let disk_usage = capture_disk().await;
        let memory = capture_memory().await;
        let network = capture_network().await;
        let packages = capture_packages().await;
        let config_hash = capture_config_hash().await;

        let snapshot = Snapshot {
            id: id.clone(),
            timestamp,
            epoch,
            label,
            services,
            config_hash,
            packages,
            disk_usage,
            memory,
            network,
            metadata: HashMap::new(),
        };

        {
            let mut snaps = self.snapshots.write().await;
            snaps.push(snapshot.clone());
            if snaps.len() > self.max_snapshots {
                let excess = snaps.len() - self.max_snapshots;
                snaps.drain(0..excess);
            }
        }

        tracing::info!("Captured snapshot {}", id);
        Ok(snapshot)
    }

    /// List all snapshots.
    pub async fn list(&self) -> Vec<SnapshotSummary> {
        let snaps = self.snapshots.read().await;
        snaps
            .iter()
            .map(|s| SnapshotSummary {
                id: s.id.clone(),
                timestamp: s.timestamp.clone(),
                epoch: s.epoch,
                label: s.label.clone(),
                service_count: s.services.len(),
                failed_services: s
                    .services
                    .iter()
                    .filter(|svc| svc.status == "failed")
                    .map(|svc| svc.name.clone())
                    .collect(),
                config_hash: s.config_hash.clone(),
            })
            .collect()
    }

    /// Get a specific snapshot by ID.
    pub async fn get(&self, id: &str) -> Option<Snapshot> {
        let snaps = self.snapshots.read().await;
        snaps.iter().find(|s| s.id == id).cloned()
    }

    /// Compare two snapshots.
    pub async fn diff(&self, from_id: &str, to_id: &str) -> Result<SnapshotDiff, AppError> {
        let snaps = self.snapshots.read().await;
        let from = snaps
            .iter()
            .find(|s| s.id == from_id)
            .ok_or_else(|| AppError::NotFound {
                resource: format!("snapshot {from_id}"),
            })?;
        let to = snaps
            .iter()
            .find(|s| s.id == to_id)
            .ok_or_else(|| AppError::NotFound {
                resource: format!("snapshot {to_id}"),
            })?;

        let time_delta = if to.epoch > from.epoch {
            to.epoch - from.epoch
        } else {
            from.epoch - to.epoch
        };

        // Service diffs
        let mut services_changed = Vec::new();
        let from_svc: HashMap<&str, &ServiceState> =
            from.services.iter().map(|s| (s.name.as_str(), s)).collect();
        let to_svc: HashMap<&str, &ServiceState> =
            to.services.iter().map(|s| (s.name.as_str(), s)).collect();
        for (name, ts) in &to_svc {
            if let Some(fs) = from_svc.get(name) {
                if fs.status != ts.status {
                    services_changed.push(ServiceDiff {
                        name: name.to_string(),
                        from_status: fs.status.clone(),
                        to_status: ts.status.clone(),
                    });
                }
            }
        }

        // Package diffs
        let from_pkgs: std::collections::HashSet<&str> =
            from.packages.iter().map(|s| s.as_str()).collect();
        let to_pkgs: std::collections::HashSet<&str> =
            to.packages.iter().map(|s| s.as_str()).collect();
        let packages_added: Vec<String> = to_pkgs
            .difference(&from_pkgs)
            .map(|s| s.to_string())
            .collect();
        let packages_removed: Vec<String> = from_pkgs
            .difference(&to_pkgs)
            .map(|s| s.to_string())
            .collect();

        // Disk diffs
        let mut disk_changes = Vec::new();
        let from_disk: HashMap<&str, &DiskMount> =
            from.disk_usage.iter().map(|d| (d.mount.as_str(), d)).collect();
        for td in &to.disk_usage {
            if let Some(fd) = from_disk.get(td.mount.as_str()) {
                let delta = td.used_gb - fd.used_gb;
                if delta.abs() > 0.1 {
                    disk_changes.push(DiskDiff {
                        mount: td.mount.clone(),
                        delta_gb: delta,
                        from_pct: fd.used_pct,
                        to_pct: td.used_pct,
                    });
                }
            }
        }

        let memory_delta = to.memory.used_mb as i64 - from.memory.used_mb as i64;
        let config_changed = from.config_hash != to.config_hash;

        // Port diffs
        let from_ports: std::collections::HashSet<u16> =
            from.network.open_ports.iter().copied().collect();
        let to_ports: std::collections::HashSet<u16> =
            to.network.open_ports.iter().copied().collect();
        let open_ports_added: Vec<u16> = to_ports.difference(&from_ports).copied().collect();
        let open_ports_removed: Vec<u16> = from_ports.difference(&to_ports).copied().collect();

        Ok(SnapshotDiff {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            from_time: from.timestamp.clone(),
            to_time: to.timestamp.clone(),
            time_delta_secs: time_delta,
            services_changed,
            packages_added,
            packages_removed,
            disk_changes,
            memory_delta_mb: memory_delta,
            config_changed,
            open_ports_added,
            open_ports_removed,
        })
    }

    /// Replay: produce a sequence of frames for a time range.
    pub async fn replay(
        &self,
        from_epoch: Option<u64>,
        to_epoch: Option<u64>,
        limit: Option<usize>,
    ) -> Vec<ReplayFrame> {
        let snaps = self.snapshots.read().await;
        let limit = limit.unwrap_or(100);
        snaps
            .iter()
            .filter(|s| {
                let after_start = from_epoch.map_or(true, |from| s.epoch >= from);
                let before_end = to_epoch.map_or(true, |to| s.epoch <= to);
                after_start && before_end
            })
            .take(limit)
            .map(|s| {
                let mem_pct = if s.memory.total_mb > 0 {
                    (s.memory.used_mb as f64 / s.memory.total_mb as f64) * 100.0
                } else {
                    0.0
                };
                let disk_max = s
                    .disk_usage
                    .iter()
                    .map(|d| d.used_pct)
                    .fold(0.0f64, f64::max);
                ReplayFrame {
                    snapshot_id: s.id.clone(),
                    timestamp: s.timestamp.clone(),
                    label: s.label.clone(),
                    service_count: s.services.len(),
                    failed_services: s
                        .services
                        .iter()
                        .filter(|svc| svc.status == "failed")
                        .map(|svc| svc.name.clone())
                        .collect(),
                    memory_used_pct: mem_pct,
                    disk_max_pct: disk_max,
                }
            })
            .collect()
    }
}

// ─── Summary type for listing ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub timestamp: String,
    pub epoch: u64,
    pub label: Option<String>,
    pub service_count: usize,
    pub failed_services: Vec<String>,
    pub config_hash: String,
}

// ─── Capture helpers ─────────────────────────────────────────────────────

async fn capture_services() -> Vec<ServiceState> {
    let Ok(output) = tokio::process::Command::new("systemctl")
        .args(&["list-units", "--type=service", "--state=running,failed", "--no-pager", "--plain"])
        .output()
        .await
    else {
        return vec![];
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let name = parts[0].trim_end_matches(".service").to_string();
                let status = if parts.contains(&"failed") {
                    "failed".to_string()
                } else {
                    "active".to_string()
                };
                Some(ServiceState {
                    name,
                    status,
                    pid: None,
                    uptime_secs: None,
                })
            } else {
                None
            }
        })
        .collect()
}

async fn capture_disk() -> Vec<DiskMount> {
    let Ok(output) = tokio::process::Command::new("df")
        .args(&["-BG", "--output=target,size,used,pcent"])
        .output()
        .await
    else {
        return vec![];
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let total = parts[1].trim_end_matches('G').parse::<f64>().unwrap_or(0.0);
                let used = parts[2].trim_end_matches('G').parse::<f64>().unwrap_or(0.0);
                let pct = parts[3].trim_end_matches('%').parse::<f64>().unwrap_or(0.0);
                Some(DiskMount {
                    mount: parts[0].to_string(),
                    total_gb: total,
                    used_gb: used,
                    used_pct: pct,
                })
            } else {
                None
            }
        })
        .collect()
}

async fn capture_memory() -> MemoryState {
    let Ok(output) = tokio::process::Command::new("free")
        .args(&["-m"])
        .output()
        .await
    else {
        return MemoryState { total_mb: 0, used_mb: 0, available_mb: 0 };
    };
    let text = String::from_utf8_lossy(&output.stdout);
    if let Some(mem_line) = text.lines().find(|l| l.starts_with("Mem:")) {
        let parts: Vec<&str> = mem_line.split_whitespace().collect();
        if parts.len() >= 4 {
            return MemoryState {
                total_mb: parts[1].parse().unwrap_or(0),
                used_mb: parts[2].parse().unwrap_or(0),
                available_mb: parts[6].parse().unwrap_or(parts[3].parse().unwrap_or(0)),
            };
        }
    }
    MemoryState { total_mb: 0, used_mb: 0, available_mb: 0 }
}

async fn capture_network() -> NetworkState {
    let interfaces = if let Ok(output) = tokio::process::Command::new("ip")
        .args(&["-j", "addr"])
        .output()
        .await
    {
        let text = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str::<Vec<serde_json::Value>>(&text)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|iface| {
                let name = iface["if_name"].as_str()?;
                let state = iface["operstate"].as_str().unwrap_or("UNKNOWN");
                let ip = iface["addr_info"]
                    .as_array()
                    .and_then(|addrs| {
                        addrs.iter()
                            .find(|a| a["family"].as_str() == Some("inet"))
                            .and_then(|a| a["local"].as_str())
                            .map(|s| s.to_string())
                    });
                Some(InterfaceState { name: name.to_string(), ip, state: state.to_string() })
            })
            .collect()
    } else {
        vec![]
    };

    let open_ports = if let Ok(output) = tokio::process::Command::new("ss")
        .args(&["-tlnH"])
        .output()
        .await
    {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut ports: Vec<u16> = text.lines()
            .filter_map(|line| {
                line.split_whitespace().nth(3)
                    .and_then(|addr| addr.rsplit(':').next())
                    .and_then(|p| p.parse::<u16>().ok())
            })
            .collect();
        ports.sort_unstable();
        ports.dedup();
        ports
    } else {
        vec![]
    };

    NetworkState { interfaces, open_ports }
}

async fn capture_packages() -> Vec<String> {
    let Ok(output) = tokio::process::Command::new("nix-store")
        .args(&["-qR", "/run/current-system"])
        .output()
        .await
    else {
        return vec![];
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .filter_map(|line| {
            let name = line.split('/').last()?;
            if let Some(dash_pos) = name.find('-') {
                Some(name[dash_pos + 1..].to_string())
            } else {
                Some(name.to_string())
            }
        })
        .collect()
}

async fn capture_config_hash() -> String {
    let config_path = "/etc/nixos/configuration.nix";
    match tokio::fs::read(config_path).await {
        Ok(contents) => {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            contents.hash(&mut hasher);
            format!("{:016x}", hasher.finish())
        }
        Err(_) => "unavailable".to_string(),
    }
}

fn nanoid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{n:08x}")
}

fn format_timestamp(epoch: u64) -> String {
    let secs = epoch;
    let days = secs / 86400;
    let day_secs = secs % 86400;
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let seconds = day_secs % 60;
    let (y, m, d) = days_to_ymd(days as i64);
    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

// ─── Singleton ───────────────────────────────────────────────────────────

use std::sync::OnceLock;
pub static ENGINE: OnceLock<TimeTravelEngine> = OnceLock::new();

pub fn engine() -> &'static TimeTravelEngine {
    ENGINE.get_or_init(TimeTravelEngine::new)
}

// ─── HTTP Handlers ───────────────────────────────────────────────────────

/// POST /api/timetravel/snapshot
#[derive(Debug, Deserialize)]
pub struct SnapshotRequest {
    pub label: Option<String>,
}

pub async fn handle_snapshot(
    Json(req): Json<SnapshotRequest>,
) -> Result<Json<Snapshot>, AppError> {
    let snap = engine().capture(req.label).await?;
    Ok(Json(snap))
}

/// GET /api/timetravel/snapshots
pub async fn handle_list() -> Json<serde_json::Value> {
    let summaries = engine().list().await;
    Json(serde_json::json!({
        "count": summaries.len(),
        "snapshots": summaries,
    }))
}

/// GET /api/timetravel/diff
#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    pub from: String,
    pub to: String,
}

pub async fn handle_diff(Query(q): Query<DiffQuery>) -> Result<Json<SnapshotDiff>, AppError> {
    let diff = engine().diff(&q.from, &q.to).await?;
    Ok(Json(diff))
}

/// GET /api/timetravel/replay
#[derive(Debug, Deserialize)]
pub struct ReplayQuery {
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub limit: Option<usize>,
}

pub async fn handle_replay(Query(q): Query<ReplayQuery>) -> Json<serde_json::Value> {
    let frames = engine().replay(q.from, q.to, q.limit).await;
    Json(serde_json::json!({
        "frame_count": frames.len(),
        "frames": frames,
    }))
}
