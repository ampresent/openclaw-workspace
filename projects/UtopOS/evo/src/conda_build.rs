//! Conda Build Automation
//!
//! Wrap conda-build / boa for building custom packages.
//! Track build status, logs, output artifacts.
//! Integration with local package cache.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::error::AppError;

/// Request to start a build
#[derive(Debug, Deserialize)]
pub struct BuildRequest {
    pub recipe_path: String,
    pub host: Option<String>,
    pub output_dir: Option<String>,
    pub use_boa: Option<bool>,
    pub channels: Option<Vec<String>>,
    pub python_version: Option<String>,
    pub build_vars: Option<HashMap<String, String>>,
}

/// Build status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BuildStatus {
    Queued,
    Running,
    Success,
    Failed,
    Cancelled,
}

/// Single build record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRecord {
    pub id: String,
    pub recipe_path: String,
    pub status: BuildStatus,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_seconds: Option<f64>,
    pub log_path: Option<String>,
    pub output_artifacts: Vec<String>,
    pub error_message: Option<String>,
    pub use_boa: bool,
}

/// Build response
#[derive(Debug, Clone, Serialize)]
pub struct BuildResponse {
    pub build_id: String,
    pub status: BuildStatus,
    pub message: String,
}

/// Status query
#[derive(Debug, Deserialize)]
pub struct BuildStatusQuery {
    pub build_id: Option<String>,
    pub host: Option<String>,
}

/// Full build history
#[derive(Debug, Clone, Serialize)]
pub struct BuildHistory {
    pub builds: Vec<BuildRecord>,
    pub total: usize,
    pub active: usize,
    pub succeeded: usize,
    pub failed: usize,
}

/// In-memory build state (simplified — production would use persistent storage)
static BUILD_STORE: std::sync::LazyLock<Arc<RwLock<Vec<BuildRecord>>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

/// Generate a unique build ID
fn generate_build_id() -> String {
    let now = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) % 10000;
    format!("build-{}-{:04}", now, nanos)
}

/// Check if boa is available
async fn check_boa_available() -> bool {
    run_cmd("which", &["boa"]).await.is_ok()
}

/// Run conda-build or boa
async fn run_build(
    backend: &str,
    request: &BuildRequest,
    build_id: &str,
) -> Result<BuildRecord, AppError> {
    let use_boa = request.use_boa.unwrap_or(false) && check_boa_available().await;
    let build_tool = if use_boa { "boa" } else { "conda-build" };
    let now = chrono::Utc::now().to_rfc3339();

    let output_dir = request
        .output_dir
        .clone()
        .unwrap_or_else(|| "/tmp/nix-evo-builds".to_string());

    tokio::fs::create_dir_all(&output_dir).await?;

    let log_path = format!("{}/{}.log", output_dir, build_id);

    // Build command args
    let mut args: Vec<String> = Vec::new();

    if use_boa {
        args.push("build".to_string());
    }

    if let Some(channels) = &request.channels {
        for ch in channels {
            args.push("-c".to_string());
            args.push(ch.clone());
        }
    }

    args.push("--output-folder".to_string());
    args.push(output_dir.clone());

    if let Some(py_ver) = &request.python_version {
        args.push("--python".to_string());
        args.push(py_ver.clone());
    }

    args.push(request.recipe_path.clone());

    // Execute build
    let cmd_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let result = run_cmd(build_tool, &cmd_args).await;

    let finished = chrono::Utc::now().to_rfc3339();

    match result {
        Ok(output) => {
            // Write log
            let _ = tokio::fs::write(&log_path, &output).await;

            // Find output artifacts
            let artifacts = find_artifacts(&output_dir).await;

            Ok(BuildRecord {
                id: build_id.to_string(),
                recipe_path: request.recipe_path.clone(),
                status: BuildStatus::Success,
                started_at: Some(now),
                finished_at: Some(finished),
                duration_seconds: None,
                log_path: Some(log_path),
                output_artifacts: artifacts,
                error_message: None,
                use_boa,
            })
        }
        Err(e) => {
            let err_msg = e.to_string();
            Ok(BuildRecord {
                id: build_id.to_string(),
                recipe_path: request.recipe_path.clone(),
                status: BuildStatus::Failed,
                started_at: Some(now),
                finished_at: Some(finished),
                duration_seconds: None,
                log_path: Some(log_path),
                output_artifacts: vec![],
                error_message: Some(err_msg),
                use_boa,
            })
        }
    }
}

/// Find output artifacts in the build directory
async fn find_artifacts(output_dir: &str) -> Vec<String> {
    let mut artifacts = Vec::new();
    if let Ok(mut entries) = tokio::fs::read_dir(output_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".tar.bz2") || name.ends_with(".conda") {
                    artifacts.push(format!("{}/{}", output_dir, name));
                }
            }
        }
    }
    artifacts
}

/// POST /api/conda/build — start a build
pub async fn build_handler(
    state: AppStateRef,
    Json(body): Json<BuildRequest>,
) -> Result<Json<BuildResponse>, AppError> {
    let backend = crate::conda::detect_backend().await?;
    let build_id = generate_build_id();

    // Check recipe path exists
    if !tokio::fs::metadata(&body.recipe_path).await.is_ok() {
        return Err(AppError::Validation { field: "recipe_path".to_string(), message: format!("Recipe path does not exist: {}", body.recipe_path) });
    }

    // Record as queued
    let mut store = BUILD_STORE.write().await;
    store.push(BuildRecord {
        id: build_id.clone(),
        recipe_path: body.recipe_path.clone(),
        status: BuildStatus::Queued,
        started_at: None,
        finished_at: None,
        duration_seconds: None,
        log_path: None,
        output_artifacts: vec![],
        error_message: None,
        use_boa: body.use_boa.unwrap_or(false),
    });
    drop(store);

    // Run build (simplified: synchronous in this implementation)
    let record = run_build(&backend, &body, &build_id).await?;

    // Update store
    let mut store = BUILD_STORE.write().await;
    if let Some(existing) = store.iter_mut().find(|r| r.id == build_id) {
        *existing = record.clone();
    }

    Ok(Json(BuildResponse {
        build_id,
        status: record.status,
        message: match record.status {
            BuildStatus::Success => "Build completed successfully".to_string(),
            BuildStatus::Failed => format!("Build failed: {}",
                record.error_message.unwrap_or_else(|| "Unknown error".to_string())),
            _ => "Build status unknown".to_string(),
        },
    }))
}

/// GET /api/conda/build/status — get build status
pub async fn build_status_handler(
    state: AppStateRef,
    Query(query): Query<BuildStatusQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let store = BUILD_STORE.read().await;

    if let Some(build_id) = &query.build_id {
        if let Some(record) = store.iter().find(|r| r.id == *build_id) {
            Ok(Json(serde_json::to_value(record)?))
        } else {
            Err(AppError::NotFound { resource: format!("Build '{}'", build_id) })
        }
    } else {
        // Return full history
        let total = store.len();
        let active = store.iter().filter(|r| r.status == BuildStatus::Running || r.status == BuildStatus::Queued).count();
        let succeeded = store.iter().filter(|r| r.status == BuildStatus::Success).count();
        let failed = store.iter().filter(|r| r.status == BuildStatus::Failed).count();

        Ok(Json(serde_json::to_value(BuildHistory {
            builds: store.clone(),
            total,
            active,
            succeeded,
            failed,
        })?))
    }
}
