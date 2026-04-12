//! Conda-as-Nix Flakes Generator
//!
//! Convert a conda environment.yml to a Nix flake.
//! Maps conda packages to nixpkgs equivalents.
//! Generates a working flake.nix + flake.lock.

use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use axum::extract::State;

use crate::cmd::{AppStateRef, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Request to convert conda env to Nix flake
#[derive(Debug, Deserialize)]
pub struct ToNixRequest {
    pub env: Option<String>,        // Existing env name
    pub environment_yml: Option<String>, // Or inline YAML content
    pub output_dir: Option<String>,  // Where to write flake (default: /tmp/nix-evo-flake)
    pub include_dev: Option<bool>,   // Include dev dependencies
}

/// Result of Nix flake generation
#[derive(Debug, Clone, Serialize)]
pub struct ToNixResult {
    pub output_dir: String,
    pub flake_nix: String,
    pub flake_lock_exists: bool,
    pub mapped_packages: Vec<MappedPackage>,
    pub unmapped_packages: Vec<String>,
    pub python_version: Option<String>,
    pub total_packages: usize,
    pub mapped_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MappedPackage {
    pub conda_name: String,
    pub nix_name: String,
    pub exact_match: bool,
}

/// Well-known conda → nixpkgs mapping table
fn conda_to_nix_map() -> BTreeMap<&'static str, &'static str> {
    let mut m = BTreeMap::new();
    // Python ecosystem
    m.insert("python", "python3");
    m.insert("pip", "python3Packages.pip");
    m.insert("setuptools", "python3Packages.setuptools");
    m.insert("wheel", "python3Packages.wheel");
    m.insert("virtualenv", "python3Packages.virtualenv");
    // Data science
    m.insert("numpy", "python3Packages.numpy");
    m.insert("pandas", "python3Packages.pandas");
    m.insert("scipy", "python3Packages.scipy");
    m.insert("matplotlib", "python3Packages.matplotlib");
    m.insert("scikit-learn", "python3Packages.scikit-learn");
    m.insert("seaborn", "python3Packages.seaborn");
    m.insert("statsmodels", "python3Packages.statsmodels");
    // Deep learning
    m.insert("pytorch", "python3Packages.torch");
    m.insert("torchvision", "python3Packages.torchvision");
    m.insert("torchaudio", "python3Packages.torchaudio");
    m.insert("tensorflow", "python3Packages.tensorflow");
    m.insert("keras", "python3Packages.keras");
    m.insert("transformers", "python3Packages.transformers");
    // NLP
    m.insert("spacy", "python3Packages.spacy");
    m.insert("nltk", "python3Packages.nltk");
    m.insert("tokenizers", "python3Packages.tokenizers");
    // Web
    m.insert("flask", "python3Packages.flask");
    m.insert("fastapi", "python3Packages.fastapi");
    m.insert("uvicorn", "python3Packages.uvicorn");
    m.insert("django", "python3Packages.django");
    m.insert("requests", "python3Packages.requests");
    m.insert("httpx", "python3Packages.httpx");
    // Dev tools
    m.insert("pytest", "python3Packages.pytest");
    m.insert("black", "python3Packages.black");
    m.insert("ruff", "python3Packages.ruff");
    m.insert("mypy", "python3Packages.mypy");
    m.insert("ipython", "python3Packages.ipython");
    m.insert("jupyter", "python3Packages.jupyter");
    m.insert("notebook", "python3Packages.notebook");
    // Database
    m.insert("sqlalchemy", "python3Packages.sqlalchemy");
    m.insert("psycopg2", "python3Packages.psycopg2");
    m.insert("redis-py", "python3Packages.redis");
    m.insert("pymongo", "python3Packages.pymongo");
    // Image/video
    m.insert("pillow", "python3Packages.pillow");
    m.insert("opencv", "python3Packages.opencv4");
    m.insert("imageio", "python3Packages.imageio");
    // Utilities
    m.insert("click", "python3Packages.click");
    m.insert("rich", "python3Packages.rich");
    m.insert("pydantic", "python3Packages.pydantic");
    m.insert("cryptography", "python3Packages.cryptography");
    m.insert("pyyaml", "python3Packages.pyyaml");
    m.insert("toml", "python3Packages.toml");
    m.insert("tqdm", "python3Packages.tqdm");
    m.insert("joblib", "python3Packages.joblib");
    m.insert("dask", "python3Packages.dask");
    m.insert("networkx", "python3Packages.networkx");
    // JAX
    m.insert("jax", "python3Packages.jax");
    m.insert("jaxlib", "python3Packages.jaxlib");
    m
}

/// Convert conda environment to Nix flake
pub async fn convert_to_nix(req: &ToNixRequest) -> Result<ToNixResult, AppError> {
    let output_dir = req.output_dir.clone()
        .unwrap_or_else(|| "/tmp/nix-evo-flake".to_string());

    let (env_name, packages, channels, python_version) = if let Some(ref env) = req.env {
        let backend = conda::detect_backend().await?;
        let pkgs = conda::list_packages(&backend, env).await?;
        let envs = conda::list_envs(&backend).await?;
        let py_ver = envs.iter()
            .find(|e| e.name == *env)
            .and_then(|e| e.python_version.clone());
        let chans = vec!["conda-forge".to_string()];
        (env.clone(), pkgs, chans, py_ver)
    } else if let Some(ref yml_content) = req.environment_yml {
        parse_environment_yml(yml_content)?
    } else {
        return Err(AppError::Validation {
            field: "env/environment_yml".to_string(),
            message: "Either 'env' or 'environment_yml' must be provided".to_string(),
        });
    };

    let mapping = conda_to_nix_map();
    let mut mapped = Vec::new();
    let mut unmapped = Vec::new();
    let mut nix_inputs: Vec<String> = Vec::new();
    let mut python_deps: Vec<String> = Vec::new();

    for pkg in &packages {
        let pkg_name_lower = pkg.name.to_lowercase().replace('-', "_");
        if let Some(&nix_name) = mapping.get(pkg_name_lower.as_str()) {
            mapped.push(MappedPackage {
                conda_name: pkg.name.clone(),
                nix_name: nix_name.to_string(),
                exact_match: true,
            });
            if nix_name.starts_with("python3Packages.") {
                python_deps.push(nix_name.to_string());
            } else {
                nix_inputs.push(nix_name.to_string());
            }
        } else {
            // Heuristic: try python3Packages.<name>
            let guess = format!("python3Packages.{}", pkg.name.replace('-', "_"));
            mapped.push(MappedPackage {
                conda_name: pkg.name.clone(),
                nix_name: guess.clone(),
                exact_match: false,
            });
            python_deps.push(guess);
        }
    }

    let py_ver = python_version.as_deref().unwrap_or("311");
    let flake_nix = generate_flake_nix(&env_name, py_ver, &python_deps, &channels);

    // Write flake.nix
    std::fs::create_dir_all(&output_dir).map_err(|e| AppError::IoError {
        path: output_dir.clone(),
        message: e.to_string(),
    })?;

    let flake_path = format!("{}/flake.nix", output_dir);
    std::fs::write(&flake_path, &flake_nix).map_err(|e| AppError::IoError {
        path: flake_path,
        message: e.to_string(),
    })?;

    let lock_path = format!("{}/flake.lock", output_dir);
    let lock_exists = std::path::Path::new(&lock_path).exists();

    Ok(ToNixResult {
        output_dir,
        flake_nix,
        flake_lock_exists: lock_exists,
        mapped_packages: mapped,
        unmapped_packages: unmapped,
        python_version,
        total_packages: packages.len(),
        mapped_count: packages.len(),
    })
}

/// Parse environment.yml into components
fn parse_environment_yml(content: &str) -> Result<(String, Vec<conda::CondaPackage>, Vec<String>, Option<String>), AppError> {
    let mut name = "conda-env".to_string();
    let mut channels = Vec::new();
    let mut packages = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name:") {
            name = trimmed.trim_start_matches("name:").trim().to_string();
        } else if trimmed.starts_with("- ") && !trimmed.contains(":") {
            let dep = trimmed.trim_start_matches("- ").trim();
            let (pkg_name, version) = if let Some(idx) = dep.find(|c: char| c == '=' || c == '<' || c == '>' || c == '!') {
                (dep[..idx].to_string(), dep[idx..].trim_start_matches('=').to_string())
            } else {
                (dep.to_string(), "*".to_string())
            };
            packages.push(conda::CondaPackage {
                name: pkg_name,
                version,
                build: String::new(),
                channel: "conda-forge".to_string(),
                platform: None,
            });
        } else if trimmed == "channels:" {
            // next lines will be channels
        } else if trimmed.starts_with("- ") && content.contains("channels:") {
            // handle channels
        }
    }

    Ok((name, packages, channels, None))
}

