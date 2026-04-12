use axum::Json;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Deserialize)]
pub struct ConfigQuery {
    pub host: Option<String>,
    pub path: Option<String>,
}

#[derive(Serialize)]
pub struct ConfigResponse {
    pub path: String,
    pub content: String,
}

pub async fn handle(
    State(state): AppStateRef,
    Query(query): Query<ConfigQuery>,
) -> Result<Json<ConfigResponse>, AppError> {
    let path = query
        .path
        .unwrap_or_else(|| format!("{}/configuration.nix", state.config.nixos_dir));

    // Validate path: must be absolute and under /etc/nixos
    if !path.starts_with("/etc/nixos/") && path != format!("{}/configuration.nix", state.config.nixos_dir) {
        return Err(AppError::Validation {
            field: "path".into(),
            message: "配置文件路径必须在 /etc/nixos/ 下".into(),
        });
    }

    let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
        AppError::IoError {
            path: path.clone(),
            message: format!("无法读取: {e}"),
        }
    })?;

    Ok(Json(ConfigResponse { path, content }))
}
