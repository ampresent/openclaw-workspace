//! NixOS + conda hybrid management
//!
//! Provides a unified view of both NixOS system state AND conda environment state.
//! Detects conflicts between system Python and conda environments.
//! Suggests alignment strategies.

use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Unified snapshot combining NixOS system state and conda environments
#[derive(Debug, Clone, Serialize)]
pub struct HybridSnapshot {
    pub system: SystemState,
    pub conda: CondaState,
    pub conflicts: Vec<Conflict>,
    pub alignment_suggestions: Vec<AlignmentSuggestion>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemState {
    pub nixos_version: String,
    pub system_python: Option<PythonInfo>,
    pub system_python_packages: Vec<String>,
    pub has_python_module: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PythonInfo {
    pub version: String,
    pub path: String,
    pub is_managed_by_nix: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CondaState {
    pub backend: String,
    pub environments: Vec<conda::CondaEnv>,
    pub conda_python_versions: HashMap<String, String>,
    pub total_packages: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Conflict {
    pub severity: String, // info, warning, error
    pub category: String, // python_version, path_precedence, duplicated_tool
    pub message: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlignmentSuggestion {
    pub strategy: String,
    pub description: String,
    pub actions: Vec<String>,
}

// ─── Core Logic ───────────────────────────────────────────────────────

/// Build a hybrid snapshot combining NixOS and conda state
pub async fn build_hybrid_snapshot() -> Result<HybridSnapshot, AppError> {
    let system_state = get_system_state().await;
    let conda_state = get_conda_state().await;
    let conflicts = detect_conflicts(&system_state, &conda_state);
    let suggestions = generate_alignment_suggestions(&system_state, &conda_state, &conflicts);

    Ok(HybridSnapshot {
        system: system_state,
        conda: conda_state,
        conflicts,
        alignment_suggestions: suggestions,
    })
}

async fn get_system_state() -> SystemState {
    let nixos_version = run_cmd("nixos-version", &["--configuration-revision"])
        .await
        .or_else(|_| run_cmd("nixos-version", &[]).await)
        .unwrap_or_default()
        .trim()
        .to_string();

    // Detect system Python
    let system_python = detect_system_python().await;

    // Check if python is managed by NixOS module
    let has_python_module = check_nix_python_module().await;

    // List system Python packages (via pip list or nix-store)
    let system_python_packages = get_system_python_packages().await;

    SystemState {
        nixos_version,
        system_python,
        system_python_packages,
        has_python_module,
    }
}

async fn get_conda_state() -> CondaState {
    match conda::detect_backend().await {
        Ok(backend) => {
            let envs = conda::list_envs(&backend).await.unwrap_or_default();
            let mut conda_python_versions = HashMap::new();
            let mut total_packages = 0;

            for env in &envs {
                if let Ok(packages) = conda::list_packages(&backend, &env.name).await {
                    total_packages += packages.len();
                    if let Some(py_pkg) = packages.iter().find(|p| p.name == "python") {
                        conda_python_versions.insert(env.name.clone(), py_pkg.version.clone());
                    }
                }
            }

            CondaState {
                backend,
                environments: envs,
                conda_python_versions,
                total_packages,
            }
        }
        Err(_) => CondaState {
            backend: "none".to_string(),
            environments: vec![],
            conda_python_versions: HashMap::new(),
            total_packages: 0,
        },
    }
}

async fn detect_system_python() -> Option<PythonInfo> {
    // Try python3 first
    let output = run_cmd("python3", &["--version"]).await.ok()?;
    let version = output.trim().replace("Python ", "");

    // Get path
    let path = run_cmd("which", &["python3"]).await
        .unwrap_or_default()
        .trim()
        .to_string();

    // Check if it's from /nix/store (NixOS-managed)
    let is_managed_by_nix = path.starts_with("/nix/store");

    Some(PythonInfo {
        version,
        path,
        is_managed_by_nix,
    })
}

async fn check_nix_python_module() -> bool {
    // Check /etc/nixos/configuration.nix for python-related modules
    let config_content = std::fs::read_to_string("/etc/nixos/configuration.nix").unwrap_or_default();
    config_content.contains("python3") ||
    config_content.contains("programs.python") ||
    config_content.contains("pythonPackages")
}

async fn get_system_python_packages() -> Vec<String> {
    // Try pip list --format=freeze
    match run_cmd("python3", &["-m", "pip", "list", "--format=freeze"]).await {
        Ok(output) => {
            output.lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split("==").collect();
                    if parts.len() >= 2 {
                        Some(format!("{}={}", parts[0], parts[1]))
                    } else {
                        None
                    }
                })
                .take(50) // Limit to avoid huge lists
                .collect()
        }
        Err(_) => vec![],
    }
}

// ─── Conflict Detection ──────────────────────────────────────────────

fn detect_conflicts(system: &SystemState, conda: &CondaState) -> Vec<Conflict> {
    let mut conflicts = Vec::new();

    if conda.backend == "none" {
        return conflicts;
    }

    // 1. Python version conflict
    if let Some(ref sys_py) = system.system_python {
        for (env_name, conda_ver) in &conda.conda_python_versions {
            if sys_py.version != *conda_ver {
                let sys_major_minor: Vec<&str> = sys_py.version.split('.').take(2).collect();
                let conda_major_minor: Vec<&str> = conda_ver.split('.').take(2).collect();

                let severity = if sys_major_minor != conda_major_minor {
                    "warning".to_string()
                } else {
                    "info".to_string()
                };

                conflicts.push(Conflict {
                    severity,
                    category: "python_version".to_string(),
                    message: format!(
                        "System Python {} ({}) differs from conda env '{}' Python {} ({})",
                        sys_py.version,
                        if sys_py.is_managed_by_nix { "Nix-managed" } else { "system" },
                        env_name,
                        conda_ver,
                        conda.backend,
                    ),
                    details: format!(
                        "System: {}\nConda env '{}': {}",
                        sys_py.path, env_name, conda_ver
                    ),
                });
            }
        }
    }

    // 2. PATH precedence: conda python shadowing system python
    if let Some(ref sys_py) = system.system_python {
        for env in &conda.environments {
            let conda_bin = format!("{}/bin", env.path);
            if sys_py.path.starts_with(&conda_bin) || sys_py.path.contains("/condabin/") {
                conflicts.push(Conflict {
                    severity: "warning".to_string(),
                    category: "path_precedence".to_string(),
                    message: format!(
                        "conda environment '{}' bin ({}) shadows system Python PATH",
                        env.name, conda_bin
                    ),
                    details: "conda activate modifies PATH. Run `conda deactivate` to restore system Python.".to_string(),
                });
            }
        }
    }

    // 3. Duplicated tools: both system and conda have the same CLI tools
    let common_tools = ["pip", "jupyter", "black", "pytest", "mypy"];
    for tool in common_tools {
        let sys_has = std::process::Command::new("which")
            .arg(tool)
            .output()
            .ok()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if sys_has {
            for env in &conda.environments {
                let conda_tool_path = format!("{}/bin/{}", env.path, tool);
                if std::path::Path::new(&conda_tool_path).exists() {
                    conflicts.push(Conflict {
                        severity: "info".to_string(),
                        category: "duplicated_tool".to_string(),
                        message: format!(
                            "'{}' exists in both system PATH and conda env '{}'",
                            tool, env.name
                        ),
                        details: format!("System: via system install\nConda: {}", conda_tool_path),
                    });
                }
            }
        }
    }

    // 4. NixOS python module configured + conda usage
    if system.has_python_module && !conda.environments.is_empty() {
        conflicts.push(Conflict {
            severity: "info".to_string(),
            category: "duplicated_tool".to_string(),
            message: "NixOS python module is configured AND conda environments exist".to_string(),
            details: "Consider using one approach consistently. If conda manages your Python needs, the NixOS python module may be redundant.".to_string(),
        });
    }

    conflicts
}

// ─── Alignment Strategies ─────────────────────────────────────────────

fn generate_alignment_suggestions(
    system: &SystemState,
    conda: &CondaState,
    conflicts: &[Conflict],
) -> Vec<AlignmentSuggestion> {
    let mut suggestions = Vec::new();

    if conda.backend == "none" {
        return suggestions;
    }

    // Strategy 1: Nix manages base, conda manages projects
    if system.has_python_module {
        suggestions.push(AlignmentSuggestion {
            strategy: "nix-base-conda-projects".to_string(),
            description: "Use NixOS for system-level Python (system scripts, tools), use conda for project-specific ML/DS environments".to_string(),
            actions: vec![
                "Keep NixOS python3 module for system utilities".to_string(),
                "Use conda exclusively for data science/ML workloads".to_string(),
                "Document the separation: Nix = infra, conda = projects".to_string(),
            ],
        });
    }

    // Strategy 2: Full conda
    suggestions.push(AlignmentSuggestion {
        strategy: "full-conda".to_string(),
        description: "Remove NixOS python module, let conda manage all Python".to_string(),
        actions: vec![
            "Remove `python3` from NixOS configuration.nix packages".to_string(),
            "Install micromamba via Nix (nix profile install nixpkgs#micromamba)".to_string(),
            "Use conda for all Python environments".to_string(),
        ],
    });

    // Strategy 3: Full Nix
    suggestions.push(AlignmentSuggestion {
        strategy: "full-nix".to_string(),
        description: "Replace conda with Nix flakes for reproducible Python environments".to_string(),
        actions: vec![
            "Use devenv or nix develop with Python flake".to_string(),
            "Pin all Python packages via Nix".to_string(),
            "Remove conda environments".to_string(),
            "Note: some conda-only packages (CUDA, MKL) may not be available in nixpkgs".to_string(),
        ],
    });

    // Strategy 4: Hybrid with conda-lock
    suggestions.push(AlignmentSuggestion {
        strategy: "hybrid-lock".to_string(),
        description: "Use conda-lock for reproducibility + Nix for infrastructure management".to_string(),
        actions: vec![
            "Pin conda environments with conda-lock.yml".to_string(),
            "Use nix-evo to provision conda environments declaratively".to_string(),
            "Nix manages: system packages, services, firewall".to_string(),
            "conda-lock manages: Python packages, CUDA, ML frameworks".to_string(),
        ],
    });

    suggestions
}

// ─── HTTP Handler ─────────────────────────────────────────────────────

/// GET /api/hybrid/snapshot
pub async fn snapshot_handler(
    State(_state): AppStateRef,
    Query(_query): Query<HostQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let snapshot = build_hybrid_snapshot().await?;
    Ok(Json(serde_json::to_value(&snapshot).unwrap()))
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_conflicts_python_version() {
        let system = SystemState {
            nixos_version: "24.05".to_string(),
            system_python: Some(PythonInfo {
                version: "3.11.8".to_string(),
                path: "/usr/bin/python3".to_string(),
                is_managed_by_nix: false,
            }),
            system_python_packages: vec![],
            has_python_module: false,
        };

        let mut conda_python_versions = HashMap::new();
        conda_python_versions.insert("ml".to_string(), "3.10.12".to_string());

        let conda = CondaState {
            backend: "micromamba".to_string(),
            environments: vec![],
            conda_python_versions,
            total_packages: 0,
        };

        let conflicts = detect_conflicts(&system, &conda);
        assert!(!conflicts.is_empty());
        assert!(conflicts.iter().any(|c| c.category == "python_version"));
    }

    #[test]
    fn test_no_conflicts_when_matching() {
        let system = SystemState {
            nixos_version: "24.05".to_string(),
            system_python: Some(PythonInfo {
                version: "3.11.8".to_string(),
                path: "/usr/bin/python3".to_string(),
                is_managed_by_nix: false,
            }),
            system_python_packages: vec![],
            has_python_module: false,
        };

        let mut conda_python_versions = HashMap::new();
        conda_python_versions.insert("ml".to_string(), "3.11.7".to_string());

        let conda = CondaState {
            backend: "micromamba".to_string(),
            environments: vec![],
            conda_python_versions,
            total_packages: 0,
        };

        let conflicts = detect_conflicts(&system, &conda);
        // Same major.minor, should be info-level not warning
        assert!(conflicts.iter().all(|c| c.severity != "warning"));
    }

    #[test]
    fn test_alignment_suggestions() {
        let system = SystemState {
            nixos_version: "24.05".to_string(),
            system_python: None,
            system_python_packages: vec![],
            has_python_module: true,
        };
        let conda = CondaState {
            backend: "micromamba".to_string(),
            environments: vec![],
            conda_python_versions: HashMap::new(),
            total_packages: 0,
        };
        let suggestions = generate_alignment_suggestions(&system, &conda, &[]);
        assert!(suggestions.len() >= 2);
        assert!(suggestions.iter().any(|s| s.strategy == "nix-base-conda-projects"));
    }
}
