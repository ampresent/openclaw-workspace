//! Conda Runtime Optimizer
//!
//! Analyze environment for optimization opportunities.
//! Detect: unused packages, duplicate dependencies, oversized envs.
//! Suggest: mamba solver, conda-pack for deployment, pip-only for speed.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Query params for optimization analysis
#[derive(Debug, Deserialize)]
pub struct OptimizeQuery {
    pub env: String,
    pub host: Option<String>,
    pub check_disk: Option<bool>,
}

/// Full optimization report
#[derive(Debug, Clone, Serialize)]
pub struct OptimizeReport {
    pub environment: String,
    pub total_packages: usize,
    pub env_size_mb: Option<f64>,
    pub health_score: f64, // 0-100
    pub findings: Vec<Finding>,
    pub suggestions: Vec<Suggestion>,
    pub dependency_stats: DependencyStats,
    pub duplicate_packages: Vec<DuplicateEntry>,
    pub potentially_unused: Vec<String>,
    pub channel_distribution: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub severity: String, // "info", "warning", "critical"
    pub category: String,
    pub message: String,
    pub impact: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub action: String,
    pub rationale: String,
    pub estimated_savings: Option<String>,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyStats {
    pub unique_channels: usize,
    pub packages_with_build_string: usize,
    pub pip_only_count: usize,
    pub max_dependency_chain: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateEntry {
    pub name: String,
    pub versions: Vec<String>,
    pub channels: Vec<String>,
}

/// Known "leaf" packages (commonly installed but rarely depended upon)
const LEAF_PACKAGES: &[&str] = &[
    "jupyter", "notebook", "ipython", "pytest", "black", "ruff",
    "flake8", "mypy", "sphinx", "twine", "wheel",
];

/// Run full optimization analysis
pub async fn analyze_env(backend: &str, query: &OptimizeQuery) -> Result<OptimizeReport, AppError> {
    let packages = conda::list_packages(backend, &query.env).await?;
    let check_disk = query.check_disk.unwrap_or(false);

    let mut findings = Vec::new();
    let mut suggestions = Vec::new();

    // 1. Channel distribution
    let mut channel_dist: BTreeMap<String, usize> = BTreeMap::new();
    for pkg in &packages {
        *channel_dist.entry(pkg.channel.clone()).or_default() += 1;
    }

    // 2. Check for mixed channels (potential conflict source)
    if channel_dist.len() > 3 {
        findings.push(Finding {
            severity: "warning".to_string(),
            category: "channels".to_string(),
            message: format!("Environment uses {} different channels", channel_dist.len()),
            impact: Some("Mixed channels can cause dependency conflicts".to_string()),
        });
        suggestions.push(Suggestion {
            action: "Consolidate to conda-forge".to_string(),
            rationale: "Using a single channel reduces conflicts and improves reproducibility".to_string(),
            estimated_savings: Some("Faster solve times".to_string()),
            command: Some(format!("conda install -n {} --channel conda-forge --override-channels <packages>", query.env)),
        });
    }

    // 3. Detect potential duplicates (same name, different case or suffix)
    let mut name_map: HashMap<String, Vec<&conda::CondaPackage>> = HashMap::new();
    for pkg in &packages {
        name_map.entry(pkg.name.to_lowercase()).or_default().push(pkg);
    }
    let mut duplicates = Vec::new();
    for (_, pkgs) in &name_map {
        if pkgs.len() > 1 {
            let versions: Vec<String> = pkgs.iter().map(|p| p.version.clone()).collect();
            let channels: Vec<String> = pkgs.iter().map(|p| p.channel.clone()).collect();
            if versions.windows(2).any(|w| w[0] != w[1]) {
                duplicates.push(DuplicateEntry {
                    name: pkgs[0].name.clone(),
                    versions,
                    channels,
                });
            }
        }
    }

    // 4. Detect potentially unused packages
    let pkg_names: BTreeSet<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    let mut potentially_unused = Vec::new();
    for leaf in LEAF_PACKAGES {
        if pkg_names.contains(leaf) {
            // Check if it's a dev/testing tool
            potentially_unused.push(leaf.to_string());
        }
    }

    if !potentially_unused.is_empty() {
        findings.push(Finding {
            severity: "info".to_string(),
            category: "unused".to_string(),
            message: format!("{} potentially unused dev/test packages detected", potentially_unused.len()),
            impact: Some("Consider removing if not actively needed".to_string()),
        });
    }

    // 5. Environment size analysis
    let env_size_mb = if check_disk {
        get_env_size(backend, &query.env).await.ok()
    } else {
        None
    };

    if let Some(size) = env_size_mb {
        if size > 5000.0 {
            findings.push(Finding {
                severity: "warning".to_string(),
                category: "size".to_string(),
                message: format!("Environment is {:.0} MB — consider conda-pack for deployment", size),
                impact: Some("Large envs slow down CI/CD and container builds".to_string()),
            });
            suggestions.push(Suggestion {
                action: "Use conda-pack for deployment".to_string(),
                rationale: "conda-pack creates relocatable archives, eliminating solver overhead".to_string(),
                estimated_savings: Some(format!("~{:.0} MB faster deployments", size * 0.3)),
                command: Some(format!("conda-pack -n {} -o {}.tar.gz", query.env, query.env)),
            });
        }
    }

    // 6. Solver suggestion
    if packages.len() > 100 {
        suggestions.push(Suggestion {
            action: "Use mamba as solver".to_string(),
            rationale: "mamba is 10-100x faster than conda's classic solver for large envs".to_string(),
            estimated_savings: Some("Minutes saved per solve".to_string()),
            command: Some("conda install -n base mamba".to_string()),
        });
    }

    // 7. Build string analysis
    let with_build = packages.iter().filter(|p| !p.build.is_empty() && p.build != "0").count();

    // 8. Calculate health score
    let mut score: f64 = 100.0;
    score -= (channel_dist.len() as f64 - 1.0) * 5.0; // -5 per extra channel
    score -= duplicates.len() as f64 * 10.0;
    score -= potentially_unused.len() as f64 * 2.0;
    if packages.len() > 200 {
        score -= 10.0;
    }
    let health_score = score.max(0.0).min(100.0);

    if health_score < 70.0 {
        suggestions.push(Suggestion {
            action: "Consider creating a fresh minimal environment".to_string(),
            rationale: "Health score below 70 suggests accumulated cruft".to_string(),
            estimated_savings: Some("Cleaner dependency tree".to_string()),
            command: None,
        });
    }

    Ok(OptimizeReport {
        environment: query.env.clone(),
        total_packages: packages.len(),
        env_size_mb,
        health_score: (health_score * 100.0).round() / 100.0,
        findings,
        suggestions,
        dependency_stats: DependencyStats {
            unique_channels: channel_dist.len(),
            packages_with_build_string: with_build,
            pip_only_count: 0,
            max_dependency_chain: 0,
        },
        duplicate_packages: duplicates,
        potentially_unused,
        channel_distribution: channel_dist,
    })
}

/// Get environment size on disk (MB)
async fn get_env_size(backend: &str, env_name: &str) -> Result<f64, AppError> {
    let envs = conda::list_envs(backend).await?;
    let env = envs.iter().find(|e| e.name == env_name)
        .ok_or_else(|| AppError::NotFound { resource: format!("env: {}", env_name) })?;

    let output = run_cmd("du", &["-sm", &env.path]).await?;
    let size_str = output.split_whitespace().next().unwrap_or("0");
    Ok(size_str.parse::<f64>().unwrap_or(0.0))
}

// ─── Axum Handler ─────────────────────────────────────────────────────

pub async fn optimize_handler(
    State(_state): AppStateRef,
    Query(query): Query<OptimizeQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = conda::detect_backend().await?;
    let result = analyze_env(&backend, &query).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}
