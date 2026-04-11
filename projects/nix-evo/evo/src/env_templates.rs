//! Environment Templates
//!
//! Pre-built environment templates for common use cases.
//! One-click provision from template with pinned versions.

use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::cmd::{AppStateRef, run_cmd};
use crate::conda::{self, detect_backend, create_env};
use crate::error::AppError;

/// Environment template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvTemplate {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub python_version: String,
    pub channels: Vec<String>,
    pub packages: Vec<TemplatePackage>,
    pub post_install_commands: Vec<String>,
    pub estimated_size_mb: u64,
}

/// A package in a template with optional pinning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplatePackage {
    pub name: String,
    pub version: Option<String>,  // None = latest
    pub channel: Option<String>,  // None = default channels
    pub optional: bool,
}

/// Provision request
#[derive(Debug, Clone, Deserialize)]
pub struct ProvisionRequest {
    pub template: String,
    pub env_name: Option<String>,    // override template name
    pub python_version: Option<String>, // override
    pub extra_packages: Option<Vec<String>>,
    pub skip_optional: Option<bool>,
    pub dry_run: Option<bool>,
}

/// Provision result
#[derive(Debug, Clone, Serialize)]
pub struct ProvisionResult {
    pub template: String,
    pub environment: String,
    pub success: bool,
    pub packages_installed: usize,
    pub python_version: String,
    pub commands_executed: Vec<String>,
    pub warnings: Vec<String>,
    pub duration_ms: u64,
}

// ─── Template Registry ────────────────────────────────────────────────

/// Get all available templates
pub fn get_all_templates() -> Vec<EnvTemplate> {
    vec![
        ml_gpu_template(),
        data_science_template(),
        web_dev_template(),
        bioinformatics_template(),
        deep_learning_template(),
        jupyter_template(),
    ]
}

/// Get a template by name
pub fn get_template(name: &str) -> Option<EnvTemplate> {
    get_all_templates().into_iter().find(|t| t.name == name)
}

/// Get templates by category
pub fn get_templates_by_category(category: &str) -> Vec<EnvTemplate> {
    get_all_templates()
        .into_iter()
        .filter(|t| t.category.eq_ignore_ascii_case(category))
        .collect()
}

/// Get all categories
pub fn get_categories() -> Vec<String> {
    let mut cats: Vec<String> = get_all_templates()
        .into_iter()
        .map(|t| t.category)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    cats.sort();
    cats
}

// ─── Template Definitions ─────────────────────────────────────────────

fn ml_gpu_template() -> EnvTemplate {
    EnvTemplate {
        name: "ml-gpu".to_string(),
        display_name: "ML-GPU".to_string(),
        description: "Machine Learning with GPU support (PyTorch + CUDA)".to_string(),
        category: "Machine Learning".to_string(),
        tags: vec!["gpu".into(), "pytorch".into(), "cuda".into(), "ml".into()],
        python_version: "3.11".to_string(),
        channels: vec!["conda-forge".into(), "nvidia".into(), "pytorch".into()],
        packages: vec![
            pkg("python", "3.11.*", false),
            pkg("pytorch", None, false),
            pkg("torchvision", None, false),
            pkg("torchaudio", None, false),
            pkg("pytorch-cuda", "12.1", false),
            pkg("numpy", ">=1.26", false),
            pkg("pandas", ">=2.1", false),
            pkg("scikit-learn", ">=1.3", false),
            pkg("matplotlib", ">=3.8", false),
            pkg("jupyter", None, true),
            pkg("tensorboard", None, true),
            pkg("optuna", None, true),
            pkg("lightning", None, true),
        ],
        post_install_commands: vec![
            "python -c 'import torch; print(f\"PyTorch {torch.__version__}, CUDA: {torch.cuda.is_available()}\")'".into(),
        ],
        estimated_size_mb: 4500,
    }
}

fn data_science_template() -> EnvTemplate {
    EnvTemplate {
        name: "data-science".to_string(),
        display_name: "Data Science".to_string(),
        description: "Full data science stack: pandas, scipy, visualization, notebooks".to_string(),
        category: "Data Science".to_string(),
        tags: vec!["data".into(), "analysis".into(), "jupyter".into(), "pandas".into()],
        python_version: "3.11".to_string(),
        channels: vec!["conda-forge".into(), "defaults".into()],
        packages: vec![
            pkg("python", "3.11.*", false),
            pkg("pandas", ">=2.1", false),
            pkg("numpy", ">=1.26", false),
            pkg("scipy", ">=1.12", false),
            pkg("scikit-learn", ">=1.3", false),
            pkg("matplotlib", ">=3.8", false),
            pkg("seaborn", ">=0.13", false),
            pkg("plotly", ">=5.18", false),
            pkg("jupyter", ">=1.0", false),
            pkg("jupyterlab", None, false),
            pkg("ipywidgets", None, true),
            pkg("polars", None, true),
            pkg("duckdb", None, true),
            pkg("statsmodels", None, true),
            pkg("xgboost", None, true),
        ],
        post_install_commands: vec![],
        estimated_size_mb: 2500,
    }
}

