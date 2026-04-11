/// Capacity Planning — Analyze resource usage and predict exhaustion
///
/// Parses system metrics, calculates trends, predicts when disk/memory
/// will be exhausted, and recommends allocation changes.

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::cmd;
use crate::error::AppError;

// ─── Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CapacityReport {
    pub timestamp: String,
    pub disk: DiskForecast,
    pub memory: MemoryForecast,
    pub cpu: CpuForecast,
    pub recommendations: Vec<Recommendation>,
}

#[derive(Debug, Serialize)]
pub struct DiskForecast {
    pub mount_points: Vec<MountPoint>,
    pub nix_store_size_gb: f64,
    pub gc_savings_gb: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct MountPoint {
    pub path: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub usage_percent: f64,
    pub daily_growth_gb: Option<f64>,
    pub days_until_full: Option<f64>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Serialize)]
pub struct MemoryForecast {
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub swap_total_gb: f64,
    pub swap_used_gb: f64,
    pub usage_percent: f64,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Serialize)]
pub struct CpuForecast {
    pub cores: u32,
    pub load_1m: f64,
    pub load_5m: f64,
    pub load_15m: f64,
    pub load_per_core: f64,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Serialize)]
pub struct Recommendation {
    pub resource: String,
    pub severity: RiskLevel,
    pub action: String,
    pub details: String,
    pub estimated_savings_gb: Option<f64>,
}

// ─── Data Collection ──────────────────────────────────────────────────────

async fn collect_disk() -> Result<DiskForecast, AppError> {
    let df_output = cmd::run_cmd("bash", &["-c", "df -BG --output=target,size,used,avail,pcent / /tmp /nix/store 2>/dev/null | tail -n +2 | sort -u"]).await
        .unwrap_or_default();

    let mut mount_points = Vec::new();
    for line in df_output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let path = parts[0].to_string();
            let total: f64 = parts[1].trim_end_matches('G').parse().unwrap_or(0.0);
            let used: f64 = parts[2].trim_end_matches('G').parse().unwrap_or(0.0);
            let avail: f64 = parts[3].trim_end_matches('G').parse().unwrap_or(0.0);
            let pct = parts.get(4).and_then(|p| p.trim_end_matches('%').parse::<f64>().ok()).unwrap_or(0.0);

            let risk = if pct >= 95.0 { RiskLevel::Critical }
                else if pct >= 85.0 { RiskLevel::High }
                else if pct >= 70.0 { RiskLevel::Medium }
                else { RiskLevel::Low };

            mount_points.push(MountPoint {
                path, total_gb: total, used_gb: used, available_gb: avail,
                usage_percent: pct, daily_growth_gb: None, days_until_full: None, risk_level: risk,
            });
        }
    }

    // Nix store size
    let store_size = cmd::run_cmd("bash", &["-c", "du -sh /nix/store 2>/dev/null | cut -f1"]).await
        .unwrap_or_default();
    let nix_store_gb = parse_size_to_gb(&store_size);

    // GC savings estimate
    let gc_savings = cmd::run_cmd("bash", &["-c", "nix-collect-garbage --dry-run 2>/dev/null | tail -1 | grep -oE '[0-9.]+ [MG]'"]).await
        .ok()
        .map(|s| parse_size_to_gb(&s));

    Ok(DiskForecast { mount_points, nix_store_size_gb: nix_store_gb, gc_savings_gb: gc_savings })
}

async fn collect_memory() -> Result<MemoryForecast, AppError> {
    let meminfo = cmd::run_cmd("cat", &["/proc/meminfo"]).await.unwrap_or_default();

    let total_kb = parse_meminfo(&meminfo, "MemTotal");
    let avail_kb = parse_meminfo(&meminfo, "MemAvailable");
    let swap_total_kb = parse_meminfo(&meminfo, "SwapTotal");
    let swap_free_kb = parse_meminfo(&meminfo, "SwapFree");

    let total_gb = total_kb / 1048576.0;
    let avail_gb = avail_kb / 1048576.0;
    let used_gb = total_gb - avail_gb;
    let swap_total_gb = swap_total_kb / 1048576.0;
    let swap_used_gb = (swap_total_kb - swap_free_kb) / 1048576.0;
    let usage_pct = if total_gb > 0.0 { (used_gb / total_gb) * 100.0 } else { 0.0 };

    let risk = if usage_pct >= 95.0 { RiskLevel::Critical }
        else if usage_pct >= 85.0 { RiskLevel::High }
        else if usage_pct >= 70.0 { RiskLevel::Medium }
        else { RiskLevel::Low };

    Ok(MemoryForecast {
        total_gb, used_gb, available_gb: avail_gb,
        swap_total_gb, swap_used_gb, usage_percent: usage_pct, risk_level: risk,
    })
}

