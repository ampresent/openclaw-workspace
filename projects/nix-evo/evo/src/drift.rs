use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::cmd::run_cmd;
use crate::error::AppError;

/// A detected drift from declared NixOS state
#[derive(Debug, Clone, Serialize)]
pub struct DriftEntry {
    pub path: String,
    pub kind: String,       // "file_modified", "file_missing", "file_extra", "service_state", "package_missing"
    pub severity: String,   // "info", "warning", "critical"
    pub declared: String,   // What nix declared
    pub actual: String,     // What's on disk
    pub fix_suggestion: String,
}

/// Full drift report
#[derive(Debug, Serialize)]
pub struct DriftReport {
    pub scan_time: String,
    pub generation: u64,
    pub total_drifts: usize,
    pub critical: usize,
    pub warnings: usize,
    pub infos: usize,
    pub drifts: Vec<DriftEntry>,
    pub health_score: f64,  // 0-100
}

#[derive(Debug, Deserialize)]
pub struct DriftQuery {
    pub paths: Option<Vec<String>>,
    pub depth: Option<usize>,
    pub include_services: Option<bool>,
}

/// Scan /etc for drift against the NixOS generation store
pub async fn scan_drift(query: &DriftQuery) -> Result<DriftReport, AppError> {
    let gen = get_current_generation().await?;
    let gen_path = format!("/nix/var/nix/profiles/system-{gen}-link");
    let mut drifts = Vec::new();

    // Default paths to scan
    let scan_paths = query.paths.clone().unwrap_or_else(|| vec![
        "/etc/nginx".into(),
        "/etc/ssh".into(),
        "/etc/postgresql".into(),
        "/etc/systemd/system".into(),
        "/etc/nixos".into(),
    ]);

    let depth = query.depth.unwrap_or(3);
    let include_services = query.include_services.unwrap_or(true);

    // 1. Check /etc files against generation's /etc
    let gen_etc = format!("{gen_path}/etc");
    for base_path in &scan_paths {
        if !Path::new(base_path).exists() {
            continue;
        }
        scan_directory_drift(base_path, &gen_etc, depth, &mut drifts).await;
    }

    // 2. Check service states against declared
    if include_services {
        scan_service_drift(&gen_path, &mut drifts).await;
    }

    // 3. Check for missing packages
    scan_package_drift(&gen_path, &mut drifts).await;

    // Calculate stats
    let critical = drifts.iter().filter(|d| d.severity == "critical").count();
    let warnings = drifts.iter().filter(|d| d.severity == "warning").count();
    let infos = drifts.iter().filter(|d| d.severity == "info").count();

    // Health score: start at 100, deduct per drift
    let health_score = (100.0
        - (critical as f64 * 15.0)
        - (warnings as f64 * 5.0)
        - (infos as f64 * 1.0))
        .max(0.0);

    Ok(DriftReport {
        scan_time: chrono_now(),
        generation: gen,
        total_drifts: drifts.len(),
        critical,
        warnings,
        infos,
        drifts,
        health_score,
    })
}

async fn scan_directory_drift(
    current_path: &str,
    gen_etc: &str,
    depth: usize,
    drifts: &mut Vec<DriftEntry>,
) {
    let mut entries = match tokio::fs::read_dir(current_path).await {
        Ok(e) => e,
        Err(_) => return,
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let path_str = path.to_string_lossy().to_string();

        if path.is_dir() && depth > 0 {
            Box::pin(scan_directory_drift(
                &path_str,
                gen_etc,
                depth - 1,
                drifts,
            )).await;
            continue;
        }

        if !path.is_file() {
            continue;
        }

        // Check if this file is managed by NixOS generation
        let relative = path_str.strip_prefix("/etc/").unwrap_or(&path_str);
        let gen_file = format!("{gen_etc}/{relative}");

        if !Path::new(&gen_file).exists() {
            // File exists in /etc but not in generation → extra file
            drifts.push(DriftEntry {
                path: path_str.clone(),
                kind: "file_extra".into(),
                severity: "info".into(),
                declared: "(not managed by NixOS)".into(),
                actual: "exists on disk".into(),
                fix_suggestion: format!("Consider adding to configuration.nix or ignore"),
            });
        } else {
            // Compare contents
            if let (Ok(current), Ok(declared)) = (
                tokio::fs::read_to_string(&path_str).await,
                tokio::fs::read_to_string(&gen_file).await,
            ) {
                if current != declared {
                    // Check if it's a symlink (NixOS uses symlinks)
                    let is_symlink = tokio::fs::symlink_metadata(&path_str)
                        .await
                        .map(|m| m.file_type().is_symlink())
                        .unwrap_or(false);

                    if !is_symlink {
                        drifts.push(DriftEntry {
                            path: path_str.clone(),
                            kind: "file_modified".into(),
                            severity: if is_critical_config(&path_str) {
                                "critical".into()
                            } else {
                                "warning".into()
                            },
                            declared: summarize(&declared),
                            actual: summarize(&current),
                            fix_suggestion: format!(
                                "File differs from NixOS generation. Run: nixos-rebuild switch"
                            ),
                        });
                    }
                }
            }
        }
    }
}