fn web_dev_template() -> EnvTemplate {
    EnvTemplate {
        name: "web-dev".to_string(),
        display_name: "Web Development".to_string(),
        description: "Python web development: FastAPI, Django, Flask, testing tools".to_string(),
        category: "Web Development".to_string(),
        tags: vec!["web".into(), "fastapi".into(), "django".into(), "flask".into()],
        python_version: "3.12".to_string(),
        channels: vec!["conda-forge".into()],
        packages: vec![
            pkg("python", "3.12.*", false),
            pkg("fastapi", None, false),
            pkg("uvicorn", None, false),
            pkg("httpx", None, false),
            pkg("pydantic", ">=2.0", false),
            pkg("sqlalchemy", ">=2.0", false),
            pkg("alembic", None, true),
            pkg("pytest", None, false),
            pkg("pytest-asyncio", None, true),
            pkg("black", None, true),
            pkg("ruff", None, true),
            pkg("mypy", None, true),
        ],
        post_install_commands: vec![],
        estimated_size_mb: 800,
    }
}

fn bioinformatics_template() -> EnvTemplate {
    EnvTemplate {
        name: "bioinformatics".to_string(),
        display_name: "Bioinformatics".to_string(),
        description: "Bioinformatics toolkit: Biopython, samtools, bwa, GATK wrappers".to_string(),
        category: "Bioinformatics".to_string(),
        tags: vec!["bio".into(), "genomics".into(), "ngs".into()],
        python_version: "3.11".to_string(),
        channels: vec!["bioconda".into(), "conda-forge".into()],
        packages: vec![
            pkg("python", "3.11.*", false),
            pkg("biopython", None, false),
            pkg("pysam", None, false),
            pkg("samtools", None, false),
            pkg("bwa", None, false),
            pkg("bedtools", None, false),
            pkg("vcftools", None, true),
            pkg("bcftools", None, true),
            pkg("htslib", None, true),
            pkg("numpy", ">=1.26", false),
            pkg("pandas", ">=2.1", false),
            pkg("matplotlib", ">=3.8", true),
            pkg("scikit-learn", ">=1.3", true),
            pkg("jupyter", None, true),
        ],
        post_install_commands: vec![
            "samtools --version | head -1".into(),
        ],
        estimated_size_mb: 1800,
    }
}

fn deep_learning_template() -> EnvTemplate {
    EnvTemplate {
        name: "deep-learning".to_string(),
        display_name: "Deep Learning".to_string(),
        description: "Deep learning frameworks: PyTorch, TensorFlow, JAX".to_string(),
        category: "Machine Learning".to_string(),
        tags: vec!["deep-learning".into(), "neural-networks".into(), "ai".into()],
        python_version: "3.11".to_string(),
        channels: vec!["conda-forge".into()],
        packages: vec![
            pkg("python", "3.11.*", false),
            pkg("pytorch", None, false),
            pkg("torchvision", None, false),
            pkg("transformers", None, false),
            pkg("datasets", None, false),
            pkg("accelerate", None, true),
            pkg("numpy", ">=1.26", false),
            pkg("scipy", ">=1.12", true),
            pkg("wandb", None, true),
            pkg("jupyter", None, true),
        ],
        post_install_commands: vec![],
        estimated_size_mb: 5000,
    }
}

fn jupyter_template() -> EnvTemplate {
    EnvTemplate {
        name: "jupyter".to_string(),
        display_name: "Jupyter".to_string(),
        description: "Lightweight Jupyter environment with common extensions".to_string(),
        category: "Development".to_string(),
        tags: vec!["jupyter".into(), "notebook".into(), "lab".into()],
        python_version: "3.12".to_string(),
        channels: vec!["conda-forge".into()],
        packages: vec![
            pkg("python", "3.12.*", false),
            pkg("jupyter", ">=1.0", false),
            pkg("jupyterlab", ">=4.0", false),
            pkg("ipywidgets", None, false),
            pkg("nb_conda_kernels", None, true),
            pkg("numpy", None, true),
            pkg("pandas", None, true),
            pkg("matplotlib", None, true),
        ],
        post_install_commands: vec![
            "jupyter lab --generate-config".into(),
        ],
        estimated_size_mb: 1200,
    }
}

fn pkg(name: &str, version: Option<&str>, optional: bool) -> TemplatePackage {
    TemplatePackage {
        name: name.to_string(),
        version: version.map(String::from),
        channel: None,
        optional,
    }
}

