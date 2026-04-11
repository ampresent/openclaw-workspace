//! Conda Environment Simulation
//!
//! "What if I install package X?" — simulate the solve without actually doing it.
//! Dry-run for conda: predict what would change.
//! Dependency resolution simulation.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Request body for simulation
#[derive(Debug, Deserialize)]
pub struct SimulateRequest {
    pub env: String,
    pub host: Option<String>,
    pub packages: Vec<String>,
    pub action: Option<String>, // "install" (default), "remove", "update", "update-all"
    pub channels: Option<Vec<String>>,
    pub python_version: Option<String>,
}

/// Full simulation result
#[derive(Debug, Clone, Serialize)]
pub struct SimulationResult {
    pub environment: String,
    pub action: String,
    pub requested_packages: Vec<String>,
    pub success: bool,
    pub dry_run_output: Option<String>,
    pub predicted_changes: PredictedChanges,
    pub dependency_tree: DependencyTree,
    pub conflicts: Vec<Conflict>,
    pub risk_assessment: SimRiskAssessment,
    pub estimated_download_mb: Option<f64>,
    pub estimated_disk_change_mb: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PredictedChanges {
    pub packages_to_install: Vec<PackageChange>,
    pub packages_to_remove: Vec<PackageChange>,
    pub packages_to_update: Vec<VersionChange>,
    pub packages_to_downgrade: Vec<VersionChange>,
    pub new_packages: usize,
    pub removed_packages: usize,
    pub updated_packages: usize,
    pub total_affected: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageChange {
    pub name: String,
    pub version: String,
    pub build: Option<String>,
    pub channel: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionChange {
    pub name: String,
    pub from_version: String,
    pub to_version: String,
    pub from_build: Option<String>,
    pub to_build: Option<String>,
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyTree {
    pub direct_dependencies: Vec<DepNode>,
    pub transitive_count: usize,
    pub max_depth: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DepNode {
    pub name: String,
    pub version: String,
    pub children: Vec<DepNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Conflict {
    pub package_a: String,
    pub package_b: String,
    pub reason: String,
    pub severity: String, // "warning", "error"
}

#[derive(Debug, Clone, Serialize)]
pub struct SimRiskAssessment {
    pub risk_level: String, // "low", "medium", "high", "critical"
    pub risk_score: f64,    // 0-100
    pub warnings: Vec<String>,
    pub breaking_changes: Vec<String>,
}

/// Packages known to cause conflicts
const CONFLICT_PRONE_PACKAGES: &[&str] = &[
    "tensorflow", "pytorch", "torch", "jax",
    "opencv", "opencv-python", "opencv-contrib-python",
    "pillow", "pillow-simd",
];

/// Parse conda/micromamba dry-run output to extract predicted changes
fn parse_dry_run_output(output: &str) -> PredictedChanges {
    let mut to_install = Vec::new();
    let mut to_remove = Vec::new();
    let mut to_update = Vec::new();
    let mut to_downgrade = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        if line.contains("will be installed") || line.starts_with("+  ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(pkg_str) = parts.get(if line.starts_with("+") { 1 } else { 0 }) {
                let (name, version, build) = parse_pkg_string(pkg_str);
                to_install.push(PackageChange {
                    name,
                    version,
                    build,
                    channel: parts.last().map(|s| s.to_string()),
                    size_bytes: None,
                });
            }
        } else if line.contains("will be removed") || line.starts_with("-  ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(pkg_str) = parts.get(if line.starts_with("-") { 1 } else { 0 }) {
                let (name, version, build) = parse_pkg_string(pkg_str);
                to_remove.push(PackageChange {
                    name,
                    version,
                    build,
                    channel: None,
                    size_bytes: None,
                });
            }
        } else if line.contains("will be updated") || line.contains("will be upgraded") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                to_update.push(VersionChange {
                    name: parts[0].to_string(),
                    from_version: parts.get(1).unwrap_or(&"").to_string(),
                    to_version: parts.get(3).unwrap_or(&"").to_string(),
                    from_build: None,
                    to_build: None,
                    channel: parts.last().map(|s| s.to_string()),
                });
            }
        } else if line.contains("will be downgraded") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                to_downgrade.push(VersionChange {
                    name: parts[0].to_string(),
                    from_version: parts.get(1).unwrap_or(&"").to_string(),
                    to_version: parts.get(3).unwrap_or(&"").to_string(),
                    from_build: None,
                    to_build: None,
                    channel: parts.last().map(|s| s.to_string()),
                });
            }
        }
    }

    let total_affected = to_install.len() + to_remove.len() + to_update.len() + to_downgrade.len();

    PredictedChanges {
        new_packages: to_install.len(),
        removed_packages: to_remove.len(),
        updated_packages: to_update.len(),
        total_affected,
        packages_to_install: to_install,
        packages_to_remove: to_remove,
        packages_to_update: to_update,
        packages_to_downgrade: to_downgrade,
    }
}

