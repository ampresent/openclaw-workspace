use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Subcommand)]
pub enum AiCmd {
    /// Ask AI to analyze a package's source
    Analyze(AiAnalyzeArgs),
    /// Generate a patch from a description
    Patch(AiPatchArgs),
    /// Suggest conflict resolution during rebase
    Resolve(AiResolveArgs),
    /// Show current AI config
    Config,
}

#[derive(Args)]
pub struct AiAnalyzeArgs {
    /// Package name
    package: String,

    /// Question or analysis request
    #[arg(long, short)]
    ask: Option<String>,
}

#[derive(Args)]
pub struct AiPatchArgs {
    /// Package name
    package: String,

    /// What to implement
    #[arg(long, short)]
    description: String,
}

#[derive(Args)]
pub struct AiResolveArgs {
    /// Package name
    package: String,
}

pub fn run(cmd: AiCmd, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;
    let config = crate::config::load_config(&root)?;

    match cmd {
        AiCmd::Analyze(args) => analyze(&root, &config, &args),
        AiCmd::Patch(args) => generate_patch(&root, &config, &args),
        AiCmd::Resolve(args) => resolve_conflicts(&root, &config, &args),
        AiCmd::Config => show_config(&config),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AiConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "openai-compatible".to_string(),
            base_url: "https://api.xiaomimimo.com/v1".to_string(),
            model: "mimo-v2-pro".to_string(),
            api_key_env: Some("EVO_AI_API_KEY".to_string()),
            api_key: None,
        }
    }
}

/// Call the AI model via curl (OpenAI-compatible API)
pub fn call_model(
    config: &crate::config::EvoConfig,
    system: &str,
    user: &str,
) -> Result<String> {
    
    let ai = config.ai.clone().unwrap_or_else(|| AiConfig::default());

    let api_key = resolve_api_key(&ai)?;

    let url = format!("{}/chat/completions", ai.base_url.trim_end_matches('/'));

    let request_body = serde_json::json!({
        "model": ai.model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ],
        "temperature": 0.3,
        "max_tokens": 4096
    });

    let output = Command::new("curl")
        .args([
            "-s",
            "-X", "POST",
            &url,
            "-H", &format!("Authorization: Bearer {}", api_key),
            "-H", "Content-Type: application/json",
            "-d", &request_body.to_string(),
            "--max-time", "120",
        ])
        .output()
        .context("failed to call AI model API")?;

    if !output.status.success() {
        bail!("curl failed with status: {}", output.status);
    }

    let response: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("failed to parse AI response")?;

    if let Some(error) = response.get("error") {
        bail!("AI API error: {}", error);
    }

    let content = response
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or("No response from model");

    Ok(content.to_string())
}

fn resolve_api_key(ai: &AiConfig) -> Result<String> {
    if let Some(ref key) = ai.api_key {
        return Ok(key.clone());
    }
    if let Some(ref env_var) = ai.api_key_env {
        if let Ok(key) = std::env::var(env_var) {
            return Ok(key);
        }
    }
    // Try common env vars
    for var in &["EVO_AI_API_KEY", "OPENAI_API_KEY", "AI_API_KEY"] {
        if let Ok(key) = std::env::var(var) {
            return Ok(key);
        }
    }
    bail!(
        "no API key found. Set {} env var or add api_key to .evo/config.toml",
        ai.api_key_env.as_deref().unwrap_or("EVO_AI_API_KEY")
    );
}

fn analyze(root: &Path, config: &crate::config::EvoConfig, args: &AiAnalyzeArgs) -> Result<()> {
    let src = root.join("src").join(&args.package);
    if !src.exists() {
        bail!("package '{}' not found", args.package);
    }

    // Gather context
    let patches_dir = root.join("patches").join(&args.package);
    let patches = super::util::list_patches(&patches_dir)?;
    let patch_list: Vec<String> = patches
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    // Get recent git log
    let git_log = super::util::git_output(&src, &["log", "--oneline", "-10"]).unwrap_or_default();

    // Get file list
    let file_count = super::util::count_files(&src);

    let question = args.ask.as_deref().unwrap_or("Describe the structure and key files of this package.");

    let system = "You are an expert Linux systems programmer analyzing source code packages. Be concise and technical.";
    let user = format!(
        "Analyze this package:\n\nPackage: {}\nFiles: {}\nPatches: {}\nGit log:\n{}\n\nQuestion: {}",
        args.package, file_count, patch_list.join(", "), git_log, question
    );

    println!("{} asking AI about {}...", "→".dimmed(), args.package);
    let response = call_model(config, system, &user)?;
    println!();
    println!("{}", response);
    Ok(())
}

