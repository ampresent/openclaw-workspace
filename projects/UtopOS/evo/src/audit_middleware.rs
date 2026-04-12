/// Audit middleware — automatically logs every API call
///
/// Add to the router as middleware to capture all requests/responses.

use axum::{
    body::Body,
    extract::State,
    http::Request,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use std::time::Instant;

use crate::audit::{self, AuditEntry};
use crate::AppState;

/// Audit middleware function
///
/// Logs: timestamp, method, path, query params hash, status code, duration
pub async fn audit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let query = request.uri().query().unwrap_or("").to_string();
    let start = Instant::now();

    // Extract client IP from headers or connection info
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Call the handler
    let response = next.run(request).await;

    let duration_ms = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();

    // Determine result string
    let result = if status < 400 {
        "success".to_string()
    } else if status < 500 {
        format!("client_error_{status}")
    } else {
        format!("server_error_{status}")
    };

    // Log the audit entry
    audit::log_api_call(
        &method,
        &path,
        &query,
        &result,
        &client_ip,
        duration_ms,
    );

    response
}
