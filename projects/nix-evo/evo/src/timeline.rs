use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::cmd::{run_cmd, read_generation_description};
use crate::error::AppError;

/// A single NixOS generation entry
#[derive(Debug, Clone, Serialize)]
pub struct Generation {
    pub number: u64,
    pub date: String,
    pub description: String,
    pub nixos_version: String,
    pub kernel_version: String,
    pub is_current: bool,
    pub risk_level: String, // "low", "medium", "high", "critical"
    pub changes: Vec<ChangeEntry>,
}

/// A detected change between generations
#[derive(Debug, Clone, Serialize)]
pub struct ChangeEntry {
    pub category: String, // "service", "package", "kernel", "config"
    pub summary: String,
    pub severity: String, // "info", "warning", "breaking"
}

/// Comparison between two generations
#[derive(Debug, Clone, Serialize)]
pub struct GenerationDiff {
    pub from: u64,
    pub to: u64,
    pub added_services: Vec<String>,
    pub removed_services: Vec<String>,
    pub changed_packages: Vec<String>,
    pub config_diff_summary: String,
}

#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct CompareQuery {
    pub from: u64,
    pub to: u64,
}

/// List all NixOS generations
pub async fn list_generations() -> Result<Vec<Generation>, AppError> {
    let output = run_cmd("nixos-rebuild", &["list-generations"]).await
        .or_else(|_| {
            // Fallback: read from /nix/var/nix/profiles
            Err(AppError::CommandFailed {
                command: "nixos-rebuild".into(),
                message: "Fallback to direct listing".into(),
            })
        });

    let raw = match output {
        Ok(o) => o,
        Err(_) => {
            // Fallback: parse directory listing
            let dir = tokio::fs::read_dir("/nix/var/nix/profiles").await
                .map_err(|e| AppError::IoError {
                    path: "/nix/var/nix/profiles".into(),
                    message: e.to_string(),
                })?;
            return parse_generations_from_dir(dir).await;
        }
    };

    parse_generation_output(&raw)
}

async fn parse_generations_from_dir(
    mut dir: tokio::fs::ReadDir,
) -> Result<Vec<Generation>, AppError> {
    let mut generations = Vec::new();
    let mut current_gen = 0u64;

    // Find current generation
    if let Ok(link) = tokio::fs::read_link("/nix/var/nix/profiles/system").await {
        if let Some(name) = link.file_name() {
            let name_str = name.to_string_lossy();
            // system-123-link
            if let Some(num) = name_str.strip_prefix("system-").and_then(|s| s.strip_suffix("-link")) {
                current_gen = num.parse().unwrap_or(0);
            }
        }
    }

    while let Ok(Some(entry)) = dir.next_entry().await {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if let Some(num_str) = name_str.strip_prefix("system-").and_then(|s| s.strip_suffix("-link")) {
            if let Ok(num) = num_str.parse::<u64>() {
                let metadata = entry.metadata().await.ok();
                let date = metadata
                    .and_then(|m| m.modified().ok())
                    .map(|t| format_system_time(&t))
                    .unwrap_or_else(|| "unknown".into());

                let description = read_generation_description(num);
                let risk = assess_risk(num, &generations);

                generations.push(Generation {
                    number: num,
                    date,
                    description,
                    nixos_version: get_nixos_version(num).await,
                    kernel_version: get_kernel_version(num).await,
                    is_current: num == current_gen,
                    risk_level: risk,
                    changes: vec![],
                });
            }
        }
    }

    generations.sort_by_key(|g| g.number);
    Ok(generations)
}

fn parse_generation_output(output: &str) -> Result<Vec<Generation>, AppError> {
    let mut generations = Vec::new();
    let mut current_gen = 0u64;

    // Detect current generation
    if let Ok(o) = std::process::Command::new("nixos-rebuild").arg("list-generations").output() {
        let out = String::from_utf8_lossy(&o.stdout);
        for line in out.lines() {
            if line.contains("(current)") {
                if let Some(num) = line.trim().split_whitespace().next() {
                    current_gen = num.parse().unwrap_or(0);
                }
            }
        }
    }

    for line in output.lines() {
        let parts: Vec<&str> = line.trim().splitn(3, |c: char| c.is_whitespace()).collect();
        if parts.len() >= 2 {
            if let Ok(num) = parts[0].parse::<u64>() {
                let date = parts.get(1).unwrap_or(&"").to_string();
                let description = parts.get(2).unwrap_or(&"").trim().to_string();
                generations.push(Generation {
                    number: num,
                    date,
                    description,
                    nixos_version: String::new(),
                    kernel_version: String::new(),
                    is_current: num == current_gen,
                    risk_level: "low".into(),
                    changes: vec![],
                });
            }
        }
    }

    Ok(generations)
}

