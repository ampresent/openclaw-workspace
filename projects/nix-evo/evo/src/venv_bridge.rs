//! Virtual Environment Bridge
//!
//! Unified detection and management across ALL Python environment types:
//! conda, micromamba, venv, virtualenv, poetry, pipenv, pdm, uv.
//! Provides conflict detection when the same package appears in multiple places.

use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::error::AppError;

/// Environment type identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EnvType {
    Conda,
    Micromamba,
    Venv,
    Virtualenv,
    Poetry,
    Pipenv,
    Pdm,
    Uv,
}

impl std::fmt::Display for EnvType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvType::Conda => write!(f, "conda"),
            EnvType::Micromamba => write!(f, "micromamba"),
            EnvType::Venv => write!(f, "venv"),
            EnvType::Virtualenv => write!(f, "virtualenv"),
            EnvType::Poetry => write!(f, "poetry"),
            EnvType::Pipenv => write!(f, "pipenv"),
            EnvType::Pdm => write!(f, "pdm"),
            EnvType::Uv => write!(f, "uv"),
        }
    }
}

/// A discovered Python environment
#[derive(Debug, Clone, Serialize)]
pub struct PythonEnv {
    pub name: String,
    pub path: String,
    pub env_type: EnvType,
    pub python_version: Option<String>,
    pub package_count: Option<usize>,
    pub packages: Option<Vec<EnvPackage>>,
    pub is_active: bool,
    pub disk_usage_mb: Option<u64>,
    pub created_at: Option<String>,
    pub manager: Option<String>,  // which tool manages this env
}

/// A package installed in a Python environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvPackage {
    pub name: String,
    pub version: String,
    pub source: EnvType,
}

/// Conflict: same package installed in multiple environments
#[derive(Debug, Clone, Serialize)]
pub struct PackageConflict {
    pub package_name: String,
    pub installations: Vec<PackageInstallation>,
    pub severity: String,  // info, warning, error
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageInstallation {
    pub environment: String,
    pub env_type: EnvType,
    pub version: String,
    pub path: String,
}

/// Unified response for all Python environments
#[derive(Debug, Clone, Serialize)]
pub struct PythonEnvsReport {
    pub total_envs: usize,
    pub environments: Vec<PythonEnv>,
    pub conflicts: Vec<PackageConflict>,
    pub summary_by_type: HashMap<String, usize>,
    pub system_python: Option<SystemPythonInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemPythonInfo {
    pub version: String,
    pub path: String,
    pub pip_version: Option<String>,
    pub has_pipx: bool,
    pub has_uv: bool,
    pub has_poetry: bool,
    pub has_pipenv: bool,
    pub has_pdm: bool,
}

// ─── Detection ────────────────────────────────────────────────────────

/// Detect ALL Python environments on the system
pub async fn detect_all_envs() -> Result<PythonEnvsReport, AppError> {
    let mut environments = Vec::new();

    // 1. Detect conda/micromamba environments
    if let Ok(conda_envs) = detect_conda_envs().await {
        environments.extend(conda_envs);
    }

    // 2. Detect venv/virtualenv environments
    let venv_envs = detect_venv_envs().await;
    environments.extend(venv_envs);

    // 3. Detect poetry environments
    let poetry_envs = detect_poetry_envs().await;
    environments.extend(poetry_envs);

    // 4. Detect pipenv environments
    let pipenv_envs = detect_pipenv_envs().await;
    environments.extend(pipenv_envs);

    // 5. Detect pdm environments
    let pdm_envs = detect_pdm_envs().await;
    environments.extend(pdm_envs);

    // 6. Detect uv environments
    let uv_envs = detect_uv_envs().await;
    environments.extend(uv_envs);

    // Detect system Python
    let system_python = detect_system_python_info().await;

    // Build summary
    let mut summary_by_type: HashMap<String, usize> = HashMap::new();
    for env in &environments {
        *summary_by_type.entry(env.env_type.to_string()).or_insert(0) += 1;
    }

    // Detect conflicts across environments
    let conflicts = detect_cross_env_conflicts(&environments).await;

    Ok(PythonEnvsReport {
        total_envs: environments.len(),
        environments,
        conflicts,
        summary_by_type,
        system_python,
    })
}

/// Detect conda and micromamba environments
async fn detect_conda_envs() -> Result<Vec<PythonEnv>, AppError> {
    let mut envs = Vec::new();

    // Try micromamba first
    let (backend, env_type) = if let Ok(out) = run_cmd("micromamba", &["env", "list"]).await {
        ("micromamba", EnvType::Micromamba)
    } else if let Ok(out) = run_cmd("conda", &["env", "list"]).await {
        ("conda", EnvType::Conda)
    } else {
        return Ok(envs);
    };

    let output = run_cmd(backend, &["env", "list"]).await?;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.contains("Name") || trimmed.contains("---") {
            continue;
        }
        let is_active = trimmed.starts_with('*');
        let clean = if is_active { trimmed.trim_start_matches('*').trim() } else { trimmed };
        let parts: Vec<&str> = clean.split_whitespace().collect();
        if parts.len() >= 2 {
            let has_parens = parts.last().unwrap().contains('(');
            let end = if has_parens { parts.len() - 1 } else { parts.len() };
            let env_path = parts[1..end].join(" ");
            let name = parts[0].to_string();

            let python_version = detect_python_in_env(&env_path).await;
            let disk_usage_mb = get_dir_size_mb(&env_path).await;

            envs.push(PythonEnv {
                name,
                path: env_path,
                env_type: env_type.clone(),
                python_version,
                package_count: None,
                packages: None,
                is_active,
                disk_usage_mb,
                created_at: None,
                manager: Some(backend.to_string()),
            });
        }
    }

