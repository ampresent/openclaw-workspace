pub mod system_snapshot;
pub mod service_logs;
pub mod config_read;
pub mod package_info;
pub mod generation_diff;
pub mod config_validate;
pub mod config_apply;
pub mod rollback;

use axum::extract::{Query, State};
use serde::Deserialize;
use std::sync::Arc;
use crate::AppState;

/// Common host query parameter
#[derive(Deserialize)]
pub struct HostQuery {
    pub host: Option<String>,
}

/// Helper: run a shell command and return stdout
pub async fn run_cmd(cmd: &str, args: &[&str]) -> anyhow::Result<String> {
    let output = tokio::process::Command::new(cmd)
        .args(args)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("failed to run {cmd}: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{cmd} failed (exit {}): {stderr}", output.status);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// App state type alias for handlers
pub type AppStateRef = State<Arc<AppState>>;
