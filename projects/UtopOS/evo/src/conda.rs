//! micromamba CLI wrapper module
//!
//! Wraps common micromamba/conda commands into structured Rust types.
//! Provides environment management: create, install, remove, list, export/import.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::cmd::run_cmd;
use crate::error::AppError;

/// Represents a conda/micromamba environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CondaEnv {
    pub name: String,
    pub path: String,
    pub is_active: bool,
    pub python_version: Option<String>,
    pub package_count: Option<usize>,
}

/// A single installed package in a conda environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CondaPackage {
    pub name: String,
    pub version: String,
    pub build: String,
    pub channel: String,
    pub platform: Option<String>,
}

/// Represents an environment.yml declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentYml {
    pub name: String,
    pub channels: Vec<String>,
    pub dependencies: Vec<EnvDependency>,
}

/// A dependency in environment.yml
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnvDependency {
    Conda(String),
    Pip { pip: Vec<String> },
}

/// Result of environment creation
#[derive(Debug, Clone, Serialize)]
pub struct CreateResult {
    pub name: String,
    pub path: String,
    pub success: bool,
    pub packages_installed: usize,
}

/// Result of package installation
#[derive(Debug, Clone, Serialize)]
pub struct InstallResult {
    pub packages: Vec<String>,
    pub environment: String,
    pub success: bool,
    pub changed: bool,
}

/// Detect whether micromamba or conda is available
pub async fn detect_backend() -> Result<String, AppError> {
    if let Ok(output) = run_cmd("micromamba", &["--version"]).await {
        tracing::info!("Detected micromamba {}", output.trim());
        return Ok("micromamba".to_string());
    }
    if let Ok(output) = run_cmd("conda", &["--version"]).await {
        tracing::info!("Detected conda {}", output.trim());
        return Ok("conda".to_string());
    }
    Err(AppError::CommandFailed {
        command: "micromamba/conda".to_string(),
        message: "No conda-compatible backend found. Install micromamba or conda.".to_string(),
    })
}

/// List all conda/micromamba environments
pub async fn list_envs(backend: &str) -> Result<Vec<CondaEnv>, AppError> {
    let output = run_cmd(backend, &["env", "list"]).await?;
    Ok(parse_env_list(&output))
}

pub fn parse_env_list(output: &str) -> Vec<CondaEnv> {
    let mut envs = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.contains("Name")
            || trimmed.contains("---")
        {
            continue;
        }
        let is_active = trimmed.starts_with('*');
        let clean = if is_active {
            trimmed.trim_start_matches('*').trim()
        } else {
            trimmed
        };
        let parts: Vec<&str> = clean.split_whitespace().collect();
        if parts.len() >= 2 {
            let has_parens = parts.last().unwrap().contains('(');
            let end = if has_parens { parts.len() - 1 } else { parts.len() };
            let path = parts[1..end].join(" ");
            envs.push(CondaEnv {
                name: parts[0].to_string(),
                path,
                is_active,
                python_version: None,
                package_count: None,
            });
        }
    }
    envs
}

/// List packages in a specific environment
pub async fn list_packages(backend: &str, env_name: &str) -> Result<Vec<CondaPackage>, AppError> {
    let output = run_cmd(backend, &["list", "-n", env_name, "--json"]).await
        .or_else(|_| run_cmd(backend, &["list", "-n", env_name]).await)?;

    if let Ok(json_pkgs) = serde_json::from_str::<Vec<serde_json::Value>>(&output) {
        return Ok(json_pkgs.into_iter().filter_map(|pkg| {
            Some(CondaPackage {
                name: pkg.get("name")?.as_str()?.to_string(),
                version: pkg.get("version")?.as_str()?.to_string(),
                build: pkg.get("build_string").and_then(|b| b.as_str())
                    .or_else(|| pkg.get("build").and_then(|b| b.as_str()))
                    .unwrap_or("").to_string(),
                channel: pkg.get("channel").and_then(|c| c.as_str())
                    .unwrap_or("unknown").to_string(),
                platform: pkg.get("platform").and_then(|p| p.as_str()).map(String::from),
            })
        }).collect());
    }

    Ok(parse_list_output(&output))
}

