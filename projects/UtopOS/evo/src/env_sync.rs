//! Environment Sync Engine
//!
//! Sync environment state between machines: export → transfer → recreate.
//! Supports multiple serialization formats: conda-pack, conda-lock, pip freeze, requirements.txt.

use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Supported export/sync formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncFormat {
    #[serde(rename = "conda-pack")]
    CondaPack,
    #[serde(rename = "conda-lock")]
    CondaLock,
    #[serde(rename = "pip-freeze")]
    PipFreeze,
    #[serde(rename = "requirements")]
    Requirements,
    #[serde(rename = "environment-yml")]
    EnvironmentYml,
    #[serde(rename = "explicit")]
    Explicit,
}

impl std::fmt::Display for SyncFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncFormat::CondaPack => write!(f, "conda-pack"),
            SyncFormat::CondaLock => write!(f, "conda-lock"),
            SyncFormat::PipFreeze => write!(f, "pip-freeze"),
            SyncFormat::Requirements => write!(f, "requirements"),
            SyncFormat::EnvironmentYml => write!(f, "environment-yml"),
            SyncFormat::Explicit => write!(f, "explicit"),
        }
    }
}

/// Sync request body
#[derive(Debug, Clone, Deserialize)]
pub struct SyncRequest {
    pub source_env: String,
    pub target_name: Option<String>,
    pub format: Option<SyncFormat>,
    pub target_host: Option<String>,
    pub include_pip: Option<bool>,
    pub platforms: Option<Vec<String>>,
}

/// Sync result
#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub source_env: String,
    pub format_used: SyncFormat,
    pub exported_content: String,
    pub target_name: String,
    pub packages_exported: usize,
    pub success: bool,
    pub recreate_command: Option<String>,
    pub warnings: Vec<String>,
}

/// Export environment in all available formats
#[derive(Debug, Clone, Serialize)]
pub struct MultiFormatExport {
    pub environment: String,
    pub formats: HashMap<String, String>,
    pub package_count: usize,
    pub python_version: Option<String>,
}

// ─── Sync Engine ──────────────────────────────────────────────────────

/// Sync an environment: export in requested format and optionally create on target
pub async fn sync_environment(request: &SyncRequest) -> Result<SyncResult, AppError> {
    let backend = conda::detect_backend().await?;
    let format = request.format.clone().unwrap_or(SyncFormat::EnvironmentYml);
    let target_name = request.target_name.clone().unwrap_or_else(|| request.source_env.clone());
    let mut warnings = Vec::new();

    // Export based on format
    let (exported_content, packages_count) = export_in_format(
        &backend,
        &request.source_env,
        &format,
        request.include_pip.unwrap_or(true),
        request.platforms.as_deref(),
        &mut warnings,
    ).await?;

    // Build recreate command
    let recreate_command = build_recreate_command(&format, &target_name);

    Ok(SyncResult {
        source_env: request.source_env.clone(),
        format_used: format,
        exported_content,
        target_name,
        packages_exported: packages_count,
        success: true,
        recreate_command: Some(recreate_command),
        warnings,
    })
}

