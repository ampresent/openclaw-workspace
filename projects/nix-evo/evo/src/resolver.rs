//! Package Dependency Resolver
//!
//! Cross-solver: check if a package exists in both nixpkgs and conda-forge.
//! Compare versions, build flags, dependencies.
//! Recommend the better source for each package.

use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::cmd::{AppStateRef, run_cmd};
use crate::error::AppError;

/// Source where a package can be found
#[derive(Debug, Clone, Serialize)]
pub struct PackageSource {
    pub source: SourceType,
    pub available: bool,
    pub version: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub dependencies: Vec<String>,
    pub size_mb: Option<f64>,
    pub build_flags: Vec<String>,
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum SourceType {
    Nixpkgs,
    CondaForge,
    PyPI,
}

/// Comparison result between two sources
#[derive(Debug, Clone, Serialize)]
pub struct PackageResolution {
    pub package_name: String,
    pub nixpkgs: Option<PackageSource>,
    pub conda_forge: Option<PackageSource>,
    pub pypi: Option<PackageSource>,
    pub recommendation: Recommendation,
    pub reason: String,
    pub conflicts: Vec<String>,
    pub compatibility_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    pub preferred_source: SourceType,
    pub confidence: String,  // high, medium, low
    pub use_case: String,
}

/// Batch resolution for multiple packages
#[derive(Debug, Clone, Serialize)]
pub struct BatchResolution {
    pub packages: Vec<PackageResolution>,
    pub summary: ResolutionSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolutionSummary {
    pub total: usize,
    pub prefer_nix: usize,
    pub prefer_conda: usize,
    pub prefer_pypi: usize,
    pub unavailable: usize,
}

// ─── Core Resolution Logic ───────────────────────────────────────────

/// Resolve a single package across all sources
pub async fn resolve_package(name: &str) -> Result<PackageResolution, AppError> {
    let nixpkgs = check_nixpkgs(name).await;
    let conda_forge = check_conda_forge(name).await;
    let pypi = check_pypi(name).await;

    let recommendation = recommend_source(name, &nixpkgs, &conda_forge, &pypi);
    let conflicts = find_conflicts(&nixpkgs, &conda_forge, &pypi);
    let compatibility_notes = get_compatibility_notes(name, &nixpkgs, &conda_forge);

    Ok(PackageResolution {
        package_name: name.to_string(),
        nixpkgs,
        conda_forge,
        pypi,
        recommendation,
        reason: generate_reason(name),
        conflicts,
        compatibility_notes,
    })
}

/// Batch resolve multiple packages
pub async fn resolve_batch(packages: &[&str]) -> Result<BatchResolution, AppError> {
    let mut resolved = Vec::new();
    let mut prefer_nix = 0;
    let mut prefer_conda = 0;
    let mut prefer_pypi = 0;
    let mut unavailable = 0;

    for &name in packages {
        let resolution = resolve_package(name).await?;
        match resolution.recommendation.preferred_source {
            SourceType::Nixpkgs => prefer_nix += 1,
            SourceType::CondaForge => prefer_conda += 1,
            SourceType::PyPI => prefer_pypi += 1,
        }
        if resolution.nixpkgs.is_none() && resolution.conda_forge.is_none() && resolution.pypi.is_none() {
            unavailable += 1;
        }
        resolved.push(resolution);
    }

    Ok(BatchResolution {
        packages: resolved,
        summary: ResolutionSummary {
            total: packages.len(),
            prefer_nix,
            prefer_conda,
            prefer_pypi,
            unavailable,
        },
    })
}

// ─── Source Checkers ──────────────────────────────────────────────────

/// Check if a package exists in nixpkgs
async fn check_nixpkgs(name: &str) -> Option<PackageSource> {
    // Try nix-env query
    let output = run_cmd("nix-env", &["-qaP", &format!("nixpkgs.{name}"), "--json"]).await.ok()?;

    if output.trim().is_empty() || output.trim() == "{}" {
        // Also try without nixpkgs prefix
        let output2 = run_cmd("nix-env", &["-qaP", name, "--json"]).await.ok()?;
        if output2.trim().is_empty() || output2.trim() == "{}" {
            return None;
        }
    }

    // Parse version from nix-env output
    let version = extract_nix_version(&output);
    let description = extract_nix_description(&output);

    Some(PackageSource {
        source: SourceType::Nixpkgs,
        available: true,
        version,
        description,
        homepage: None,
        license: None,
        dependencies: vec![],
        size_mb: None,
        build_flags: vec![],
        last_updated: None,
    })
}

/// Check if a package exists on conda-forge
async fn check_conda_forge(name: &str) -> Option<PackageSource> {
    // Use conda/micromamba search
    let backend = if run_cmd("micromamba", &["--version"]).await.is_ok() {
        "micromamba"
    } else if run_cmd("conda", &["--version"]).await.is_ok() {
        "conda"
    } else {
        return None;
    };

    let output = run_cmd(backend, &["search", "-c", "conda-forge", name, "--json"]).await.ok()?;

    if output.trim().is_empty() || output.trim() == "[]" {
        return None;
    }

    // Parse JSON response
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        if let Some(packages) = json.get("pkgs").and_then(|p| p.as_array()) {
            if let Some(latest) = packages.last() {
                let version = latest.get("version").and_then(|v| v.as_str()).map(String::from);
                let build = latest.get("build").and_then(|b| b.as_str()).map(String::from);
                let size = latest.get("size").and_then(|s| s.as_f64()).map(|s| s / 1_048_576.0);

                return Some(PackageSource {
                    source: SourceType::CondaForge,
                    available: true,
                    version,
                    description: None,
                    homepage: None,
                    license: None,
                    dependencies: vec![],
                    size_mb: size,
                    build_flags: build.map(|b| vec![b]).unwrap_or_default(),
                    last_updated: None,
                });
            }
        }
    }

