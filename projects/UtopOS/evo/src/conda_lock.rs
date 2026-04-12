//! conda-lock integration
//!
//! Parse and generate conda-lock.yml files.
//! Supports platform-specific lock generation (linux-64, linux-aarch64, etc.)

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::cmd::{AppStateRef, run_cmd};
use crate::error::AppError;

/// A single locked package entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub build: String,
    pub channel: String,
    pub platform: String,
    pub url: String,
    pub sha256: Option<String>,
    pub md5: Option<String>,
    pub depends: Vec<String>,
}

/// The full lockfile structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CondaLockfile {
    pub metadata: LockfileMetadata,
    pub package: Vec<LockedPackage>,
    pub platform_packages: std::collections::HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockfileMetadata {
    pub content_hash: std::collections::HashMap<String, String>,
    pub channels: Vec<ChannelEntry>,
    pub platforms: Vec<String>,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelEntry {
    pub url: String,
    pub used_env_vars: Vec<String>,
}

/// Lock generation request
#[derive(Debug, Clone, Deserialize)]
pub struct LockRequest {
    pub env: Option<String>,
    pub yml: Option<String>,
    pub platforms: Option<Vec<String>>,
    pub filename: Option<String>,
}

/// Lock generation response
#[derive(Debug, Clone, Serialize)]
pub struct LockResponse {
    pub success: bool,
    pub filename: String,
    pub platforms: Vec<String>,
    pub package_count: usize,
    pub packages: Vec<LockedPackage>,
    pub error: Option<String>,
}

/// Default platforms for lock generation
pub const DEFAULT_PLATFORMS: &[&str] = &["linux-64"];

/// All supported platforms
pub const SUPPORTED_PLATFORMS: &[&str] = &[
    "linux-64",
    "linux-aarch64",
    "linux-ppc64le",
    "osx-64",
    "osx-arm64",
    "win-64",
    "noarch",
];

/// Generate a conda-lock.yml from an environment
///
/// This wraps `conda-lock lock` or `micromamba` with conda-lock.
pub async fn generate_lock(
    backend: &str,
    yml_path: &str,
    platforms: &[&str],
    output_filename: &str,
) -> Result<LockResponse, AppError> {
    // Check if conda-lock is available
    let lock_backend = detect_lock_backend().await?;

    let mut args = vec![
        "lock",
        "--file", yml_path,
        "--lockfile", output_filename,
    ];

    for platform in platforms {
        args.push("--platform");
        args.push(platform);
    }

    let output = run_cmd(&lock_backend, &args).await;

    match output {
        Ok(out) => {
            // Try to parse the generated lockfile
            let packages = parse_lockfile(output_filename).await.unwrap_or_default();
            Ok(LockResponse {
                success: true,
                filename: output_filename.to_string(),
                platforms: platforms.iter().map(|s| s.to_string()).collect(),
                package_count: packages.len(),
                packages,
                error: None,
            })
        }
        Err(e) => Ok(LockResponse {
            success: false,
            filename: output_filename.to_string(),
            platforms: platforms.iter().map(|s| s.to_string()).collect(),
            package_count: 0,
            packages: vec![],
            error: Some(format!("Lock generation failed: {e}")),
        }),
    }
}

/// Detect available lock backend
async fn detect_lock_backend() -> Result<String, AppError> {
    if run_cmd("conda-lock", &["--version"]).await.is_ok() {
        return Ok("conda-lock".to_string());
    }

    // micromamba can also generate locks via `micromamba lock`
    if run_cmd("micromamba", &["lock", "--help"]).await.is_ok() {
        return Ok("micromamba".to_string());
    }

    Err(AppError::CommandFailed {
        command: "conda-lock/micromamba".to_string(),
        message: "No lock backend found. Install conda-lock: pip install conda-lock".to_string(),
    })
}

/// Parse a conda-lock.yml file into structured data
///
/// conda-lock.yml format is YAML with:
/// ```yaml
/// version: 1
/// metadata:
///   content_hash:
///     linux-64: abc123
///   channels:
///     - url: https://conda.anaconda.org/conda-forge
///   platforms:
///     - linux-64
/// package:
///   - name: python
///     version: 3.11.7
///     build: h955ad1f_0
///     platform: linux-64
///     depends: [...]
/// ```
pub async fn parse_lockfile(path: &str) -> Result<Vec<LockedPackage>, AppError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| AppError::IoError {
            path: path.to_string(),
            message: e.to_string(),
        })?;

    parse_lockfile_content(&content)
}

