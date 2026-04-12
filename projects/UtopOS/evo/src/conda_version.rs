//! Conda Environment Versioning
//!
//! Git-like version control for environments.
//! Commit: capture current env state
//! Log: show version history
//! Checkout: restore a previous version

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

const VERSION_STORE_PATH: &str = "/var/lib/nix-evo/conda-versions";

/// Request to commit an environment version
#[derive(Debug, Deserialize)]
pub struct CommitRequest {
    pub env: String,
    pub host: Option<String>,
    pub message: Option<String>,
    pub tag: Option<String>,
}

/// Request to checkout a previous version
#[derive(Debug, Deserialize)]
pub struct CheckoutRequest {
    pub env: String,
    pub host: Option<String>,
    pub commit_id: String,
    pub restore: Option<bool>, // actually restore or just preview
}

/// Query for version log
#[derive(Debug, Deserialize)]
pub struct VersionLogQuery {
    pub env: String,
    pub host: Option<String>,
    pub limit: Option<usize>,
}

/// A single version snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionSnapshot {
    pub commit_id: String,
    pub env_name: String,
    pub message: String,
    pub tag: Option<String>,
    pub timestamp: String,
    pub package_count: usize,
    pub packages: BTreeMap<String, String>, // name → version
    pub python_version: Option<String>,
    pub fingerprint: Option<String>,
}

/// Commit result
#[derive(Debug, Clone, Serialize)]
pub struct CommitResult {
    pub commit_id: String,
    pub env_name: String,
    pub message: String,
    pub timestamp: String,
    pub package_count: usize,
    pub diff_from_previous: Option<VersionDiff>,
}

/// Diff between two versions
#[derive(Debug, Clone, Serialize)]
pub struct VersionDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub updated: Vec<VersionChange>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionChange {
    pub name: String,
    pub from: String,
    pub to: String,
}

/// Version log
#[derive(Debug, Clone, Serialize)]
pub struct VersionLog {
    pub env_name: String,
    pub total_commits: usize,
    pub snapshots: Vec<VersionSnapshot>,
}

/// Generate commit ID (short SHA-like)
fn generate_commit_id() -> String {
    let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    format!("{:08x}", now as u64)
}

/// Get version store directory for an environment
fn env_version_dir(env_name: &str) -> PathBuf {
    PathBuf::from(VERSION_STORE_PATH).join(env_name)
}

/// Load all snapshots for an environment
async fn load_snapshots(env_name: &str) -> Result<Vec<VersionSnapshot>, AppError> {
    let dir = env_version_dir(env_name);
    let index_path = dir.join("index.json");

    if !index_path.exists() {
        return Ok(vec![]);
    }

    let content = tokio::fs::read_to_string(&index_path).await?;
    let snapshots: Vec<VersionSnapshot> = serde_json::from_str(&content)?;
    Ok(snapshots)
}

/// Save snapshots index
async fn save_snapshots(env_name: &str, snapshots: &[VersionSnapshot]) -> Result<(), AppError> {
    let dir = env_version_dir(env_name);
    tokio::fs::create_dir_all(&dir).await?;
    let index_path = dir.join("index.json");
    let content = serde_json::to_string_pretty(snapshots)?;
    tokio::fs::write(&index_path, content).await?;
    Ok(())
}

/// Compute diff between two package sets
fn compute_diff(
    old_pkgs: &BTreeMap<String, String>,
    new_pkgs: &BTreeMap<String, String>,
) -> VersionDiff {
    let old_names: std::collections::BTreeSet<&str> = old_pkgs.keys().map(|s| s.as_str()).collect();
    let new_names: std::collections::BTreeSet<&str> = new_pkgs.keys().map(|s| s.as_str()).collect();

    let added: Vec<String> = new_names.difference(&old_names).map(|s| s.to_string()).collect();
    let removed: Vec<String> = old_names.difference(&new_names).map(|s| s.to_string()).collect();

    let updated: Vec<VersionChange> = old_names
        .intersection(&new_names)
        .filter(|name| old_pkgs.get(**name) != new_pkgs.get(**name))
        .map(|name| VersionChange {
            name: name.to_string(),
            from: old_pkgs.get(*name).cloned().unwrap_or_default(),
            to: new_pkgs.get(*name).cloned().unwrap_or_default(),
        })
        .collect();

    VersionDiff { added, removed, updated }
}

/// Simple fingerprint from package map
fn compute_fingerprint(packages: &BTreeMap<String, String>) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    for (name, version) in packages {
        hasher.update(format!("{}={}\n", name, version));
    }
    hex::encode(hasher.finalize())
}

/// POST /api/conda/version/commit — commit current env state
pub async fn commit_handler(
    state: AppStateRef,
    Json(body): Json<CommitRequest>,
) -> Result<Json<CommitResult>, AppError> {
    let backend = crate::conda::detect_backend().await?;

    // Get current packages
    let packages = conda::list_packages(&backend, &body.env).await?;
    let pkg_map: BTreeMap<String, String> = packages
        .iter()
        .map(|p| (p.name.clone(), p.version.clone()))
        .collect();

    let python_version = packages
        .iter()
        .find(|p| p.name == "python")
        .map(|p| p.version.clone());

    let commit_id = generate_commit_id();
    let timestamp = chrono::Utc::now().to_rfc3339();
    let fingerprint = compute_fingerprint(&pkg_map);

    // Load previous snapshots to compute diff
    let mut snapshots = load_snapshots(&body.env).await?;
    let diff = snapshots.last().map(|prev| compute_diff(&prev.packages, &pkg_map));

    let snapshot = VersionSnapshot {
        commit_id: commit_id.clone(),
        env_name: body.env.clone(),
        message: body.message.clone().unwrap_or_else(|| format!("Snapshot of {}", body.env)),
        tag: body.tag.clone(),
        timestamp: timestamp.clone(),
        package_count: packages.len(),
        packages: pkg_map,
        python_version,
        fingerprint: Some(fingerprint),
    };

    // Also save the full snapshot as a separate file
    let dir = env_version_dir(&body.env);
    tokio::fs::create_dir_all(&dir).await?;
    let snapshot_path = dir.join(format!("{}.json", commit_id));
    let snapshot_json = serde_json::to_string_pretty(&snapshot)?;
    tokio::fs::write(&snapshot_path, snapshot_json).await?;

    // Update index
    snapshots.push(snapshot);
    save_snapshots(&body.env, &snapshots).await?;

    Ok(Json(CommitResult {
        commit_id,
        env_name: body.env.clone(),
        message: body.message.unwrap_or_else(|| format!("Snapshot of {}", body.env)),
        timestamp,
        package_count: packages.len(),
        diff_from_previous: diff,
    }))
}

/// GET /api/conda/version/log — show version history
pub async fn log_handler(
    state: AppStateRef,
    Query(query): Query<VersionLogQuery>,
) -> Result<Json<VersionLog>, AppError> {
    let snapshots = load_snapshots(&query.env).await?;
    let limit = query.limit.unwrap_or(50);

    let total = snapshots.len();
    let recent: Vec<VersionSnapshot> = snapshots
        .into_iter()
        .rev()
        .take(limit)
        .collect();

    Ok(Json(VersionLog {
        env_name: query.env,
        total_commits: total,
        snapshots: recent,
    }))
}
