use axum::Json;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Deserialize)]
pub struct ApplyRequest {
    pub host: Option<String>,
    pub config: Option<String>,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct ApplyResponse {
    pub success: bool,
    pub generation: Option<u64>,
    pub summary: String,
    pub rollback_command: String,
}

pub async fn handle(
    State(state): AppStateRef,
    Json(req): Json<ApplyRequest>,
) -> Result<Json<ApplyResponse>, String> {
    // If config provided, write it first
    if let Some(config_content) = &req.config {
        let target = format!("{}/configuration.nix", state.config.nixos_dir);
        // Backup current config
        let backup = format!("{target}.bak");
        let _ = tokio::fs::copy(&target, &backup).await;

        tokio::fs::write(&target, config_content)
            .await
            .map_err(|e| format!("failed to write config: {e}"))?;
    }

    // Run nixos-rebuild switch
    let output = run_cmd("nixos-rebuild", &["switch", "--fast"])
        .await
        .unwrap_or_else(|e| format!("rebuild failed: {e}"));

    let success = !output.contains("error:");

    // Get current generation number
    let generation = get_current_generation().await;

    // Record generation description if message provided
    if let (Some(gen), Some(msg)) = (generation, &req.message) {
        let desc_path = format!("/nix/var/nix/profiles/system-{}-link/nix-evo-description", gen);
        let _ = tokio::fs::write(&desc_path, msg).await;
    }

    let prev_gen = generation.map(|g| g.saturating_sub(1)).unwrap_or(0);
    let rollback_cmd = format!("nixos-rebuild switch --rollback");

    let summary = if success {
        if let Some(gen) = generation {
            format!("配置已生效，generation {gen}")
        } else {
            "配置已生效".to_string()
        }
    } else {
        format!("rebuild 失败: {output}")
    };

    Ok(Json(ApplyResponse {
        success,
        generation,
        summary,
        rollback_command: rollback_cmd,
    }))
}

async fn get_current_generation() -> Option<u64> {
    let output = run_cmd(
        "nixos-rebuild",
        &["list-generations", "--no-pager"],
    )
    .await
    .ok()?;

    output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.first()?.parse::<u64>().ok()
        })
        .max()
}