fn assess_risk(gen_num: u64, previous: &[Generation]) -> String {
    // Heuristic risk assessment
    let recent_count = previous.len();
    if recent_count == 0 {
        return "low".into();
    }

    // If many generations in short time, something might be wrong
    if recent_count > 20 {
        "medium".into()
    } else if recent_count > 50 {
        "high".into()
    } else {
        "low".into()
    }
}

async fn get_nixos_version(gen: u64) -> String {
    let path = format!("/nix/var/nix/profiles/system-{gen}-link/nixos-version");
    tokio::fs::read_to_string(&path).await.unwrap_or_default().trim().to_string()
}

async fn get_kernel_version(gen: u64) -> String {
    let path = format!("/nix/var/nix/profiles/system-{gen}-link/kernel");
    if let Ok(link) = tokio::fs::read_link(&path).await {
        if let Some(name) = link.file_name() {
            return name.to_string_lossy().to_string();
        }
    }
    String::new()
}

fn format_system_time(time: &std::time::SystemTime) -> String {
    let duration = time.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    // Simple formatting: YYYY-MM-DD HH:MM:SS
    let days = secs / 86400;
    let _years = 1970 + days / 365;
    // Use chrono if available, else simple format
    format!("{secs}")
}

/// Compare two generations
pub async fn compare_generations(from: u64, to: u64) -> Result<GenerationDiff, AppError> {
    // Try to run nix store diff-closures
    let diff_cmd = format!("nix store diff-closures /nix/var/nix/profiles/system-{from}-link /nix/var/nix/profiles/system-{to}-link");
    let diff_output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&diff_cmd)
        .output()
        .await;

    let diff_text = match diff_output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => format!("Generation {from} → {to} (diff unavailable)"),
    };

    // Parse added/removed from diff output
    let mut added_services = Vec::new();
    let mut removed_services = Vec::new();
    let mut changed_packages = Vec::new();

    for line in diff_text.lines() {
        if line.contains("Added:") || line.contains("+") {
            if let Some(pkg) = line.split(':').nth(1) {
                let pkg = pkg.trim().to_string();
                if !pkg.is_empty() {
                    if pkg.contains("service") || pkg.contains("nginx") || pkg.contains("ssh") {
                        added_services.push(pkg.clone());
                    }
                    changed_packages.push(pkg);
                }
            }
        }
        if line.contains("Removed:") || line.contains("-") {
            if let Some(pkg) = line.split(':').nth(1) {
                let pkg = pkg.trim().to_string();
                if !pkg.is_empty() {
                    removed_services.push(pkg);
                }
            }
        }
    }

    Ok(GenerationDiff {
        from,
        to,
        added_services,
        removed_services,
        changed_packages,
        config_diff_summary: diff_text.lines().take(20).collect::<Vec<_>>().join("\n"),
    })
}

// ─── HTTP Handlers ─────────────────────────────────────────────────────

/// GET /api/timeline?limit=50
pub async fn handle_timeline(Query(query): Query<TimelineQuery>) -> Result<Json<serde_json::Value>, AppError> {
    let mut generations = list_generations().await?;
    if let Some(limit) = query.limit {
        let start = generations.len().saturating_sub(limit);
        generations = generations[start..].to_vec();
    }

    Ok(Json(serde_json::json!({
        "total": generations.len(),
        "generations": generations,
    })))
}

/// GET /api/timeline/compare?from=42&to=45
pub async fn handle_compare(Query(query): Query<CompareQuery>) -> Result<Json<GenerationDiff>, AppError> {
    let diff = compare_generations(query.from, query.to).await?;
    Ok(Json(diff))
}