/// Parse "name-version-build" into components
fn parse_pkg_string(s: &str) -> (String, String, Option<String>) {
    let parts: Vec<&str> = s.rsplitn(3, '-').collect();
    match parts.len() {
        3 => (parts[2].to_string(), parts[1].to_string(), Some(parts[0].to_string())),
        2 => (parts[1].to_string(), parts[0].to_string(), None),
        _ => (s.to_string(), String::new(), None),
    }
}

/// Estimate download size from dry-run output
fn estimate_download_size(output: &str) -> Option<f64> {
    for line in output.lines() {
        let lower = line.to_lowercase();
        if lower.contains("total download") || lower.contains("download and extract") {
            for word in line.split_whitespace() {
                if let Ok(mb) = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '.').parse::<f64>() {
                    if mb > 0.0 {
                        return Some(mb);
                    }
                }
            }
        }
    }
    None
}

/// Assess risk of the proposed changes
fn assess_risk(changes: &PredictedChanges, requested: &[String]) -> SimRiskAssessment {
    let mut warnings = Vec::new();
    let mut breaking_changes = Vec::new();
    let mut score: f64 = 0.0;

    for pkg in requested {
        if CONFLICT_PRONE_PACKAGES.iter().any(|cp| pkg.to_lowercase().contains(cp)) {
            warnings.push(format!("Package '{}' is known to cause dependency conflicts", pkg));
            score += 15.0;
        }
    }

    if changes.total_affected > 50 {
        warnings.push(format!(
            "This operation affects {} packages — high blast radius",
            changes.total_affected
        ));
        score += 20.0;
        breaking_changes.push("Large-scale environment modification".to_string());
    } else if changes.total_affected > 20 {
        warnings.push(format!(
            "This operation affects {} packages",
            changes.total_affected
        ));
        score += 10.0;
    }

    if !changes.packages_to_downgrade.is_empty() {
        warnings.push(format!(
            "{} package(s) will be downgraded — this may break dependents",
            changes.packages_to_downgrade.len()
        ));
        score += changes.packages_to_downgrade.len() as f64 * 5.0;
        for dc in &changes.packages_to_downgrade {
            breaking_changes.push(format!(
                "{}: {} → {} (downgrade)",
                dc.name, dc.from_version, dc.to_version
            ));
        }
    }

    if changes.removed_packages > 5 {
        warnings.push(format!(
            "{} packages will be removed — verify nothing depends on them",
            changes.removed_packages
        ));
        score += 10.0;
    }

    let channels: BTreeSet<&str> = changes.packages_to_install
        .iter()
        .filter_map(|p| p.channel.as_deref())
        .collect();
    if channels.len() > 2 {
        warnings.push("Packages from multiple channels may cause conflicts".to_string());
        score += 10.0;
    }

    let risk_level = if score >= 50.0 {
        "critical"
    } else if score >= 30.0 {
        "high"
    } else if score >= 15.0 {
        "medium"
    } else {
        "low"
    };

    SimRiskAssessment {
        risk_level: risk_level.to_string(),
        risk_score: score.min(100.0),
        warnings,
        breaking_changes,
    }
}

/// Build a simple dependency tree from current packages and changes
fn build_dependency_tree(
    current: &[conda::CondaPackage],
    changes: &PredictedChanges,
) -> DependencyTree {
    let mut dep_map: HashMap<String, Vec<String>> = HashMap::new();

    for pkg in current {
        let libs = ["numpy", "scipy", "pandas", "requests", "urllib3", "certifi", "six"];
        for lib in libs {
            if pkg.name != lib && current.iter().any(|p| p.name == lib) {
                dep_map.entry(lib.to_string()).or_default().push(pkg.name.clone());
            }
        }
    }

    let direct = changes.packages_to_install.iter().map(|p| DepNode {
        name: p.name.clone(),
        version: p.version.clone(),
        children: vec![],
    }).collect();

    let transitive = dep_map.values().map(|v| v.len()).sum();

    DependencyTree {
        direct_dependencies: direct,
        transitive_count: transitive,
        max_depth: 3,
    }
}

