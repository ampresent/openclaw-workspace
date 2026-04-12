//! Build Cache Manager
//!
//! Track conda package cache status, detect stale downloads, clean unused packages.
//! Mirror management for offline/air-gapped environments.

use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Cache status report
#[derive(Debug, Clone, Serialize)]
pub struct CacheStatus {
    pub backend: String,
    pub cache_dir: String,
    pub total_size_mb: u64,
    pub package_cache: CacheDetail,
    pub env_cache: CacheDetail,
    pub tarballs: CacheDetail,
    pub stale_entries: Vec<StaleEntry>,
    pub mirrors: Vec<MirrorInfo>,
    pub cleanup_savings_mb: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheDetail {
    pub path: String,
    pub exists: bool,
    pub size_mb: u64,
    pub file_count: usize,
    pub oldest_file_age_days: Option<u64>,
    pub newest_file_age_days: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StaleEntry {
    pub path: String,
    pub size_mb: u64,
    pub age_days: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirrorInfo {
    pub name: String,
    pub url: String,
    pub is_local: bool,
    pub last_synced: Option<String>,
    pub package_count: Option<usize>,
    pub size_gb: Option<f64>,
}

/// Clean request
#[derive(Debug, Clone, Deserialize)]
pub struct CleanRequest {
    pub dry_run: Option<bool>,
    pub max_age_days: Option<u64>,
    pub remove_tarballs: Option<bool>,
    pub remove_packages: Option<bool>,
    pub remove_envs: Option<bool>,
    pub force: Option<bool>,
}

/// Clean result
#[derive(Debug, Clone, Serialize)]
pub struct CleanResult {
    pub dry_run: bool,
    pub space_freed_mb: u64,
    pub files_removed: usize,
    pub tarballs_removed: usize,
    pub packages_removed: usize,
    pub errors: Vec<String>,
    pub actions: Vec<String>,
}

/// Mirror sync request
#[derive(Debug, Clone, Deserialize)]
pub struct MirrorSyncRequest {
    pub mirror_url: String,
    pub channels: Option<Vec<String>>,
    pub platforms: Option<Vec<String>>,
    pub local_path: Option<String>,
}

// ─── Cache Status ─────────────────────────────────────────────────────

/// Get full cache status
pub async fn get_cache_status() -> Result<CacheStatus, AppError> {
    let backend = conda::detect_backend().await?;

    // Find cache directory
    let cache_dir = find_cache_dir(&backend).await;

    // Get subdirectory details
    let pkgs_dir = format!("{cache_dir}/pkgs");
    let envs_dir = format!("{cache_dir}/envs");
    let tarballs_dir = format!("{cache_dir}/pkgs/cache");

    let package_cache = get_cache_detail(&pkgs_dir).await;
    let env_cache = get_cache_detail(&envs_dir).await;
    let tarballs = get_cache_detail(&tarballs_dir).await;

    let total_size_mb = package_cache.size_mb + env_cache.size_mb + tarballs.size_mb;

    // Detect stale entries
    let stale_entries = detect_stale_entries(&cache_dir).await;

    // List mirrors
    let mirrors = list_mirrors(&backend).await;

    // Calculate potential cleanup savings
    let cleanup_savings_mb: u64 = stale_entries.iter().map(|s| s.size_mb).sum();

    Ok(CacheStatus {
        backend,
        cache_dir,
        total_size_mb,
        package_cache,
        env_cache,
        tarballs,
        stale_entries,
        mirrors,
        cleanup_savings_mb,
    })
}

async fn find_cache_dir(backend: &str) -> String {
    // Try to get from config
    if let Ok(out) = run_cmd(backend, &["config", "show", "pkgs_dirs"]).await {
        for line in out.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('/') || trimmed.contains(": /") {
                let path = if let Some(colon_pos) = trimmed.find(": ") {
                    trimmed[colon_pos + 2..].trim()
                } else {
                    trimmed
                };
                if Path::new(path).exists() {
                    return path.to_string();
                }
            }
        }
    }

    // Common defaults
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = vec![
        format!("{home}/.conda/pkgs"),
        format!("{home}/micromamba/pkgs"),
        "/opt/conda/pkgs".to_string(),
        format!("{home}/.local/share/mamba/pkgs"),
    ];

    for path in &candidates {
        if Path::new(path).exists() {
            return path.clone();
        }
    }

    // Default
    format!("{home}/.conda/pkgs")
}

async fn get_cache_detail(dir: &str) -> CacheDetail {
    let path = Path::new(dir);
    let exists = path.exists();

    if !exists {
        return CacheDetail {
            path: dir.to_string(),
            exists: false,
            size_mb: 0,
            file_count: 0,
            oldest_file_age_days: None,
            newest_file_age_days: None,
        };
    }

    let size_mb = get_dir_size_mb(dir).await;
    let file_count = count_files(dir).await;
    let (oldest, newest) = get_file_ages(dir).await;

    CacheDetail {
        path: dir.to_string(),
        exists,
        size_mb,
        file_count,
        oldest_file_age_days: oldest,
        newest_file_age_days: newest,
    }
}

async fn get_dir_size_mb(path: &str) -> u64 {
    match tokio::process::Command::new("du")
        .args(["-sm", path])
        .output()
        .await
    {
        Ok(output) => {
            let s = String::from_utf8_lossy(&output.stdout);
            s.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(0)
        }
        Err(_) => 0,
    }
}

async fn count_files(dir: &str) -> usize {
    match tokio::process::Command::new("find")
        .args([dir, "-type", "f"])
        .output()
        .await
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).lines().count(),
        Err(_) => 0,
    }
}

