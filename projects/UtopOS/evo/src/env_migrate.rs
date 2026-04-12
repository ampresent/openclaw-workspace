//! Environment Migration Assistant
//!
//! Migrate between environment formats and tools:
//! - conda ↔ micromamba
//! - pip → conda (detect pip-only packages, find conda equivalents)
//! - requirements.txt → environment.yml → conda-lock.yml

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::cmd::{AppStateRef, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Migration source type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationSource {
    #[serde(rename = "conda")]
    Conda,
    #[serde(rename = "micromamba")]
    Micromamba,
    #[serde(rename = "pip")]
    Pip,
    #[serde(rename = "requirements.txt")]
    RequirementsTxt,
    #[serde(rename = "environment.yml")]
    EnvironmentYml,
    #[serde(rename = "conda-lock.yml")]
    CondaLockYml,
}

/// Migration target type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationTarget {
    #[serde(rename = "conda")]
    Conda,
    #[serde(rename = "micromamba")]
    Micromamba,
    #[serde(rename = "environment.yml")]
    EnvironmentYml,
    #[serde(rename = "conda-lock.yml")]
    CondaLockYml,
    #[serde(rename = "requirements.txt")]
    RequirementsTxt,
}

/// Migration request
#[derive(Debug, Clone, Deserialize)]
pub struct MigrateRequest {
    pub source: MigrationSource,
    pub target: MigrationTarget,
    pub env_name: Option<String>,
    pub file_path: Option<String>,
    pub dry_run: Option<bool>,
    pub pip_packages: Option<Vec<String>>,
}

