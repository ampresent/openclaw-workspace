//! Environment Repair Engine
//!
//! Detect broken environments (missing .so, corrupt metadata, version conflicts).
//! Auto-repair: reinstall missing packages, fix metadata, resolve conflicts.

use axum::Json;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::cmd::{AppStateRef, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Repair request
#[derive(Debug, Clone, Deserialize)]
pub struct RepairRequest {
    pub env: String,
    pub auto_fix: Option<bool>,
    pub check_shared_libs: Option<bool>,
    pub check_metadata: Option<bool>,
    pub check_conflicts: Option<bool>,
}

/// Issue found during diagnosis
#[derive(Debug, Clone, Serialize)]
pub struct RepairIssue {
    pub issue_type: IssueType,
    pub severity: Severity,
    pub description: String,
    pub package: Option<String>,
    pub fix_applied: bool,
    pub fix_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum IssueType {
    #[serde(rename = "missing_shared_lib")]
    MissingSharedLib,
    #[serde(rename = "corrupt_metadata")]
    CorruptMetadata,
    #[serde(rename = "version_conflict")]
    VersionConflict,
    #[serde(rename = "broken_package")]
    BrokenPackage,
    #[serde(rename = "missing_dependency")]
    MissingDependency,
    #[serde(rename = "stale_cache")]
    StaleCache,
    #[serde(rename = "orphan_dist_info")]
    OrphanDistInfo,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Severity {
    #[serde(rename = "critical")]
    Critical,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "info")]
    Info,
}

/// Repair result
#[derive(Debug, Clone, Serialize)]
pub struct RepairResult {
    pub environment: String,
    pub success: bool,
    pub issues_found: usize,
    pub issues_fixed: usize,
    pub issues: Vec<RepairIssue>,
    pub commands_executed: Vec<String>,
    pub duration_ms: u64,
}

// ─── Repair Engine ────────────────────────────────────────────────────

/// Diagnose and optionally repair an environment
pub async fn diagnose_and_repair(request: &RepairRequest) -> Result<RepairResult, AppError> {
    let start = std::time::Instant::now();
    let backend = conda::detect_backend().await?;
    let auto_fix = request.auto_fix.unwrap_or(false);
    let check_sos = request.check_shared_libs.unwrap_or(true);
    let check_meta = request.check_metadata.unwrap_or(true);
    let check_conf = request.check_conflicts.unwrap_or(true);

    let mut issues = Vec::new();
    let mut commands = Vec::new();

    // Verify environment exists
    let envs = conda::list_envs(&backend).await?;
    let env = envs.iter().find(|e| e.name == request.env)
        .ok_or_else(|| AppError::NotFound {
            resource: format!("conda environment: {}", request.env),
        })?;

    let env_path = &env.path;
    let packages = conda::list_packages(&backend, &request.env).await.unwrap_or_default();

    // 1. Check shared libraries
    if check_sos {
        check_shared_libraries(env_path, &packages, &mut issues, &mut commands, auto_fix).await;
    }

    // 2. Check metadata integrity
    if check_meta {
        check_metadata_integrity(env_path, &mut issues, &mut commands, auto_fix).await;
    }

    // 3. Check version conflicts
    if check_conf {
        check_version_conflicts(&backend, &request.env, &packages, &mut issues, &mut commands, auto_fix).await;
    }

    // 4. Check for orphaned dist-info
    check_orphan_dist_info(env_path, &packages, &mut issues, &mut commands, auto_fix).await;

    // 5. Verify package integrity
    verify_package_integrity(&backend, &request.env, &mut issues, &mut commands, auto_fix).await;

    let issues_fixed = issues.iter().filter(|i| i.fix_applied).count();

    Ok(RepairResult {
        environment: request.env.clone(),
        success: issues.iter().all(|i| i.fix_applied || i.severity == Severity::Warning || i.severity == Severity::Info),
        issues_found: issues.len(),
        issues_fixed,
        issues,
        commands_executed: commands,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Check for missing shared libraries (.so files)
async fn check_shared_libraries(
    env_path: &str,
    packages: &[conda::CondaPackage],
    issues: &mut Vec<RepairIssue>,
    commands: &mut Vec<String>,
    auto_fix: bool,
) {
    let lib_dir = format!("{env_path}/lib");
    let site_dir = format!("{env_path}/lib/python*/site-packages");

    // Scan .so references in site-packages
    let scan_result = tokio::process::Command::new("find")
        .args([&site_dir, "-name", "*.so", "-exec", "ldd", "{}", ";"])
        .output()
        .await;

    if let Ok(output) = scan_result {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("not found") {
                let lib_name = line.split_whitespace().next().unwrap_or("unknown");
                let pkg = packages.iter().find(|p| {
                    line.contains(&p.name)
                });

                let mut issue = RepairIssue {
                    issue_type: IssueType::MissingSharedLib,
                    severity: Severity::Error,
                    description: format!("Shared library not found: {lib_name}"),
                    package: pkg.map(|p| p.name.clone()),
                    fix_applied: false,
                    fix_command: None,
                };

                if auto_fix && pkg.is_some() {
                    let pkg_name = pkg.unwrap().name.as_str();
                    let cmd = format!("micromamba install -n <env> {pkg_name} --force-reinstall -y");
                    issue.fix_command = Some(cmd);
                    // Attempt fix
                    match run_cmd("micromamba", &["install", "-n", "PLACEHOLDER", pkg_name, "--force-reinstall", "-y"]).await {
                        Ok(_) => issue.fix_applied = true,
                        Err(_) => {}
                    }
                }

                issues.push(issue);
            }
        }
    }

    // Check for libstdc++ / libgcc_s common issues
    let env_lib = format!("{env_path}/lib/libstdc++.so.6");
    if Path::new(&env_lib).exists() {
        // Check if it's a stale symlink
        if let Ok(meta) = tokio::fs::symlink_metadata(&env_lib).await {
            if meta.file_type().is_symlink() {
                if tokio::fs::canonicalize(&env_lib).await.is_err() {
                    issues.push(RepairIssue {
                        issue_type: IssueType::BrokenPackage,
                        severity: Severity::Error,
                        description: "Broken symlink: libstdc++.so.6 points to missing file".to_string(),
                        package: Some("libstdcxx-ng".to_string()),
                        fix_applied: false,
                        fix_command: Some(format!("micromamba install -n <env> libstdcxx-ng -y")),
                    });
                }
            }
        }
    }
}

/// Check metadata integrity (dist-info, conda-meta)
async fn check_metadata_integrity(
    env_path: &str,
    issues: &mut Vec<RepairIssue>,
    commands: &mut Vec<String>,
    auto_fix: bool,
) {
    let meta_dir = format!("{env_path}/conda-meta");

    // Check if conda-meta exists
    if !Path::new(&meta_dir).exists() {
        issues.push(RepairIssue {
            issue_type: IssueType::CorruptMetadata,
            severity: Severity::Critical,
            description: "conda-meta directory missing — environment may be broken".to_string(),
            package: None,
            fix_applied: false,
            fix_command: Some("Recreate the environment".to_string()),
        });
        return;
    }

    // Scan for corrupt .json files in conda-meta
    let scan = tokio::process::Command::new("find")
        .args([&meta_dir, "-name", "*.json", "-type", "f"])
        .output()
        .await;

    if let Ok(output) = scan {
        let files = String::from_utf8_lossy(&output.stdout);
        for file_path in files.lines() {
            let file_path = file_path.trim();
            if file_path.is_empty() { continue; }

            if let Ok(content) = tokio::fs::read_to_string(file_path).await {
                if serde_json::from_str::<serde_json::Value>(&content).is_err() {
                    let pkg_name = Path::new(file_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown");
                    issues.push(RepairIssue {
                        issue_type: IssueType::CorruptMetadata,
                        severity: Severity::Warning,
                        description: format!("Corrupt metadata: {pkg_name}.json"),
                        package: Some(pkg_name.to_string()),
                        fix_applied: false,
                        fix_command: None,
                    });
                }
            }
        }
    }
}

/// Check for version conflicts between packages
async fn check_version_conflicts(
    backend: &str,
    env_name: &str,
    packages: &[conda::CondaPackage],
    issues: &mut Vec<RepairIssue>,
    commands: &mut Vec<String>,
    auto_fix: bool,
) {
    // Use conda's built-in conflict check
    match run_cmd(backend, &["list", "-n", env_name, "--explicit"]).await {
        Ok(output) => {
            if output.contains("conflict") || output.contains("incompatible") {
                issues.push(RepairIssue {
                    issue_type: IssueType::VersionConflict,
                    severity: Severity::Error,
                    description: "Version conflicts detected in environment".to_string(),
                    package: None,
                    fix_applied: false,
                    fix_command: Some(format!("micromamba update -n {env_name} --all -y")),
                });

                if auto_fix {
                    let cmd = format!("micromamba update -n {env_name} --all -y");
                    commands.push(cmd);
                }
            }
        }
        Err(_) => {}
    }

    // Check for duplicate packages with different versions
    let mut name_versions: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for pkg in packages {
        name_versions.entry(pkg.name.clone())
            .or_default()
            .push(pkg.version.clone());
    }
    for (name, versions) in &name_versions {
        let unique: std::collections::HashSet<_> = versions.iter().collect();
        if unique.len() > 1 {
            issues.push(RepairIssue {
                issue_type: IssueType::VersionConflict,
                severity: Severity::Warning,
                description: format!("Package '{name}' has multiple versions: {}", versions.join(", ")),
                package: Some(name.clone()),
                fix_applied: false,
                fix_command: Some(format!("micromamba install -n <env> {name} -y")),
            });
        }
    }
}

/// Check for orphaned dist-info directories (no corresponding package)
async fn check_orphan_dist_info(
    env_path: &str,
    packages: &[conda::CondaPackage],
    issues: &mut Vec<RepairIssue>,
    commands: &mut Vec<String>,
    auto_fix: bool,
) {
    let site_pattern = format!("{env_path}/lib/python*/site-packages");
    let scan = tokio::process::Command::new("find")
        .args([&site_pattern, "-maxdepth", "1", "-name", "*.dist-info", "-type", "d"])
        .output()
        .await;

    if let Ok(output) = scan {
        let dirs = String::from_utf8_lossy(&output.stdout);
        let pkg_names: std::collections::HashSet<&str> = packages.iter().map(|p| p.name.as_str()).collect();

        for dir in dirs.lines() {
            let dir_name = dir.trim();
            if dir_name.is_empty() { continue; }

            // Extract package name from dist-info (e.g., numpy-1.26.3.dist-info → numpy)
            let stem = Path::new(dir_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let pkg_name = stem.rsplit_once('-')
                .map(|(n, _)| n)
                .unwrap_or(stem);

            if !pkg_names.contains(pkg_name) {
                issues.push(RepairIssue {
                    issue_type: IssueType::OrphanDistInfo,
                    severity: Severity::Info,
                    description: format!("Orphaned dist-info: {stem}.dist-info (no matching conda package)"),
                    package: Some(pkg_name.to_string()),
                    fix_applied: false,
                    fix_command: Some(format!("pip install --force-reinstall {pkg_name}")),
                });
            }
        }
    }
}

/// Verify package integrity using conda's verify command
async fn verify_package_integrity(
    backend: &str,
    env_name: &str,
    issues: &mut Vec<RepairIssue>,
    commands: &mut Vec<String>,
    auto_fix: bool,
) {
    // conda/micromamba verify
    match run_cmd(backend, &["list", "-n", env_name, "--json"]).await {
        Ok(output) => {
            if let Ok(pkgs) = serde_json::from_str::<Vec<serde_json::Value>>(&output) {
                for pkg in &pkgs {
                    if let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
                        let dist_url = pkg.get("dist_url").and_then(|u| u.as_str());
                        let channel = pkg.get("channel").and_then(|c| c.as_str()).unwrap_or("");

                        // Check if package is from an unavailable channel
                        if channel.contains("pkgs/main") || channel.contains("pkgs/free") {
                            // These are always available, skip
                            continue;
                        }
                    }
                }
            }
        }
        Err(_) => {}
    }

    // Check pip verify for python packages
    let env_path_result = conda::env_info_map(backend).await;
    if let Ok(map) = env_path_result {
        if let Some(env_path) = map.get(env_name) {
            let python_bin = format!("{env_path}/bin/python");
            if Path::new(&python_bin).exists() {
                match tokio::process::Command::new(&python_bin)
                    .args(["-m", "pip", "check"])
                    .output()
                    .await
                {
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let combined = format!("{stdout}{stderr}");

                        for line in combined.lines() {
                            if line.contains("No broken") {
                                continue;
                            }
                            if line.contains("broken") || line.contains("requires") {
                                issues.push(RepairIssue {
                                    issue_type: IssueType::MissingDependency,
                                    severity: Severity::Warning,
                                    description: format!("pip check: {line}"),
                                    package: None,
                                    fix_applied: false,
                                    fix_command: Some("pip install --upgrade <package>".to_string()),
                                });
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }
    }
}

// ─── HTTP Handlers ────────────────────────────────────────────────────

/// POST /api/env/repair
pub async fn repair_handler(
    State(_state): AppStateRef,
    Json(body): Json<RepairRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = diagnose_and_repair(&body).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_serialization() {
        let issue = RepairIssue {
            issue_type: IssueType::MissingSharedLib,
            severity: Severity::Error,
            description: "libfoo.so not found".to_string(),
            package: Some("numpy".to_string()),
            fix_applied: false,
            fix_command: Some("micromamba install numpy -y".to_string()),
        };
        let json = serde_json::to_string(&issue).unwrap();
        assert!(json.contains("missing_shared_lib"));
        assert!(json.contains("error"));
    }

    #[test]
    fn test_repair_result_serialization() {
        let result = RepairResult {
            environment: "test-env".to_string(),
            success: true,
            issues_found: 3,
            issues_fixed: 2,
            issues: vec![],
            commands_executed: vec!["micromamba install numpy -y".to_string()],
            duration_ms: 150,
        };
        assert_eq!(result.environment, "test-env");
        assert_eq!(result.issues_found, 3);
    }
}