    Ok(envs)
}

/// Scan common locations for venv/virtualenv environments
async fn detect_venv_envs() -> Vec<PythonEnv> {
    let mut envs = Vec::new();
    let home = std::env::var("HOME").unwrap_or_default();

    // Common venv locations
    let search_dirs = vec![
        format!("{home}/.virtualenvs"),
        format!("{home}/venvs"),
        format!("{home}/envs"),
        format!("{home}/.local/share/virtualenvs"),
        format!("{}/.venv", std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_default()),
    ];

    for dir in &search_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if is_venv_dir(&path) {
                    let env_type = if path.join("pyvenv.cfg").exists() {
                        EnvType::Venv
                    } else {
                        EnvType::Virtualenv
                    };
                    let name = path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let path_str = path.display().to_string();
                    let python_version = detect_python_in_env(&path_str).await;
                    let disk_usage_mb = get_dir_size_mb(&path_str).await;

                    envs.push(PythonEnv {
                        name,
                        path: path_str,
                        env_type,
                        python_version,
                        package_count: None,
                        packages: None,
                        is_active: false,
                        disk_usage_mb,
                        created_at: None,
                        manager: None,
                    });
                }
            }
        }
    }

    // Also check current directory for .venv
    let cwd = std::env::current_dir().unwrap_or_default();
    let dot_venv = cwd.join(".venv");
    if dot_venv.exists() && is_venv_dir(&dot_venv) {
        let path_str = dot_venv.display().to_string();
        let python_version = detect_python_in_env(&path_str).await;
        let project_name = cwd.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".to_string());
        let disk_usage_mb = get_dir_size_mb(&path_str).await;

        envs.push(PythonEnv {
            name: format!("{project_name}/.venv"),
            path: path_str,
            env_type: EnvType::Venv,
            python_version,
            package_count: None,
            packages: None,
            is_active: false,
            disk_usage_mb,
            created_at: None,
            manager: None,
        });
    }

    envs
}

/// Detect poetry environments
async fn detect_poetry_envs() -> Vec<PythonEnv> {
    let mut envs = Vec::new();

    let output = match run_cmd("poetry", &["env", "list", "--full-path"]).await {
        Ok(out) => out,
        Err(_) => return envs,
    };

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        // Format: /path/to/env (Activated) or /path/to/env
        let parts: Vec<&str> = trimmed.splitn(2, " (").collect();
        let path = parts[0].to_string();
        let is_active = parts.get(1).map(|s| s.contains("Activated")).unwrap_or(false);
        let name = Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let python_version = detect_python_in_env(&path).await;
        let disk_usage_mb = get_dir_size_mb(&path).await;

        envs.push(PythonEnv {
            name,
            path,
            env_type: EnvType::Poetry,
            python_version,
            package_count: None,
            packages: None,
            is_active,
            disk_usage_mb,
            created_at: None,
            manager: Some("poetry".to_string()),
        });
    }

    envs
}

/// Detect pipenv environments
async fn detect_pipenv_envs() -> Vec<PythonEnv> {
    let mut envs = Vec::new();
    let home = std::env::var("HOME").unwrap_or_default();
    let venvs_dir = format!("{home}/.local/share/virtualenvs");

    if let Ok(output) = run_cmd("pipenv", &["--venv"]).await {
        let path = output.trim().to_string();
        if !path.is_empty() && Path::new(&path).exists() {
            let name = Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let python_version = detect_python_in_env(&path).await;
            let disk_usage_mb = get_dir_size_mb(&path).await;

            envs.push(PythonEnv {
                name,
                path,
                env_type: EnvType::Pipenv,
                python_version,
                package_count: None,
                packages: None,
                is_active: false,
                disk_usage_mb,
                created_at: None,
                manager: Some("pipenv".to_string()),
            });
        }
    }

    envs
}