pub fn parse_list_output(output: &str) -> Vec<CondaPackage> {
    let mut packages = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.contains("Name")
            || trimmed.contains("===")
            || trimmed.contains("---")
        {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 3 {
            packages.push(CondaPackage {
                name: parts[0].to_string(),
                version: parts[1].to_string(),
                build: parts[2].to_string(),
                channel: if parts.len() > 3 { parts[3..].join(" ") } else { "defaults".to_string() },
                platform: None,
            });
        }
    }
    packages
}

/// Create a new conda environment
pub async fn create_env(
    backend: &str,
    name: &str,
    python_version: Option<&str>,
    packages: Option<&[&str]>,
) -> Result<CreateResult, AppError> {
    let mut args: Vec<&str> = vec!["create", "-n", name, "-y"];
    if let Some(py_ver) = python_version {
        args.push("python");
        args.push(py_ver);
    }
    if let Some(pkgs) = packages {
        for pkg in pkgs {
            args.push(pkg);
        }
    }

    let output = run_cmd(backend, &args).await;
    match output {
        Ok(out) => {
            let installed = out.lines()
                .filter(|l| l.contains("Installing") || l.contains("Download"))
                .count();
            let env_path = get_env_path(backend, name).await.unwrap_or_default();
            Ok(CreateResult {
                name: name.to_string(),
                path: env_path,
                success: true,
                packages_installed: installed,
            })
        }
        Err(_) => Ok(CreateResult {
            name: name.to_string(),
            path: String::new(),
            success: false,
            packages_installed: 0,
        }),
    }
}

/// Install packages into an existing environment
pub async fn install_packages(
    backend: &str,
    env_name: &str,
    packages: &[&str],
) -> Result<InstallResult, AppError> {
    let mut args = vec!["install", "-n", env_name, "-y", "--json"];
    for pkg in packages {
        args.push(pkg);
    }
    match run_cmd(backend, &args).await {
        Ok(out) => Ok(InstallResult {
            packages: packages.iter().map(|s| s.to_string()).collect(),
            environment: env_name.to_string(),
            success: true,
            changed: !out.trim().is_empty() && !out.contains("\"actions\": []"),
        }),
        Err(_) => Ok(InstallResult {
            packages: packages.iter().map(|s| s.to_string()).collect(),
            environment: env_name.to_string(),
            success: false,
            changed: false,
        }),
    }
}

/// Remove packages from an environment
pub async fn remove_packages(
    backend: &str,
    env_name: &str,
    packages: &[&str],
) -> Result<InstallResult, AppError> {
    let mut args = vec!["remove", "-n", env_name, "-y", "--json"];
    for pkg in packages {
        args.push(pkg);
    }
    match run_cmd(backend, &args).await {
        Ok(_) => Ok(InstallResult {
            packages: packages.iter().map(|s| s.to_string()).collect(),
            environment: env_name.to_string(),
            success: true,
            changed: true,
        }),
        Err(_) => Ok(InstallResult {
            packages: packages.iter().map(|s| s.to_string()).collect(),
            environment: env_name.to_string(),
            success: false,
            changed: false,
        }),
    }
}

/// Export an environment to environment.yml
pub async fn export_env(backend: &str, env_name: &str) -> Result<String, AppError> {
    run_cmd(backend, &["env", "export", "-n", env_name, "--no-builds"]).await
}

/// Export environment with explicit URLs
pub async fn export_env_explicit(backend: &str, env_name: &str) -> Result<String, AppError> {
    run_cmd(backend, &["env", "export", "-n", env_name, "--explicit"]).await
}

/// Create environment from environment.yml
pub async fn create_from_yml(backend: &str, yml_path: &str) -> Result<CreateResult, AppError> {
    match run_cmd(backend, &["env", "create", "-f", yml_path, "-y", "--json"]).await {
        Ok(_) => {
            let name = std::fs::read_to_string(yml_path)
                .ok()
                .and_then(|c| parse_yml_name(&c))
                .unwrap_or_else(|| "unknown".to_string());
            let env_path = get_env_path(backend, &name).await.unwrap_or_default();
            Ok(CreateResult { name, path: env_path, success: true, packages_installed: 0 })
        }
        Err(_) => Ok(CreateResult {
            name: "unknown".to_string(),
            path: String::new(),
            success: false,
            packages_installed: 0,
        }),
    }
}

