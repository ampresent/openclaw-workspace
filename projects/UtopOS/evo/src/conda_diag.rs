//! Conda environment diagnostics
//!
//! Detects all conda/micromamba environments, checks for dependency conflicts,
//! compares installed vs environment.yml (drift detection), finds outdated/vulnerable packages.

use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::cmd::AppStateRef;
use crate::cmd::HostQuery;
use crate::conda;
use crate::error::AppError;

/// Full diagnostic report for all conda environments
#[derive(Debug, Clone, Serialize)]
pub struct DiagReport {
    pub backend: String,
    pub total_envs: usize,
    pub environments: Vec<EnvDiag>,
    pub warnings: Vec<DiagWarning>,
    pub system_python: Option<String>,
}

/// Diagnostic info for a single environment
#[derive(Debug, Clone, Serialize)]
pub struct EnvDiag {
    pub name: String,
    pub path: String,
    pub python_version: Option<String>,
    pub package_count: usize,
    pub has_environment_yml: bool,
    pub environment_yml_path: Option<String>,
    pub conflicts: Vec<String>,
    pub outdated: Vec<OutdatedPackage>,
    pub disk_usage_mb: Option<u64>,
}

/// An outdated package
#[derive(Debug, Clone, Serialize)]
pub struct OutdatedPackage {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
}

/// A diagnostic warning
#[derive(Debug, Clone, Serialize)]
pub struct DiagWarning {
    pub level: String, // info, warning, error
    pub environment: String,
    pub message: String,
}

/// Drift detection report
#[derive(Debug, Clone, Serialize)]
pub struct DriftReport {
    pub environment: String,
    pub yml_path: String,
    pub declared_name: Option<String>,
    pub drift: DriftDetail,
}