/// Migration result
#[derive(Debug, Clone, Serialize)]
pub struct MigrateResult {
    pub source: MigrationSource,
    pub target: MigrationTarget,
    pub success: bool,
    pub packages_found: usize,
    pub packages_migrated: usize,
    pub pip_only_packages: Vec<PipOnlyPackage>,
    pub conda_equivalents_found: usize,
    pub output_content: Option<String>,
    pub commands_executed: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// A package found only in pip, with conda equivalent lookup
#[derive(Debug, Clone, Serialize)]
pub struct PipOnlyPackage {
    pub pip_name: String,
    pub pip_version: String,
    pub conda_name: Option<String>,
    pub conda_available: bool,
    pub install_command: Option<String>,
}

// ─── Migration Engine ─────────────────────────────────────────────────

/// Execute a migration
pub async fn migrate(request: &MigrateRequest) -> Result<MigrateResult, AppError> {
    match (&request.source, &request.target) {
        (MigrationSource::Conda, MigrationTarget::Micromamba) => {
            migrate_conda_to_micromamba(request).await
        }
        (MigrationSource::Micromamba, MigrationTarget::Conda) => {
            migrate_micromamba_to_conda(request).await
        }
        (MigrationSource::Pip, MigrationTarget::Conda) => {
            migrate_pip_to_conda(request).await
        }
        (MigrationSource::Pip, MigrationTarget::Micromamba) => {
            migrate_pip_to_micromamba(request).await
        }
        (MigrationSource::RequirementsTxt, MigrationTarget::EnvironmentYml) => {
            migrate_requirements_to_yml(request).await
        }
        (MigrationSource::EnvironmentYml, MigrationTarget::CondaLockYml) => {
            migrate_yml_to_conda_lock(request).await
        }
        (MigrationSource::RequirementsTxt, MigrationTarget::CondaLockYml) => {
            migrate_requirements_to_conda_lock(request).await
        }
        (MigrationSource::EnvironmentYml, MigrationTarget::RequirementsTxt) => {
            migrate_yml_to_requirements(request).await
        }
        _ => Err(AppError::Validation {
            field: "source/target".to_string(),
            message: format!(
                "Unsupported migration: {:?} → {:?}",
                request.source, request.target
            ),
        }),
    }
}

/// Migrate from conda to micromamba
async fn migrate_conda_to_micromamba(request: &MigrateRequest) -> Result<MigrateResult, AppError> {
    let env_name = request.env_name.as_deref().ok_or_else(|| AppError::Validation {
        field: "env_name".to_string(),
        message: "env_name required for conda → micromamba migration".to_string(),
    })?;

    // Export env from conda
    let yml_content = run_cmd("conda", &["env", "export", "-n", env_name, "--no-builds"]).await?;
    let mut commands = Vec::new();
    let mut warnings = Vec::new();
    let dry_run = request.dry_run.unwrap_or(false);

    // Parse package count
    let pkg_count = yml_content.lines().filter(|l| l.trim().starts_with("- ")).count();

    if !dry_run {
        // Create with micromamba
        let tmp_file = format!("/tmp/{env_name}-migration.yml");
        tokio::fs::write(&tmp_file, &yml_content).await.map_err(|e| AppError::IoError {
            path: tmp_file.clone(),
            message: e.to_string(),
        })?;

        let create_cmd = format!("micromamba env create -f {tmp_file} -y");
        commands.push(create_cmd.clone());
        match run_cmd("micromamba", &["env", "create", "-f", &tmp_file, "-y"]).await {
            Ok(_) => warnings.push("Environment created with micromamba. Remove old conda env manually.".to_string()),
            Err(e) => {
                return Ok(MigrateResult {
                    source: MigrationSource::Conda,
                    target: MigrationTarget::Micromamba,
                    success: false,
                    packages_found: pkg_count,
                    packages_migrated: 0,
                    pip_only_packages: vec![],
                    conda_equivalents_found: 0,
                    output_content: Some(yml_content),
                    commands_executed: commands,
                    warnings,
                    errors: vec![format!("micromamba create failed: {e}")],
                });
            }
        }
    } else {
        let cmd = format!("micromamba env create -f <exported.yml> -y");
        commands.push(cmd);
        warnings.push("Dry run — no changes made".to_string());
    }

    Ok(MigrateResult {
        source: MigrationSource::Conda,
        target: MigrationTarget::Micromamba,
        success: true,
        packages_found: pkg_count,
        packages_migrated: pkg_count,
        pip_only_packages: vec![],
        conda_equivalents_found: 0,
        output_content: Some(yml_content),
        commands_executed: commands,
        warnings,
        errors: vec![],
    })
}

/// Migrate from micromamba to conda
async fn migrate_micromamba_to_conda(request: &MigrateRequest) -> Result<MigrateResult, AppError> {
    let env_name = request.env_name.as_deref().ok_or_else(|| AppError::Validation {
        field: "env_name".to_string(),
        message: "env_name required for micromamba → conda migration".to_string(),
    })?;

    let yml_content = run_cmd("micromamba", &["env", "export", "-n", env_name, "--no-builds"]).await?;
    let pkg_count = yml_content.lines().filter(|l| l.trim().starts_with("- ")).count();
    let dry_run = request.dry_run.unwrap_or(false);
    let mut commands = Vec::new();
    let mut warnings = Vec::new();

    if !dry_run {
        let tmp_file = format!("/tmp/{env_name}-migration.yml");
        tokio::fs::write(&tmp_file, &yml_content).await.map_err(|e| AppError::IoError {
            path: tmp_file.clone(),
            message: e.to_string(),
        })?;

        let create_cmd = format!("conda env create -f {tmp_file} -y");
        commands.push(create_cmd);
        match run_cmd("conda", &["env", "create", "-f", &tmp_file, "-y"]).await {
            Ok(_) => warnings.push("Environment created with conda. Remove old micromamba env manually.".to_string()),
            Err(e) => {
                return Ok(MigrateResult {
                    source: MigrationSource::Micromamba,
                    target: MigrationTarget::Conda,
                    success: false,
                    packages_found: pkg_count,
                    packages_migrated: 0,
                    pip_only_packages: vec![],
                    conda_equivalents_found: 0,
                    output_content: Some(yml_content),
                    commands_executed: commands,
                    warnings,
                    errors: vec![format!("conda create failed: {e}")],
                });
            }
        }
    } else {
        commands.push("conda env create -f <exported.yml> -y".to_string());
        warnings.push("Dry run — no changes made".to_string());
    }

    Ok(MigrateResult {
        source: MigrationSource::Micromamba,
        target: MigrationTarget::Conda,
        success: true,
        packages_found: pkg_count,
        packages_migrated: pkg_count,
        pip_only_packages: vec![],
        conda_equivalents_found: 0,
        output_content: Some(yml_content),
        commands_executed: commands,
        warnings,
        errors: vec![],
    })
}

/// Migrate pip environment to conda
async fn migrate_pip_to_conda(request: &MigrateRequest) -> Result<MigrateResult, AppError> {
    let env_name = request.env_name.as_deref().unwrap_or("migrated-from-pip");
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut pip_only = Vec::new();
    let mut commands = Vec::new();
    let dry_run = request.dry_run.unwrap_or(false);

    // Get pip packages (from virtualenv or from provided list)
    let pip_packages = if let Some(pkgs) = &request.pip_packages {
        pkgs.clone()
    } else {
        // Try to get from active pip
        match run_cmd("pip", &["list", "--format=json"]).await {
            Ok(output) => {
                let pkgs: Vec<serde_json::Value> = serde_json::from_str(&output).unwrap_or_default();
                pkgs.iter()
                    .filter_map(|p| {
                        let name = p.get("name")?.as_str()?;
                        let version = p.get("version")?.as_str()?;
                        Some(format!("{name}=={version}"))
                    })
                    .collect()
            }
            Err(e) => {
                errors.push(format!("Could not list pip packages: {e}"));
                vec![]
            }
        }
    };

    let total_pip = pip_packages.len();

    // For each pip package, check if available in conda-forge
    let mut conda_deps = Vec::new();
    let mut equiv_count = 0;

    for pkg_spec in &pip_packages {
        let pkg_name = pkg_spec.split("==").next().unwrap_or(pkg_spec).split(">=").next().unwrap_or(pkg_spec).trim();

        // Check conda search
        match run_cmd("micromamba", &["search", "-c", "conda-forge", pkg_name, "--json"]).await {
            Ok(_) => {
                // Found in conda-forge
                equiv_count += 1;
                if let Some(version) = pkg_spec.split("==").nth(1) {
                    conda_deps.push(format!("{pkg_name}={version}"));
                } else {
                    conda_deps.push(pkg_name.to_string());
                }
            }
            Err(_) => {
                // Not in conda-forge — pip-only
                let version = pkg_spec.split("==").nth(1).unwrap_or("*");
                pip_only.push(PipOnlyPackage {
                    pip_name: pkg_name.to_string(),
                    pip_version: version.to_string(),
                    conda_name: None,
                    conda_available: false,
                    install_command: Some(format!("pip install {pkg_spec}")),
                });
            }
        }
    }

    // Generate environment.yml
    let mut yml_lines = vec![
        format!("name: {env_name}"),
        "channels:".to_string(),
        "  - conda-forge".to_string(),
        "  - defaults".to_string(),
        "dependencies:".to_string(),
    ];
    for dep in &conda_deps {
        yml_lines.push(format!("  - {dep}"));
    }
    if !pip_only.is_empty() {
        yml_lines.push("  - pip:".to_string());
        for p in &pip_only {
            yml_lines.push(format!("    - {}=={}", p.pip_name, p.pip_version));
        }
    }
    let yml_content = yml_lines.join("\n");

    if !pip_only.is_empty() {
        warnings.push(format!(
            "{} packages are pip-only (not in conda-forge) and will be installed via pip section",
            pip_only.len()
        ));
    }

    if !dry_run {
        let tmp_file = format!("/tmp/{env_name}-from-pip.yml");
        tokio::fs::write(&tmp_file, &yml_content).await.map_err(|e| AppError::IoError {
            path: tmp_file.clone(),
            message: e.to_string(),
        })?;
        let cmd = format!("micromamba env create -f {tmp_file} -y");
        commands.push(cmd);
    } else {
        commands.push(format!("micromamba env create -f <generated.yml> -n {env_name} -y"));
        warnings.push("Dry run — environment.yml generated but not created".to_string());
    }

    Ok(MigrateResult {
        source: MigrationSource::Pip,
        target: MigrationTarget::Conda,
        success: true,
        packages_found: total_pip,
        packages_migrated: equiv_count,
        pip_only_packages: pip_only,
        conda_equivalents_found: equiv_count,
        output_content: Some(yml_content),
        commands_executed: commands,
        warnings,
        errors,
    })
}

/// Migrate pip environment to micromamba
async fn migrate_pip_to_micromamba(request: &MigrateRequest) -> Result<MigrateResult, AppError> {
    let mut req = request.clone();
    req.target = MigrationTarget::Conda;
    let mut result = migrate_pip_to_conda(&req).await?;
    result.target = MigrationTarget::Micromamba;

    // Update commands to use micromamba
    result.commands_executed = result.commands_executed
        .into_iter()
        .map(|c| c.replace("conda", "micromamba"))
        .collect();

    Ok(result)
}

/// Migrate requirements.txt to environment.yml
async fn migrate_requirements_to_yml(request: &MigrateRequest) -> Result<MigrateResult, AppError> {
    let file_path = request.file_path.as_deref().unwrap_or("requirements.txt");
    let env_name = request.env_name.as_deref().unwrap_or("migrated");

    let content = tokio::fs::read_to_string(file_path).await.map_err(|e| AppError::IoError {
        path: file_path.to_string(),
        message: e.to_string(),
    })?;

    let mut pip_deps = Vec::new();
    let mut conda_deps = Vec::new();
    let mut pip_only = Vec::new();
    let mut equiv_count = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
            continue;
        }

