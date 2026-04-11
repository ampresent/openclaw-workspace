use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Resolve the Evolution OS source root directory.
///
/// Priority: --root flag > EVO_ROOT env > /opt/evo
pub fn resolve_root(explicit: Option<&str>) -> Result<PathBuf> {
    if let Some(r) = explicit {
        return Ok(PathBuf::from(r));
    }
    if let Ok(env) = std::env::var("EVO_ROOT") {
        return Ok(PathBuf::from(env));
    }
    // Default for server deployments
    let default = PathBuf::from("/opt/evo");
    if default.exists() {
        return Ok(default);
    }
    // Dev fallback: current directory
    let cwd = std::env::current_dir()?;
    if cwd.join("src").exists() && cwd.join("patches").exists() {
        return Ok(cwd);
    }
    bail!(
        "cannot determine Evolution OS root. Use --root, set EVO_ROOT, or run from source tree"
    );
}

/// Top-level evo.toml config
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct EvoConfig {
    pub rocky_version: String,
    pub frozen: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai: Option<crate::cmd::ai::AiConfig>,
}

impl Default for EvoConfig {
    fn default() -> Self {
        Self {
            rocky_version: "9".into(),
            frozen: false,
            ai: None,
        }
    }
}

/// Status snapshot for `evo status --json`
#[derive(Debug, Serialize, Deserialize)]
pub struct EvoStatus {
    pub packages: Vec<PackageStatus>,
    pub frozen: bool,
    pub building: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageStatus {
    pub name: String,
    pub patches: usize,
    pub modified: bool,
}

#[allow(dead_code)]
pub fn load_config(root: &Path) -> Result<EvoConfig> {
    let config_path = root.join(".evo").join("config.toml");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        Ok(toml::from_str(&content)?)
    } else {
        Ok(EvoConfig::default())
    }
}

pub fn load_status(root: &Path) -> Result<EvoStatus> {
    // TODO: actually scan the source tree
    Ok(EvoStatus {
        packages: vec![],
        frozen: root.join(".evo").join("frozen").exists(),
        building: vec![],
    })
}