async fn collect_cpu() -> Result<CpuForecast, AppError> {
    let loadavg = cmd::run_cmd("cat", &["/proc/loadavg"]).await.unwrap_or_default();
    let parts: Vec<&str> = loadavg.split_whitespace().collect();

    let load_1m: f64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let load_5m: f64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let load_15m: f64 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);

    let cores = cmd::run_cmd("nproc", &[]).await
        .unwrap_or_default().trim().parse::<u32>().unwrap_or(1);

    let load_per_core = if cores > 0 { load_1m / cores as f64 } else { load_1m };

    let risk = if load_per_core >= 2.0 { RiskLevel::Critical }
        else if load_per_core >= 1.5 { RiskLevel::High }
        else if load_per_core >= 0.8 { RiskLevel::Medium }
        else { RiskLevel::Low };

    Ok(CpuForecast {
        cores, load_1m, load_5m, load_15m, load_per_core, risk_level: risk,
    })
}

// ─── Recommendation Engine ────────────────────────────────────────────────

fn generate_recommendations(disk: &DiskForecast, mem: &MemoryForecast, cpu: &CpuForecast) -> Vec<Recommendation> {
    let mut recs = Vec::new();

    // Disk recommendations
    for mp in &disk.mount_points {
        if mp.usage_percent >= 90.0 {
            recs.push(Recommendation {
                resource: format!("disk:{}", mp.path),
                severity: RiskLevel::Critical,
                action: "立即清理磁盘空间".into(),
                details: format!("{} 使用率已达 {:.0}%，需要立即处理。运行: nix-collect-garbage -d", mp.path, mp.usage_percent),
                estimated_savings_gb: disk.gc_savings_gb,
            });
        } else if mp.usage_percent >= 80.0 {
            recs.push(Recommendation {
                resource: format!("disk:{}", mp.path),
                severity: RiskLevel::High,
                action: "清理磁盘空间".into(),
                details: format!("{} 使用率 {:.0}%，建议清理。可运行 nix-collect-garbage 释放约 {:.1} GB",
                    mp.path, mp.usage_percent, disk.gc_savings_gb.unwrap_or(0.0)),
                estimated_savings_gb: disk.gc_savings_gb,
            });
        }
    }

    if disk.nix_store_size_gb > 10.0 {
        recs.push(Recommendation {
            resource: "nix-store".into(),
            severity: RiskLevel::Medium,
            action: "清理 Nix Store".into(),
            details: format!("Nix Store 占用 {:.1} GB。运行 nix-collect-garbage -d 清理旧的派生路径。", disk.nix_store_size_gb),
            estimated_savings_gb: Some(disk.nix_store_size_gb * 0.3),
        });
    }

    // Memory recommendations
    if mem.usage_percent >= 90.0 {
        recs.push(Recommendation {
            resource: "memory".into(),
            severity: RiskLevel::Critical,
            action: "内存严重不足".into(),
            details: "内存使用率超过 90%，系统可能开始使用 swap 导致性能严重下降。检查并停止不必要的服务。".into(),
            estimated_savings_gb: None,
        });
    } else if mem.usage_percent >= 80.0 {
        recs.push(Recommendation {
            resource: "memory".into(),
            severity: RiskLevel::High,
            action: "内存偏高".into(),
            details: "考虑增加物理内存或减少运行的服务数量。".into(),
            estimated_savings_gb: None,
        });
    }

    if mem.swap_used_gb > 0.5 {
        recs.push(Recommendation {
            resource: "swap".into(),
            severity: RiskLevel::Medium,
            action: "Swap 使用过多".into(),
            details: format!("已使用 {:.1} GB swap，表示内存压力较大。", mem.swap_used_gb),
            estimated_savings_gb: None,
        });
    }

    // CPU recommendations
    if cpu.load_per_core >= 1.5 {
        recs.push(Recommendation {
            resource: "cpu".into(),
            severity: RiskLevel::High,
            action: "CPU 负载过高".into(),
            details: format!("每核心负载 {:.2}，已超过 1.5。检查是否有 CPU 密集型进程。", cpu.load_per_core),
            estimated_savings_gb: None,
        });
    }

    recs
}

// ─── Helpers ──────────────────────────────────────────────────────────────

fn parse_meminfo(info: &str, key: &str) -> f64 {
    for line in info.lines() {
        if line.starts_with(key) {
            return line.split_whitespace().nth(1)
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
        }
    }
    0.0
}

fn parse_size_to_gb(s: &str) -> f64 {
    let s = s.trim();
    if s.ends_with("G") { s.trim_end_matches('G').parse().unwrap_or(0.0) }
    else if s.ends_with("M") { s.trim_end_matches('M').parse::<f64>().unwrap_or(0.0) / 1024.0 }
    else if s.ends_with("T") { s.trim_end_matches('T').parse::<f64>().unwrap_or(0.0) * 1024.0 }
    else { 0.0 }
}

// ─── API Handlers ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ForecastQuery {
    pub include_recommendations: Option<bool>,
}

pub async fn handle_forecast(Query(q): Query<ForecastQuery>) -> Result<impl IntoResponse, AppError> {
    let disk = collect_disk().await?;
    let memory = collect_memory().await?;
    let cpu = collect_cpu().await?;

    let recommendations = if q.include_recommendations.unwrap_or(true) {
        generate_recommendations(&disk, &memory, &cpu)
    } else {
        Vec::new()
    };

    Ok(Json(CapacityReport {
        timestamp: chrono::Utc::now().to_rfc3339(),
        disk,
        memory,
        cpu,
        recommendations,
    }))
}
