//! Environment Fingerprinting
//!
//! Create a unique hash for each environment based on packages + versions + python version.
//! Detect if two environments on different machines are "the same".
//! Track environment evolution over time (fingerprint history).

use axum::Json;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Fingerprint of a conda environment — captures identity and content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnvFingerprint {
    pub environment: String,
    pub hash: String,
    pub short_hash: String,
    pub python_version: Option<String>,
    pub package_count: usize,
    pub packages: BTreeMap<String, String>, // name -> version
    pub channels: Vec<String>,
    pub backend: String,
    pub timestamp: String,
    pub platform: Option<String>,
}

/// Snapshot for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintSnapshot {
    pub environment: String,
    pub hash: String,
    pub short_hash: String,
    pub timestamp: String,
    pub package_count: usize,
    pub python_version: Option<String>,
}

/// Compare result between two fingerprints
#[derive(Debug, Clone, Serialize)]
pub struct FingerprintCompare {
    pub env_a: String,
    pub env_b: String,
    pub hash_a: String,
    pub hash_b: String,
    pub identical: bool,
    pub shared_packages: Vec<PackageDiff>,
    pub only_in_a: Vec<String>,
    pub only_in_b: Vec<String>,
    pub version_diffs: Vec<VersionDiff>,
    pub similarity_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageDiff {
    pub name: String,
    pub version_a: String,
    pub version_b: String,
    pub version_match: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionDiff {
    pub name: String,
    pub version_a: String,
    pub version_b: String,
}

// ─── Core Fingerprinting ──────────────────────────────────────────────

/// Compute a fingerprint for a named environment
pub async fn compute_fingerprint(env_name: &str) -> Result<EnvFingerprint, AppError> {
    let backend = conda::detect_backend().await?;
    let packages = conda::list_packages(&backend, env_name).await?;

    if packages.is_empty() {
        return Err(AppError::NotFound {
            resource: format!("environment '{env_name}' or it has no packages"),
        });
    }

    let mut pkg_map = BTreeMap::new();
    let mut python_version = None;
    let mut channels = std::collections::HashSet::new();

    for pkg in &packages {
        pkg_map.insert(pkg.name.clone(), pkg.version.clone());
        if pkg.name == "python" {
            python_version = Some(pkg.version.clone());
        }
        if pkg.channel != "unknown" {
            channels.insert(pkg.channel.clone());
        }
    }

    let mut sorted_channels: Vec<String> = channels.into_iter().collect();
    sorted_channels.sort();

    // Compute deterministic hash
    let hash = compute_hash(&pkg_map, &python_version, &sorted_channels);
    let short_hash = hash[..12].to_string();

    let platform = get_platform_info().await.ok();

    Ok(EnvFingerprint {
        environment: env_name.to_string(),
        hash,
        short_hash,
        python_version,
        package_count: packages.len(),
        packages: pkg_map,
        channels: sorted_channels,
        backend,
        timestamp: chrono::Utc::now().to_rfc3339(),
        platform,
    })
}

/// Deterministic hash from sorted packages + python version + channels
fn compute_hash(
    packages: &BTreeMap<String, String>,
    python_version: &Option<String>,
    channels: &[String],
) -> String {
    let mut hasher = Sha256::new();

    // Python version first (if present)
    if let Some(py) = python_version {
        hasher.update(format!("python:{py}").as_bytes());
    }

    // Channels
    for ch in channels {
        hasher.update(format!("channel:{ch}").as_bytes());
    }

    // Packages in sorted order (BTreeMap guarantees this)
    for (name, version) in packages {
        hasher.update(format!("{name}:{version}").as_bytes());
    }

    hex::encode(hasher.finalize())
}

/// Compare two fingerprints and report differences
pub fn compare_fingerprints(a: &EnvFingerprint, b: &EnvFingerprint) -> FingerprintCompare {
    let all_keys: std::collections::BTreeSet<_> = a
        .packages
        .keys()
        .chain(b.packages.keys())
        .cloned()
        .collect();

    let mut shared = Vec::new();
    let mut only_a = Vec::new();
    let mut only_b = Vec::new();
    let mut version_diffs = Vec::new();
    let mut matching_count = 0usize;

    for name in &all_keys {
        let in_a = a.packages.get(name);
        let in_b = b.packages.get(name);

        match (in_a, in_b) {
            (Some(va), Some(vb)) => {
                let version_match = va == vb;
                if version_match {
                    matching_count += 1;
                } else {
                    version_diffs.push(VersionDiff {
                        name: name.clone(),
                        version_a: va.clone(),
                        version_b: vb.clone(),
                    });
                }
                shared.push(PackageDiff {
                    name: name.clone(),
                    version_a: va.clone(),
                    version_b: vb.clone(),
                    version_match,
                });
            }
            (Some(_), None) => only_a.push(name.clone()),
            (None, Some(_)) => only_b.push(name.clone()),
            (None, None) => {}
        }
    }

    let total = all_keys.len().max(1);
    let similarity_score = (matching_count as f64) / (total as f64) * 100.0;

    FingerprintCompare {
        env_a: a.environment.clone(),
        env_b: b.environment.clone(),
        hash_a: a.short_hash.clone(),
        hash_b: b.short_hash.clone(),
        identical: a.hash == b.hash,
        shared,
        only_in_a: only_a,
        only_in_b: only_b,
        version_diffs,
        similarity_score: (similarity_score * 100.0).round() / 100.0,
    }
}

/// Check if two environments are the same (by fingerprint hash)
pub async fn is_same_environment(env_a: &str, env_b: &str) -> Result<bool, AppError> {
    let fp_a = compute_fingerprint(env_a).await?;
    let fp_b = compute_fingerprint(env_b).await?;
    Ok(fp_a.hash == fp_b.hash)
}

// ─── Fingerprint History ──────────────────────────────────────────────

/// Get the fingerprint history directory
fn history_dir() -> std::path::PathBuf {
    std::path::PathBuf::from("/var/lib/nix-evo/fingerprints")
}

/// Save a fingerprint snapshot for history tracking
pub async fn save_fingerprint_snapshot(env_name: &str) -> Result<FingerprintSnapshot, AppError> {
    let fp = compute_fingerprint(env_name).await?;
    let dir = history_dir();
    tokio::fs::create_dir_all(&dir).await.map_err(|e| AppError::IoError {
        path: dir.display().to_string(),
        message: e.to_string(),
    })?;

    let snapshot = FingerprintSnapshot {
        environment: fp.environment.clone(),
        hash: fp.hash.clone(),
        short_hash: fp.short_hash.clone(),
        timestamp: fp.timestamp.clone(),
        package_count: fp.package_count,
        python_version: fp.python_version.clone(),
    };

    let filename = format!(
        "{}-{}.json",
        env_name,
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let path = dir.join(&filename);

    let content = serde_json::to_string_pretty(&snapshot).map_err(|e| AppError::Internal {
        message: format!("Serialization failed: {e}"),
    })?;

    tokio::fs::write(&path, content).await.map_err(|e| AppError::IoError {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;

    Ok(snapshot)
}

/// Load fingerprint history for an environment
pub async fn load_fingerprint_history(env_name: &str) -> Result<Vec<FingerprintSnapshot>, AppError> {
    let dir = history_dir();
    let mut snapshots = Vec::new();

    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(e) => e,
        Err(_) => return Ok(snapshots), // No history yet
    };

    let prefix = format!("{env_name}-");

    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(&prefix) || !name.ends_with(".json") {
            continue;
        }
        if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
            if let Ok(snapshot) = serde_json::from_str::<FingerprintSnapshot>(&content) {
                snapshots.push(snapshot);
            }
        }
    }

    // Sort by timestamp
    snapshots.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    Ok(snapshots)
}

/// Detect if environment has changed since last snapshot
pub async fn detect_env_drift(env_name: &str) -> Result<DriftReport, AppError> {
    let current = compute_fingerprint(env_name).await?;
    let history = load_fingerprint_history(env_name).await?;

    let last = history.last();
    let has_drift = match last {
        Some(prev) => prev.hash != current.hash,
        None => false, // No history = can't drift
    };

    let mut changes = Vec::new();
    if let Some(_prev) = last {
        // Compare against current live state — detect what changed
        // Since we can't fully reconstruct the old package list from snapshot alone,
        // we record that drift was detected based on hash mismatch
        if has_drift {
            changes.push(format!(
                "Fingerprint changed: {} → {}",
                _prev.short_hash, current.short_hash
            ));
        }
    }

    Ok(DriftReport {
        environment: env_name.to_string(),
        current_hash: current.short_hash.clone(),
        previous_hash: last.map(|s| s.short_hash.clone()),
        has_drift,
        snapshot_count: history.len(),
        changes,
        current_package_count: current.package_count,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct DriftReport {
    pub environment: String,
    pub current_hash: String,
    pub previous_hash: Option<String>,
    pub has_drift: bool,
    pub snapshot_count: usize,
    pub changes: Vec<String>,
    pub current_package_count: usize,
}

// ─── Helpers ──────────────────────────────────────────────────────────

async fn get_platform_info() -> Result<String, AppError> {
    let output = run_cmd("uname", &["-srm"]).await?;
    Ok(output.trim().to_string())
}

// ─── HTTP Handlers ────────────────────────────────────────────────────

/// GET /api/env/fingerprint?env=<name>
pub async fn fingerprint_handler(
    State(_state): AppStateRef,
    axum::extract::Query(params): axum::extract::Query<FingerprintQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let env_name = params.env.ok_or_else(|| AppError::Validation {
        field: "env".to_string(),
        message: "env parameter is required".to_string(),
    })?;

    let fp = compute_fingerprint(&env_name).await?;

    // Optionally save snapshot
    if params.save.unwrap_or(false) {
        let _ = save_fingerprint_snapshot(&env_name).await;
    }

    Ok(Json(serde_json::to_value(&fp).unwrap()))
}

/// POST /api/env/fingerprint/compare
pub async fn fingerprint_compare_handler(
    State(_state): AppStateRef,
    Json(body): Json<FingerprintCompareBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let fp_a = compute_fingerprint(&body.env_a).await?;
    let fp_b = compute_fingerprint(&body.env_b).await?;
    let result = compare_fingerprints(&fp_a, &fp_b);
    Ok(Json(serde_json::to_value(&result).unwrap()))
}

/// GET /api/env/fingerprint/history?env=<name>
pub async fn fingerprint_history_handler(
    State(_state): AppStateRef,
    axum::extract::Query(params): axum::extract::Query<FingerprintQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let env_name = params.env.ok_or_else(|| AppError::Validation {
        field: "env".to_string(),
        message: "env parameter is required".to_string(),
    })?;

    let history = load_fingerprint_history(&env_name).await?;
    Ok(Json(serde_json::json!({
        "environment": env_name,
        "snapshots": history,
        "count": history.len()
    })))
}

/// GET /api/env/fingerprint/drift?env=<name>
pub async fn fingerprint_drift_handler(
    State(_state): AppStateRef,
    axum::extract::Query(params): axum::extract::Query<FingerprintQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let env_name = params.env.ok_or_else(|| AppError::Validation {
        field: "env".to_string(),
        message: "env parameter is required".to_string(),
    })?;

    let report = detect_env_drift(&env_name).await?;
    Ok(Json(serde_json::to_value(&report).unwrap()))
}

#[derive(Deserialize)]
pub struct FingerprintQuery {
    pub env: Option<String>,
    pub save: Option<bool>,
}

#[derive(Deserialize)]
pub struct FingerprintCompareBody {
    pub env_a: String,
    pub env_b: String,
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_fingerprint(name: &str, packages: &[(&str, &str)]) -> EnvFingerprint {
        let mut pkg_map = BTreeMap::new();
        for (n, v) in packages {
            pkg_map.insert(n.to_string(), v.to_string());
        }
        let channels = vec!["conda-forge".to_string()];
        let hash = compute_hash(&pkg_map, &Some("3.11.7".to_string()), &channels);
        let short_hash = hash[..12].to_string();

        EnvFingerprint {
            environment: name.to_string(),
            hash,
            short_hash,
            python_version: Some("3.11.7".to_string()),
            package_count: packages.len(),
            packages: pkg_map,
            channels,
            backend: "micromamba".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            platform: Some("Linux x86_64".to_string()),
        }
    }

    #[test]
    fn test_fingerprint_deterministic() {
        let fp1 = make_test_fingerprint("test", &[("numpy", "1.26.3"), ("pandas", "2.1.4")]);
        let fp2 = make_test_fingerprint("test", &[("numpy", "1.26.3"), ("pandas", "2.1.4")]);
        assert_eq!(fp1.hash, fp2.hash);
    }

    #[test]
    fn test_fingerprint_different_versions() {
        let fp1 = make_test_fingerprint("test", &[("numpy", "1.26.3")]);
        let fp2 = make_test_fingerprint("test", &[("numpy", "1.25.0")]);
        assert_ne!(fp1.hash, fp2.hash);
    }

    #[test]
    fn test_fingerprint_order_independent() {
        let fp1 = make_test_fingerprint("test", &[("numpy", "1.26.3"), ("pandas", "2.1.4")]);
        let fp2 = make_test_fingerprint("test", &[("pandas", "2.1.4"), ("numpy", "1.26.3")]);
        assert_eq!(fp1.hash, fp2.hash);
    }

    #[test]
    fn test_compare_identical() {
        let fp1 = make_test_fingerprint("env-a", &[("numpy", "1.26.3"), ("pandas", "2.1.4")]);
        let fp2 = make_test_fingerprint("env-b", &[("numpy", "1.26.3"), ("pandas", "2.1.4")]);
        let cmp = compare_fingerprints(&fp1, &fp2);
        assert!(cmp.identical);
        assert_eq!(cmp.only_in_a.len(), 0);
        assert_eq!(cmp.only_in_b.len(), 0);
        assert_eq!(cmp.version_diffs.len(), 0);
    }

    #[test]
    fn test_compare_different() {
        let fp1 = make_test_fingerprint("env-a", &[("numpy", "1.26.3"), ("scipy", "1.12.0")]);
        let fp2 = make_test_fingerprint("env-b", &[("numpy", "1.25.0"), ("pandas", "2.1.4")]);
        let cmp = compare_fingerprints(&fp1, &fp2);
        assert!(!cmp.identical);
        assert_eq!(cmp.only_in_a, vec!["scipy"]);
        assert_eq!(cmp.only_in_b, vec!["pandas"]);
        assert_eq!(cmp.version_diffs.len(), 1);
        assert_eq!(cmp.version_diffs[0].name, "numpy");
    }

    #[test]
    fn test_compare_similarity_score() {
        let fp1 = make_test_fingerprint("env-a", &[("numpy", "1.26.3"), ("pandas", "2.1.4"), ("scipy", "1.12.0")]);
        let fp2 = make_test_fingerprint("env-b", &[("numpy", "1.26.3"), ("pandas", "2.1.4"), ("scipy", "1.12.0")]);
        let cmp = compare_fingerprints(&fp1, &fp2);
        assert_eq!(cmp.similarity_score, 100.0);
    }

    #[test]
    fn test_hash_uses_sha256() {
        let packages = BTreeMap::from([("numpy".to_string(), "1.26.3".to_string())]);
        let hash = compute_hash(&packages, &None, &[]);
        assert_eq!(hash.len(), 64); // SHA-256 = 64 hex chars
    }
}
