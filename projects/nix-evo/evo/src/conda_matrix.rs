//! Environment Comparison Matrix
//!
//! Compare N environments side by side.
//! Show: version differences, missing packages, extra packages.
//! Generate a "compatibility score" between environments.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Request for environment comparison
#[derive(Debug, Deserialize)]
pub struct CompareRequest {
    pub envs: Vec<String>,
    pub host: Option<String>,
}

/// Single environment summary
#[derive(Debug, Clone, Serialize)]
pub struct EnvSummary {
    pub name: String,
    pub package_count: usize,
    pub python_version: Option<String>,
    pub packages: BTreeMap<String, String>, // name → version
}

/// Version difference entry
#[derive(Debug, Clone, Serialize)]
pub struct VersionDiff {
    pub package: String,
    pub versions: BTreeMap<String, String>, // env_name → version
    pub has_difference: bool,
}

/// Pairwise compatibility result
#[derive(Debug, Clone, Serialize)]
pub struct PairwiseCompatibility {
    pub env_a: String,
    pub env_b: String,
    pub compatibility_score: f64, // 0.0 - 100.0
    pub shared_packages: usize,
    pub only_in_a: usize,
    pub only_in_b: usize,
    pub version_mismatches: usize,
    pub only_in_a_packages: Vec<String>,
    pub only_in_b_packages: Vec<String>,
    pub mismatched_packages: Vec<VersionMismatch>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionMismatch {
    pub name: String,
    pub version_a: String,
    pub version_b: String,
}

/// Full comparison report
#[derive(Debug, Clone, Serialize)]
pub struct ComparisonReport {
    pub generated_at: String,
    pub environments: Vec<EnvSummary>,
    pub universal_packages: Vec<String>,     // in ALL envs
    pub package_matrix: Vec<VersionDiff>,    // package × env version grid
    pub pairwise: Vec<PairwiseCompatibility>,
    pub overall_compatibility: f64,
    pub unique_packages_per_env: BTreeMap<String, Vec<String>>,
}

/// Compute compatibility score between two envs
fn compatibility_score(
    pkgs_a: &BTreeMap<String, String>,
    pkgs_b: &BTreeMap<String, String>,
) -> (f64, usize, usize, usize, usize) {
    let names_a: BTreeSet<&str> = pkgs_a.keys().map(|s| s.as_str()).collect();
    let names_b: BTreeSet<&str> = pkgs_b.keys().map(|s| s.as_str()).collect();

    let shared = names_a.intersection(&names_b).count();
    let only_a = names_a.difference(&names_b).count();
    let only_b = names_b.difference(&names_a).count();

    let mut mismatches = 0;
    for name in &names_a.intersection(&names_b) {
        let va = pkgs_a.get(*name).map(|s| s.as_str()).unwrap_or("");
        let vb = pkgs_b.get(*name).map(|s| s.as_str()).unwrap_or("");
        if va != vb {
            mismatches += 1;
        }
    }

    let total_unique = names_a.union(&names_b).count();
    let score = if total_unique == 0 {
        100.0
    } else {
        let matching = shared - mismatches;
        (matching as f64 / total_unique as f64) * 100.0
    };

    (score, shared, only_a, only_b, mismatches)
}

/// GET/POST /api/conda/compare
pub async fn compare_handler(
    state: AppStateRef,
    Json(body): Json<CompareRequest>,
) -> Result<Json<ComparisonReport>, AppError> {
    let backend = crate::conda::detect_backend().await?;
    let now = chrono::Utc::now().to_rfc3339();

    if body.envs.len() < 2 {
        return Err(AppError::Validation { field: "envs".to_string(), message: "At least 2 environments required for comparison".to_string() });
    }

    // Collect all environment data
    let mut summaries = Vec::new();
    let mut all_packages: HashMap<String, BTreeMap<String, String>> = HashMap::new();

    for env_name in &body.envs {
        let packages = match conda::list_packages(&backend, env_name).await {
            Ok(pkgs) => pkgs,
            Err(e) => {
                return Err(AppError::Validation { field: env_name.clone(), message: format!("Could not list packages: {}", e) });
            }
        };

        let pkg_map: BTreeMap<String, String> = packages
            .iter()
            .map(|p| (p.name.clone(), p.version.clone()))
            .collect();

        let python_version = packages
            .iter()
            .find(|p| p.name == "python")
            .map(|p| p.version.clone());

        summaries.push(EnvSummary {
            name: env_name.clone(),
            package_count: packages.len(),
            python_version,
            packages: pkg_map.clone(),
        });

        all_packages.insert(env_name.clone(), pkg_map);
    }

    // Build package matrix: for each unique package, show versions across envs
    let mut all_pkg_names: BTreeSet<String> = BTreeSet::new();
    for pkgs in all_packages.values() {
        all_pkg_names.extend(pkgs.keys().cloned());
    }

    let mut package_matrix = Vec::new();
    for pkg_name in &all_pkg_names {
        let mut versions: BTreeMap<String, String> = BTreeMap::new();
        let mut has_diff = false;
        let mut prev_version: Option<&str> = None;

        for env_name in &body.envs {
            let version = all_packages
                .get(env_name)
                .and_then(|pkgs| pkgs.get(pkg_name))
                .map(|s| s.as_str())
                .unwrap_or("—");

            if let Some(prev) = prev_version {
                if prev != version && version != "—" && prev != "—" {
                    has_diff = true;
                }
            }
            if version != "—" {
                prev_version = Some(version);
            }

            versions.insert(env_name.clone(), version.to_string());
        }

        package_matrix.push(VersionDiff {
            package: pkg_name.clone(),
            versions,
            has_difference: has_diff,
        });
    }

    // Universal packages (in all envs)
    let universal: Vec<String> = package_matrix
        .iter()
        .filter(|v| v.versions.values().all(|v| v != "—"))
        .map(|v| v.package.clone())
        .collect();

    // Pairwise comparisons
    let mut pairwise = Vec::new();
    for i in 0..body.envs.len() {
        for j in (i + 1)..body.envs.len() {
            let a = &body.envs[i];
            let b = &body.envs[j];
            let pkgs_a = all_packages.get(a).unwrap();
            let pkgs_b = all_packages.get(b).unwrap();

            let (score, shared, only_a, only_b, mismatches) = compatibility_score(pkgs_a, pkgs_b);

            let only_a_pkgs: Vec<String> = pkgs_a
                .keys()
                .filter(|k| !pkgs_b.contains_key(*k))
                .cloned()
                .collect();
            let only_b_pkgs: Vec<String> = pkgs_b
                .keys()
                .filter(|k| !pkgs_a.contains_key(*k))
                .cloned()
                .collect();
            let mismatched: Vec<VersionMismatch> = pkgs_a
                .keys()
                .filter(|k| pkgs_b.contains_key(*k) && pkgs_a[*k] != pkgs_b[*k])
                .map(|k| VersionMismatch {
                    name: k.clone(),
                    version_a: pkgs_a[k].clone(),
                    version_b: pkgs_b[k].clone(),
                })
                .collect();

            pairwise.push(PairwiseCompatibility {
                env_a: a.clone(),
                env_b: b.clone(),
                compatibility_score: (score * 100.0).round() / 100.0,
                shared_packages: shared,
                only_in_a: only_a,
                only_in_b: only_b,
                version_mismatches: mismatches,
                only_in_a_packages: only_a_pkgs,
                only_in_b_packages: only_b_pkgs,
                mismatched_packages: mismatched,
            });
        }
    }

    // Unique packages per env
    let mut unique_per_env: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for env_name in &body.envs {
        let names: BTreeSet<&str> = all_packages
            .get(env_name)
            .map(|pkgs| pkgs.keys().map(|s| s.as_str()).collect())
            .unwrap_or_default();

        let others: BTreeSet<&str> = all_packages
            .iter()
            .filter(|(k, _)| *k != env_name)
            .flat_map(|(_, pkgs)| pkgs.keys().map(|s| s.as_str()))
            .collect();

        let unique: Vec<String> = names.difference(&others).map(|s| s.to_string()).collect();
        unique_per_env.insert(env_name.clone(), unique);
    }

    // Overall compatibility
    let overall = if pairwise.is_empty() {
        100.0
    } else {
        let sum: f64 = pairwise.iter().map(|p| p.compatibility_score).sum();
        sum / pairwise.len() as f64
    };

    Ok(Json(ComparisonReport {
        generated_at: now,
        environments: summaries,
        universal_packages: universal,
        package_matrix,
        pairwise,
        overall_compatibility: (overall * 100.0).round() / 100.0,
        unique_packages_per_env: unique_per_env,
    }))
}
