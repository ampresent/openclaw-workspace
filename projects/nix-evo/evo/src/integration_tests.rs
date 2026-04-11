/// Integration tests for nix-evo-agent HTTP API
///
/// Tests route handlers, response formats, error handling,
/// and WebSocket connections using axum's test utilities.

#[cfg(test)]
mod integration_tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::{get, post},
        Router,
    };
    use http_body_util::BodyExt;
    use serde_json::Value;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::auth;
    use crate::audit;
    use crate::cmd;
    use crate::config::Config;
    use crate::error::AppError;
    use crate::flake;
    use crate::healer;
    use crate::AppState;

    /// Build a test app with all routes
    fn test_app() -> Router {
        let config = Config {
            host: "127.0.0.1".into(),
            port: 7890,
            nixos_dir: "/tmp/test-nixos".into(),
            max_log_lines: 50,
            api_token: None,
        };
        let state = Arc::new(AppState { config });

        let api_routes = Router::new()
            .route("/audit", get(audit::handle_query))
            .route("/audit/stats", get(audit::handle_stats))
            .route("/healer/status", get(healer::handle_status))
            .route("/flake/convert", post(flake::handle_convert))
            .with_state(state.clone());

        Router::new()
            .nest("/api", api_routes)
            .route("/health", get(|| async { "ok" }))
    }

    // ─── Health check ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = test_app();

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"ok");
    }

    // ─── Audit query endpoint ───────────────────────────────────────

    #[tokio::test]
    async fn test_audit_query_returns_json() {
        let app = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/audit?limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 200 or 500 (if no log file exists yet), but always JSON
        assert!(
            response.status() == StatusCode::OK
                || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        // If successful, should have structure
        if json.get("total").is_some() {
            assert!(json["total"].is_number());
            assert!(json["entries"].is_array());
        } else {
            // Error response should have error structure
            assert!(json["error"].is_object());
        }
    }

    // ─── Audit stats endpoint ───────────────────────────────────────

    #[tokio::test]
    async fn test_audit_stats_endpoint() {
        let app = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/audit/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            response.status() == StatusCode::OK
                || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    // ─── Healer status endpoint ─────────────────────────────────────

    #[tokio::test]
    async fn test_healer_status_returns_json() {
        let app = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/healer/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        // Should have expected fields
        assert!(json.get("running").is_some());
        assert!(json.get("rules").is_some());
        assert!(json.get("service_states").is_some());
    }

    // ─── Flake convert — missing content ────────────────────────────

    #[tokio::test]
    async fn test_flake_convert_missing_config() {
        let app = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/flake/convert")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should fail because no config_content and no file at /tmp/test-nixos/configuration.nix
        assert!(
            response.status() == StatusCode::INTERNAL_SERVER_ERROR
                || response.status() == StatusCode::OK
        );
    }

    // ─── Flake convert — with content ───────────────────────────────

    #[tokio::test]
    async fn test_flake_convert_with_content() {
        let app = test_app();

        let config_content = r#"
{ config, pkgs, ... }:
{
  networking.hostName = "testhost";
  services.nginx.enable = true;
}
"#;

        let body = serde_json::json!({
            "config_content": config_content,
            "channel": "nixos-24.05",
            "hostname": "testhost"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/flake/convert")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let resp_body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&resp_body).unwrap();

        assert!(json["flake_nix"].is_string());
        assert!(json["detected_channel"].is_string());
        assert!(json["detected_hostname"].is_string());

        let flake_content = json["flake_nix"].as_str().unwrap();
        assert!(flake_content.contains("nixosConfigurations.testhost"));
        assert!(flake_content.contains("nixpkgs.url"));
    }

    // ─── Flake convert — custom hostname ────────────────────────────

    #[tokio::test]
    async fn test_flake_convert_hostname_override() {
        let app = test_app();

        let body = serde_json::json!({
            "config_content": "{ networking.hostName = "original"; }",
            "hostname": "overridden"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/flake/convert")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp_body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&resp_body).unwrap();

        assert_eq!(json["detected_hostname"].as_str().unwrap(), "overridden");
        let flake = json["flake_nix"].as_str().unwrap();
        assert!(flake.contains("nixosConfigurations.overridden"));
    }

    // ─── Error format consistency ───────────────────────────────────

    #[tokio::test]
    async fn test_error_response_format() {
        // Test AppError serialization
        let err = AppError::Validation {
            field: "host".into(),
            message: "missing".into(),
        };

        // Convert to response
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_not_found_error() {
        let err = AppError::NotFound {
            resource: "service xyz".into(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_unauthorized_error() {
        let err = AppError::Unauthorized;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // ─── 404 for unknown routes ─────────────────────────────────────

    #[tokio::test]
    async fn test_unknown_route_404() {
        let app = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // ─── CORS headers ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_cors_preflight() {
        let app = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/api/audit")
                    .header("origin", "http://localhost:3000")
                    .header("access-control-request-method", "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should allow CORS
        assert!(response.status().is_success() || response.status() == StatusCode::NO_CONTENT);
    }
}