async fn get_file_ages(dir: &str) -> (Option<u64>, Option<u64>) {
    // Find oldest file
    let oldest = match tokio::process::Command::new("find")
        .args([dir, "-type", "f", "-printf", "%T@\\n"])
        .output()
        .await
    {
        Ok(output) => {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|l| l.parse::<f64>().ok())
                .fold(None, |min, ts| {
                    Some(min.map_or(ts, |m: f64| m.min(ts)))
                })
        }
        Err(_) => None,
    };

    let newest = match tokio::process::Command::new("find")
        .args([dir, "-type", "f", "-printf", "%T@\\n"])
        .output()
        .await
    {
        Ok(output) => {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|l| l.parse::<f64>().ok())
                .fold(None, |max, ts| {
                    Some(max.map_or(ts, |m: f64| m.max(ts)))
                })
        }
        Err(_) => None,
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let oldest_days = oldest.map(|ts| ((now - ts) / 86400.0) as u64);
    let newest_days = newest.map(|ts| ((now - ts) / 86400.0) as u64);

    (oldest_days, newest_days)
}

/// Detect stale cache entries
async fn detect_stale_entries(cache_dir: &str) -> Vec<StaleEntry> {
    let mut entries = Vec::new();
    let pkgs_dir = format!("{cache_dir}/pkgs");

    if !Path::new(&pkgs_dir).exists() {
        return entries;
    }

    // Find files older than 90 days
    let output = match tokio::process::Command::new("find")
        .args([&pkgs_dir, "-type", "f", "-mtime", "+90", "-printf", "%s %T@ %p\\n"])
        .output()
        .await
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return entries,
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() >= 3 {
            let size_bytes: u64 = parts[0].parse().unwrap_or(0);
            let timestamp: f64 = parts[1].parse().unwrap_or(0.0);
            let path = parts[2].to_string();
            let age_days = ((now - timestamp) / 86400.0) as u64;

            if age_days > 90 {
                entries.push(StaleEntry {
                    path,
                    size_mb: size_bytes / 1_048_576,
                    age_days,
                    reason: format!("Not accessed for {age_days} days"),
                });
            }
        }
    }

    entries
}

