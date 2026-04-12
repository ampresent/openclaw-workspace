use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::timeout;

use crate::error::AppError;

/// Package search result from search.nixos.org
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageEntry {
    pub name: String,
    pub pname: String,
    pub version: String,
    pub description: String,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub platforms: Vec<String>,
    pub position: Option<String>,
}

/// Detailed package info
#[derive(Debug, Clone, Serialize)]
pub struct PackageDetail {
    pub name: String,
    pub pname: String,
    pub version: String,
    pub description: String,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub maintainers: Vec<String>,
    pub dependencies: Vec<String>,
    pub size_bytes: Option<u64>,
    pub nix_config: String,
    pub nixpkgs_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub channel: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct InfoQuery {
    pub package: String,
    pub channel: Option<String>,
}

/// Search nixpkgs via search.nixos.org API
pub async fn search_packages(query: &SearchQuery) -> Result<Vec<PackageEntry>, AppError> {
    let channel = query.channel.as_deref().unwrap_or("unstable");
    let limit = query.limit.unwrap_or(20).min(100);
    let _offset = query.offset.unwrap_or(0);

    // Use the official search.nixos.org API
    let api_url = format!("https://api.nixos.org/search/v2?query={}&channel={}&type=packages&from={}&size={}",
        url_encode(&query.q), url_encode(channel), _offset, limit);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("nix-evo-agent/0.2")
        .build()
        .map_err(|e| AppError::Internal { message: e.to_string() })?;

    let resp = client
        .get(&api_url)
        .send()
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Search API request failed: {e}"),
        })?;

    if !resp.status().is_success() {
        return Err(AppError::Internal {
            message: format!("Search API returned {}", resp.status()),
        });
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| AppError::Internal {
        message: format!("Failed to parse search response: {e}"),
    })?;

    let mut entries = Vec::new();
    if let Some(results) = body.get("results").and_then(|r| r.as_array()) {
        for item in results {
            let entry = PackageEntry {
                name: item.get("attrName").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                pname: item.get("pname").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                version: item.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                description: item.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                homepage: item.get("homepage").and_then(|v| v.as_str()).map(String::from),
                license: item.get("license").and_then(|v| {
                    if let Some(arr) = v.as_array() {
                        arr.first().and_then(|l| l.get("fullName").and_then(|f| f.as_str()))
                            .or_else(|| arr.first().and_then(|l| l.get("shortName").and_then(|f| f.as_str())))
                    } else {
                        v.as_str()
                    }
                }).map(String::from),
                platforms: item.get("platforms").and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|p| p.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
                position: item.get("position").and_then(|v| v.as_str()).map(String::from),
            };
            entries.push(entry);
        }
    }

    Ok(entries)
}

/// Get detailed info about a specific package
pub async fn get_package_info(package: &str, channel: Option<&str>) -> Result<PackageDetail, AppError> {
    let channel = channel.unwrap_or("unstable");

    // Search for the exact package
    let results = search_packages(&SearchQuery {
        q: package.to_string(),
        channel: Some(channel.to_string()),
        limit: Some(5),
        offset: Some(0),
    }).await?;

    let entry = results.into_iter()
        .find(|e| e.name == package || e.pname == package)
        .unwrap_or_else(|| PackageEntry {
            name: package.to_string(),
            pname: package.to_string(),
            version: "?".to_string(),
            description: "Package info not available".to_string(),
            homepage: None,
            license: None,
            platforms: vec![],
            position: None,
        });

    // Generate nix config snippet
    let nix_config = generate_nix_config(&entry.name, &entry.version);

    // Build nixpkgs source URL
    let nixpkgs_url = entry.position.as_ref().map(|pos| {
        // position format: "pkgs/some/path/default.nix:123"
        let path = pos.split(':').next().unwrap_or(pos);
        format!("https://github.com/NixOS/nixpkgs/blob/nixos-{channel}/{path}")
    });

    Ok(PackageDetail {
        name: entry.name.clone(),
        pname: entry.pname,
        version: entry.version,
        description: entry.description,
        homepage: entry.homepage,
        license: entry.license,
        maintainers: vec![],
        dependencies: vec![],
        size_bytes: None,
        nix_config,
        nixpkgs_url,
    })
}

/// Generate a ready-to-use nix configuration snippet
fn generate_nix_config(name: &str, version: &str) -> String {
    format!(
        r#"# Add to configuration.nix environment.systemPackages:
environment.systemPackages = with pkgs; [
  {name}
];

# Or in a flake.nix nixosConfiguration:
environment.systemPackages = with pkgs; [
  {name}  # version: {version}
];

# Home Manager variant:
home.packages = with pkgs; [
  {name}
];
"#,
        name = name,
        version = version,
    )
}

fn url_encode(s: &str) -> String {
    s.replace('%', "%25")
        .replace(' ', "%20")
        .replace('?', "%3F")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('+', "%2B")
}

// ─── HTTP Handlers ─────────────────────────────────────────────────────

/// GET /api/marketplace/search?q=nginx
pub async fn handle_search(Query(query): Query<SearchQuery>) -> Result<Json<serde_json::Value>, AppError> {
    let results = search_packages(&query).await?;
    Ok(Json(serde_json::json!({
        "query": query.q,
        "channel": query.channel.unwrap_or_else(|| "unstable".into()),
        "count": results.len(),
        "packages": results,
    })))
}

/// GET /api/marketplace/info?package=nginx
pub async fn handle_info(Query(query): Query<InfoQuery>) -> Result<Json<PackageDetail>, AppError> {
    let detail = get_package_info(&query.package, query.channel.as_deref()).await?;
    Ok(Json(detail))
}
