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
) -> Result<Json<ApplyResponse>, AppError> {
    // If config provided, write it first
    if let Some(config_content) = &req.config {
        if config_content.trim().is_empty() {
            return Err(AppError::Validation {
                field: "config".into(),
                message: "配置内容不能为空".into(),
            });
        }

        let target = format!("{}/configuration.nix", state.config.nixos_dir);
        // Backup current config
        let backup = format!("{target}.bak");
        let _ = tokio::fs::copy(&target, &backup).await;

        tokio::fs::write(&target, config_content).await.map_err(|e| {
            AppError::IoError {
                path: target.clone(),
                message: format!("无法写入配置文件: {e}"),
            }
        })?;
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

    let rollback_cmd = "nixos-rebuild switch --rollback".to_string();

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