async fn scan_service_drift(gen_path: &str, drifts: &mut Vec<DriftEntry>) {
    // Get declared services from generation
    let units_path = format!("{gen_path}/etc/systemd/system");
    let mut entries = match tokio::fs::read_dir(&units_path).await {
        Ok(e) => e,
        Err(_) => return,
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if !name_str.ends_with(".service") || name_str.contains('@') {
            continue;
        }

        let service_name = name_str.trim_end_matches(".service");

        // Check actual state
        let state = match tokio::process::Command::new("systemctl")
            .args(&["is-active", &service_name])
            .output()
            .await
        {
            Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            Err(_) => "unknown".into(),
        };

        // Check if enabled
        let enabled = match tokio::process::Command::new("systemctl")
            .args(&["is-enabled", &service_name])
            .output()
            .await
        {
            Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            Err(_) => "unknown".into(),
        };

        // Detect drift: service should be active but isn't
        if state != "active" && state != "inactive" && state != "unknown" {
            drifts.push(DriftEntry {
                path: format!("systemd:{}", service_name),
                kind: "service_state".into(),
                severity: "warning".into(),
                declared: "managed by NixOS".into(),
                actual: format!("state={state}, enabled={enabled}"),
                fix_suggestion: format!("systemctl restart {service_name}"),
            });
        }
    }
}

async fn scan_package_drift(gen_path: &str, drifts: &mut Vec<DriftEntry>) {
    // Check if key binaries from the generation still exist
    let sw_path = format!("{gen_path}/sw/bin");
    let mut entries = match tokio::fs::read_dir(&sw_path).await {
        Ok(e) => e,
        Err(_) => return,
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();
        let system_bin = format!("/run/current-system/sw/bin/{name_str}");

        if !Path::new(&system_bin).exists() {
            drifts.push(DriftEntry {
                path: format!("package:{name_str}"),
                kind: "package_missing".into(),
                severity: "info".into(),
                declared: format!("present in generation {gen_path}"),
                actual: "missing from /run/current-system".into(),
                fix_suggestion: "nixos-rebuild switch".into(),
            });
        }
    }
}

fn is_critical_config(path: &str) -> bool {
    let critical = ["/etc/ssh/sshd_config", "/etc/sudoers", "/etc/passwd", "/etc/shadow"];
    critical.iter().any(|c| path.starts_with(c))
}

fn summarize(content: &str) -> String {
    let lines: Vec<&str> = content.lines().take(3).collect();
    let summary = lines.join(" | ");
    if summary.len() > 120 {
        format!("{}...", &summary[..117])
    } else {
        summary
    }
}

async fn get_current_generation() -> Result<u64, AppError> {
    let output = run_cmd("nix-env", &["-p", "/nix/var/nix/profiles/system", "--list-generations"]).await?;
    Ok(output
        .lines()
        .filter_map(|l| {
            if l.contains("(current)") {
                l.trim().split_whitespace().next()?.parse().ok()
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0))
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_default()
}

// ─── HTTP Handler ──────────────────────────────────────────────────────

/// GET /api/drift/scan
pub async fn handle_scan(Query(query): Query<DriftQuery>) -> Result<Json<DriftReport>, AppError> {
    let report = scan_drift(&query).await?;
    Ok(Json(report))
}
