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
        // Auto-detect AI config from OpenClaw if available
        let mut config = EvoConfig::default();
        if let Some(ai) = detect_openclaw_ai_config() {
            config.ai = Some(ai);
            // Persist detected config
            if let Err(e) = save_config(root, &config) {
                eprintln!("warning: could not save auto-detected config: {}", e);
            }
        }
        Ok(config)
    }
}

/// Save config to .evo/config.toml
pub fn save_config(root: &Path, config: &EvoConfig) -> Result<()> {
    let evo_dir = root.join(".evo");
    std::fs::create_dir_all(&evo_dir)?;
    let config_path = evo_dir.join("config.toml");
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&config_path, &content)?;
    Ok(())
}

/// Try to detect AI config from OpenClaw installation.
///
/// Reads ~/.openclaw/openclaw.json and extracts model/base_url
/// if a compatible provider is configured. Does NOT read or expose
/// the API key — user must set EVO_AI_API_KEY separately.
fn detect_openclaw_ai_config() -> Option<crate::cmd::ai::AiConfig> {
    let home = std::env::var("HOME").ok()?;
    let oc_config = std::path::PathBuf::from(&home)
        .join(".openclaw")
        .join("openclaw.json");

    if !oc_config.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&oc_config).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    // Extract model info from OpenClaw config
    let model = json.get("model")
        .and_then(|v| v.as_str())
        .or_else(|| json.get("default_model").and_then(|v| v.as_str()))?;

    // Determine base_url from provider
    let base_url = json.get("base_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| infer_base_url(model));

    // Skip if no meaningful model found
    if model.is_empty() {
        return None;
    }

    let mut ai = crate::cmd::ai::AiConfig::default();
    ai.model = model.to_string();
    ai.base_url = base_url;
    // Do NOT copy api_key — user sets EVO_AI_API_KEY env var
    ai.api_key = None;
    ai.api_key_env = Some("EVO_AI_API_KEY".to_string());

    Some(ai)
}

/// Infer API base URL from model name patterns
fn infer_base_url(model: &str) -> String {
    if model.contains("mimo") {
        "https://api.xiaomimimo.com/v1".to_string()
    } else if model.contains("gpt") || model.contains("o1") || model.contains("o3") {
        "https://api.openai.com/v1".to_string()
    } else if model.contains("claude") {
        "https://api.anthropic.com/v1".to_string()
    } else if model.contains("deepseek") {
        "https://api.deepseek.com/v1".to_string()
    } else {
        "https://api.openai.com/v1".to_string()
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