/// Parse conda-lock.yml content string
pub fn parse_lockfile_content(content: &str) -> Result<Vec<LockedPackage>, AppError> {
    let mut packages = Vec::new();
    let mut in_package_section = false;
    let mut current_pkg: Option<LockedPackage> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "package:" {
            in_package_section = true;
            continue;
        }

        if !in_package_section {
            continue;
        }

        // New package entry
        if trimmed.starts_with("- name:") {
            // Save previous package
            if let Some(pkg) = current_pkg.take() {
                packages.push(pkg);
            }
            let name = trimmed.trim_start_matches("- name:").trim().to_string();
            current_pkg = Some(LockedPackage {
                name,
                version: String::new(),
                build: String::new(),
                channel: String::new(),
                platform: String::new(),
                url: String::new(),
                sha256: None,
                md5: None,
                depends: vec![],
            });
        } else if trimmed.starts_with("name:") {
            if let Some(pkg) = current_pkg.take() {
                packages.push(pkg);
            }
            let name = trimmed.trim_start_matches("name:").trim().to_string();
            current_pkg = Some(LockedPackage {
                name,
                version: String::new(),
                build: String::new(),
                channel: String::new(),
                platform: String::new(),
                url: String::new(),
                sha256: None,
                md5: None,
                depends: vec![],
            });
        } else if let Some(ref mut pkg) = current_pkg {
            if trimmed.starts_with("version:") {
                pkg.version = trimmed.trim_start_matches("version:").trim().to_string();
            } else if trimmed.starts_with("build:") {
                pkg.build = trimmed.trim_start_matches("build:").trim().to_string();
            } else if trimmed.starts_with("platform:") {
                pkg.platform = trimmed.trim_start_matches("platform:").trim().to_string();
            } else if trimmed.starts_with("channel:") || trimmed.starts_with("url:") {
                let val = trimmed.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
                if trimmed.starts_with("channel:") {
                    pkg.channel = val;
                } else {
                    pkg.url = val;
                }
            } else if trimmed.starts_with("sha256:") {
                pkg.sha256 = Some(trimmed.trim_start_matches("sha256:").trim().to_string());
            } else if trimmed.starts_with("md5:") {
                pkg.md5 = Some(trimmed.trim_start_matches("md5:").trim().to_string());
            }
        }
    }

    // Don't forget the last package
    if let Some(pkg) = current_pkg {
        packages.push(pkg);
    }

    Ok(packages)
}

/// Generate a lockfile from current environment state (without conda-lock)
///
/// Fallback: use micromamba list --explicit to generate a simple lock
pub async fn generate_explicit_lock(
    backend: &str,
    env_name: &str,
) -> Result<String, AppError> {
    let args = vec!["list", "-n", env_name, "--explicit", "--no-pip"];
    run_cmd(backend, &args).await
}