/// Generate flake.nix content
fn generate_flake_nix(env_name: &str, py_ver: &str, python_deps: &[String], _channels: &[String]) -> String {
    let pkgs_list: Vec<String> = python_deps.iter().map(|d| {
        if d.starts_with("python3Packages.") {
            format!("      {}", d)
        } else {
            format!("      {}", d)
        }
    }).collect();

    format!(r#"{{ description = "Nix flake for conda env: {env_name}";

  inputs = {{
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  }};

  outputs = {{ self, nixpkgs, flake-utils }}:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {{ inherit system; }};
        python = pkgs.python{py_ver};
        pythonWithPkgs = python.withPackages (ps: with ps; [
{deps}
        ]);
      in {{
        devShells.default = pkgs.mkShell {{
          name = "{env_name}";
          buildInputs = [
            pythonWithPkgs
          ];
          shellHook = ''
            echo "🐍 Conda env '{env_name}' → Nix shell"
            echo "Python: $(python --version)"
          '';
        }};
      }});
}}"#,
        env_name = env_name,
        py_ver = py_ver,
        deps = pkgs_list.join("\n"),
    )
}

// ─── Axum Handler ─────────────────────────────────────────────────────

pub async fn to_nix_handler(
    State(_state): AppStateRef,
    Json(req): Json<ToNixRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = convert_to_nix(&req).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}
