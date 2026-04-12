//! Auto-Conda-Forge Monitor
//!
//! Watch conda-forge for new versions of pinned packages.
//! Alert when a pinned package has a newer version available.
//! Auto-generate PR-like change proposals.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Watch configuration — pinned packages to monitor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    pub packages: Vec<PinnedPackage>,
    pub check_interval_hours: Option<u64>,
    pub channels: Vec<String>,
    pub notify_on: Option<String>, // "major", "minor", "patch", "any"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedPackage {
    pub name: String,
    pub current_version: String,
    pub channel: Option<String>,
    pub pin_type: Option<String>, // "exact", "minor", "any"
}

/// Status of a watched package
#[derive(Debug, Clone, Serialize)]
pub struct PackageWatchStatus {
    pub name: String,
    pub pinned_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub version_jump: String, // "major", "minor", "patch"
    pub channel: String,
    pub last_checked: String,
}

/// Change proposal (PR-like)
#[derive(Debug, Clone, Serialize)]
pub struct ChangeProposal {
    pub title: String,
    pub description: String,
    pub packages: Vec<ProposedChange>,
    pub risk_level: String,
    pub test_command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposedChange {
    pub name: String,
    pub from_version: String,
    pub to_version: String,
    pub changelog_url: Option<String>,
}

/// Full watch report
#[derive(Debug, Clone, Serialize)]
pub struct WatchReport {
    pub generated_at: String,
    pub total_pinned: usize,
    pub packages_with_updates: usize,
    pub packages_up_to_date: usize,
    pub statuses: Vec<PackageWatchStatus>,
    pub change_proposal: Option<ChangeProposal>,
    pub errors: Vec<String>,
}

const WATCH_CONFIG_PATH: &str = "/var/lib/nix-evo/conda-watch.json";

/// Classify version jump: major, minor, patch
fn classify_version_jump(from: &str, to: &str) -> String {
    let parse_version = |v: &str| -> Vec<u32> {
        v.trim_start_matches(|c: char| !c.is_ascii_digit())
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let fv = parse_version(from);
    let tv = parse_version(to);

    if fv.is_empty() || tv.is_empty() {
        return "unknown".to_string();
    }

    if fv[0] != tv[0] {
        "major".to_string()
    } else if fv.get(1) != tv.get(1) {
        "minor".to_string()
    } else {
        "patch".to_string()
    }
}

/// Check conda-forge for latest version of a package
async fn check_latest_version(
    backend: &str,
    package: &str,
    channel: &str,
) -> Result<String, AppError> {
    let args = ["search", "--channel", channel, "--json", package];
    let output = run_cmd(backend, &args).await?;

    // Parse JSON output to find latest version
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        if let Some(packages) = json.get("packages").and_then(|p| p.as_object()) {
            let mut versions: Vec<String> = packages
                .values()
                .filter_map(|v| v.get("version").and_then(|ver| ver.as_str()).map(String::from))
                .collect();
            versions.sort();
            if let Some(latest) = versions.last() {
                return Ok(latest.clone());
            }
        }
        // Alternative format: array of results
        if let Some(results) = json.as_array() {
            let mut versions: Vec<String> = results
                .iter()
                .filter_map(|v| v.get("version").and_then(|ver| ver.as_str()).map(String::from))
                .collect();
            versions.sort();
            if let Some(latest) = versions.last() {
                return Ok(latest.clone());
            }
        }
    }

    // Fallback: parse text output
    for line in output.lines().rev() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[0] == package {
            return Ok(parts[1].to_string());
        }
    }

