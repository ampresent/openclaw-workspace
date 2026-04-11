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
) -> Result<Json<ConfigResponse>, String> {
    let path = query
        .path
        .unwrap_or_else(|| format!("{}/configuration.nix", state.config.nixos_dir));

    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("failed to read {path}: {e}"))?;

    Ok(Json(ConfigResponse { path, content }))
}
