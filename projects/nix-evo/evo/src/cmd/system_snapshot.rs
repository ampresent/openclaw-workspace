use axum::Json;
use serde::Serialize;
use super::*;

#[derive(Serialize)]
pub struct SnapshotResponse {
    hostname: String,
    nixos_version: String,
    kernel: String,
    uptime: String,
    services: Vec<ServiceStatus>,
    disk: Vec<DiskUsage>,
    memory: MemoryInfo,
    recent_failures: Vec<FailedService>,
}

#[derive(Serialize)]
pub struct ServiceStatus {
    name: String,
    active: String,
    sub: String,
    description: String,
}

#[derive(Serialize)]
pub struct DiskUsage {
    mount: String,
    used_pct: u64,
}

#[derive(Serialize)]
pub struct MemoryInfo {
    total: String,
    used: String,
    available: String,
}

#[derive(Serialize)]
pub struct FailedService {
    unit: String,
    since: String,
    log_excerpt: String,
}

pub async fn handle(
    State(_state): AppStateRef,
    Query(_query): Query<HostQuery>,
) -> Result<Json<SnapshotResponse>, AppError> {
    // Hostname
    let hostname = run_cmd("hostname", &[])
        .await
        .unwrap_or_else(|_| String::new())
        .trim()
        .to_string();

    // NixOS version
    let nixos_version = run_cmd("nixos-version", &["--configuration-revision"])
        .await
        .or_else(|_| run_cmd("nixos-version", &[]))
        .unwrap_or_default()
        .trim()
        .to_string();

    // Kernel
    let kernel = run_cmd("uname", &["-r"])
        .await
        .unwrap_or_default()
        .trim()
        .to_string();

    // Uptime
    let uptime = run_cmd("uptime", &["-p"])
        .await
        .unwrap_or_default()
        .trim()
        .to_string();

    // List running services
    let services_raw = run_cmd(
        "systemctl",
        &["list-units", "--type=service", "--state=running", "--no-pager", "--plain"],
    )
    .await
    .unwrap_or_default();

    let services: Vec<ServiceStatus> = services_raw
        .lines()
        .filter(|l| l.contains(".service"))
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                Some(ServiceStatus {
                    name: parts[0].to_string(),
                    active: parts[2].to_string(),
                    sub: parts[3].to_string(),
                    description: parts[4..].join(" "),
                })
            } else {
                None
            }
        })
        .collect();

    // Disk usage
    let df_raw = run_cmd("df", &["-h", "--output=target,pcent"])
        .await
        .unwrap_or_default();
    let disk: Vec<DiskUsage> = df_raw
        .lines()
        .skip(1)
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let pct = parts[1].trim_end_matches('%').parse::<u64>().ok()?;
                Some(DiskUsage {
                    mount: parts[0].to_string(),
                    used_pct: pct,
                })
            } else {
                None
            }
        })
        .collect();

    // Memory
    let mem_raw = run_cmd("free", &["-h"]).await.unwrap_or_default();
    let memory = parse_memory(&mem_raw);

    // Recent failures
    let failed_raw = run_cmd(
        "systemctl",
        &["list-units", "--type=service", "--state=failed", "--no-pager", "--plain"],
    )
    .await
    .unwrap_or_default();

    let recent_failures: Vec<FailedService> = failed_raw
        .lines()
        .filter(|l| l.contains(".service"))
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                return None;
            }
            let unit = parts[0].to_string();
            let log = get_last_log(&unit);
            Some(FailedService {
                unit,
                since: "recent".to_string(),
                log_excerpt: log,
            })
        })
        .collect();

    Ok(Json(SnapshotResponse {
        hostname,
        nixos_version,
        kernel,
        uptime,
        services,
        disk,
        memory,
        recent_failures,
    }))
}

fn parse_memory(raw: &str) -> MemoryInfo {
    for line in raw.lines() {
        if line.starts_with("Mem:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                return MemoryInfo {
                    total: parts[1].to_string(),
                    used: parts[2].to_string(),
                    // free -h on newer versions puts available at index 6
                    available: if parts.len() >= 7 {
                        parts[6].to_string()
                    } else {
                        parts[3].to_string()
                    },
                };
            }
        }
    }
    MemoryInfo {
        total: "未知".into(),
        used: "未知".into(),
        available: "未知".into(),
    }
}

fn get_last_log(unit: &str) -> String {
    std::process::Command::new("journalctl")
        .args(["-u", unit, "-n", "3", "--no-pager", "-q"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}