/// Export environment in a specific format
async fn export_in_format(
    backend: &str,
    env_name: &str,
    format: &SyncFormat,
    include_pip: bool,
    platforms: Option<&[String]>,
    warnings: &mut Vec<String>,
) -> Result<(String, usize), AppError> {
    match format {
        SyncFormat::EnvironmentYml => {
            let content = conda::export_env(backend, env_name).await?;
            let pkg_count = content.lines().filter(|l| l.trim().starts_with("- ")).count();
            Ok((content, pkg_count))
        }
        SyncFormat::Explicit => {
            let content = conda::export_env_explicit(backend, env_name).await?;
            let pkg_count = content.lines().filter(|l| l.starts_with("http")).count();
            Ok((content, pkg_count))
        }
        SyncFormat::CondaLock => {
            // Use conda-lock to generate lockfile
            let platforms = platforms.map(|p| p.to_vec())
                .unwrap_or_else(|| vec!["linux-64".to_string()]);
            let plat_args: Vec<&str> = platforms.iter().map(|s| s.as_str()).collect();
            let mut args = vec!["lock", "-n", env_name];
            for p in &plat_args {
                args.push("-p");
                args.push(p);
            }
            match run_cmd("conda-lock", &args).await {
                Ok(output) => {
                    // Read the generated lockfile
                    let lockfile_path = "conda-lock.yml";
                    let content = std::fs::read_to_string(lockfile_path)
                        .unwrap_or_else(|_| output.clone());
                    let pkg_count = content.lines().filter(|l| l.contains("- name:")).count();
                    Ok((content, pkg_count))
                }
                Err(e) => {
                    warnings.push(format!("conda-lock failed, falling back to environment.yml: {e}"));
                    // Fallback to environment.yml
                    let content = conda::export_env(backend, env_name).await?;
                    let pkg_count = content.lines().filter(|l| l.trim().starts_with("- ")).count();
                    Ok((content, pkg_count))
                }
            }
        }
        SyncFormat::PipFreeze => {
            let env_path = get_env_path(backend, env_name).await?;
            let python_bin = format!("{env_path}/bin/python");
            match tokio::process::Command::new(&python_bin)
                .args(["-m", "pip", "freeze"])
                .output()
                .await
            {
                Ok(output) => {
                    let content = String::from_utf8_lossy(&output.stdout).to_string();
                    let pkg_count = content.lines()
                        .filter(|l| l.contains("==") && !l.trim().is_empty())
                        .count();
                    Ok((content, pkg_count))
                }
                Err(e) => Err(AppError::CommandFailed {
                    command: "pip freeze".to_string(),
                    message: format!("Failed to run pip freeze: {e}"),
                }),
            }
        }
        SyncFormat::Requirements => {
            // Same as pip freeze but named requirements.txt
            let env_path = get_env_path(backend, env_name).await?;
            let python_bin = format!("{env_path}/bin/python");
            match tokio::process::Command::new(&python_bin)
                .args(["-m", "pip", "freeze"])
                .output()
                .await
            {
                Ok(output) => {
                    let content = String::from_utf8_lossy(&output.stdout).to_string();
                    let pkg_count = content.lines()
                        .filter(|l| l.contains("==") && !l.trim().is_empty())
                        .count();
                    // Add header
                    let header = format!("# requirements.txt — exported from {env_name}\n# Generated by nix-evo\n\n");
                    Ok((format!("{header}{content}"), pkg_count))
                }
                Err(e) => Err(AppError::CommandFailed {
                    command: "pip freeze".to_string(),
                    message: format!("Failed to generate requirements: {e}"),
                }),
            }
        }
        SyncFormat::CondaPack => {
            // conda-pack creates a tarball
            match run_cmd("conda-pack", &["-n", env_name, "-o", &format!("{env_name}.tar.gz")]).await {
                Ok(output) => {
                    warnings.push(format!("conda-pack output saved to {env_name}.tar.gz"));
                    Ok((format!("[conda-pack archive: {env_name}.tar.gz]"), 0))
                }
                Err(e) => {
                    warnings.push(format!("conda-pack not available: {e}. Install with: micromamba install -c conda-forge conda-pack"));
                    // Fallback
                    let content = conda::export_env(backend, env_name).await?;
                    let pkg_count = content.lines().filter(|l| l.trim().starts_with("- ")).count();
                    Ok((content, pkg_count))
                }
            }
        }
    }
}

