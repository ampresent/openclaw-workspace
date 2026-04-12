//! Remote Environment Sync
//!
//! Sync environments between machines via nix-evo-agent API.
//! Push: export env → send to remote → recreate.
//! Pull: fetch env from remote → recreate locally.

use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::cmd::{AppStateRef, run_cmd};
use crate::conda;
use crate::env_sync;
use crate::error::AppError;

/// Remote host definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteHost {
    pub name: String,
    pub api_url: String,
    pub api_token: Option<String>,
    pub platform: Option<String>,
}

/// Push request
#[derive(Debug, Clone, Deserialize)]
pub struct PushRequest {
    pub env: String,
    pub remote_host: RemoteHost,
    pub remote_env_name: Option<String>,
    pub format: Option<env_sync::SyncFormat>,
    pub overwrite: Option<bool>,
}

/// Pull request
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequest {
    pub remote_host: RemoteHost,
    pub remote_env: String,
    pub local_env_name: Option<String>,
    pub format: Option<env_sync::SyncFormat>,
    pub overwrite: Option<bool>,
}

/// Remote sync result
#[derive(Debug, Clone, Serialize)]
pub struct RemoteSyncResult {
    pub operation: SyncOperation,
    pub local_env: String,
    pub remote_env: String,
    pub remote_host: String,
    pub success: bool,
    pub format_used: String,
    pub packages_transferred: usize,
    pub bytes_transferred: u64,
    pub commands_executed: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum SyncOperation {
    #[serde(rename = "push")]
    Push,
    #[serde(rename = "pull")]
    Pull,
}

/// Remote environment listing
#[derive(Debug, Clone, Serialize)]
pub struct RemoteEnvList {
    pub host: String,
    pub environments: Vec<conda::CondaEnv>,
    pub backend: String,
}

// ─── Push / Pull Engine ───────────────────────────────────────────────

/// Push a local environment to a remote machine
pub async fn push_environment(request: &PushRequest) -> Result<RemoteSyncResult, AppError> {
    let start = std::time::Instant::now();
    let backend = conda::detect_backend().await?;
    let format = request.format.clone().unwrap_or(env_sync::SyncFormat::EnvironmentYml);
    let remote_env_name = request.remote_env_name.as_deref().unwrap_or(&request.env);
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut commands = Vec::new();

    // 1. Export local environment
    let (content, pkg_count) = export_for_remote(&backend, &request.env, &format).await?;

    // 2. Send to remote machine
    let json_body = serde_json::json!({
        "name": remote_env_name,
        "channels": extract_channels(&content),
        "dependencies": extract_dependencies(&content),
        "_source_format": format.to_string(),
        "_pushed_from": "local",
        "_pushed_at": chrono::Utc::now().to_rfc3339(),
    });

    let client = reqwest::Client::new();
    let url = format!("{}/api/conda/create-from-yml", request.remote_host.api_url.trim_end_matches('/'));

    // Write yml to temp, then post as file
    let tmp_file = format!("/tmp/push-{}-{}.yml", request.env, chrono::Utc::now().timestamp());
    tokio::fs::write(&tmp_file, &content).await.map_err(|e| AppError::IoError {
        path: tmp_file.clone(),
        message: e.to_string(),
    })?;

    // Try to push via the remote API
    let mut success = false;
    let create_body = serde_json::json!({
        "path": tmp_file,
    });

    let mut req_builder = client.post(&format!("{}/api/conda/create-from-yml", request.remote_host.api_url.trim_end_matches('/')))
        .json(&create_body);

    if let Some(token) = &request.remote_host.api_token {
        req_builder = req_builder.header("Authorization", format!("Bearer {token}"));
    }

    match req_builder.send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                success = true;
                commands.push(format!("POST {url} — success"));
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                errors.push(format!("Remote API returned {status}: {body}"));
                commands.push(format!("POST {url} — failed ({status})"));
            }
        }
        Err(e) => {
            errors.push(format!("Failed to reach remote host: {e}"));
            // Fallback: provide manual commands
            warnings.push("Remote API unreachable. Manual migration commands:".to_string());
            commands.push(format!(
                "# On remote machine '{}':\nmicromamba env create -f <exported.yml> -n {} -y",
                request.remote_host.name, remote_env_name
            ));
        }
    }

    // Calculate approximate bytes
    let bytes = content.len() as u64;

    Ok(RemoteSyncResult {
        operation: SyncOperation::Push,
        local_env: request.env.clone(),
        remote_env: remote_env_name.to_string(),
        remote_host: request.remote_host.name.clone(),
        success,
        format_used: format.to_string(),
        packages_transferred: pkg_count,
        bytes_transferred: bytes,
        commands_executed: commands,
        warnings,
        errors,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Pull an environment from a remote machine
pub async fn pull_environment(request: &PullRequest) -> Result<RemoteSyncResult, AppError> {
    let start = std::time::Instant::now();
    let backend = conda::detect_backend().await?;
    let local_name = request.local_env_name.as_deref().unwrap_or(&request.remote_env);
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut commands = Vec::new();

    // 1. Fetch environment from remote
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/conda/export?env={}",
        request.remote_host.api_url.trim_end_matches('/'),
        request.remote_env
    );

    let mut req_builder = client.get(&url);
    if let Some(token) = &request.remote_host.api_token {
        req_builder = req_builder.header("Authorization", format!("Bearer {token}"));
    }

    let content = match req_builder.send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                errors.push(format!("Remote API returned {}", resp.status()));
                return Ok(RemoteSyncResult {
                    operation: SyncOperation::Pull,
                    local_env: local_name.to_string(),
                    remote_env: request.remote_env.clone(),
                    remote_host: request.remote_host.name.clone(),
                    success: false,
                    format_used: "environment-yml".to_string(),
                    packages_transferred: 0,
                    bytes_transferred: 0,
                    commands_executed: commands,
                    warnings,
                    errors,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            let body: serde_json::Value = resp.json().await.map_err(|e| AppError::Internal {
                message: format!("Failed to parse remote response: {e}"),
            })?;
            body.get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string()
        }
        Err(e) => {
            errors.push(format!("Failed to reach remote host: {e}"));
            // Provide manual instructions
            warnings.push("Remote API unreachable. Manual steps:".to_string());
            commands.push(format!(
                "# On remote '{}':\nmicromamba env export -n {} --no-builds > remote-env.yml",
                request.remote_host.name, request.remote_env
            ));
            commands.push(format!(
                "# Then locally:\nmicromamba env create -f remote-env.yml -n {} -y",
                local_name
            ));
            return Ok(RemoteSyncResult {
                operation: SyncOperation::Pull,
                local_env: local_name.to_string(),
                remote_env: request.remote_env.clone(),
                remote_host: request.remote_host.name.clone(),
                success: false,
                format_used: "environment-yml".to_string(),
                packages_transferred: 0,
                bytes_transferred: 0,
                commands_executed: commands,
                warnings,
                errors,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
    };

    let pkg_count = content.lines().filter(|l| l.trim().starts_with("- ")).count();
    let bytes = content.len() as u64;

    // 2. Write to temp file and create locally
    let tmp_file = format!("/tmp/pull-{}-{}.yml", request.remote_env, chrono::Utc::now().timestamp());
    tokio::fs::write(&tmp_file, &content).await.map_err(|e| AppError::IoError {
        path: tmp_file.clone(),
        message: e.to_string(),
    })?;

    let mut success = false;
    let args = if request.overwrite.unwrap_or(false) {
        vec!["env", "create", "-f", &tmp_file, "-n", local_name, "-y", "--force"]
    } else {
        vec!["env", "create", "-f", &tmp_file, "-n", local_name, "-y"]
    };

    let cmd_str = format!("{backend} {}", args.join(" "));
    commands.push(cmd_str);

    match run_cmd(&backend, &args).await {
        Ok(_) => {
            success = true;
        }
        Err(e) => {
            errors.push(format!("Local env creation failed: {e}"));
            warnings.push("Environment export saved but local creation failed.".to_string());
        }
    }

    Ok(RemoteSyncResult {
        operation: SyncOperation::Pull,
        local_env: local_name.to_string(),
        remote_env: request.remote_env.clone(),
        remote_host: request.remote_host.name.clone(),
        success,
        format_used: "environment-yml".to_string(),
        packages_transferred: pkg_count,
        bytes_transferred: bytes,
        commands_executed: commands,
        warnings,
        errors,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

// ─── Helpers ──────────────────────────────────────────────────────────

async fn export_for_remote(
    backend: &str,
    env_name: &str,
    format: &env_sync::SyncFormat,
) -> Result<(String, usize), AppError> {
    let sync_req = env_sync::SyncRequest {
        source_env: env_name.to_string(),
        target_name: None,
        format: Some(format.clone()),
        target_host: None,
        include_pip: Some(true),
        platforms: None,
    };
    let result = env_sync::sync_environment(&sync_req).await?;
    Ok((result.exported_content, result.packages_exported))
}

fn extract_channels(content: &str) -> Vec<String> {
    let mut channels = Vec::new();
    let mut in_channels = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "channels:" {
            in_channels = true;
            continue;
        }
        if in_channels && trimmed.starts_with("- ") {
            channels.push(trimmed.trim_start_matches("- ").to_string());
        } else if in_channels && !trimmed.starts_with('-') && !trimmed.is_empty() {
            in_channels = false;
        }
    }
    channels
}

fn extract_dependencies(content: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_deps = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "dependencies:" {
            in_deps = true;
            continue;
        }
        if in_deps && trimmed.starts_with("- ") {
            deps.push(trimmed.trim_start_matches("- ").to_string());
        } else if in_deps && !trimmed.starts_with('-') && !trimmed.starts_with(' ') {
            in_deps = false;
        }
    }
    deps
}

// ─── HTTP Handlers ────────────────────────────────────────────────────

/// POST /api/env/push
pub async fn push_handler(
    State(_state): AppStateRef,
    Json(body): Json<PushRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = push_environment(&body).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}

/// POST /api/env/pull
pub async fn pull_handler(
    State(_state): AppStateRef,
    Json(body): Json<PullRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = pull_environment(&body).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_channels() {
        let yml = "name: test\nchannels:\n- conda-forge\n- defaults\ndependencies:\n- numpy";
        let channels = extract_channels(yml);
        assert_eq!(channels, vec!["conda-forge", "defaults"]);
    }

    #[test]
    fn test_extract_dependencies() {
        let yml = "name: test\nchannels:\n- conda-forge\ndependencies:\n- numpy>=1.24\n- pandas\n- pip:\n  - transformers";
        let deps = extract_dependencies(yml);
        assert!(deps.contains(&"numpy>=1.24".to_string()));
        assert!(deps.contains(&"pandas".to_string()));
    }

    #[test]
    fn test_sync_operation_serialization() {
        assert_eq!(serde_json::to_string(&SyncOperation::Push).unwrap(), "\"push\"");
        assert_eq!(serde_json::to_string(&SyncOperation::Pull).unwrap(), "\"pull\"");
    }
}