/// Check lockfile staleness: compare against current environment
pub async fn check_lock_staleness(
    backend: &str,
    env_name: &str,
    lockfile_path: &str,
) -> Result<LockStalenessReport, AppError> {
    let locked = parse_lockfile(lockfile_path).await?;
    let installed = crate::conda::list_packages(backend, env_name).await?;

    let locked_map: std::collections::HashMap<&str, &str> = locked.iter()
        .map(|p| (p.name.as_str(), p.version.as_str()))
        .collect();

    let installed_map: std::collections::HashMap<&str, &str> = installed.iter()
        .map(|p| (p.name.as_str(), p.version.as_str()))
        .collect();

    // Packages installed but not in lockfile
    let new_packages: Vec<String> = installed.iter()
        .filter(|p| !locked_map.contains_key(p.name.as_str()))
        .map(|p| format!("{}={}", p.name, p.version))
        .collect();

    // Packages in lockfile but not installed
    let removed_packages: Vec<String> = locked.iter()
        .filter(|p| !installed_map.contains_key(p.name.as_str()))
        .map(|p| format!("{}={}", p.name, p.version))
        .collect();

    // Version mismatches
    let version_drifts: Vec<String> = installed.iter()
        .filter_map(|p| {
            if let Some(locked_ver) = locked_map.get(p.name.as_str()) {
                if *locked_ver != p.version {
                    Some(format!("{}: lock={} installed={}", p.name, locked_ver, p.version))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let is_stale = !new_packages.is_empty() || !removed_packages.is_empty() || !version_drifts.is_empty();

    Ok(LockStalenessReport {
        environment: env_name.to_string(),
        lockfile: lockfile_path.to_string(),
        is_stale,
        new_packages,
        removed_packages,
        version_drifts,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct LockStalenessReport {
    pub environment: String,
    pub lockfile: String,
    pub is_stale: bool,
    pub new_packages: Vec<String>,
    pub removed_packages: Vec<String>,
    pub version_drifts: Vec<String>,
}

// ─── HTTP Handler ─────────────────────────────────────────────────────

/// POST /api/conda/lock
pub async fn lock_handler(
    State(_state): AppStateRef,
    Json(body): Json<LockRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = crate::conda::detect_backend().await?;

    // Resolve yml path
    let yml_path = match (&body.yml, &body.env) {
        (Some(path), _) => path.clone(),
        (None, Some(env_name)) => {
            // Try to find environment.yml
            format!("/etc/nix-evo/conda/{env_name}/environment.yml")
        }
        (None, None) => {
            return Err(AppError::Validation {
                field: "env/yml".to_string(),
                message: "Either 'env' or 'yml' parameter is required".to_string(),
            });
        }
    };

    let platforms: Vec<&str> = body.platforms
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect())
        .unwrap_or_else(|| DEFAULT_PLATFORMS.to_vec());

    let filename = body.filename.as_deref().unwrap_or("conda-lock.yml");

    let response = generate_lock(&backend, &yml_path, &platforms, filename).await?;
    Ok(Json(serde_json::to_value(&response).unwrap()))
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lockfile_content() {
        let content = r#"version: 1
metadata:
  content_hash:
    linux-64: abc123
  channels:
    - url: https://conda.anaconda.org/conda-forge
  platforms:
    - linux-64
package:
  - name: python
    version: 3.11.7
    build: h955ad1f_0
    platform: linux-64
    channel: conda-forge
    url: https://conda.anaconda.org/conda-forge/linux-64/python-3.11.7-h955ad1f_0.conda
    sha256: abc123def456
  - name: numpy
    version: 1.26.3
    build: py311h5d0b8f6_0
    platform: linux-64
    channel: conda-forge
"#;

        let packages = parse_lockfile_content(content).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "python");
        assert_eq!(packages[0].version, "3.11.7");
        assert_eq!(packages[0].build, "h955ad1f_0");
        assert_eq!(packages[0].platform, "linux-64");
        assert_eq!(packages[0].sha256, Some("abc123def456".to_string()));
        assert_eq!(packages[1].name, "numpy");
        assert_eq!(packages[1].version, "1.26.3");
    }

    #[test]
    fn test_parse_empty_lockfile() {
        let content = "version: 1\nmetadata:\n  content_hash: {}\n";
        let packages = parse_lockfile_content(content).unwrap();
        assert!(packages.is_empty());
    }
}