/// Export environment in ALL available formats at once
pub async fn export_all_formats(env_name: &str) -> Result<MultiFormatExport, AppError> {
    let backend = conda::detect_backend().await?;
    let mut formats = HashMap::new();
    let mut total_packages = 0;

    // environment.yml
    if let Ok(content) = conda::export_env(&backend, env_name).await {
        let count = content.lines().filter(|l| l.trim().starts_with("- ")).count();
        total_packages = total_packages.max(count);
        formats.insert("environment_yml".to_string(), content);
    }

    // explicit
    if let Ok(content) = conda::export_env_explicit(&backend, env_name).await {
        formats.insert("explicit".to_string(), content);
    }

    // pip freeze
    let env_path = get_env_path(&backend, env_name).await?;
    let python_bin = format!("{env_path}/bin/python");
    if let Ok(output) = tokio::process::Command::new(&python_bin)
        .args(["-m", "pip", "freeze"])
        .output()
        .await
    {
        let content = String::from_utf8_lossy(&output.stdout).to_string();
        formats.insert("pip_freeze".to_string(), content);
    }

    // Get Python version
    let python_version = if let Ok(output) = tokio::process::Command::new(&python_bin)
        .args(["--version"])
        .output()
        .await
    {
        Some(String::from_utf8_lossy(&output.stdout).trim().replace("Python ", ""))
    } else {
        None
    };

    Ok(MultiFormatExport {
        environment: env_name.to_string(),
        formats,
        package_count: total_packages,
        python_version,
    })
}

// ─── Helpers ──────────────────────────────────────────────────────────

async fn get_env_path(backend: &str, env_name: &str) -> Result<String, AppError> {
    let envs = conda::list_envs(backend).await?;
    envs.into_iter()
        .find(|e| e.name == env_name)
        .map(|e| e.path)
        .ok_or_else(|| AppError::NotFound {
            resource: format!("conda environment: {env_name}"),
        })
}

fn build_recreate_command(format: &SyncFormat, env_name: &str) -> String {
    match format {
        SyncFormat::EnvironmentYml => {
            format!("micromamba env create -f environment.yml -n {env_name} -y")
        }
        SyncFormat::CondaLock => {
            format!("conda-lock install -n {env_name} conda-lock.yml")
        }
        SyncFormat::PipFreeze | SyncFormat::Requirements => {
            format!("python -m venv {env_name} && {env_name}/bin/pip install -r requirements.txt")
        }
        SyncFormat::Explicit => {
            format!("micromamba create -n {env_name} --file explicit.txt -y")
        }
        SyncFormat::CondaPack => {
            format!("mkdir -p {env_name} && tar -xzf {env_name}.tar.gz -C {env_name} && {env_name}/bin/conda-unpack")
        }
    }
}

// ─── HTTP Handlers ────────────────────────────────────────────────────

/// POST /api/env/sync — sync environment state
pub async fn sync_handler(
    State(_state): AppStateRef,
    Json(body): Json<SyncRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sync_environment(&body).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}

/// POST /api/env/export-all — export in all formats
#[derive(Deserialize)]
pub struct ExportAllBody {
    pub env: String,
}

pub async fn export_all_handler(
    State(_state): AppStateRef,
    Json(body): Json<ExportAllBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = export_all_formats(&body.env).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_display() {
        assert_eq!(SyncFormat::CondaPack.to_string(), "conda-pack");
        assert_eq!(SyncFormat::PipFreeze.to_string(), "pip-freeze");
        assert_eq!(SyncFormat::EnvironmentYml.to_string(), "environment-yml");
    }

    #[test]
    fn test_recreate_commands() {
        assert_eq!(
            build_recreate_command(&SyncFormat::EnvironmentYml, "test-env"),
            "micromamba env create -f environment.yml -n test-env -y"
        );
        assert_eq!(
            build_recreate_command(&SyncFormat::CondaLock, "test-env"),
            "conda-lock install -n test-env conda-lock.yml"
        );
        assert_eq!(
            build_recreate_command(&SyncFormat::PipFreeze, "test-env"),
            "python -m venv test-env && test-env/bin/pip install -r requirements.txt"
        );
        assert!(build_recreate_command(&SyncFormat::CondaPack, "test-env").contains("conda-unpack"));
    }

    #[test]
    fn test_sync_request_defaults() {
        let req = SyncRequest {
            source_env: "ml-project".to_string(),
            target_name: None,
            format: None,
            target_host: None,
            include_pip: None,
            platforms: None,
        };
        assert_eq!(req.source_env, "ml-project");
    }
}