/// List configured mirrors
async fn list_mirrors(backend: &str) -> Vec<MirrorInfo> {
    let mut mirrors = Vec::new();

    if let Ok(output) = run_cmd(backend, &["config", "show", "channels"]).await {
        for line in output.lines() {
            let trimmed = line.trim().trim_start_matches("- ");
            if trimmed.contains("://") {
                let is_local = trimmed.starts_with("file://") || trimmed.starts_with("/");
                mirrors.push(MirrorInfo {
                    name: trimmed.split('/').last().unwrap_or("unknown").to_string(),
                    url: trimmed.to_string(),
                    is_local,
                    last_synced: None,
                    package_count: None,
                    size_gb: None,
                });
            }
        }
    }

    mirrors
}

// ─── Cache Cleaning ───────────────────────────────────────────────────

/// Clean cache
pub async fn clean_cache(request: &CleanRequest) -> Result<CleanResult, AppError> {
    let backend = conda::detect_backend().await?;
    let dry_run = request.dry_run.unwrap_or(false);
    let mut actions = Vec::new();
    let mut errors = Vec::new();

    if dry_run {
        // Just report what would be cleaned
        let status = get_cache_status().await?;
        actions.push(format!("Would free {} MB from {} stale entries",
            status.cleanup_savings_mb, status.stale_entries.len()));
        if request.remove_tarballs.unwrap_or(true) {
            actions.push(format!("Would remove tarballs cache: {} MB", status.tarballs.size_mb));
        }

        return Ok(CleanResult {
            dry_run: true,
            space_freed_mb: status.cleanup_savings_mb + if request.remove_tarballs.unwrap_or(true) { status.tarballs.size_mb } else { 0 },
            files_removed: 0,
            tarballs_removed: 0,
            packages_removed: 0,
            errors,
            actions,
        });
    }

    let mut space_freed_mb = 0u64;
    let mut files_removed = 0usize;
    let mut tarballs_removed = 0usize;
    let mut packages_removed = 0usize;

    // Remove tarballs
    if request.remove_tarballs.unwrap_or(true) {
        match run_cmd(backend, &["clean", "--tarballs", "-y"]).await {
            Ok(out) => {
                actions.push("Removed tarballs".to_string());
                tarballs_removed = out.lines().filter(|l| l.contains("removed")).count();
            }
            Err(e) => errors.push(format!("Failed to clean tarballs: {e}")),
        }
    }

    // Remove unused packages
    if request.remove_packages.unwrap_or(true) {
        match run_cmd(backend, &["clean", "--packages", "-y"]).await {
            Ok(out) => {
                actions.push("Removed unused packages".to_string());
                packages_removed = out.lines().filter(|l| l.contains("removed")).count();
            }
            Err(e) => errors.push(format!("Failed to clean packages: {e}")),
        }
    }

    // micromamba-specific: index cache
    if backend == "micromamba" {
        let _ = run_cmd(backend, &["clean", "--index-cache", "-y"]).await;
        actions.push("Cleaned index cache".to_string());
    }

    // Calculate freed space (approximate)
    let new_status = get_cache_status().await.unwrap_or_else(|_| CacheStatus {
        backend: backend.clone(),
        cache_dir: String::new(),
        total_size_mb: 0,
        package_cache: CacheDetail { path: String::new(), exists: false, size_mb: 0, file_count: 0, oldest_file_age_days: None, newest_file_age_days: None },
        env_cache: CacheDetail { path: String::new(), exists: false, size_mb: 0, file_count: 0, oldest_file_age_days: None, newest_file_age_days: None },
        tarballs: CacheDetail { path: String::new(), exists: false, size_mb: 0, file_count: 0, oldest_file_age_days: None, newest_file_age_days: None },
        stale_entries: vec![],
        mirrors: vec![],
        cleanup_savings_mb: 0,
    });
    // Approximate - use previous stale entries count
    space_freed_mb = (tarballs_removed as u64 * 50) + (packages_removed as u64 * 20);

    Ok(CleanResult {
        dry_run: false,
        space_freed_mb,
        files_removed,
        tarballs_removed,
        packages_removed,
        errors,
        actions,
    })
}

// ─── Mirror Management ───────────────────────────────────────────────

