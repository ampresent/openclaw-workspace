//! Conda Cloud Sync
//!
//! Sync environment state to cloud storage (S3/GCS/MinIO).
//! Backup and restore from cloud.
//! Multi-machine environment sharing.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

const CLOUD_CONFIG_PATH: &str = "/var/lib/nix-evo/conda-cloud-config.json";
const LOCAL_BACKUP_PATH: &str = "/var/lib/nix-evo/conda-cloud-backups";

/// Cloud provider type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CloudProvider {
    S3,
    GCS,
    MinIO,
    Local, // file-based fallback
}

/// Cloud sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    pub provider: CloudProvider,
    pub bucket: String,
    pub prefix: String, // e.g. "nix-evo/conda-envs/"
    pub endpoint: Option<String>, // for MinIO/custom S3
    pub region: Option<String>,
    pub auto_sync: Option<bool>,
    pub sync_interval_hours: Option<u64>,
}

/// Sync request
#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub env: Option<String>, // specific env or all
    pub host: Option<String>,
    pub direction: Option<String>, // "push", "pull", "both" (default: push)
    pub force: Option<bool>,
    pub dry_run: Option<bool>,
}

/// Sync result for a single environment
#[derive(Debug, Clone, Serialize)]
pub struct EnvSyncResult {
    pub env_name: String,
    pub direction: String,
    pub success: bool,
    pub packages_synced: usize,
    pub remote_path: String,
    pub message: String,
    pub timestamp: String,
}

/// Full sync report
#[derive(Debug, Clone, Serialize)]
pub struct SyncReport {
    pub generated_at: String,
    pub provider: String,
    pub bucket: String,
    pub total_envs: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<EnvSyncResult>,
    pub errors: Vec<String>,
}

/// Backup info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub env_name: String,
    pub timestamp: String,
    pub remote_path: String,
    pub package_count: usize,
    pub python_version: Option<String>,
    pub fingerprint: Option<String>,
}

/// Backup manifest (list of all backups)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub backups: Vec<BackupInfo>,
    pub last_sync: Option<String>,
}

/// Load cloud config
async fn load_cloud_config() -> Result<CloudConfig, AppError> {
    let path = PathBuf::from(CLOUD_CONFIG_PATH);
    if path.exists() {
        let content = tokio::fs::read_to_string(&path).await?;
        let config: CloudConfig = serde_json::from_str(&content)?;
        Ok(config)
    } else {
        // Default config: local storage
        Ok(CloudConfig {
            provider: CloudProvider::Local,
            bucket: "nix-evo-backups".to_string(),
            prefix: "conda-envs/".to_string(),
            endpoint: None,
            region: None,
            auto_sync: Some(false),
            sync_interval_hours: Some(24),
        })
    }
}

/// Save cloud config
async fn save_cloud_config(config: &CloudConfig) -> Result<(), AppError> {
    let path = PathBuf::from(CLOUD_CONFIG_PATH);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let content = serde_json::to_string_pretty(config)?;
    tokio::fs::write(&path, content).await?;
    Ok(())
}

/// Build cloud CLI command for sync
fn build_sync_command(
    config: &CloudConfig,
    local_path: &str,
    remote_key: &str,
    direction: &str,
) -> Vec<String> {
    match config.provider {
        CloudProvider::S3 => {
            let remote = format!("s3://{}/{}{}", config.bucket, config.prefix, remote_key);
            if direction == "push" {
                vec!["aws".to_string(), "s3".to_string(), "cp".to_string(),
                     local_path.to_string(), remote]
            } else {
                vec!["aws".to_string(), "s3".to_string(), "cp".to_string(),
                     remote, local_path.to_string()]
            }
        }
        CloudProvider::GCS => {
            let remote = format!("gs://{}/{}{}", config.bucket, config.prefix, remote_key);
            if direction == "push" {
                vec!["gsutil".to_string(), "cp".to_string(),
                     local_path.to_string(), remote]
            } else {
                vec!["gsutil".to_string(), "cp".to_string(),
                     remote, local_path.to_string()]
            }
        }
        CloudProvider::MinIO => {
            let remote = format!("{}/{}{}",
                config.endpoint.as_deref().unwrap_or("http://localhost:9000"),
                config.bucket, config.prefix);
            if direction == "push" {
                vec!["mc".to_string(), "cp".to_string(),
                     local_path.to_string(), format!("{}{}", remote, remote_key)]
            } else {
                vec!["mc".to_string(), "cp".to_string(),
                     format!("{}{}", remote, remote_key), local_path.to_string()]
            }
        }
        CloudProvider::Local => {
            let remote_dir = format!("{}/{}", LOCAL_BACKUP_PATH, config.prefix);
            let remote = format!("{}{}", remote_dir, remote_key);
            if direction == "push" {
                vec!["cp".to_string(), local_path.to_string(), remote]
            } else {
                vec!["cp".to_string(), remote, local_path.to_string()]
            }
        }
    }
}

/// Export environment to a temp file
async fn export_env_to_file(backend: &str, env_name: &str) -> Result<String, AppError> {
    let temp_path = format!("/tmp/nix-evo-cloud-{}-{}.yml",
        env_name, chrono::Utc::now().timestamp());

    let args = ["env", "export", "-n", env_name];
    let output = run_cmd(backend, &args).await?;
    tokio::fs::write(&temp_path, &output).await?;
    Ok(temp_path)
}