        let pkg_name = trimmed.split("==").next().unwrap_or(trimmed)
            .split(">=").next().unwrap_or(trimmed)
            .split("<=").next().unwrap_or(trimmed)
            .split("!=").next().unwrap_or(trimmed)
            .trim();

        if pkg_name.is_empty() {
            continue;
        }

        // Check conda availability
        match run_cmd("micromamba", &["search", "-c", "conda-forge", pkg_name, "--json"]).await {
            Ok(_) => {
                equiv_count += 1;
                if let Some(version) = trimmed.split("==").nth(1) {
                    conda_deps.push(format!("{pkg_name}={version}"));
                } else {
                    conda_deps.push(pkg_name.to_string());
                }
            }
            Err(_) => {
                pip_deps.push(trimmed.to_string());
                let version = trimmed.split("==").nth(1).unwrap_or("*");
                pip_only.push(PipOnlyPackage {
                    pip_name: pkg_name.to_string(),
                    pip_version: version.to_string(),
                    conda_name: None,
                    conda_available: false,
                    install_command: Some(format!("pip install {trimmed}")),
                });
            }
        }
    }

    let total = conda_deps.len() + pip_deps.len();

    // Build environment.yml
    let mut yml_lines = vec![
        format!("name: {env_name}"),
        "channels:".to_string(),
        "  - conda-forge".to_string(),
        "  - defaults".to_string(),
        "dependencies:".to_string(),
    ];
    for dep in &conda_deps {
        yml_lines.push(format!("  - {dep}"));
    }
    if !pip_deps.is_empty() {
        yml_lines.push("  - pip:".to_string());
        for p in &pip_deps {
            yml_lines.push(format!("    - {p}"));
        }
    }
    let yml_content = yml_lines.join("\n");

    let mut warnings = Vec::new();
    if !pip_only.is_empty() {
        warnings.push(format!(
            "{} packages are pip-only and kept in pip section",
            pip_only.len()
        ));
    }

    Ok(MigrateResult {
        source: MigrationSource::RequirementsTxt,
        target: MigrationTarget::EnvironmentYml,
        success: true,
        packages_found: total,
        packages_migrated: equiv_count,
        pip_only_packages: pip_only,
        conda_equivalents_found: equiv_count,
        output_content: Some(yml_content),
        commands_executed: vec![format!("micromamba env create -f <generated.yml> -n {env_name} -y")],
        warnings,
        errors: vec![],
    })
}