/// Remove an entire environment
pub async fn remove_env(backend: &str, env_name: &str) -> Result<bool, AppError> {
    if env_name == "base" {
        return Err(AppError::Validation {
            field: "name".to_string(),
            message: "Cannot remove the base environment".to_string(),
        });
    }
    run_cmd(backend, &["env", "remove", "-n", env_name, "-y"]).await?;
    Ok(true)
}

/// Parse environment.yml content into structured data
pub fn parse_environment_yml(content: &str) -> Result<EnvironmentYml, AppError> {
    let mut name = String::new();
    let mut channels = Vec::new();
    let mut dependencies = Vec::new();
    let mut in_deps = false;
    let mut in_pip = false;
    let mut pip_packages = Vec::new();
    let mut indent_level = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        let leading_spaces = line.len() - line.trim_start().len();

        if trimmed.starts_with("name:") {
            name = trimmed.trim_start_matches("name:").trim().trim_matches('\'').trim_matches('"').to_string();
        } else if trimmed == "channels:" {
            indent_level = leading_spaces;
            in_deps = false;
            in_pip = false;
        } else if trimmed == "dependencies:" {
            indent_level = leading_spaces;
            in_deps = true;
            in_pip = false;
        } else if trimmed == "- pip:" && in_deps {
            in_pip = true;
            pip_packages.clear();
        } else if trimmed.starts_with("- ") {
            let dep_str = trimmed.trim_start_matches("- ").trim().trim_matches('\'').trim_matches('"').to_string();
            if in_pip {
                pip_packages.push(dep_str);
            } else if in_deps {
                dependencies.push(EnvDependency::Conda(dep_str));
            } else {
                // In channels section
                if !dep_str.is_empty() {
                    channels.push(dep_str);
                }
            }
        } else if in_pip && !trimmed.starts_with('-') && trimmed != "- pip:" {
            if !pip_packages.is_empty() {
                dependencies.push(EnvDependency::Pip { pip: pip_packages.clone() });
            }
            in_pip = false;
        }
    }

    if in_pip && !pip_packages.is_empty() {
        dependencies.push(EnvDependency::Pip { pip: pip_packages });
    }

    Ok(EnvironmentYml { name, channels, dependencies })
}

fn parse_yml_name(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name:") {
            return Some(trimmed.trim_start_matches("name:").trim().to_string());
        }
    }
    None
}

async fn get_env_path(backend: &str, env_name: &str) -> Result<String, AppError> {
    let envs = list_envs(backend).await?;
    envs.into_iter()
        .find(|e| e.name == env_name)
        .map(|e| e.path)
        .ok_or_else(|| AppError::NotFound {
            resource: format!("conda environment: {env_name}"),
        })
}

