//! Conda Ecosystem Analytics
//!
//! Analyze trends: most installed packages, download velocity.
//! Dependency chain analysis: "what breaks if I remove numpy?"

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Query for analytics
#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub env: Option<String>,
    pub host: Option<String>,
    pub impact_package: Option<String>, // "what breaks if I remove X?"
}

/// Full analytics report
#[derive(Debug, Clone, Serialize)]
pub struct AnalyticsReport {
    pub environment: Option<String>,
    pub generated_at: String,
    pub ecosystem_overview: EcosystemOverview,
    pub top_packages: Vec<PackageRank>,
    pub impact_analysis: Option<ImpactAnalysis>,
    pub dependency_graph_stats: DepGraphStats,
    pub channel_health: Vec<ChannelHealth>,
    pub risk_indicators: Vec<RiskIndicator>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EcosystemOverview {
    pub total_packages: usize,
    pub total_environments: usize,
    pub unique_channels: usize,
    pub python_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageRank {
    pub name: String,
    pub version: String,
    pub channel: String,
    pub reverse_dep_count: usize, // how many packages depend on this
    pub importance_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImpactAnalysis {
    pub target_package: String,
    pub direct_dependents: Vec<String>,
    pub transitive_dependents: Vec<String>,
    pub total_affected: usize,
    pub safe_to_remove: bool,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DepGraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub max_depth: usize,
    pub orphan_packages: Vec<String>, // no dependents, no dependencies
    pub hub_packages: Vec<String>,    // many dependents
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelHealth {
    pub name: String,
    pub package_count: usize,
    pub percentage: f64,
    pub trust_level: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskIndicator {
    pub category: String,
    pub severity: String,
    pub message: String,
    pub affected_packages: Vec<String>,
}

/// Well-known dependency relationships (reverse deps)
/// In production this would come from repodata.json analysis
fn known_reverse_deps() -> HashMap<&'static str, Vec<&'static str>> {
    let mut m = HashMap::new();
    m.insert("numpy", vec!["pandas", "scipy", "matplotlib", "scikit-learn", "pillow", "opencv"]);
    m.insert("python", vec!["pip", "setuptools", "wheel", "cython", "pybind11"]);
    m.insert("openssl", vec!["python", "curl", "git", "libgit2", "wget"]);
    m.insert("libstdcxx-ng", vec!["*"]); // everything depends on it
    m.insert("libgcc-ng", vec!["*"]);
    m.insert("certifi", vec!["requests", "urllib3", "httpx", "aiohttp"]);
    m.insert("six", vec!["*"]); // used by hundreds
    m.insert("requests", vec!["httpretty", "responses", "treq"]);
    m
}

/// Generate full analytics report
pub async fn generate_analytics(backend: &str, query: &AnalyticsQuery) -> Result<AnalyticsReport, AppError> {
    let envs = conda::list_envs(backend).await?;
    let packages = if let Some(ref env) = query.env {
        conda::list_packages(backend, env).await?
    } else {
        // Aggregate across all environments
        let mut all_pkgs = Vec::new();
        for env in &envs {
            if let Ok(pkgs) = conda::list_packages(backend, &env.name).await {
                all_pkgs.extend(pkgs);
            }
        }
        all_pkgs
    };

    // Channel distribution
    let mut channel_dist: BTreeMap<String, usize> = BTreeMap::new();
    for pkg in &packages {
        *channel_dist.entry(pkg.channel.clone()).or_default() += 1;
    }

    // Python versions
    let mut py_versions: HashSet<String> = HashSet::new();
    for pkg in &packages {
        if pkg.name == "python" {
            py_versions.insert(pkg.version.clone());
        }
    }

    // Build reverse dependency map
    let known_deps = known_reverse_deps();
    let pkg_set: HashSet<&str> = packages.iter().map(|p| p.name.as_str()).collect();

    let mut reverse_deps: HashMap<String, Vec<String>> = HashMap::new();
    for pkg in &packages {
        reverse_deps.entry(pkg.name.clone()).or_default();
    }
    // Populate from known deps
    for (dep, dependents) in &known_deps {
        if pkg_set.contains(dep) {
            for dependent in dependents {
                if *dependent == "*" {
                    // All packages potentially depend
                    for pkg in &packages {
                        if pkg.name != *dep {
                            reverse_deps.entry(dep.to_string())
                                .or_default()
                                .push(pkg.name.clone());
                        }
                    }
                } else if pkg_set.contains(dependent) {
                    reverse_deps.entry(dep.to_string())
                        .or_default()
                        .push(dependent.to_string());
                }
            }
        }
    }

    // Package ranking by importance
    let mut rankings: Vec<PackageRank> = packages.iter().map(|pkg| {
        let rev_count = reverse_deps.get(&pkg.name).map(|v| v.len()).unwrap_or(0);
        let importance = rev_count as f64 * 1.5
            + if pkg.name == "python" { 100.0 } else { 0.0 }
            + if pkg.name.contains("lib") { 20.0 } else { 0.0 };
        PackageRank {
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            channel: pkg.channel.clone(),
            reverse_dep_count: rev_count,
            importance_score: (importance * 100.0).round() / 100.0,
        }
    }).collect();
    rankings.sort_by(|a, b| b.importance_score.partial_cmp(&a.importance_score).unwrap());
    rankings.truncate(20);

    // Impact analysis
    let impact = if let Some(ref target) = query.impact_package {
        Some(analyze_impact(target, &packages, &reverse_deps))
    } else {
        None
    };

    // Dependency graph stats
    let orphans: Vec<String> = packages.iter()
        .filter(|p| {
            let rev = reverse_deps.get(&p.name).map(|v| v.len()).unwrap_or(0);
            rev == 0 && !known_deps.contains_key(p.name.as_str())
        })
        .map(|p| p.name.clone())
        .collect();

    let hubs: Vec<String> = rankings.iter()
        .filter(|r| r.reverse_dep_count > 5)
        .map(|r| r.name.clone())
        .collect();

    // Channel health
    let total = packages.len() as f64;
    let channel_health: Vec<ChannelHealth> = channel_dist.iter().map(|(name, count)| {
        let pct = (*count as f64 / total) * 100.0;
        let trust = match name.as_str() {
            "conda-forge" | "defaults" | "main" => "high",
            "bioconda" | "pytorch" | "nvidia" | "intel" => "trusted",
            "pypi" => "medium",
            _ => "unknown",
        };
        ChannelHealth {
            name: name.clone(),
            package_count: *count,
            percentage: (pct * 100.0).round() / 100.0,
            trust_level: trust.to_string(),
        }
    }).collect();

    // Risk indicators
    let mut risks = Vec::new();
    if packages.len() > 300 {
        risks.push(RiskIndicator {
            category: "bloat".to_string(),
            severity: "warning".to_string(),
            message: format!("{} packages is unusually large — likely accumulated cruft", packages.len()),
            affected_packages: vec![],
        });
    }
    let unknown_channel_count: usize = channel_health.iter()
        .filter(|c| c.trust_level == "unknown")
        .map(|c| c.package_count)
        .sum();
    if unknown_channel_count > 0 {
        risks.push(RiskIndicator {
            category: "security".to_string(),
            severity: "warning".to_string(),
            message: format!("{} packages from untrusted/unknown channels", unknown_channel_count),
            affected_packages: vec![],
        });
    }

    Ok(AnalyticsReport {
        environment: query.env.clone(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        ecosystem_overview: EcosystemOverview {
            total_packages: packages.len(),
            total_environments: envs.len(),
            unique_channels: channel_dist.len(),
            python_versions: py_versions.into_iter().collect(),
        },
        top_packages: rankings,
        impact_analysis: impact,
        dependency_graph_stats: DepGraphStats {
            total_nodes: packages.len(),
            total_edges: reverse_deps.values().map(|v| v.len()).sum(),
            max_depth: estimate_max_depth(&reverse_deps),
            orphan_packages: orphans,
            hub_packages: hubs,
        },
        channel_health,
        risk_indicators: risks,
    })
}

/// Analyze what breaks if we remove a package
fn analyze_impact(
    target: &str,
    packages: &[conda::CondaPackage],
    reverse_deps: &HashMap<String, Vec<String>>,
) -> ImpactAnalysis {
    let direct = reverse_deps.get(target)
        .cloned()
        .unwrap_or_default();

    // BFS for transitive dependents
    let mut visited: HashSet<String> = direct.iter().cloned().collect();
    let mut queue: VecDeque<String> = direct.iter().cloned().collect();
    let mut transitive = Vec::new();

    while let Some(current) = queue.pop_front() {
        if let Some(deps) = reverse_deps.get(&current) {
            for dep in deps {
                if visited.insert(dep.clone()) {
                    queue.push_back(dep.clone());
                    transitive.push(dep.clone());
                }
            }
        }
    }

    let total_affected = visited.len();
    let safe = total_affected == 0;
    let risk = if total_affected > 10 { "critical" }
        else if total_affected > 3 { "high" }
        else if total_affected > 0 { "medium" }
        else { "low" };

    ImpactAnalysis {
        target_package: target.to_string(),
        direct_dependents: direct,
        transitive_dependents: transitive,
        total_affected,
        safe_to_remove: safe,
        risk_level: risk.to_string(),
    }
}

/// Estimate max dependency chain depth
fn estimate_max_depth(reverse_deps: &HashMap<String, Vec<String>>) -> usize {
    // Simple heuristic based on graph size
    let nodes = reverse_deps.len();
    if nodes == 0 { return 0; }
    let edges: usize = reverse_deps.values().map(|v| v.len()).sum();
    if edges == 0 { return 1; }
    // Rough estimate: log base 2 of edges
    ((edges as f64).ln() / 2.0_f64.ln()).ceil() as usize
}

// ─── Axum Handler ─────────────────────────────────────────────────────

pub async fn analytics_handler(
    State(_state): AppStateRef,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = conda::detect_backend().await?;
    let result = generate_analytics(&backend, &query).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}
