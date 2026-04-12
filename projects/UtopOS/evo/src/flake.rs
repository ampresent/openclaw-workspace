use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::cmd::{run_cmd, AppStateRef};
use crate::error::AppError;

/// Request to convert configuration.nix to flake.nix
#[derive(Debug, Deserialize)]
pub struct FlakeConvertRequest {
    /// Optional: specific NixOS channel, e.g. "nixos-24.05"
    pub channel: Option<String>,
    /// Optional: host name for the NixOS configuration
    pub hostname: Option<String>,
    /// Optional: provide configuration.nix content directly (otherwise reads from disk)
    pub config_content: Option<String>,
    /// Extra flake inputs (e.g., {"home-manager": "github:nix-community/home-manager"})
    pub extra_inputs: Option<std::collections::HashMap<String, String>>,
}

/// Result of flake conversion
#[derive(Debug, Serialize)]
pub struct FlakeConvertResult {
    pub flake_nix: String,
    pub detected_channel: String,
    pub detected_hostname: String,
    pub detected_modules: Vec<String>,
    pub detected_inputs: Vec<String>,
    pub warnings: Vec<String>,
}

/// Analyze configuration.nix content to extract metadata
fn analyze_config(content: &str, hostname_override: Option<&str>) -> ConfigAnalysis {
    let mut imports = Vec::new();
    let mut services = Vec::new();
    let mut hardware_modules = Vec::new();
    let mut warnings = Vec::new();

    // Detect imports
    for line in content.lines() {
        let trimmed = line.trim();

        // import = [ ... ] entries
        if trimmed.starts_with("./hardware-configuration.nix") {
            hardware_modules.push("hardware-configuration.nix".to_string());
        }
        if trimmed.contains("home-manager") {
            imports.push("home-manager".to_string());
        }
        if trimmed.contains("nixos-hardware") {
            imports.push("nixos-hardware".to_string());
        }
    }

    // Detect services mentioned
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("services.") {
            if let Some(svc) = trimmed.split('.').nth(1) {
                let svc = svc.trim_end_matches(".enable").trim_end_matches(" =");
                if !services.contains(&svc.to_string()) {
                    services.push(svc.to_string());
                }
            }
        }
    }

    // Detect hostname
    let hostname = hostname_override.map(String::from).or_else(|| {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("networking.hostName") {
                if let Some(val) = trimmed.split('=').nth(1) {
                    let val = val.trim().trim_end_matches(';').trim_matches('"').trim_matches(''');
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
        }
        None
    });

    // Detect if using legacy channels approach
    if content.contains("<nixpkgs>") || content.contains("<nixos>") {
        warnings.push(
            "检测到 <nixpkgs> 或 <nixos> 引用 — flake 模式使用 nixpkgs input 替代".to_string(),
        );
    }

    if content.contains("system.stateVersion") == false {
        warnings.push("未检测到 system.stateVersion — flake 需要此项".to_string());
    }

    // Detect overlays
    let has_overlays = content.contains("nixpkgs.overlays") || content.contains("overlays =");

    ConfigAnalysis {
        imports,
        services,
        hardware_modules,
        hostname: hostname.unwrap_or_else(|| "nixos".to_string()),
        has_legacy_refs: content.contains("<nixpkgs>"),
        has_overlays,
        warnings,
    }
}

struct ConfigAnalysis {
    imports: Vec<String>,
    services: Vec<String>,
    hardware_modules: Vec<String>,
    hostname: String,
    has_legacy_refs: bool,
    has_overlays: bool,
    warnings: Vec<String>,
}

/// Detect NixOS channel from system
async fn detect_channel() -> Result<String, AppError> {
    // Try reading from nix-channel
    if let Ok(output) = run_cmd("nix-channel", &["--list"]).await {
        for line in output.lines() {
            if line.contains("nixos") {
                // e.g., "nixos https://nixos.org/channels/nixos-24.05"
                if let Some(ch) = line.split("channels/nixos-").last() {
                    return Ok(format!("nixos-{}", ch.trim()));
                }
                if let Some(ch) = line.split(' ').nth(1) {
                    return Ok(ch.to_string());
                }
            }
        }
    }

    // Fallback: check NIX_PATH
    if let Ok(nix_path) = std::env::var("NIX_PATH") {
        if let Some(pos) = nix_path.find("nixpkgs=") {
            let rest = &nix_path[pos + 8..];
            let end = rest.find(':').unwrap_or(rest.len());
            return Ok(rest[..end].to_string());
        }
    }

    // Default
    Ok("nixos-24.05".to_string())
}

/// Generate flake.nix content
fn generate_flake_nix(
    channel: &str,
    hostname: &str,
    analysis: &ConfigAnalysis,
    extra_inputs: &std::collections::HashMap<String, String>,
) -> String {
    let mut inputs = vec![
        format!(
            r#"    nixpkgs.url = "github:NixOS/nixpkgs/{}""#,
            if channel.contains("nixos-") || channel.contains("nixpkgs/") {
                channel.to_string()
            } else {
                format!("nixpkgs/{}", channel)
            }
        ),
    ];

    // Add detected extra inputs
    for (name, url) in extra_inputs {
        inputs.push(format!(r#"    {}.url = "{}""#, name, url));
    }

    // Add home-manager if detected
    if analysis.imports.contains(&"home-manager".to_string()) {
        if !extra_inputs.contains_key("home-manager") {
            inputs.push(format!(
                r#"    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";"#,
            ));
        }
    }

    let inputs_str = inputs.join("
");

    // Build the outputs section
    let home_manager_output = if analysis.imports.contains(&"home-manager".to_string()) {
        format!(
            r#"
      home-manager.nixosModules.home-manager {{
        home-manager.useGlobalPkgs = true;
        home-manager.useUserPackages = true;
      }}"#,
        )
    } else {
        String::new()
    };

    format!(
        r#"{{
  description = "NixOS configuration for {hostname}";

  inputs {{
{inputs_str}
  }}

  outputs = {{ self, nixpkgs, ... }}@inputs: {{
    nixosConfigurations.{hostname} = nixpkgs.lib.nixosSystem {{
      system = "x86_64-linux";
      specialArgs = {{ inherit inputs; }};
      modules = [
        ./configuration.nix{home_manager_output}
      ];
    }};
  }};
}}
"#,
        hostname = hostname,
        inputs_str = inputs_str,
        home_manager_output = home_manager_output,
    )
}

/// POST /api/flake/convert — analyze and generate flake.nix
pub async fn handle_convert(
    State(state): AppStateRef,
    Json(req): Json<FlakeConvertRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Read configuration.nix content
    let content = match &req.config_content {
        Some(c) => c.clone(),
        None => {
            let config_path = format!("{}/configuration.nix", state.config.nixos_dir);
            tokio::fs::read_to_string(&config_path).await.map_err(|e| AppError::IoError {
                path: config_path,
                message: e.to_string(),
            })?
        }
    };

    // Detect channel
    let channel = match &req.channel {
        Some(ch) => ch.clone(),
        None => detect_channel().await.unwrap_or_else(|_| "nixos-24.05".to_string()),
    };

    // Analyze configuration
    let analysis = analyze_config(&content, req.hostname.as_deref());

    // Extra inputs
    let extra_inputs = req.extra_inputs.unwrap_or_default();

    // Generate flake.nix
    let flake_nix = generate_flake_nix(&channel, &analysis.hostname, &analysis, &extra_inputs);

    // Collect all detected inputs
    let mut detected_inputs = vec!["nixpkgs".to_string()];
    for imp in &analysis.imports {
        if !detected_inputs.contains(imp) {
            detected_inputs.push(imp.clone());
        }
    }
    for name in extra_inputs.keys() {
        if !detected_inputs.contains(name) {
            detected_inputs.push(name.clone());
        }
    }

    let mut warnings = analysis.warnings.clone();
    if analysis.has_overlays {
        warnings.push("检测到 nixpkgs.overlays — 需要手动迁移到 flake input".to_string());
    }
    if !analysis.hardware_modules.is_empty() {
        warnings.push(format!(
            "检测到 {} — 确保 flake.nix 同目录下有此文件",
            analysis.hardware_modules.join(", ")
        ));
    }

    Ok(Json(serde_json::to_value(&FlakeConvertResult {
        flake_nix,
        detected_channel: channel,
        detected_hostname: analysis.hostname,
        detected_modules: analysis.imports,
        detected_inputs,
        warnings,
    }).unwrap()))
}
