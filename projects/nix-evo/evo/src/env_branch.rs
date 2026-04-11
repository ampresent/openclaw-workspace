//! Environment Branching & Cloning
//!
//! Branch an environment like git: create a copy for testing, diff branches,
//! merge branches back together. Uses conda env create --clone for real cloning.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::cmd::{AppStateRef, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Request to branch (clone) an environment
#[derive(Debug, Deserialize)]
pub struct BranchRequest {
    pub source: String,
    pub branch_name: String,
    pub description: Option<String>,
}

/// Request to diff two environment branches
#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    pub env_a: String,
    pub env_b: String,
}

/// Request to merge two branches
#[derive(Debug, Deserialize)]
pub struct MergeRequest {
    pub source: String,
    pub target: String,
    pub strategy: Option<String>, // "prefer-source", "prefer-target", "union"
}

/// Result of branching operation
#[derive(Debug, Clone, Serialize)]
pub struct BranchResult {
    pub source: String,
    pub branch_name: String,
    pub success: bool,
    pub path: Option<String>,
    pub package_count: usize,
    pub description: Option<String>,
}

/// Diff between two environment branches
#[derive(Debug, Clone, Serialize)]
pub struct EnvDiff {
    pub env_a: String,
    pub env_b: String,
    pub packages_only_in_a: Vec<DiffEntry>,
    pub packages_only_in_b: Vec<DiffEntry>,
    pub version_differences: Vec<VersionDifference>,
    pub channel_differences: Vec<ChannelDifference>,
    pub total_a: usize,
    pub total_b: usize,
    pub similarity_percent: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffEntry {
    pub name: String,
    pub version: String,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionDifference {
    pub name: String,
    pub version_a: String,
    pub version_b: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelDifference {
    pub name: String,
    pub channel_a: String,
    pub channel_b: String,
}

/// Result of merge operation
#[derive(Debug, Clone, Serialize)]
pub struct MergeResult {
    pub source: String,
    pub target: String,
    pub strategy: String,
    pub packages_added: Vec<String>,
    pub packages_updated: Vec<VersionDifference>,
    pub packages_conflict: Vec<String>,
    pub success: bool,
}

/// Branch (clone) a conda environment
pub async fn branch_env(backend: &str, req: &BranchRequest) -> Result<BranchResult, AppError> {
    tracing::info!("Branching env '{}' -> '{}'", req.source, req.branch_name);

    let _output = run_cmd(backend, &[
        "env", "create", "-n", &req.branch_name,
        "--clone", &req.source,
    ]).await?;

    let envs = conda::list_envs(backend).await?;
    let path = envs.iter()
        .find(|e| e.name == req.branch_name)
        .map(|e| e.path.clone());

    let packages = conda::list_packages(backend, &req.branch_name).await?;

    Ok(BranchResult {
        source: req.source.clone(),
        branch_name: req.branch_name.clone(),
        success: true,
        path,
        package_count: packages.len(),
        description: req.description.clone(),
    })
}

/// Diff two environment branches
pub async fn diff_envs(backend: &str, query: &DiffQuery) -> Result<EnvDiff, AppError> {
    let pkgs_a = conda::list_packages(backend, &query.env_a).await?;
    let pkgs_b = conda::list_packages(backend, &query.env_b).await?;

    let map_a: BTreeMap<String, &conda::CondaPackage> = pkgs_a.iter()
        .map(|p| (p.name.clone(), p)).collect();
    let map_b: BTreeMap<String, &conda::CondaPackage> = pkgs_b.iter()
        .map(|p| (p.name.clone(), p)).collect();

    let mut only_in_a = Vec::new();
    let mut only_in_b = Vec::new();
    let mut version_diffs = Vec::new();
    let mut channel_diffs = Vec::new();

    for (name, pkg_a) in &map_a {
        match map_b.get(name) {
            Some(pkg_b) => {
                if pkg_a.version != pkg_b.version {
                    version_diffs.push(VersionDifference {
                        name: name.clone(),
                        version_a: pkg_a.version.clone(),
                        version_b: pkg_b.version.clone(),
                    });
                }
                if pkg_a.channel != pkg_b.channel {
                    channel_diffs.push(ChannelDifference {
                        name: name.clone(),
                        channel_a: pkg_a.channel.clone(),
                        channel_b: pkg_b.channel.clone(),
                    });
                }
            }
            None => {
                only_in_a.push(DiffEntry {
                    name: name.clone(),
                    version: pkg_a.version.clone(),
                    channel: pkg_a.channel.clone(),
                });
            }
        }
    }

    for (name, pkg_b) in &map_b {
        if !map_a.contains_key(name) {
            only_in_b.push(DiffEntry {
                name: name.clone(),
                version: pkg_b.version.clone(),
                channel: pkg_b.channel.clone(),
            });
        }
    }

    let shared_count = map_a.len() - only_in_a.len();
    let total_unique = map_a.len() + map_b.len() - shared_count;
    let similarity = if total_unique > 0 {
        (shared_count as f64 / total_unique as f64) * 100.0
    } else {
        100.0
    };

    Ok(EnvDiff {
        env_a: query.env_a.clone(),
        env_b: query.env_b.clone(),
        packages_only_in_a: only_in_a,
        packages_only_in_b: only_in_b,
        version_differences: version_diffs,
        channel_differences: channel_diffs,
        total_a: map_a.len(),
        total_b: map_b.len(),
        similarity_percent: (similarity * 100.0).round() / 100.0,
    })
}

/// Merge two environment branches
pub async fn merge_envs(backend: &str, req: &MergeRequest) -> Result<MergeResult, AppError> {
    let strategy = req.strategy.as_deref().unwrap_or("prefer-source");
    tracing::info!("Merging '{}' into '{}' (strategy: {})", req.source, req.target, strategy);

    let pkgs_source = conda::list_packages(backend, &req.source).await?;
    let pkgs_target = conda::list_packages(backend, &req.target).await?;

    let target_map: BTreeMap<String, &conda::CondaPackage> = pkgs_target.iter()
        .map(|p| (p.name.clone(), p)).collect();

    let mut packages_added = Vec::new();
    let mut packages_updated = Vec::new();
    let mut packages_conflict = Vec::new();
    let mut to_install: Vec<String> = Vec::new();

    for pkg in &pkgs_source {
        match target_map.get(&pkg.name) {
            Some(target_pkg) => {
                if pkg.version != target_pkg.version {
                    match strategy {
                        "prefer-source" => {
                            to_install.push(format!("{}={}", pkg.name, pkg.version));
                            packages_updated.push(VersionDifference {
                                name: pkg.name.clone(),
                                version_a: pkg.version.clone(),
                                version_b: target_pkg.version.clone(),
                            });
                        }
                        "prefer-target" => {}
                        "union" => {
                            if pkg.version > target_pkg.version {
                                to_install.push(format!("{}={}", pkg.name, pkg.version));
                                packages_updated.push(VersionDifference {
                                    name: pkg.name.clone(),
                                    version_a: pkg.version.clone(),
                                    version_b: target_pkg.version.clone(),
                                });
                            }
                        }
                        _ => {
                            packages_conflict.push(format!(
                                "{}: {} vs {}", pkg.name, pkg.version, target_pkg.version
                            ));
                        }
                    }
                }
            }
            None => {
                to_install.push(format!("{}={}", pkg.name, pkg.version));
                packages_added.push(pkg.name.clone());
            }
        }
    }

    if !to_install.is_empty() {
        let pkg_refs: Vec<&str> = to_install.iter().map(|s| s.as_str()).collect();
        let mut args = vec!["install", "-n", &req.target, "-y"];
        args.extend_from_slice(&pkg_refs);
        let _ = run_cmd(backend, &args).await;
    }

    Ok(MergeResult {
        source: req.source.clone(),
        target: req.target.clone(),
        strategy: strategy.to_string(),
        packages_added,
        packages_updated,
        packages_conflict,
        success: true,
    })
}

// ─── Axum Handlers ────────────────────────────────────────────────────

pub async fn branch_handler(
    State(_state): AppStateRef,
    Json(req): Json<BranchRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = conda::detect_backend().await?;
    let result = branch_env(&backend, &req).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}

pub async fn diff_handler(
    State(_state): AppStateRef,
    Query(query): Query<DiffQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = conda::detect_backend().await?;
    let result = diff_envs(&backend, &query).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}

pub async fn merge_handler(
    State(_state): AppStateRef,
    Json(req): Json<MergeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = conda::detect_backend().await?;
    let result = merge_envs(&backend, &req).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}