/// Detect potential conflicts between requested packages
fn detect_conflicts(requested: &[String], current: &[conda::CondaPackage]) -> Vec<Conflict> {
    let mut conflicts = Vec::new();
    let current_names: BTreeSet<&str> = current.iter().map(|p| p.name.as_str()).collect();

    let exclusive_pairs: &[(&str, &str, &str)] = &[
        ("tensorflow", "pytorch", "ML frameworks"),
        ("tensorflow", "torch", "ML frameworks"),
        ("pillow", "pillow-simd", "Image processing"),
        ("opencv-python", "opencv-contrib-python", "OpenCV variants"),
    ];

    for (a, b, reason) in exclusive_pairs {
        let has_a = requested.iter().any(|p| p.contains(a)) || current_names.contains(a);
        let has_b = requested.iter().any(|p| p.contains(b)) || current_names.contains(b);
        if has_a && has_b {
            let installing_a = requested.iter().any(|p| p.contains(a));
            let installing_b = requested.iter().any(|p| p.contains(b));
            if installing_a && installing_b {
                conflicts.push(Conflict {
                    package_a: a.to_string(),
                    package_b: b.to_string(),
                    reason: format!("Both {} and {} detected — {}", a, b, reason),
                    severity: "warning".to_string(),
                });
            }
        }
    }

    conflicts
}

/// Run simulation using dry-run
pub async fn simulate(
    backend: &str,
    request: &SimulateRequest,
) -> Result<SimulationResult, AppError> {
    let action = request.action.as_deref().unwrap_or("install");
    let packages = &request.packages;

    let current_packages = conda::list_packages(backend, &request.env).await.unwrap_or_default();

    let mut args: Vec<&str> = vec!["install", "-n", &request.env, "--dry-run", "--yes"];

    let channel_strs: Vec<String> = request.channels.clone().unwrap_or_default();
    for ch in &channel_strs {
        args.extend_from_slice(&["-c", ch]);
    }

    match action {
        "install" => {
            for pkg in packages {
                args.push(pkg.as_str());
            }
        }
        "remove" => {
            args = vec!["remove", "-n", &request.env, "--dry-run", "--yes"];
            for pkg in packages {
                args.push(pkg.as_str());
            }
        }
        "update" => {
            args = vec!["update", "-n", &request.env, "--dry-run", "--yes"];
            for pkg in packages {
                args.push(pkg.as_str());
            }
        }
        "update-all" => {
            args = vec!["update", "-n", &request.env, "--dry-run", "--yes", "--all"];
        }
        _ => {
            return Err(AppError::Validation { field: "action".to_string(), message: format!("Unknown action: {}", action) });
        }
    }

    let dry_run_output = match run_cmd(backend, &args).await {
        Ok(output) => output,
        Err(e) => {
            let err_msg = e.to_string();
            let changes = PredictedChanges {
                packages_to_install: vec![],
                packages_to_remove: vec![],
                packages_to_update: vec![],
                packages_to_downgrade: vec![],
                new_packages: 0,
                removed_packages: 0,
                updated_packages: 0,
                total_affected: 0,
            };
            return Ok(SimulationResult {
                environment: request.env.clone(),
                action: action.to_string(),
                requested_packages: packages.clone(),
                success: false,
                dry_run_output: Some(err_msg),
                predicted_changes: changes.clone(),
                dependency_tree: DependencyTree {
                    direct_dependencies: vec![],
                    transitive_count: 0,
                    max_depth: 0,
                },
                conflicts: vec![],
                risk_assessment: assess_risk(&changes, packages),
                estimated_download_mb: None,
                estimated_disk_change_mb: None,
            });
        }
    };

    let changes = parse_dry_run_output(&dry_run_output);
    let conflicts = detect_conflicts(packages, &current_packages);
    let dep_tree = build_dependency_tree(&current_packages, &changes);
    let download_mb = estimate_download_size(&dry_run_output);

    Ok(SimulationResult {
        environment: request.env.clone(),
        action: action.to_string(),
        requested_packages: packages.clone(),
        success: true,
        dry_run_output: Some(dry_run_output),
        predicted_changes: changes.clone(),
        dependency_tree: dep_tree,
        conflicts,
        risk_assessment: assess_risk(&changes, packages),
        estimated_download_mb: download_mb,
        estimated_disk_change_mb: download_mb,
    })
}

/// POST /api/conda/simulate
pub async fn simulate_handler(
    state: AppStateRef,
    Json(body): Json<SimulateRequest>,
) -> Result<Json<SimulationResult>, AppError> {
    let backend = crate::conda::detect_backend().await?;
    let result = simulate(&backend, &body).await?;
    Ok(Json(result))
}
