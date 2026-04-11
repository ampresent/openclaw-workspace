use axum::{
    extract::Request,
    extract::State,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use crate::AppState;

/// Bearer token auth middleware.
/// If `api_token` is not configured, all requests pass through.
/// If configured, requests must include `Authorization: Bearer <token>`.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let expected_token = match &state.config.api_token {
        Some(t) => t,
        None => return Ok(next.run(req).await),
    };

    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(val) if val.starts_with("Bearer ") => {
            let token = &val[7..];
            if token == expected_token {
                Ok(next.run(req).await)
            } else {
                tracing::warn!("Invalid API token attempt");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => {
            tracing::warn!("Missing Authorization header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
