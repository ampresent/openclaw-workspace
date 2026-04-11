//! Test-before-switch endpoint.
//!
//! Runs `nixos-rebuild test` first (doesn't modify bootloader, reversible by reboot),
//! then optionally auto-switches after a delay if no issues detected.

use axum::Json;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};
use super::*;
use crate::AppState;

#[derive(Deserialize)]
pub struct TestRequest {
    pub host: Option<String>,
    pub config: Option<String>,
    pub message: Option<String>,
    /// Seconds to wait before auto-switching. Default 300 (5 min). 0 = no auto-switch.
    #[serde(default = "default_wait")]
    pub wait_seconds: u64,
}

fn default_wait() -> u64 { 300 }

#[derive(Serialize)]
pub struct TestResponse {
    pub success: bool,
    pub test_generation: Option<u64>,
    pub switched: bool,
    pub switch_generation: Option<u64>,
    pub summary: String,
    pub status: String,  // "testing", "switched", "failed"
}

/// State tracking for active tests
static ACTIVE_TEST: std::sync::OnceLock<std::sync::Arc<tokio::sync::RwLock<Option<TestState>>>> = std::sync::OnceLock::new();

struct TestState {
    started_at: u64,
    auto_switch_at: u64,
    cancelled: bool,
}

pub async fn handle(
    State(state): State<std::sync::Arc<AppState>>,
    Json(req): Json<TestRequest>,
) -> Result<Json<TestResponse>, AppError> {
    // Write config if provided
    if let Some(config_content) = &req.config {
        if config_content.trim().is_empty() {
            return Err(AppError::Validation {
                field: "config".into(),
                message: "配置内容不能为空".into(),
            });
        }

        let target = format!("{}/configuration.nix", state.config.nixos_dir);
        let backup = format!("{target}.bak");
        let _ = tokio::fs::copy(&target, &backup).await;

        tokio::fs::write(&target, config_content).await.map_err(|e| {
            AppError::IoError {
                path: target.clone(),
                message: format!("无法写入配置文件: {e}"),
            }
        })?;
    }

    // Run nixos-rebuild test
    let test_output = run_cmd("nixos-rebuild", &["test", "--fast"]).await;
    let test_success = match &test_output {
        Ok(o) => !o.contains("error:"),
        Err(_) => false,
    };

    if !test_success {
        let err_msg = test_output.unwrap_or_else(|e| format!("{e}"));
        return Ok(Json(TestResponse {
            success: false,
            test_generation: None,
            switched: false,
            switch_generation: None,
            summary: format!("nixos-rebuild test 失败: {err_msg}"),
            status: "failed".to_string(),
        }));
    }

    let test_gen = get_current_generation().await;

    // Record test generation description
    if let (Some(gen), Some(msg)) = (test_gen, &req.message) {
        let desc_path = format!(
            "/nix/var/nix/profiles/system-{}-link/nix-evo-description",
            gen
        );
        let _ = tokio::fs::write(&desc_path, format!("[TEST] {msg}")).await;
    }

    // If no auto-switch requested, return immediately
    if req.wait_seconds == 0 {
        return Ok(Json(TestResponse {
            success: true,
            test_generation: test_gen,
            switched: false,
            switch_generation: None,
            summary: "测试已生效（未设置自动切换）。重启将恢复到之前的配置。".to_string(),
            status: "testing".to_string(),
        }));
    }

    // Auto-switch after delay
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let switch_at = now + req.wait_seconds;

    // Store active test state
    let test_state = ACTIVE_TEST.get_or_init(|| {
        std::sync::Arc::new(tokio::sync::RwLock::new(None))
    });
    {
        let mut guard = test_state.write().await;
        *guard = Some(TestState {
            started_at: now,
            auto_switch_at: switch_at,
            cancelled: false,
        });
    }

    // Wait
    sleep(Duration::from_secs(req.wait_seconds)).await;

    // Check if cancelled
    let cancelled = {
        let guard = test_state.read().await;
        guard.as_ref().map(|s| s.cancelled).unwrap_or(false)
    };

    if cancelled {
        // Clear state
        let mut guard = test_state.write().await;
        *guard = None;

        return Ok(Json(TestResponse {
            success: true,
            test_generation: test_gen,
            switched: false,
            switch_generation: None,
            summary: "测试已取消，配置保持 test 状态。重启将恢复。".to_string(),
            status: "testing".to_string(),
        }));
    }

    // Auto-switch
    let switch_output = run_cmd("nixos-rebuild", &["switch", "--fast"]).await;
    let switch_success = match &switch_output {
        Ok(o) => !o.contains("error:"),
        Err(_) => false,
    };

    let switch_gen = get_current_generation().await;

    // Clear state
    {
        let mut guard = test_state.write().await;
        *guard = None;
    }

    if switch_success {
        // Update description
        if let (Some(gen), Some(msg)) = (switch_gen, &req.message) {
            let desc_path = format!(
                "/nix/var/nix/profiles/system-{}-link/nix-evo-description",
                gen
            );
            let _ = tokio::fs::write(&desc_path, msg).await;
        }

        Ok(Json(TestResponse {
            success: true,
            test_generation: test_gen,
            switched: true,
            switch_generation: switch_gen,
            summary: format!(
                "测试通过，已自动切换。Generation {}",
                switch_gen.unwrap_or(0)
            ),
            status: "switched".to_string(),
        }))
    } else {
        let err_msg = switch_output.unwrap_or_else(|e| format!("{e}"));
        Ok(Json(TestResponse {
            success: false,
            test_generation: test_gen,
            switched: false,
            switch_generation: None,
            summary: format!("自动切换失败（测试仍有效，重启将恢复）: {err_msg}"),
            status: "failed".to_string(),
        }))
    }
}

/// Cancel an active auto-switch test
pub async fn cancel_test(
    _state: AppStateRef,
) -> Result<Json<serde_json::Value>, AppError> {
    let test_state = ACTIVE_TEST.get_or_init(|| {
        std::sync::Arc::new(tokio::sync::RwLock::new(None))
    });

    let mut guard = test_state.write().await;
    if let Some(state) = guard.as_mut() {
        state.cancelled = true;
        Ok(Json(serde_json::json!({
            "cancelled": true,
            "message": "自动切换已取消"
        })))
    } else {
        Err(AppError::NotFound {
            resource: "没有正在进行的测试".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_wait() {
        assert_eq!(default_wait(), 300);
    }
}
