//! Request tracing middleware.
//!
//! Adds X-Request-Id header and structured request logging.

use axum::{
    extract::Request,
    http::HeaderValue,
    middleware::Next,
    response::Response,
};

/// Request tracing middleware.
/// - Assigns X-Request-Id if not present
/// - Logs request method, path, status, duration
pub async fn request_tracing(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = std::time::Instant::now();

    // Get or generate request ID
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            format!("nixevo-{ts:x}")
        });

    let span = tracing::info_span!(
        "request",
        method = %method,
        path = %uri.path(),
        req_id = %request_id,
    );

    let _guard = span.enter();

    let mut response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status();

    // Add request ID to response
    if let Ok(val) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", val);
    }

    if status.is_server_error() {
        tracing::error!(status = %status, duration_ms = %duration.as_millis(), "request completed with error");
    } else if status.is_client_error() {
        tracing::warn!(status = %status, duration_ms = %duration.as_millis(), "request completed with client error");
    } else {
        tracing::info!(status = %status, duration_ms = %duration.as_millis(), "request completed");
    }

    response
}
