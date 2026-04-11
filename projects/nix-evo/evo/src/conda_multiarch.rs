//! Multi-Architecture Conda Support
//!
//! Track which packages are available for linux-64 vs linux-aarch64.
//! Cross-compile planning: "can I move this env from x86 to ARM?"

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Query for multi-arch analysis
#[derive(Debug, Deserialize)]
pub struct MultiArchQuery {
    pub env: String,
    pub host: Option<String>,
    pub target_arch: Option<String>, // "linux-aarch64", "linux-64", "osx-arm64"
}

/// Multi-architecture analysis result
#[derive(Debug, Clone, Serialize)]
pub struct MultiArchReport {
    pub environment: String,
    pub current_arch: String,
    pub target_arch: String,
    pub total_packages: usize,
    pub available: Vec<ArchPackage>,
    pub unavailable: Vec<ArchPackage>,
    pub unknown: Vec<String>,
    pub migration_feasible: bool,
    pub migration_score: f64, // 0-100
    pub blockers: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchPackage {
    pub name: String,
    pub version: String,
    pub current_channel: String,
    pub available_on_target: bool,
    pub target_channel: Option<String>,
}

/// Well-known arch-only packages (packages only available on specific architectures)
const X86_ONLY: &[&str] = &[
    "mkl", "mkl-service", "intel-openmp", "cudatoolkit-11",
];

const AARCH64_ONLY: &[&str] = &[
    // Most conda-forge packages now support aarch64
];

/// Check migration feasibility from current arch to target
pub async fn check_multiarch(backend: &str, query: &MultiArchQuery) -> Result<MultiArchReport, AppError> {
    let packages = conda::list_packages(backend, &query.env).await?;

    // Detect current arch from system
    let current_arch = detect_current_arch().await;
    let target_arch = query.target_arch.as_deref()
        .unwrap_or(if current_arch == "linux-64" { "linux-aarch64" } else { "linux-64" });

    let mut available = Vec::new();
    let mut unavailable = Vec::new();
    let mut unknown = Vec::new();
    let mut blockers = Vec::new();

    for pkg in &packages {
        // Check known architecture restrictions
        let is_x86_only = X86_ONLY.iter().any(|&p| pkg.name.starts_with(p));
        let is_aarch64_only = AARCH64_ONLY.iter().any(|&p| pkg.name.starts_with(p));

        if target_arch == "linux-aarch64" && is_x86_only {
            unavailable.push(ArchPackage {
                name: pkg.name.clone(),
                version: pkg.version.clone(),
                current_channel: pkg.channel.clone(),
                available_on_target: false,
                target_channel: None,
            });
            blockers.push(format!("{} is x86-only", pkg.name));
        } else if target_arch == "linux-64" && is_aarch64_only {
            unavailable.push(ArchPackage {
                name: pkg.name.clone(),
                version: pkg.version.clone(),
                current_channel: pkg.channel.clone(),
                available_on_target: false,
                target_channel: None,
            });
            blockers.push(format!("{} is aarch64-only", pkg.name));
        } else {
            // Query conda for availability
            let available_on_target = check_package_availability(
                backend, &pkg.name, &pkg.version, target_arch
            ).await;

            if available_on_target {
                available.push(ArchPackage {
                    name: pkg.name.clone(),
                    version: pkg.version.clone(),
                    current_channel: pkg.channel.clone(),
                    available_on_target: true,
                    target_channel: Some(pkg.channel.clone()),
                });
            } else {
                // Might just need a different version
                unknown.push(pkg.name.clone());
            }
        }
    }

    let total = packages.len();
    let avail_count = available.len();
    let migration_score = if total > 0 {
        (avail_count as f64 / total as f64) * 100.0
    } else {
        100.0
    };
    let migration_feasible = unavailable.is_empty() && migration_score > 90.0;

    let mut suggestions = Vec::new();
    if !unavailable.is_empty() {
        suggestions.push(format!(
            "Find replacements for {} x86-only packages", unavailable.len()
        ));
    }
    if !unknown.is_empty() {
        suggestions.push(format!(
            "{} packages need version pin review for {} target", unknown.len(), target_arch
        ));
    }
    if migration_feasible {
        suggestions.push(format!(
            "Environment can be migrated to {} with `conda env export | conda env create -n <new> -f -` on target",
            target_arch
        ));
    }

    Ok(MultiArchReport {
        environment: query.env.clone(),
        current_arch,
        target_arch: target_arch.to_string(),
        total_packages: total,
        available,
        unavailable,
        unknown,
        migration_feasible,
        migration_score: (migration_score * 100.0).round() / 100.0,
        blockers,
        suggestions,
    })
}

/// Detect current system architecture
async fn detect_current_arch() -> String {
    if let Ok(output) = run_cmd("uname", &["-m"]).await {
        match output.trim() {
            "x86_64" => "linux-64".to_string(),
            "aarch64" | "arm64" => "linux-aarch64".to_string(),
            other => other.to_string(),
        }
    } else {
        "linux-64".to_string()
    }
}

/// Check if a specific package version is available on a target architecture
async fn check_package_availability(backend: &str, name: &str, version: &str, arch: &str) -> bool {
    // Use conda search with platform filter
    let result = run_cmd(backend, &[
        "search", name, "--platform", arch, "--json"
    ]).await;

    match result {
        Ok(output) => {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&output) {
                if let Some(pkgs) = data.get(name).and_then(|v| v.as_array()) {
                    return pkgs.iter().any(|p| {
                        p.get("version").and_then(|v| v.as_str()) == Some(version)
                    });
                }
            }
            // Fallback: if search succeeded but no JSON, assume available
            true
        }
        Err(_) => {
            // If search fails, check known compatibility
            // Most conda-forge packages support both architectures
            !X86_ONLY.iter().any(|&p| name.starts_with(p))
        }
    }
}

// ─── Axum Handler ─────────────────────────────────────────────────────

pub async fn multiarch_handler(
    State(_state): AppStateRef,
    Query(query): Query<MultiArchQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = conda::detect_backend().await?;
    let result = check_multiarch(&backend, &query).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}
