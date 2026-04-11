use axum::Json;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Deserialize)]
pub struct PackageQuery {
    pub host: Option<String>,
    pub name: String,
}

#[derive(Serialize)]
pub struct PackageResponse {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub path: Option<String>,
}

pub async fn handle(
    State(_state): AppStateRef,
    Query(query): Query<PackageQuery>,
) -> Result<Json<PackageResponse>, String> {
    // Try nix-env first, fallback to nix-store
    let output = run_cmd("nix-env", &["-qa", &format!("{}.*", query.name), "--json"])
        .await
        .or_else(|_| {
            // Try nix search
            tokio::runtime::Handle::current().block_on(async {
                run_cmd("nix", &["search", "nixpkgs", &query.name, "--json"]).await
            })
        })
        .unwrap_or_default();

    // Also check if it's installed
    let installed = run_cmd(
        "nix-store",
        &["-qR", "/run/current-system"],
    )
    .await
    .unwrap_or_default();

    let path = installed
        .lines()
        .find(|l| l.contains(&query.name))
        .map(|l| l.to_string());

    Ok(Json(PackageResponse {
        name: query.name,
        version: extract_version(&output),
        description: extract_description(&output),
        path,
    }))
}

fn extract_version(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    // Try to find version in first entry
    v.as_object()
        .and_then(|obj| obj.values().next())
        .and_then(|v| v.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn extract_description(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    v.as_object()
        .and_then(|obj| obj.values().next())
        .and_then(|v| v.get("description").or_else(|| v.get("meta").and_then(|m| m.get("description"))))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}