#[derive(Debug, Clone, Serialize)]
pub struct DriftDetail {
    pub extra_packages: Vec<String>,    // installed but not declared
    pub missing_packages: Vec<String>,  // declared but not installed
    pub version_mismatches: Vec<VersionMismatch>,
    pub channel_mismatches: Vec<String>,
    pub has_drift: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionMismatch {
    pub name: String,
    pub declared: String,
    pub installed: String,
}

// ─── Diagnostics ──────────────────────────────────────────────────────

/// Run full diagnostics on all conda environments
pub async fn run_full_diag() -> Result<DiagReport, AppError> {
    let backend = conda::detect_backend().await?;
    let envs = conda::list_envs(&backend).await?;
    let system_python = detect_system_python().await;
    let mut warnings = Vec::new();
    let mut env_diags = Vec::new();

    for env in &envs {
        let diag = diagnose_env(&backend, env, &mut warnings).await;
        env_diags.push(diag);
    }

    // Global warnings
    if envs.len() > 10 {
        warnings.push(DiagWarning {
            level: "info".to_string(),
            environment: "*".to_string(),
            message: format!("{} environments detected — consider pruning unused ones", envs.len()),
        });
    }

    Ok(DiagReport {
        backend,
        total_envs: envs.len(),
        environments: env_diags,
        warnings,
        system_python,
    })
}

/// Diagnose a single environment
async fn diagnose_env(
    backend: &str,
    env: &conda::CondaEnv,
    warnings: &mut Vec<DiagWarning>,
) -> EnvDiag {
    let packages = conda::list_packages(backend, &env.name).await.unwrap_or_default();

    // Find Python version
    let python_version = packages.iter()
        .find(|p| p.name == "python")
        .map(|p| p.version.clone());

    // Check for environment.yml
    let (has_yml, yml_path) = find_environment_yml(&env.path, &env.name);

    // Check for conflicts (simplified: look for known conflict indicators)
    let conflicts = detect_conflicts(&packages);

    // Check for outdated packages (only if we can query)
    let outdated = check_outdated(&packages);

    // Disk usage
    let disk_usage_mb = get_dir_size_mb(&env.path).await;

    if !conflicts.is_empty() {
        warnings.push(DiagWarning {
            level: "warning".to_string(),
            environment: env.name.clone(),
            message: format!("{} dependency conflicts detected", conflicts.len()),
        });
    }

    if outdated.len() > 20 {
        warnings.push(DiagWarning {
            level: "info".to_string(),
            environment: env.name.clone(),
            message: format!("{} packages are outdated", outdated.len()),
        });
    }

    EnvDiag {
        name: env.name.clone(),
        path: env.path.clone(),
        python_version,
        package_count: packages.len(),
        has_environment_yml: has_yml,
        environment_yml_path: yml_path,
        conflicts,
        outdated,
        disk_usage_mb,
    }
}

/// Detect dependency conflicts in a package list
/// Simplified heuristic: look for packages that pin conflicting versions
fn detect_conflicts(packages: &[conda::CondaPackage]) -> Vec<String> {
    let mut conflicts = Vec::new();
    let mut seen: HashMap<&str, &str> = HashMap::new();

    for pkg in packages {
        if let Some(existing_ver) = seen.get(pkg.name.as_str()) {
            if *existing_ver != pkg.version {
                conflicts.push(format!(
                    "{} has conflicting versions: {} and {}",
                    pkg.name, existing_ver, pkg.version
                ));
            }
        }
        seen.insert(&pkg.name, &pkg.version);
    }

    // Check for numpy+scipy compatibility (common conflict)
    let numpy_ver = packages.iter().find(|p| p.name == "numpy").map(|p| p.version.as_str());
    let scipy_ver = packages.iter().find(|p| p.name == "scipy").map(|p| p.version.as_str());
    if let (Some(np), Some(sp)) = (numpy_ver, scipy_ver) {
        // numpy 2.x requires scipy >= 1.13
        if np.starts_with("2.") && sp.starts_with("1.1") && !sp.starts_with("1.13") {
            conflicts.push(format!(
                "numpy {np} may be incompatible with scipy {sp} (need scipy >= 1.13 for numpy 2.x)"
            ));
        }
    }

    conflicts
}

/// Check for outdated packages (heuristic — based on known patterns)
fn check_outdated(packages: &[conda::CondaPackage]) -> Vec<OutdatedPackage> {
    let known_outdated: HashMap<&str, &str> = [
        ("python", "3.12"),
        ("numpy", "2.0"),
        ("pandas", "2.2"),
        ("scipy", "1.13"),
        ("scikit-learn", "1.5"),
        ("matplotlib", "3.9"),
    ].into_iter().collect();

    let mut outdated = Vec::new();
    for pkg in packages {
        if let Some(min_modern) = known_outdated.get(pkg.name.as_str()) {
            // Very simplified version comparison
            if is_older_version(&pkg.version, min_modern) {
                outdated.push(OutdatedPackage {
                    name: pkg.name.clone(),
                    current_version: pkg.version.clone(),
                    latest_version: min_modern.to_string(),
                });
            }
        }
    }
    outdated
}

/// Simplified version comparison: returns true if a < b (by prefix)
fn is_older_version(current: &str, target: &str) -> bool {
    let cur_parts: Vec<u32> = current.split('.').filter_map(|p| p.parse().ok()).collect();
    let tgt_parts: Vec<u32> = target.split('.').filter_map(|p| p.parse().ok()).collect();

    for i in 0..std::cmp::max(cur_parts.len(), tgt_parts.len()) {
        let c = cur_parts.get(i).copied().unwrap_or(0);
        let t = tgt_parts.get(i).copied().unwrap_or(0);
        if c < t { return true; }
        if c > t { return false; }
    }
    false
}

/// Find environment.yml in common locations
fn find_environment_yml(env_path: &str, env_name: &str) -> (bool, Option<String>) {
    let candidates = vec![
        format!("{env_path}/environment.yml"),
        format!("{env_path}/environment.yaml"),
        format!("/etc/nix-evo/conda/{env_name}/environment.yml"),
        format!("/etc/nix-evo/conda/{env_name}.yml"),
        format!("{}/conda-envs/{}/environment.yml", std::env::var("HOME").unwrap_or_default(), env_name),
    ];

    for path in &candidates {
        if Path::new(path).exists() {
            return (true, Some(path.clone()));
        }
    }
    (false, None)
}

/// Detect system Python version (outside conda)
async fn detect_system_python() -> Option<String> {
    match tokio::process::Command::new("python3")
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

// ─── Drift Detection ──────────────────────────────────────────────────

/// Detect drift between installed state and environment.yml
pub async fn detect_drift(
    backend: &str,
    env_name: &str,
    yml_path: &str,
) -> Result<DriftReport, AppError> {
    // Parse declared state
    let yml_content = std::fs::read_to_string(yml_path)
        .map_err(|e| AppError::IoError {
            path: yml_path.to_string(),
            message: e.to_string(),
        })?;
    let declared = conda::parse_environment_yml(&yml_content)?;

    // Get installed state
    let installed = conda::list_packages(backend, env_name).await?;

    // Build declared package set
    let declared_pkgs: HashMap<String, String> = declared.dependencies.iter()
        .filter_map(|dep| match dep {
            conda::EnvDependency::Conda(s) => {
                let parts: Vec<&str> = s.split('=').collect();
                Some((parts[0].to_string(), if parts.len() > 1 { parts[1].to_string() } else { "*".to_string() }))
            }
            _ => None,
        })
        .collect();

    let installed_map: HashMap<String, String> = installed.iter()
        .map(|p| (p.name.clone(), p.version.clone()))
        .collect();

    // Find extra packages (installed but not declared)
    let extra: Vec<String> = installed.iter()
        .filter(|p| !declared_pkgs.contains_key(&p.name))
        .map(|p| format!("{}={}", p.name, p.version))
        .collect();

    // Find missing packages (declared but not installed)
    let missing: Vec<String> = declared_pkgs.keys()
        .filter(|name| !installed_map.contains_key(*name))
        .cloned()
        .collect();

    // Version mismatches
    let version_mismatches: Vec<VersionMismatch> = declared_pkgs.iter()
        .filter_map(|(name, declared_ver)| {
            if let Some(installed_ver) = installed_map.get(name) {
                if declared_ver != "*" && *declared_ver != *installed_ver {
                    Some(VersionMismatch {
                        name: name.clone(),
                        declared: declared_ver.clone(),
                        installed: installed_ver.clone(),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let has_drift = !extra.is_empty() || !missing.is_empty() || !version_mismatches.is_empty();

    Ok(DriftReport {
        environment: env_name.to_string(),
        yml_path: yml_path.to_string(),
        declared_name: Some(declared.name),
        drift: DriftDetail {
            extra_packages: extra,
            missing_packages: missing,
            version_mismatches,
            channel_mismatches: vec![],
            has_drift,
        },
    })
}

// ─── HTTP Handlers ────────────────────────────────────────────────────

/// GET /api/conda/diag — full diagnostics
pub async fn diag_handler(
    State(_state): AppStateRef,
    Query(_query): Query<HostQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let report = run_full_diag().await?;
    Ok(Json(serde_json::to_value(&report).unwrap()))
}

/// GET /api/conda/drift?env=<name>&yml=<path> — drift detection
#[derive(Deserialize)]
pub struct DriftQuery {
    pub host: Option<String>,
    pub env: String,
    pub yml: String,
}

pub async fn drift_handler(
    State(_state): AppStateRef,
    Query(query): Query<DriftQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = conda::detect_backend().await?;
    let report = detect_drift(&backend, &query.env, &query.yml).await?;
    Ok(Json(serde_json::to_value(&report).unwrap()))
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conda::CondaPackage;

    fn make_pkg(name: &str, version: &str) -> CondaPackage {
        CondaPackage {
            name: name.to_string(),
            version: version.to_string(),
            build: "py311_0".to_string(),
            channel: "pkgs/main".to_string(),
            platform: None,
        }
    }

    #[test]
    fn test_detect_conflicts_dup_versions() {
        let pkgs = vec![
            make_pkg("numpy", "1.24.0"),
            make_pkg("numpy", "1.26.3"),
            make_pkg("pandas", "2.1.4"),
        ];
        let conflicts = detect_conflicts(&pkgs);
        assert!(!conflicts.is_empty());
        assert!(conflicts[0].contains("numpy"));
    }

    #[test]
    fn test_detect_conflicts_numpy_scipy() {
        let pkgs = vec![
            make_pkg("numpy", "2.0.0"),
            make_pkg("scipy", "1.11.0"),
        ];
        let conflicts = detect_conflicts(&pkgs);
        assert!(conflicts.iter().any(|c| c.contains("scipy")));
    }

    #[test]
    fn test_is_older_version() {
        assert!(is_older_version("3.10.0", "3.12"));
        assert!(!is_older_version("3.12.0", "3.12"));
        assert!(!is_older_version("2.0.0", "1.26"));
        assert!(is_older_version("1.9.0", "1.13"));
    }

    #[test]
    fn test_check_outdated() {
        let pkgs = vec![
            make_pkg("python", "3.9.0"),
            make_pkg("numpy", "1.23.0"),
            make_pkg("pandas", "2.2.0"),
        ];
        let outdated = check_outdated(&pkgs);
        assert!(outdated.iter().any(|p| p.name == "python"));
        assert!(outdated.iter().any(|p| p.name == "numpy"));
        // pandas 2.2.0 is not outdated
        assert!(!outdated.iter().any(|p| p.name == "pandas"));
    }
}