/// Migrate environment.yml to conda-lock.yml
async fn migrate_yml_to_conda_lock(request: &MigrateRequest) -> Result<MigrateResult, AppError> {
    let file_path = request.file_path.as_deref().unwrap_or("environment.yml");
    let env_name = request.env_name.as_deref().unwrap_or("locked-env");

    let content = tokio::fs::read_to_string(file_path).await.map_err(|e| AppError::IoError {
        path: file_path.to_string(),
        message: e.to_string(),
    })?;

    let yml = conda::parse_environment_yml(&content)?;
    let pkg_count = yml.dependencies.len();
    let dry_run = request.dry_run.unwrap_or(false);
    let mut commands = Vec::new();
    let mut warnings = Vec::new();

    let platforms = vec!["linux-64", "osx-arm64", "osx-64", "win-64"];
    let plat_args: Vec<&str> = platforms.iter().copied().collect();

    let mut lock_args = vec!["lock", "--file", file_path];
    for p in &plat_args {
        lock_args.push("-p");
        lock_args.push(p);
    }

    if !dry_run {
        match run_cmd("conda-lock", &lock_args).await {
            Ok(output) => {
                commands.push(format!("conda-lock lock --file {file_path} {}", 
                    platforms.iter().map(|p| format!("-p {p}")).collect::<Vec<_>>().join(" ")));
                Ok(MigrateResult {
                    source: MigrationSource::EnvironmentYml,
                    target: MigrationTarget::CondaLockYml,
                    success: true,
                    packages_found: pkg_count,
                    packages_migrated: pkg_count,
                    pip_only_packages: vec![],
                    conda_equivalents_found: 0,
                    output_content: Some(output),
                    commands_executed: commands,
                    warnings,
                    errors: vec![],
                })
            }
            Err(e) => {
                warnings.push(format!("conda-lock not available: {e}. Install with: micromamba install -c conda-forge conda-lock"));
                commands.push("conda-lock lock --file environment.yml -p linux-64".to_string());
                Ok(MigrateResult {
                    source: MigrationSource::EnvironmentYml,
                    target: MigrationTarget::CondaLockYml,
                    success: false,
                    packages_found: pkg_count,
                    packages_migrated: 0,
                    pip_only_packages: vec![],
                    conda_equivalents_found: 0,
                    output_content: None,
                    commands_executed: commands,
                    warnings,
                    errors: vec![format!("conda-lock execution failed: {e}")],
                })
            }
        }
    } else {
        commands.push(format!("conda-lock lock --file {file_path} {}", 
            platforms.iter().map(|p| format!("-p {p}")).collect::<Vec<_>>().join(" ")));
        warnings.push("Dry run".to_string());
        Ok(MigrateResult {
            source: MigrationSource::EnvironmentYml,
            target: MigrationTarget::CondaLockYml,
            success: true,
            packages_found: pkg_count,
            packages_migrated: 0,
            pip_only_packages: vec![],
            conda_equivalents_found: 0,
            output_content: None,
            commands_executed: commands,
            warnings,
            errors: vec![],
        })
    }
}

