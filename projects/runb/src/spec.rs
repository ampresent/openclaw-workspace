use std::path::PathBuf;
use anyhow::{Result, anyhow};

/// Minimal OCI runtime spec types — only what we need for chroot-only mode.

#[derive(Debug, serde::Deserialize)]
pub struct Spec {
    pub root: Root,
    pub process: Process,
    #[serde(default)]
    pub mounts: Vec<Mount>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Root {
    pub path: String,
    #[serde(default)]
    pub readonly: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct Process {
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default = "default_cwd")]
    pub cwd: String,
    #[serde(default)]
    pub terminal: bool,
}

fn default_cwd() -> String {
    "/".to_string()
}

#[derive(Debug, serde::Deserialize)]
pub struct Mount {
    pub destination: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(rename = "type")]
    pub mount_type: Option<String>,
    #[serde(default)]
    pub options: Vec<String>,
}

pub fn load_spec(bundle_dir: &PathBuf) -> Result<Spec> {
    let config_path = bundle_dir.join("config.json");
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| anyhow!("Failed to read {}: {}", config_path.display(), e))?;
    let spec: Spec = serde_json::from_str(&content)
        .map_err(|e| anyhow!("Failed to parse config.json: {}", e))?;
    Ok(spec)
}