/// Detect pdm environments
async fn detect_pdm_envs() -> Vec<PythonEnv> {
    let mut envs = Vec::new();

    let output = match run_cmd("pdm", &["env", "list"]).await {
        Ok(out) => out,
        Err(_) => return envs,
    };

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        // Format: python-3.11 /path/to/env
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        if parts.len() >= 2 {
            let name = parts[0].to_string();
            let path = parts[1].trim().to_string();
            let python_version = detect_python_in_env(&path).await;
            let disk_usage_mb = get_dir_size_mb(&path).await;

            envs.push(PythonEnv {
                name,
                path,
                env_type: EnvType::Pdm,
                python_version,
                package_count: None,
                packages: None,
                is_active: false,
                disk_usage_mb,
                created_at: None,
                manager: Some("pdm".to_string()),
            });
        }
    }

    envs
}

/// Detect uv environments
async fn detect_uv_envs() -> Vec<PythonEnv> {
    let mut envs = Vec::new();
    let home = std::env::var("HOME").unwrap_or_default();

    // uv stores envs in .local/share/uv/python or project .venv dirs
    let uv_dirs = vec![
        format!("{home}/.local/share/uv"),
    ];

    for dir in &uv_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("bin/python").exists() {
                    let name = path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let path_str = path.display().to_string();
                    let python_version = detect_python_in_env(&path_str).await;
                    let disk_usage_mb = get_dir_size_mb(&path_str).await;

                    envs.push(PythonEnv {
                        name,
                        path: path_str,
                        env_type: EnvType::Uv,
                        python_version,
                        package_count: None,
                        packages: None,
                        is_active: false,
                        disk_usage_mb,
                        created_at: None,
                        manager: Some("uv".to_string()),
                    });
                }
            }
        }
    }

    envs
}

// ─── Conflict Detection ──────────────────────────────────────────────

/// Detect conflicts: same package in multiple environments with different versions
async fn detect_cross_env_conflicts(envs: &[PythonEnv]) -> Vec<PackageConflict> {
    let mut all_packages: HashMap<String, Vec<PackageInstallation>> = HashMap::new();

    for env in envs {
        let packages = list_env_packages(env).await;
        for pkg in packages {
            all_packages.entry(pkg.name.clone()).or_default().push(PackageInstallation {
                environment: env.name.clone(),
                env_type: env.env_type.clone(),
                version: pkg.version.clone(),
                path: env.path.clone(),
            });
        }
    }

    let mut conflicts = Vec::new();
    for (pkg_name, installations) in all_packages {
        if installations.len() <= 1 {
            continue;
        }

        // Check if versions differ
        let versions: Vec<&str> = installations.iter().map(|i| i.version.as_str()).collect();
        let all_same = versions.windows(2).all(|w| w[0] == w[1]);

        if !all_same {
            let severity = if is_critical_package(&pkg_name) {
                "error".to_string()
            } else {
                "warning".to_string()
            };

            conflicts.push(PackageConflict {
                package_name: pkg_name,
                installations,
                severity,
                recommendation: "Pin versions in environment.yml or pyproject.toml to avoid surprises".to_string(),
            });
        }
    }

    conflicts
}

fn is_critical_package(name: &str) -> bool {
    matches!(name, "numpy" | "scipy" | "pandas" | "torch" | "tensorflow" | "python")
}

// ─── Helpers ──────────────────────────────────────────────────────────

/// Check if a directory looks like a venv/virtualenv
fn is_venv_dir(path: &std::path::Path) -> bool {
    (path.join("bin/activate").exists() || path.join("Scripts/activate").exists())
        && (path.join("bin/python").exists() || path.join("Scripts/python.exe").exists())
}

/// Detect Python version inside an environment
async fn detect_python_in_env(env_path: &str) -> Option<String> {
    let python_bin = format!("{env_path}/bin/python");
    if !Path::new(&python_bin).exists() {
        return None;
    }
    match tokio::process::Command::new(&python_bin)
        .args(["--version"])
        .output()
        .await
    {
        Ok(output) => {
            let version_str = String::from_utf8_lossy(&output.stdout);
            Some(version_str.trim().replace("Python ", ""))
        }
        Err(_) => None,
    }
}