/// Migrate requirements.txt to conda-lock.yml (two-step: → yml → lock)
async fn migrate_requirements_to_conda_lock(request: &MigrateRequest) -> Result<MigrateResult, AppError> {
    // Step 1: requirements.txt → environment.yml
    let yml_result = migrate_requirements_to_yml(request).await?;

    if !yml_result.success {
        return Ok(yml_result);
    }

    // Step 2: write yml and lock it
    let env_name = request.env_name.as_deref().unwrap_or("locked-env");
    let tmp_yml = format!("/tmp/{env_name}-from-reqs.yml");
    if let Some(content) = &yml_result.output_content {
        tokio::fs::write(&tmp_yml, content).await.map_err(|e| AppError::IoError {
            path: tmp_yml.clone(),
            message: e.to_string(),
        })?;
    }

    let lock_request = MigrateRequest {
        source: MigrationSource::EnvironmentYml,
        target: MigrationTarget::CondaLockYml,
        env_name: Some(env_name.to_string()),
        file_path: Some(tmp_yml),
        dry_run: request.dry_run,
        pip_packages: None,
    };

    let mut lock_result = migrate_yml_to_conda_lock(&lock_request).await?;
    // Merge commands
    let mut all_commands = yml_result.commands_executed;
    all_commands.extend(lock_result.commands_executed);
    lock_result.commands_executed = all_commands;
    lock_result.source = MigrationSource::RequirementsTxt;
    Ok(lock_result)
}