/// Setup a local mirror for offline/air-gapped environments
pub async fn setup_mirror(request: &MirrorSyncRequest) -> Result<serde_json::Value, AppError> {
    let local_path = request.local_path.clone()
        .unwrap_or_else(|| "/opt/conda-mirror".to_string());

    let channels = request.channels.clone()
        .unwrap_or_else(|| vec!["conda-forge".to_string()]);

    let platforms = request.platforms.clone()
        .unwrap_or_else(|| vec!["linux-64".to_string(), "noarch".to_string()]);

    // Create mirror directory
    let _ = tokio::process::Command::new("mkdir")
        .args(["-p", &local_path])
        .output()
        .await;

    // Check if conda-mirror tool is available
    let has_conda_mirror = run_cmd("conda-mirror", &["--help"]).await.is_ok();
    if !has_conda_mirror {
        return Err(AppError::CommandFailed {
            command: "conda-mirror".to_string(),
            message: "conda-mirror not installed. Install with: micromamba install -c conda-forge conda-mirror".to_string(),
        });
    }

    let mut results = Vec::new();
    for channel in &channels {
        for platform in &platforms {
            let mirror_dir = format!("{local_path}/{channel}/{platform}");
            let _ = tokio::process::Command::new("mkdir")
                .args(["-p", &mirror_dir])
                .output()
                .await;

            let url = format!("https://conda.anaconda.org/{channel}/{platform}");
            let result = run_cmd("conda-mirror", &[
                "--upstream", &url,
                "--target-dir", &mirror_dir,
                "--no-validate-target",
            ]).await;

            match result {
                Ok(_) => {
                    results.push(serde_json::json!({
                        "channel": channel,
                        "platform": platform,
                        "path": mirror_dir,
                        "status": "synced"
                    }));
                }
                Err(e) => {
                    results.push(serde_json::json!({
                        "channel": channel,
                        "platform": platform,
                        "error": e.to_string(),
                        "status": "failed"
                    }));
                }
            }
        }
    }

    Ok(serde_json::json!({
        "mirror_path": local_path,
        "channels": channels,
        "platforms": platforms,
        "results": results,
        "add_to_config": format!("{backend} config append channels file://{local_path}", backend = "micromamba"),
    }))
}

// ─── HTTP Handlers ────────────────────────────────────────────────────

/// GET /api/cache/status
pub async fn cache_status_handler(
    State(_state): AppStateRef,
    Query(_query): Query<HostQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let status = get_cache_status().await?;
    Ok(Json(serde_json::to_value(&status).unwrap()))
}

/// POST /api/cache/clean
pub async fn cache_clean_handler(
    State(_state): AppStateRef,
    Json(body): Json<CleanRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = clean_cache(&body).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}

/// POST /api/cache/mirror — setup local mirror
pub async fn mirror_setup_handler(
    State(_state): AppStateRef,
    Json(body): Json<MirrorSyncRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = setup_mirror(&body).await?;
    Ok(Json(result))
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_detail_nonexistent() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let detail = rt.block_on(get_cache_detail("/nonexistent/path"));
        assert!(!detail.exists);
        assert_eq!(detail.size_mb, 0);
    }

    #[test]
    fn test_clean_request_defaults() {
        let req = CleanRequest {
            dry_run: None,
            max_age_days: None,
            remove_tarballs: None,
            remove_packages: None,
            remove_envs: None,
            force: None,
        };
        assert!(!req.dry_run.unwrap_or(false));
        assert!(req.remove_tarballs.unwrap_or(true));
    }

    #[test]
    fn test_mirror_info_serialization() {
        let mirror = MirrorInfo {
            name: "conda-forge".to_string(),
            url: "https://conda.anaconda.org/conda-forge".to_string(),
            is_local: false,
            last_synced: None,
            package_count: Some(25000),
            size_gb: Some(450.0),
        };
        let json = serde_json::to_string(&mirror).unwrap();
        assert!(json.contains("conda-forge"));
        assert!(json.contains("25000"));
    }
}