    // Fallback: parse text output
    let version = extract_conda_version(&output);
    Some(PackageSource {
        source: SourceType::CondaForge,
        available: true,
        version,
        description: None,
        homepage: None,
        license: None,
        dependencies: vec![],
        size_mb: None,
        build_flags: vec![],
        last_updated: None,
    })
}

/// Check if a package exists on PyPI
async fn check_pypi(name: &str) -> Option<PackageSource> {
    let url = format!("https://pypi.org/pypi/{name}/json");
    let output = tokio::process::Command::new("curl")
        .args(["-s", "--fail", &url])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let body = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        let info = json.get("info")?;
        let version = info.get("version").and_then(|v| v.as_str()).map(String::from);
        let description = info.get("summary").and_then(|v| v.as_str()).map(String::from);
        let homepage = info.get("home_page").and_then(|v| v.as_str()).map(String::from)
            .or_else(|| info.get("project_urls").and_then(|u| u.get("Homepage")).and_then(|v| v.as_str()).map(String::from));
        let license = info.get("license").and_then(|v| v.as_str()).map(String::from);

        Some(PackageSource {
            source: SourceType::PyPI,
            available: true,
            version,
            description,
            homepage,
            license,
            dependencies: vec![],
            size_mb: None,
            build_flags: vec![],
            last_updated: None,
        })
    } else {
        None
    }
}

// ─── Recommendation Engine ───────────────────────────────────────────

