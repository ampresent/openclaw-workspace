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
) -> Result<Json<RollbackResponse>, String> {
    // If specific target provided, switch to that generation
    if let Some(target) = req.target {
        let profile = format!("/nix/var/nix/profiles/system-{}-link", target);
        let output = run_cmd(
            "nixos-rebuild",
            &["switch", "--profile", &profile],
        )
        .await
        .or_else(|_| {
            // Fallback: directly activate the generation
            tokio::runtime::Handle::current().block_on(async {
                run_cmd(
                    &format!("{}/bin/switch-to-configuration", profile),
                    &["switch"],
                )
                .await
            })
        });

        match output {
            Ok(_) => {
                return Ok(Json(RollbackResponse {
                    success: true,
                    reverted_to: target,
                    summary: format!("已回滚到 generation {target}"),
                }));
            }
            Err(e) => {
                return Err(format!("rollback failed: {e}"));
            }
        }
    }

    // No target: rollback to previous
    let output = run_cmd("nixos-rebuild", &["switch", "--rollback"])
        .await
        .map_err(|e| format!("rollback failed: {e}"))?;

    // Get current generation after rollback
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