    Err(AppError::Internal { message(format!(
        "Could not find version info for {} on {}",
        package, channel
    )))
}

/// Generate a change proposal from watch statuses
fn generate_proposal(statuses: &[PackageWatchStatus]) -> Option<ChangeProposal> {
    let updates: Vec<&PackageWatchStatus> = statuses.iter().filter(|s| s.has_update).collect();

    if updates.is_empty() {
        return None;
    }

    let changes: Vec<ProposedChange> = updates
        .iter()
        .map(|s| ProposedChange {
            name: s.name.clone(),
            from_version: s.pinned_version.clone(),
            to_version: s.latest_version.clone(),
            changelog_url: Some(format!(
                "https://github.com/conda-forge/{}-feedstock",
                s.name
            )),
        })
        .collect();

    let has_major = updates.iter().any(|s| s.version_jump == "major");
    let risk_level = if has_major { "high" } else { "low" };

    let pkg_names: Vec<&str> = changes.iter().map(|c| c.name.as_str()).collect();

    Some(ChangeProposal {
        title: format!("Update {} conda package(s)", changes.len()),
        description: format!(
            "Automated version update for: {}",
            pkg_names.join(", ")
        ),
        packages: changes,
        risk_level: risk_level.to_string(),
        test_command: format!(
            "micromamba install --dry-run --yes {}",
            pkg_names.join(" ")
        ),
    })
}

/// Load watch config from disk or return default
async fn load_watch_config() -> Result<WatchConfig, AppError> {
    let path = PathBuf::from(WATCH_CONFIG_PATH);
    if path.exists() {
        let content = tokio::fs::read_to_string(&path).await?;
        let config: WatchConfig = serde_json::from_str(&content)?;
        Ok(config)
    } else {
        // Return empty config
        Ok(WatchConfig {
            packages: vec![],
            check_interval_hours: Some(24),
            channels: vec!["conda-forge".to_string()],
            notify_on: Some("any".to_string()),
        })
    }
}

/// Save watch config to disk
async fn save_watch_config(config: &WatchConfig) -> Result<(), AppError> {
    let path = PathBuf::from(WATCH_CONFIG_PATH);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let content = serde_json::to_string_pretty(config)?;
    tokio::fs::write(&path, content).await?;
    Ok(())
}

/// Run full watch check
pub async fn run_watch_check(backend: &str) -> Result<WatchReport, AppError> {
    let config = load_watch_config().await?;
    let mut statuses = Vec::new();
    let mut errors = Vec::new();

    let now = chrono::Utc::now().to_rfc3339();

    for pinned in &config.packages {
        let channel = pinned
            .channel
            .as_deref()
            .or(config.channels.first().map(|s| s.as_str()))
            .unwrap_or("conda-forge");

        match check_latest_version(backend, &pinned.name, channel).await {
            Ok(latest) => {
                let has_update = latest != pinned.current_version;
                let version_jump = if has_update {
                    classify_version_jump(&pinned.current_version, &latest)
                } else {
                    "none".to_string()
                };

                statuses.push(PackageWatchStatus {
                    name: pinned.name.clone(),
                    pinned_version: pinned.current_version.clone(),
                    latest_version: latest,
                    has_update,
                    version_jump,
                    channel: channel.to_string(),
                    last_checked: now.clone(),
                });
            }
            Err(e) => {
                errors.push(format!("{}: {}", pinned.name, e));
                statuses.push(PackageWatchStatus {
                    name: pinned.name.clone(),
                    pinned_version: pinned.current_version.clone(),
                    latest_version: "unknown".to_string(),
                    has_update: false,
                    version_jump: "unknown".to_string(),
                    channel: channel.to_string(),
                    last_checked: now.clone(),
                });
            }
        }
    }

    let with_updates = statuses.iter().filter(|s| s.has_update).count();
    let up_to_date = statuses.len() - with_updates;
    let proposal = generate_proposal(&statuses);

    Ok(WatchReport {
        generated_at: now,
        total_pinned: config.packages.len(),
        packages_with_updates: with_updates,
        packages_up_to_date: up_to_date,
        statuses,
        change_proposal: proposal,
        errors,
    })
}

/// GET /api/conda/watch — get watch config & last status
pub async fn watch_handler(
    state: AppStateRef,
) -> Result<Json<WatchConfig>, AppError> {
    let config = load_watch_config().await?;
    Ok(Json(config))
}

/// POST /api/conda/watch/check — run an immediate check
pub async fn watch_check_handler(
    state: AppStateRef,
) -> Result<Json<WatchReport>, AppError> {
    let backend = crate::conda::detect_backend().await?;
    let report = run_watch_check(&backend).await?;
    Ok(Json(report))
}