/// Load backup manifest
async fn load_manifest() -> Result<BackupManifest, AppError> {
    let path = PathBuf::from(LOCAL_BACKUP_PATH).join("manifest.json");
    if path.exists() {
        let content = tokio::fs::read_to_string(&path).await?;
        let manifest: BackupManifest = serde_json::from_str(&content)?;
        Ok(manifest)
    } else {
        Ok(BackupManifest {
            backups: vec![],
            last_sync: None,
        })
    }
}

/// Save backup manifest
async fn save_manifest(manifest: &BackupManifest) -> Result<(), AppError> {
    let path = PathBuf::from(LOCAL_BACKUP_PATH).join("manifest.json");
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    let content = serde_json::to_string_pretty(manifest)?;
    tokio::fs::write(&path, content).await?;
    Ok(())
}

/// Sync a single environment
async fn sync_single_env(
    backend: &str,
    config: &CloudConfig,
    env_name: &str,
    direction: &str,
    dry_run: bool,
) -> Result<EnvSyncResult, AppError> {
    let timestamp = chrono::Utc::now().to_rfc3339();

    if direction == "push" {
        // Export env
        let temp_path = export_env_to_file(backend, env_name).await?;
        let remote_key = format!("{}.yml", env_name);

        // Get package count
        let packages = conda::list_packages(backend, env_name).await.unwrap_or_default();

        if !dry_run {
            let cmd_parts = build_sync_command(config, &temp_path, &remote_key, "push");

            // For local provider, create directory
            if config.provider == CloudProvider::Local {
                let remote_dir = format!("{}/{}", LOCAL_BACKUP_PATH, config.prefix);
                let _ = tokio::fs::create_dir_all(&remote_dir).await;
            }

            let args: Vec<&str> = cmd_parts.iter().map(|s| s.as_str()).collect();
            match run_cmd(args[0], &args[1..]).await {
                Ok(_) => {
                    // Update manifest
                    let mut manifest = load_manifest().await?;
                    manifest.backups.retain(|b| b.env_name != env_name);
                    manifest.backups.push(BackupInfo {
                        env_name: env_name.to_string(),
                        timestamp: timestamp.clone(),
                        remote_path: format!("{}{}", config.prefix, remote_key),
                        package_count: packages.len(),
                        python_version: packages.iter().find(|p| p.name == "python").map(|p| p.version.clone()),
                        fingerprint: None,
                    });
                    manifest.last_sync = Some(timestamp.clone());
                    save_manifest(&manifest).await?;

                    // Cleanup temp
                    let _ = tokio::fs::remove_file(&temp_path).await;

                    Ok(EnvSyncResult {
                        env_name: env_name.to_string(),
                        direction: "push".to_string(),
                        success: true,
                        packages_synced: packages.len(),
                        remote_path: format!("{}{}", config.prefix, remote_key),
                        message: "Successfully synced to cloud".to_string(),
                        timestamp,
                    })
                }
                Err(e) => Ok(EnvSyncResult {
                    env_name: env_name.to_string(),
                    direction: "push".to_string(),
                    success: false,
                    packages_synced: 0,
                    remote_path: format!("{}{}", config.prefix, remote_key),
                    message: format!("Sync failed: {}", e),
                    timestamp,
                }),
            }
        } else {
            Ok(EnvSyncResult {
                env_name: env_name.to_string(),
                direction: "push".to_string(),
                success: true,
                packages_synced: packages.len(),
                remote_path: format!("{}{}", config.prefix, remote_key),
                message: "Dry run — no changes made".to_string(),
                timestamp,
            })
        }
    } else {
        // Pull
        Ok(EnvSyncResult {
            env_name: env_name.to_string(),
            direction: "pull".to_string(),
            success: false,
            packages_synced: 0,
            remote_path: String::new(),
            message: "Pull not yet implemented — use /api/conda/envs/:name with create-from-yml".to_string(),
            timestamp,
        })
    }
}

/// POST /api/conda/cloud/sync
pub async fn sync_handler(
    state: AppStateRef,
    Json(body): Json<SyncRequest>,
) -> Result<Json<SyncReport>, AppError> {
    let backend = crate::conda::detect_backend().await?;
    let config = load_cloud_config().await?;
    let direction = body.direction.as_deref().unwrap_or("push");
    let dry_run = body.dry_run.unwrap_or(false);
    let now = chrono::Utc::now().to_rfc3339();

    let mut results = Vec::new();
    let mut errors = Vec::new();
    let mut successful = 0;
    let mut failed = 0;

    if let Some(env_name) = &body.env {
        // Sync single env
        match sync_single_env(&backend, &config, env_name, direction, dry_run).await {
            Ok(result) => {
                if result.success { successful += 1; } else { failed += 1; }
                results.push(result);
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("{}: {}", env_name, e));
            }
        }
    } else {
        // Sync all environments
        let envs = conda::list_envs(&backend).await?;
        for env in &envs {
            match sync_single_env(&backend, &config, &env.name, direction, dry_run).await {
                Ok(result) => {
                    if result.success { successful += 1; } else { failed += 1; }
                    results.push(result);
                }
                Err(e) => {
                    failed += 1;
                    errors.push(format!("{}: {}", env.name, e));
                }
            }
        }
    }

    Ok(Json(SyncReport {
        generated_at: now,
        provider: format!("{:?}", config.provider),
        bucket: config.bucket,
        total_envs: results.len() + errors.len(),
        successful,
        failed,
        results,
        errors,
    }))
}