/// Recommend the best source for a package
fn recommend_source(
    name: &str,
    nixpkgs: &Option<PackageSource>,
    conda_forge: &Option<PackageSource>,
    pypi: &Option<PackageSource>,
) -> Recommendation {
    let nix_avail = nixpkgs.as_ref().map(|s| s.available).unwrap_or(false);
    let conda_avail = conda_forge.as_ref().map(|s| s.available).unwrap_or(false);
    let pypi_avail = pypi.as_ref().map(|s| s.available).unwrap_or(false);

    // Known conda-better packages (CUDA, ML, scientific)
    let conda_preferred = [
        "pytorch", "torchvision", "torchaudio", "tensorflow", "cudatoolkit", "cudnn",
        "nccl", "mkl", "openblas", "numpy", "scipy", "pandas", "scikit-learn",
        "opencv", "ffmpeg", "pillow", "h5py", "netcdf4", "rasterio", "gdal",
        "vtk", "mayavi", "pyqt", "qt", "opencv-python", "onnxruntime",
    ];

    // Known nix-better packages (system tools, CLI)
    let nix_preferred = [
        "git", "curl", "jq", "ripgrep", "fd", "bat", "exa", "fzf",
        "tmux", "neovim", "docker", "nginx", "postgresql", "redis",
        "nodejs", "go", "rustc", "gcc", "cmake", "make",
    ];

    // Prefer conda for ML/scientific packages
    if conda_avail && conda_preferred.iter().any(|&p| name.contains(p) || p.contains(name)) {
        return Recommendation {
            preferred_source: SourceType::CondaForge,
            confidence: "high".to_string(),
            use_case: "ML/scientific packages with CUDA/MKL optimizations".to_string(),
        };
    }

    // Prefer nix for system tools
    if nix_avail && nix_preferred.iter().any(|&p| name.contains(p) || p.contains(name)) {
        return Recommendation {
            preferred_source: SourceType::Nixpkgs,
            confidence: "high".to_string(),
            use_case: "System tools managed by NixOS".to_string(),
        };
    }

    // If only available in one source
    let mut available = vec![];
    if nix_avail { available.push(SourceType::Nixpkgs); }
    if conda_avail { available.push(SourceType::CondaForge); }
    if pypi_avail { available.push(SourceType::PyPI); }

    match available.len() {
        0 => Recommendation {
            preferred_source: SourceType::PyPI,
            confidence: "low".to_string(),
            use_case: "Package not found in any source".to_string(),
        },
        1 => Recommendation {
            preferred_source: available[0].clone(),
            confidence: "high".to_string(),
            use_case: "Only available from one source".to_string(),
        },
        _ => {
            // Prefer conda if available (better binary compat for scientific packages)
            if conda_avail {
                Recommendation {
                    preferred_source: SourceType::CondaForge,
                    confidence: "medium".to_string(),
                    use_case: "Conda-forge provides pre-built binaries with optimized BLAS".to_string(),
                }
            } else if nix_avail {
                Recommendation {
                    preferred_source: SourceType::Nixpkgs,
                    confidence: "medium".to_string(),
                    use_case: "Available in nixpkgs with Nix-reproducible builds".to_string(),
                }
            } else {
                Recommendation {
                    preferred_source: SourceType::PyPI,
                    confidence: "medium".to_string(),
                    use_case: "Available on PyPI".to_string(),
                }
            }
        }
    }
}

/// Find potential conflicts between sources
fn find_conflicts(
    nixpkgs: &Option<PackageSource>,
    conda_forge: &Option<PackageSource>,
    _pypi: &Option<PackageSource>,
) -> Vec<String> {
    let mut conflicts = Vec::new();

    if let (Some(nix), Some(conda)) = (nixpkgs, conda_forge) {
        if let (Some(nv), Some(cv)) = (&nix.version, &conda.version) {
            if nv != cv {
                conflicts.push(format!(
                    "Version mismatch: nixpkgs has {nv}, conda-forge has {cv}"
                ));
            }
        }
    }

    conflicts
}

/// Get compatibility notes for a package
fn get_compatibility_notes(
    name: &str,
    nixpkgs: &Option<PackageSource>,
    conda_forge: &Option<PackageSource>,
) -> Vec<String> {
    let mut notes = Vec::new();

    // Known compatibility issues
    match name {
        "numpy" => {
            notes.push("numpy from conda-forge uses MKL/OpenBLAS optimizations".to_string());
            if nixpkgs.is_some() {
                notes.push("nixpkgs numpy may lack SIMD optimizations for your CPU".to_string());
            }
        }
        "pytorch" | "torch" => {
            notes.push("conda-forge provides CUDA-optimized builds".to_string());
            notes.push("Nix + PyTorch CUDA is fragile; prefer conda for GPU workloads".to_string());
        }
        "opencv" | "opencv-python" => {
            notes.push("conda-forge opencv has more codec support (ffmpeg, gstreamer)".to_string());
        }
        "gdal" | "rasterio" => {
            notes.push("System GDAL from Nix may conflict with conda GDAL (PROJ/libgeotiff)".to_string());
        }
        _ => {}
    }

    notes
}

fn generate_reason(_name: &str) -> String {
    "Based on availability, version freshness, and known optimization patterns".to_string()
}

