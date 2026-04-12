use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;

/// API version info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiVersion {
    pub version: String,
    pub status: String,       // "stable", "beta", "deprecated", "removed"
    pub prefix: String,       // e.g., "/api/v1"
    pub released: String,
    pub sunset: Option<String>, // deprecation date
}

/// API stability guarantee level
#[derive(Debug, Clone, Serialize)]
pub struct StabilityGuarantee {
    pub level: String,
    pub description: String,
    pub breaking_changes_policy: String,
    pub deprecation_notice_days: u64,
}

/// Version registry
pub struct ApiVersionRegistry {
    pub versions: Vec<ApiVersion>,
    pub current: String,
}

impl ApiVersionRegistry {
    pub fn new() -> Self {
        Self {
            current: "v1".into(),
            versions: vec![
                ApiVersion {
                    version: "v1".into(),
                    status: "stable".into(),
                    prefix: "/api/v1".into(),
                    released: "2026-04-01".into(),
                    sunset: None,
                },
                ApiVersion {
                    version: "v2".into(),
                    status: "beta".into(),
                    prefix: "/api/v2".into(),
                    released: "2026-04-12".into(),
                    sunset: None,
                },
            ],
        }
    }

    pub fn get(&self, version: &str) -> Option<&ApiVersion> {
        self.versions.iter().find(|v| v.version == version)
    }

    pub fn is_supported(&self, version: &str) -> bool {
        self.get(version).map(|v| v.status != "removed").unwrap_or(false)
    }

    pub fn is_deprecated(&self, version: &str) -> bool {
        self.get(version).map(|v| v.status == "deprecated").unwrap_or(false)
    }
}

/// Version extraction from request path
/// Supports: /api/v1/..., /api/v2/..., or unversioned /api/...
pub fn extract_version(path: &str) -> (Option<String>, String) {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 3 && parts[1] == "api" {
        let potential_version = parts[2];
        if potential_version.starts_with('v') && potential_version[1..].parse::<u32>().is_ok() {
            let remaining = format!("/{}", parts[3..].join("/"));
            return (Some(potential_version.to_string()), remaining);
        }
    }
    (None, path.to_string())
}

/// Add API version headers to response
pub async fn version_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let (version, _) = extract_version(&path);

    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Always add API version headers
    headers.insert(
        "X-API-Version",
        "v1".parse().unwrap(),
    );
    headers.insert(
        "X-API-Current-Version",
        "v1".parse().unwrap(),
    );
    headers.insert(
        "X-API-Latest-Version",
        "v2".parse().unwrap(),
    );

    // Deprecation warning
    if let Some(ref ver) = version {
        let registry = ApiVersionRegistry::new();
        if registry.is_deprecated(ver) {
            headers.insert(
                "Deprecation",
                "true".parse().unwrap(),
            );
            headers.insert(
                "Sunset",
                registry.get(ver)
                    .and_then(|v| v.sunset.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("Tue, 01 Jul 2026 00:00:00 GMT")
                    .parse()
                    .unwrap(),
            );
            headers.insert(
                "X-API-Deprecation-Warning",
                format!("{ver} 已弃用，请迁移到 v1。详见 /api/versions").parse().unwrap(),
            );
        }
    }

    response
}

/// GET /api/versions — list all API versions and their status
pub async fn list_versions() -> axum::Json<serde_json::Value> {
    let registry = ApiVersionRegistry::new();

    let stability = StabilityGuarantee {
        level: "stable".into(),
        description: "稳定版本 API 保证向后兼容".into(),
        breaking_changes_policy: "仅在主版本号变更时允许破坏性变更，提前 90 天发布弃用通知".into(),
        deprecation_notice_days: 90,
    };

    axum::Json(serde_json::json!({
        "current_version": registry.current,
        "versions": registry.versions,
        "stability": stability,
        "versioning_policy": {
            "url_scheme": "/api/{version}/endpoint",
            "unversioned": "未带版本号的请求默认路由到当前稳定版本",
            "header_versioning": "也可以通过 Accept-Version 请求头指定版本",
            "deprecation_headers": [
                "Deprecation: true",
                "Sunset: <date>",
                "X-API-Deprecation-Warning: <message>"
            ],
        },
        "endpoints_by_version": {
            "v1": {
                "status": "stable",
                "endpoints": [
                    "GET  /api/v1/snapshot",
                    "GET  /api/v1/logs",
                    "GET  /api/v1/config",
                    "GET  /api/v1/package",
                    "GET  /api/v1/generations",
                    "POST /api/v1/config/validate",
                    "POST /api/v1/config/apply",
                    "POST /api/v1/config/diff",
                    "POST /api/v1/rollback",
                    "GET  /api/v1/backups",
                    "POST /api/v1/backup/create",
                    "POST /api/v1/backup/restore",
                    "POST /api/v1/backup/rotate",
                    "POST /api/v1/config/generate",
                    "POST /api/v1/config/test",
                    "POST /api/v1/config/test/cancel",
                    "GET  /api/v1/docker/status",
                    "POST /api/v1/docker/compose-validate",
                    "POST /api/v1/cicd/webhook",
                    "POST /api/v1/cicd/preview-deploy",
                    "GET  /api/v1/cicd/deployments",
                    "GET  /api/v1/cicd/deployments/:id",
                    "POST /api/v1/observability/logs",
                    "GET  /api/v1/observability/metrics",
                    "GET  /api/v1/observability/alerts",
                    "POST /api/v1/observability/alerts/check",
                    "POST /api/v1/observability/alerts/rules",
                    "GET  /api/v1/observability/config",
                    "POST /api/v1/dev/mode",
                    "GET  /api/v1/dev/status",
                    "POST /api/v1/dev/mock/service",
                    "POST /api/v1/dev/mock/generation",
                    "POST /api/v1/dev/mock/reset",
                    "GET  /api/v1/dev/mock/snapshot",
                ],
            },
            "v2": {
                "status": "beta",
                "changes_from_v1": [
                    "Multi-host support (host header required)",
                    "Batch operations endpoint",
                    "WebSocket streaming for logs",
                    "Config templates and presets",
                ],
                "planned_stable": "2026-07-01",
            },
        },
    }))
}

/// Check if a request path matches a versioned or unversioned API path
pub fn route_matches_version(path: &str, version: &str) -> bool {
    let (req_version, _) = extract_version(path);
    match req_version {
        Some(v) => v == version,
        None => version == "v1", // unversioned routes to v1
    }
}
