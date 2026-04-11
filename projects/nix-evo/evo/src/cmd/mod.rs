pub mod system_snapshot;
pub mod service_logs;
pub mod config_read;
pub mod package_info;
pub mod generation_diff;
pub mod config_validate;
pub mod config_apply;
pub mod rollback;
pub mod conda_handlers;

use axum::extract::{Query, State};
use serde::Deserialize;
use std::sync::Arc;
use crate::AppState;
use crate::error::AppError;

/// Common host query parameter
#[derive(Deserialize)]
pub struct HostQuery {
    pub host: Option<String>,
}

/// Helper: run a shell command and return stdout
pub async fn run_cmd(cmd: &str, args: &[&str]) -> Result<String, AppError> {
    let output = tokio::process::Command::new(cmd)
        .args(args)
        .output()
        .await
        .map_err(|e| AppError::CommandFailed {
            command: cmd.to_string(),
            message: format!("无法执行: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::CommandFailed {
            command: cmd.to_string(),
            message: format!("退出码 {}: {}", output.status, stderr.trim()),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// App state type alias for handlers
pub type AppStateRef = State<Arc<AppState>>;

/// Helper: read generation descriptions
pub fn read_generation_description(gen_num: u64) -> String {
    let desc_path = format!(
        "/nix/var/nix/profiles/system-{}-link/nix-evo-description",
        gen_num
    );
    std::fs::read_to_string(&desc_path)
        .unwrap_or_default()
        .trim()
        .to_string()
}
