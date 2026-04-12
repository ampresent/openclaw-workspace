//! Concurrent request limiter middleware.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Shared limiter state
pub struct ConcurrencyLimiter {
    active: AtomicU64,
    max: u64,
}

impl ConcurrencyLimiter {
    pub fn new(max_concurrent: u64) -> Self {
        Self {
            active: AtomicU64::new(0),
            max: max_concurrent,
        }
    }
}

/// Create the concurrency limiting middleware.
///
/// When the number of concurrent requests exceeds `max`,
/// returns 429 Too Many Requests.
pub fn concurrency_limit(
    limiter: Arc<ConcurrencyLimiter>,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> + Clone {
    move |req: Request, next: Next| {
        let limiter = limiter.clone();
        Box::pin(async move {
            let current = limiter.active.fetch_add(1, Ordering::Relaxed);
            if current >= limiter.max {
                limiter.active.fetch_sub(1, Ordering::Relaxed);
                tracing::warn!(
                    current = current,
                    max = limiter.max,
                    "concurrency limit reached"
                );
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    axum::Json(serde_json::json!({
                        "error": {
                            "code": "TOO_MANY_REQUESTS",
                            "message": format!("并发请求过多 ({}/{}), 请稍后重试", current, limiter.max)
                        }
                    })),
                )
                    .into_response();
            }

            let response = next.run(req).await;
            limiter.active.fetch_sub(1, Ordering::Relaxed);
            response
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limiter_creation() {
        let limiter = ConcurrencyLimiter::new(10);
        assert_eq!(limiter.active.load(Ordering::Relaxed), 0);
        assert_eq!(limiter.max, 10);
    }

    #[test]
    fn test_limiter_increment_decrement() {
        let limiter = ConcurrencyLimiter::new(5);
        limiter.active.fetch_add(1, Ordering::Relaxed);
        assert_eq!(limiter.active.load(Ordering::Relaxed), 1);
        limiter.active.fetch_sub(1, Ordering::Relaxed);
        assert_eq!(limiter.active.load(Ordering::Relaxed), 0);
    }
}