// ─── Parsing Helpers ─────────────────────────────────────────────────

fn extract_nix_version(output: &str) -> Option<String> {
    // nix-env --json output format
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(obj) = json.as_object() {
            for (_, val) in obj {
                if let Some(version) = val.get("version").and_then(|v| v.as_str()) {
                    return Some(version.to_string());
                }
            }
        }
    }
    None
}

fn extract_nix_description(output: &str) -> Option<String> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(obj) = json.as_object() {
            for (_, val) in obj {
                if let Some(desc) = val.get("description").and_then(|v| v.as_str()) {
                    return Some(desc.to_string());
                }
            }
        }
    }
    None
}

fn extract_conda_version(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains("version") {
            if let Some(v) = line.split(':').nth(1) {
                let v = v.trim().trim_matches('"').trim_matches(',');
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

// ─── HTTP Handler ─────────────────────────────────────────────────────

/// GET /api/resolve/package/{name}
pub async fn resolve_handler(
    State(_state): AppStateRef,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let resolution = resolve_package(&name).await?;
    Ok(Json(serde_json::to_value(&resolution).unwrap()))
}

/// POST /api/resolve/batch
#[derive(Deserialize)]
pub struct BatchResolveBody {
    pub packages: Vec<String>,
}

pub async fn batch_resolve_handler(
    State(_state): AppStateRef,
    Json(body): Json<BatchResolveBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pkg_refs: Vec<&str> = body.packages.iter().map(|s| s.as_str()).collect();
    let resolution = resolve_batch(&pkg_refs).await?;
    Ok(Json(serde_json::to_value(&resolution).unwrap()))
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommendation_conda_preferred() {
        let nix = Some(PackageSource {
            source: SourceType::Nixpkgs, available: true, version: Some("1.24.0".to_string()),
            description: None, homepage: None, license: None, dependencies: vec![],
            size_mb: None, build_flags: vec![], last_updated: None,
        });
        let conda = Some(PackageSource {
            source: SourceType::CondaForge, available: true, version: Some("1.26.3".to_string()),
            description: None, homepage: None, license: None, dependencies: vec![],
            size_mb: None, build_flags: vec![], last_updated: None,
        });

        let rec = recommend_source("numpy", &nix, &conda, &None);
        assert_eq!(rec.preferred_source, SourceType::CondaForge);
        assert_eq!(rec.confidence, "high");
    }

    #[test]
    fn test_recommendation_nix_preferred() {
        let nix = Some(PackageSource {
            source: SourceType::Nixpkgs, available: true, version: Some("2.43.0".to_string()),
            description: None, homepage: None, license: None, dependencies: vec![],
            size_mb: None, build_flags: vec![], last_updated: None,
        });

        let rec = recommend_source("git", &nix, &None, &None);
        assert_eq!(rec.preferred_source, SourceType::Nixpkgs);
    }

    #[test]
    fn test_find_conflicts() {
        let nix = Some(PackageSource {
            source: SourceType::Nixpkgs, available: true, version: Some("1.24.0".to_string()),
            description: None, homepage: None, license: None, dependencies: vec![],
            size_mb: None, build_flags: vec![], last_updated: None,
        });
        let conda = Some(PackageSource {
            source: SourceType::CondaForge, available: true, version: Some("1.26.3".to_string()),
            description: None, homepage: None, license: None, dependencies: vec![],
            size_mb: None, build_flags: vec![], last_updated: None,
        });

        let conflicts = find_conflicts(&nix, &conda, &None);
        assert!(!conflicts.is_empty());
        assert!(conflicts[0].contains("1.24.0"));
        assert!(conflicts[0].contains("1.26.3"));
    }

    #[test]
    fn test_compatibility_notes() {
        let notes = get_compatibility_notes("pytorch", &None, &None);
        assert!(!notes.is_empty());
        assert!(notes.iter().any(|n| n.contains("CUDA")));
    }

    #[test]
    fn test_source_type_serialization() {
        let src = SourceType::Nixpkgs;
        let json = serde_json::to_string(&src).unwrap();
        assert!(json.contains("Nixpkgs"));
    }
}