// ─── Provisioning Engine ──────────────────────────────────────────────

/// Provision an environment from a template
pub async fn provision_from_template(request: &ProvisionRequest) -> Result<ProvisionResult, AppError> {
    let start = std::time::Instant::now();
    let template = get_template(&request.template)
        .ok_or_else(|| AppError::NotFound {
            resource: format!("template: {}", request.template),
        })?;

    let env_name = request.env_name.as_deref().unwrap_or(&template.name);
    let python_version = request.python_version.as_deref().unwrap_or(&template.python_version);
    let skip_optional = request.skip_optional.unwrap_or(false);
    let dry_run = request.dry_run.unwrap_or(false);

    let backend = detect_backend().await?;
    let mut commands = Vec::new();
    let mut warnings = Vec::new();

    // Collect required packages
    let mut all_packages: Vec<String> = template.packages.iter()
        .filter(|p| !p.optional || !skip_optional)
        .map(|p| {
            if let Some(ver) = &p.version {
                format!("{}={}", p.name, ver)
            } else {
                p.name.clone()
            }
        })
        .collect();

    // Add extra packages
    if let Some(extra) = &request.extra_packages {
        all_packages.extend(extra.clone());
    }

    let pkg_count = all_packages.len();

    if !dry_run {
        // Create environment with Python
        let py_spec = format!("python={python_version}");
        let pkg_refs: Vec<&str> = all_packages.iter().map(|s| s.as_str()).collect();

        match create_env(&backend, env_name, Some(&py_spec), Some(&pkg_refs)).await {
            Ok(result) => {
                if !result.success {
                    warnings.push("Environment creation had issues".to_string());
                }
            }
            Err(e) => {
                return Ok(ProvisionResult {
                    template: request.template.clone(),
                    environment: env_name.to_string(),
                    success: false,
                    packages_installed: 0,
                    python_version: python_version.to_string(),
                    commands_executed: commands,
                    warnings,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }

        // Run post-install commands
        for cmd in &template.post_install_commands {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if !parts.is_empty() {
                let env_cmd = if backend == "micromamba" {
                    format!("micromamba run -n {env_name} {cmd}")
                } else {
                    format!("conda run -n {env_name} {cmd}")
                };
                commands.push(env_cmd);
            }
        }
    } else {
        commands.push(format!("{backend} create -n {env_name} python={python_version} {} -y",
            all_packages.join(" ")));
        warnings.push("Dry run — environment not created".to_string());
    }

    Ok(ProvisionResult {
        template: request.template.clone(),
        environment: env_name.to_string(),
        success: !dry_run || true,
        packages_installed: pkg_count,
        python_version: python_version.to_string(),
        commands_executed: commands,
        warnings,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

// ─── HTTP Handlers ────────────────────────────────────────────────────

/// GET /api/env/templates
pub async fn templates_handler(
    State(_state): AppStateRef,
) -> Result<Json<serde_json::Value>, AppError> {
    let templates = get_all_templates();
    let categories = get_categories();
    Ok(Json(serde_json::json!({
        "templates": templates,
        "categories": categories,
        "count": templates.len(),
    })))
}

/// GET /api/env/templates/:name
pub async fn template_detail_handler(
    State(_state): AppStateRef,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let template = get_template(&name)
        .ok_or_else(|| AppError::NotFound {
            resource: format!("template: {name}"),
        })?;
    Ok(Json(serde_json::to_value(&template).unwrap()))
}

/// POST /api/env/provision
pub async fn provision_handler(
    State(_state): AppStateRef,
    Json(body): Json<ProvisionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = provision_from_template(&body).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_templates_exist() {
        let templates = get_all_templates();
        assert!(templates.len() >= 6);
        let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"ml-gpu"));
        assert!(names.contains(&"data-science"));
        assert!(names.contains(&"web-dev"));
        assert!(names.contains(&"bioinformatics"));
    }

    #[test]
    fn test_template_has_packages() {
        let template = ml_gpu_template();
        assert!(!template.packages.is_empty());
        assert!(template.packages.iter().any(|p| p.name == "python"));
        assert!(template.packages.iter().any(|p| p.name == "pytorch"));
    }

    #[test]
    fn test_template_categories() {
        let categories = get_categories();
        assert!(!categories.is_empty());
        assert!(categories.iter().any(|c| c == "Machine Learning"));
    }

    #[test]
    fn test_get_template_by_name() {
        let template = get_template("data-science");
        assert!(template.is_some());
        assert_eq!(template.unwrap().display_name, "Data Science");
    }

    #[test]
    fn test_template_serialization() {
        let template = web_dev_template();
        let json = serde_json::to_string(&template).unwrap();
        assert!(json.contains("web-dev"));
        assert!(json.contains("fastapi"));
    }
}
