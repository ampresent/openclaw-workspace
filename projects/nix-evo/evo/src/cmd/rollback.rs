use axum::Json;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Deserialize)]
pub struct RollbackRequest {
    pub host: Option<String>,
    pub target: Option<u64>,
}

#[derive(Serialize)]
pub struct RollbackResponse {
    pub success: bool,
    pub reverted_to: u64,
    pub summary: String,
}

pub async fn handle(
    State(_state): AppStateRef,
    Json(req): Json<RollbackRequest>,
) -> Result<Json<RollbackResponse>, AppError> {
    // If specific target provided, switch to that generation
    if let Some(target) = req.target {
        let profile = format!("/nix/var/nix/profiles/system-{}-link", target);

        // Verify the generation exists
        if !std::path::Path::new(&profile).exists() {
            return Err(AppError::NotFound {
                resource: format!("generation {target}"),
            });
        }

        // Try nixos-rebuild with profile, fallback to switch-to-configuration
        let result = run_cmd(
            "nixos-rebuild",
            &["switch", "--profile", &profile],
        )
        .await
        .or_else(|_| {
            let switch_cmd = format!("{}/bin/switch-to-configuration", profile);
            // Use block_in_place for sync command in async context
            tokio::task::block_in_place(|| {
                std::process::Command::new(&switch_cmd)
                    .arg("switch")
                    .output()
                    .map_err(|e| AppError::CommandFailed {
                        command: switch_cmd,
                        message: format!("无法执行回滚: {e}"),
                    })
                    .and_then(|output| {
                        if output.status.success() {
                            Ok(String::from_utf8_lossy(&output.stdout).to_string())
                        } else {
                            Err(AppError::CommandFailed {
                                command: switch_cmd,
                                message: format!(
                                    "退出码 {}: {}",
                                    output.status,
                                    String::from_utf8_lossy(&output.stderr).trim()
                                ),
                            })
                        }
                    })
            })
        });

        match result {
            Ok(_) => {
                return Ok(Json(RollbackResponse {
                    success: true,
                    reverted_to: target,
                    summary: format!("已回滚到 generation {target}"),
                }));
            }
            Err(e) => return Err(e),
        }
    }

    // No target: rollback to previous generation
    let output = run_cmd("nixos-rebuild", &["switch", "--rollback"])
        .await
        .map_err(|e| AppError::CommandFailed {
            command: "nixos-rebuild".into(),
            message: format!("回滚失败: {e}"),
        })?;

    let current = get_generation_after_rollback().await.unwrap_or(0);

    Ok(Json(RollbackResponse {
        success: true,
        reverted_to: current,
        summary: format!("已回滚到 generation {current}"),
    }))
}

async fn get_generation_after_rollback() -> Option<u64> {
    let link = std::fs::read_link("/nix/var/nix/profiles/system").ok()?;
    let name = link.file_name()?.to_str()?;
    // system-42-link -> 42
    name.strip_prefix("system-")?
        .strip_suffix("-link")?
        .parse()
        .ok()
}