/// Get environment info as name -> path map
pub async fn env_info_map(backend: &str) -> Result<HashMap<String, String>, AppError> {
    let envs = list_envs(backend).await?;
    Ok(envs.into_iter().map(|e| (e.name, e.path)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_list() {
        let input = "Name           Path\n\
                     base           /opt/micromamba\n\
                     ml-project     /opt/micromamba/envs/ml-project\n\
                     datasci        /opt/micromamba/envs/datasci\n";
        let envs = parse_env_list(input);
        assert_eq!(envs.len(), 3);
        assert_eq!(envs[0].name, "base");
        assert_eq!(envs[1].name, "ml-project");
        assert_eq!(envs[2].path, "/opt/micromamba/envs/datasci");
    }

    #[test]
    fn test_parse_env_list_with_active() {
        let input = "Name           Path\n\
                     base           /opt/micromamba\n\
                     * datasci      /opt/micromamba/envs/datasci\n";
        let envs = parse_env_list(input);
        assert_eq!(envs.len(), 2);
        assert!(!envs[0].is_active);
        assert!(envs[1].is_active);
    }

    #[test]
    fn test_parse_list_output() {
        let input = "# Name    Version  Build     Channel\n\
                     python    3.11.7   h955ad1f_0  pkgs/main\n\
                     numpy     1.26.3   py311h5d0b8f6_0  pkgs/main\n\
                     pandas    2.1.4    py311h955ad1f_0  pkgs/main\n";
        let pkgs = parse_list_output(input);
        assert_eq!(pkgs.len(), 3);
        assert_eq!(pkgs[0].name, "python");
        assert_eq!(pkgs[0].version, "3.11.7");
        assert_eq!(pkgs[1].name, "numpy");
        assert_eq!(pkgs[2].channel, "pkgs/main");
    }

    #[test]
    fn test_parse_environment_yml() {
        let input = "name: ml-env\n\
                     channels:\n\
                     - defaults\n\
                     - conda-forge\n\
                     dependencies:\n\
                     - python=3.11\n\
                     - numpy>=1.24\n\
                     - pandas\n\
                     - pip:\n\
                     - transformers\n\
                     - datasets\n";
        let yml = parse_environment_yml(input).unwrap();
        assert_eq!(yml.name, "ml-env");
        assert_eq!(yml.channels.len(), 2);
        assert_eq!(yml.channels[0], "defaults");
        assert_eq!(yml.dependencies.len(), 4);

        match &yml.dependencies[0] {
            EnvDependency::Conda(s) => assert_eq!(s, "python=3.11"),
            _ => panic!("Expected conda dep"),
        }
        match &yml.dependencies[3] {
            EnvDependency::Pip { pip } => {
                assert_eq!(pip.len(), 2);
                assert_eq!(pip[0], "transformers");
            }
            _ => panic!("Expected pip dep"),
        }
    }

    #[test]
    fn test_parse_env_list_empty() {
        let envs = parse_env_list("");
        assert!(envs.is_empty());
    }

    #[test]
    fn test_parse_env_list_skips_header() {
        let input = "#\n# Name           Path\n--------------------\nbase           /opt/micromamba\n";
        let envs = parse_env_list(input);
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].name, "base");
    }

    #[test]
    fn test_parse_env_list_with_spaces_in_path() {
        let input = "myenv    /opt/my envs/myenv\n";
        let envs = parse_env_list(input);
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].name, "myenv");
    }

    #[test]
    fn test_parse_list_output_empty() {
        let pkgs = parse_list_output("");
        assert!(pkgs.is_empty());
    }

    #[test]
    fn test_parse_list_output_skips_comments() {
        let input = "# comment\n## another\npython 3.11 h00_0 defaults\n";
        let pkgs = parse_list_output(input);
        assert_eq!(pkgs.len(), 1);
    }

    #[test]
    fn test_parse_list_output_too_few_columns() {
        let input = "python 3.11\n";
        let pkgs = parse_list_output(input);
        assert!(pkgs.is_empty());
    }

    #[test]
    fn test_parse_environment_yml_minimal() {
        let input = "name: test\nchannels:\n- defaults\ndependencies:\n- python\n";
        let yml = parse_environment_yml(input).unwrap();
        assert_eq!(yml.name, "test");
        assert_eq!(yml.dependencies.len(), 1);
    }

    #[test]
    fn test_parse_environment_yml_no_pip() {
        let input = "name: base\nchannels:\n- conda-forge\ndependencies:\n- numpy\n- pandas\n";
        let yml = parse_environment_yml(input).unwrap();
        assert_eq!(yml.dependencies.len(), 2);
        for dep in &yml.dependencies {
            assert!(matches!(dep, EnvDependency::Conda(_)));
        }
    }

    #[test]
    fn test_conda_env_debug() {
        let env = CondaEnv {
            name: "test".into(),
            path: "/opt/envs/test".into(),
            is_active: false,
            python_version: Some("3.11".into()),
            package_count: Some(42),
        };
        let debug = format!("{env:?}");
        assert!(debug.contains("test"));
        assert!(debug.contains("3.11"));
    }

    #[test]
    fn test_conda_package_debug() {
        let pkg = CondaPackage {
            name: "numpy".into(),
            version: "1.26.3".into(),
            build: "py311_0".into(),
            channel: "conda-forge".into(),
            platform: Some("linux-64".into()),
        };
        assert_eq!(pkg.name, "numpy");
        assert_eq!(pkg.version, "1.26.3");
    }
}