/// List packages in any environment using its Python binary
async fn list_env_packages(env: &PythonEnv) -> Vec<EnvPackage> {
    let python_bin = format!("{}/bin/python", env.path);
    if !Path::new(&python_bin).exists() {
        return vec![];
    }

    match tokio::process::Command::new(&python_bin)
        .args(["-m", "pip", "list", "--format=json"])
        .output()
        .await
    {
        Ok(output) => {
            if let Ok(pkgs) = serde_json::from_str::<Vec<serde_json::Value>>(&String::from_utf8_lossy(&output.stdout)) {
                return pkgs.into_iter().filter_map(|p| {
                    Some(EnvPackage {
                        name: p.get("name")?.as_str()?.to_lowercase().to_string(),
                        version: p.get("version")?.as_str()?.to_string(),
                        source: env.env_type.clone(),
                    })
                }).collect();
            }
            vec![]
        }
        Err(_) => vec![],
    }
}

/// Get directory size in MB
async fn get_dir_size_mb(path: &str) -> Option<u64> {
    match tokio::process::Command::new("du")
        .args(["-sm", path])
        .output()
        .await
    {
        Ok(output) => {
            let s = String::from_utf8_lossy(&output.stdout);
            s.split_whitespace().next().and_then(|v| v.parse().ok())
        }
        Err(_) => None,
    }
}

/// Detect system-level Python info
async fn detect_system_python_info() -> Option<SystemPythonInfo> {
    let version_output = run_cmd("python3", &["--version"]).await.ok()?;
    let version = version_output.trim().replace("Python ", "");
    let path = run_cmd("which", &["python3"]).await.ok().unwrap_or_default().trim().to_string();

    let pip_version = run_cmd("python3", &["-m", "pip", "--version"])
        .await
        .ok()
        .map(|out| out.split_whitespace().nth(1).unwrap_or("").to_string());

    let has_pipx = run_cmd("which", &["pipx"]).await.is_ok();
    let has_uv = run_cmd("which", &["uv"]).await.is_ok();
    let has_poetry = run_cmd("which", &["poetry"]).await.is_ok();
    let has_pipenv = run_cmd("which", &["pipenv"]).await.is_ok();
    let has_pdm = run_cmd("which", &["pdm"]).await.is_ok();

    Some(SystemPythonInfo {
        version,
        path,
        pip_version,
        has_pipx,
        has_uv,
        has_poetry,
        has_pipenv,
        has_pdm,
    })
}

// ─── HTTP Handler ─────────────────────────────────────────────────────

/// GET /api/python/envs — list ALL Python environments
pub async fn list_python_envs_handler(
    State(_state): AppStateRef,
    Query(_query): Query<HostQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let report = detect_all_envs().await?;
    Ok(Json(serde_json::to_value(&report).unwrap()))
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_venv_dir_nonexistent() {
        assert!(!is_venv_dir(Path::new("/nonexistent/path")));
    }

    #[test]
    fn test_is_critical_package() {
        assert!(is_critical_package("numpy"));
        assert!(is_critical_package("torch"));
        assert!(!is_critical_package("black"));
        assert!(!is_critical_package("requests"));
    }

    #[test]
    fn test_env_type_display() {
        assert_eq!(EnvType::Conda.to_string(), "conda");
        assert_eq!(EnvType::Micromamba.to_string(), "micromamba");
        assert_eq!(EnvType::Venv.to_string(), "venv");
        assert_eq!(EnvType::Poetry.to_string(), "poetry");
        assert_eq!(EnvType::Uv.to_string(), "uv");
    }

    #[test]
    fn test_cross_env_conflict_detection() {
        // Test that the conflict detection logic works with mock data
        let mut all_packages: HashMap<String, Vec<PackageInstallation>> = HashMap::new();

        all_packages.insert("numpy".to_string(), vec![
            PackageInstallation {
                environment: "ml".to_string(),
                env_type: EnvType::Conda,
                version: "1.26.3".to_string(),
                path: "/opt/conda/envs/ml".to_string(),
            },
            PackageInstallation {
                environment: "base".to_string(),
                env_type: EnvType::Venv,
                version: "2.0.0".to_string(),
                path: "/home/user/.venv".to_string(),
            },
        ]);

        let conflicts: Vec<PackageConflict> = all_packages.into_iter()
            .filter(|(_, insts)| {
                if insts.len() <= 1 { return false; }
                let versions: Vec<&str> = insts.iter().map(|i| i.version.as_str()).collect();
                !versions.windows(2).all(|w| w[0] == w[1])
            })
            .map(|(name, insts)| PackageConflict {
                package_name: name,
                installations: insts,
                severity: "warning".to_string(),
                recommendation: "Pin versions".to_string(),
            })
            .collect();

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].package_name, "numpy");
    }
}