fn generate_patch(root: &Path, config: &crate::config::EvoConfig, args: &AiPatchArgs) -> Result<()> {
    let src = root.join("src").join(&args.package);
    if !src.exists() {
        bail!("package '{}' not found", args.package);
    }

    // Get current source diff (if any)
    let diff = super::util::git_output(&src, &["diff", "HEAD"]).unwrap_or_default();
    let has_changes = !diff.is_empty();

    // Get list of source files
    let source_files = list_source_files(&src);

    let system = r#"You are an expert Linux systems programmer. Generate a unified diff patch.
Output ONLY the patch content in unified diff format (--- a/file +++ b/file @@ ...).
Do not include explanations outside the patch."#;

    let user = format!(
        "Package: {}\nTask: {}\n\nSource files:\n{}\n\n{}",
        args.package,
        args.description,
        source_files.join("\n"),
        if has_changes {
            format!("Current changes:\n{}", &diff[..diff.len().min(2000)])
        } else {
            "No pending changes.".to_string()
        }
    );

    println!("{} generating patch for {}...", "→".dimmed(), args.package);
    let response = call_model(config, system, &user)?;

    // Check if response looks like a diff
    if response.contains("--- a/") || response.contains("diff --git") {
        let patches_dir = root.join("patches").join(&args.package);
        std::fs::create_dir_all(&patches_dir)?;

        let num = super::util::next_patch_number(root, &args.package)?;
        let desc = args.description
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>();
        let filename = format!("{:04}-{}.patch", num, desc);
        let patch_path = patches_dir.join(&filename);

        std::fs::write(&patch_path, &response)?;
        println!("{} saved to {}", "✓".green(), patch_path.display());
        println!("{} review the patch before applying with `evo patch apply {}`", "⚠".yellow(), args.package);
    } else {
        // Just print the response (it's a suggestion, not a patch)
        println!("{}", response);
    }

    Ok(())
}

fn resolve_conflicts(root: &Path, config: &crate::config::EvoConfig, args: &AiResolveArgs) -> Result<()> {
    let src = root.join("src").join(&args.package);
    if !src.exists() {
        bail!("package '{}' not found", args.package);
    }

    // Check for .rej files (patch rejects)
    let rej_files = find_reject_files(&src);
    if rej_files.is_empty() {
        println!("{} no conflict files found in {}", "→".dimmed(), args.package);
        return Ok(());
    }

    let system = "You are an expert Linux systems programmer resolving merge conflicts. Provide clear, actionable suggestions.";
    let mut conflicts_content = String::new();

    for rej in &rej_files {
        if let Ok(content) = std::fs::read_to_string(rej) {
            conflicts_content.push_str(&format!("=== {} ===\n{}\n\n", rej.display(), content));
        }
    }

    let user = format!(
        "Package: {}\nConflicts found in patch application:\n\n{}\n\nSuggest how to resolve these conflicts.",
        args.package, conflicts_content
    );

    println!("{} analyzing conflicts in {}...", "→".dimmed(), args.package);
    let response = call_model(config, system, &user)?;
    println!();
    println!("{}", response);
    Ok(())
}

fn show_config(config: &crate::config::EvoConfig) -> Result<()> {
    
    let ai = config.ai.clone().unwrap_or_else(|| AiConfig::default());
    println!("AI Configuration:");
    println!("  provider:  {}", ai.provider);
    println!("  base_url:  {}", ai.base_url);
    println!("  model:     {}", ai.model);
    if let Some(ref env) = ai.api_key_env {
        let has_key = std::env::var(env).is_ok();
        println!("  api_key:   {} ({})", if has_key { "✓ set" } else { "✗ not set" }, env);
    }
    Ok(())
}

fn list_source_files(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".git" || name.starts_with('.') {
                continue;
            }
            if path.is_file() {
                files.push(name);
            } else if path.is_dir() {
                files.push(format!("{}/", name));
            }
        }
    }
    files.sort();
    files.truncate(50);
    files
}

fn find_reject_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut rej = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |e| e == "rej") {
                rej.push(path);
            } else if path.is_dir() {
                rej.extend(find_reject_files(&path));
            }
        }
    }
    rej
}