/// Migrate environment.yml to requirements.txt
async fn migrate_yml_to_requirements(request: &MigrateRequest) -> Result<MigrateResult, AppError> {
    let file_path = request.file_path.as_deref().unwrap_or("environment.yml");

    let content = tokio::fs::read_to_string(file_path).await.map_err(|e| AppError::IoError {
        path: file_path.to_string(),
        message: e.to_string(),
    })?;

    let yml = conda::parse_environment_yml(&content)?;

    let mut reqs = Vec::new();
    let mut pkg_count = 0;

    for dep in &yml.dependencies {
        match dep {
            conda::EnvDependency::Conda(spec) => {
                // Convert conda spec to pip spec: python=3.11 → python==3.11
                let pip_spec = spec.replace("=", "==");
                // Remove channel specs (e.g. conda-forge::numpy)
                let clean = if let Some(pos) = pip_spec.find("::") {
                    &pip_spec[pos + 2..]
                } else {
                    &pip_spec
                };
                // Skip python itself in requirements
                if !clean.starts_with("python==") && !clean.starts_with("python>=") {
                    reqs.push(clean.to_string());
                    pkg_count += 1;
                }
            }
            conda::EnvDependency::Pip { pip } => {
                for p in pip {
                    reqs.push(p.clone());
                    pkg_count += 1;
                }
            }
        }
    }

    let header = format!(
        "# requirements.txt — migrated from {}\n# Generated by nix-evo\n\n",
        yml.name
    );
    let output = format!("{header}{}\n", reqs.join("\n"));

    Ok(MigrateResult {
        source: MigrationSource::EnvironmentYml,
        target: MigrationTarget::RequirementsTxt,
        success: true,
        packages_found: pkg_count,
        packages_migrated: pkg_count,
        pip_only_packages: vec![],
        conda_equivalents_found: 0,
        output_content: Some(output),
        commands_executed: vec![format!("pip install -r requirements.txt")],
        warnings: vec![],
        errors: vec![],
    })
}

// ─── HTTP Handlers ────────────────────────────────────────────────────

/// POST /api/env/migrate
pub async fn migrate_handler(
    State(_state): AppStateRef,
    Json(body): Json<MigrateRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = migrate(&body).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pip_only_package_serialization() {
        let pkg = PipOnlyPackage {
            pip_name: "some-lib".to_string(),
            pip_version: "1.0.0".to_string(),
            conda_name: None,
            conda_available: false,
            install_command: Some("pip install some-lib==1.0.0".to_string()),
        };
        let json = serde_json::to_string(&pkg).unwrap();
        assert!(json.contains("some-lib"));
        assert!(json.contains("1.0.0"));
    }

    #[test]
    fn test_migrate_result_defaults() {
        let result = MigrateResult {
            source: MigrationSource::Pip,
            target: MigrationTarget::Conda,
            success: true,
            packages_found: 10,
            packages_migrated: 8,
            pip_only_packages: vec![],
            conda_equivalents_found: 8,
            output_content: Some("name: test\n".to_string()),
            commands_executed: vec![],
            warnings: vec![],
            errors: vec![],
        };
        assert!(result.success);
        assert_eq!(result.packages_migrated, 8);
    }
}
