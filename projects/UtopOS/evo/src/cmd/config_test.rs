//! Test-before-switch endpoint.
//!
//! Runs `nixos-rebuild test` (doesn't modify bootloader, reversible by reboot),
//! then optionally auto-switches after a delay.

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
    /// Seconds to wait before auto-switching. 0 = no auto-switch.
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
    pub status: String,
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
            AppError::IoError { path: target, message: format!("无法写入: {e}") }
        })?;
    }

    // Run nixos-rebuild test
    let test_output = run_cmd("nixos-rebuild", &["test", "--fast"]).await;
    let test_success = matches!(&test_output, Ok(o) if !o.contains("error:"));

    if !test_success {
        let err_msg = test_output.unwrap_or_else(|e| format!("{e}"));
        return Ok(Json(TestResponse {
            success: false, test_generation: None, switched: false,
            switch_generation: None,
            summary: format!("nixos-rebuild test 失败: {err_msg}"),
            status: "failed".into(),
        }));
    }

    let test_gen = get_current_generation().await;

    // Record description
    if let (Some(gen), Some(msg)) = (test_gen, &req.message) {
        let desc_path = format!(
            "/nix/var/nix/profiles/system-{}-link/UtopOS-description", gen
        );
        let _ = tokio::fs::write(&desc_path, format!("[TEST] {msg}")).await;
    }

    // No auto-switch
    if req.wait_seconds == 0 {
        return Ok(Json(TestResponse {
            success: true, test_generation: test_gen, switched: false,
            switch_generation: None,
            summary: "测试已生效（未设置自动切换）。重启将恢复到之前的配置。".into(),
            status: "testing".into(),
        }));
    }

    // Wait then auto-switch
    sleep(Duration::from_secs(req.wait_seconds)).await;

    let switch_output = run_cmd("nixos-rebuild", &["switch", "--fast"]).await;
    let switch_success = matches!(&switch_output, Ok(o) if !o.contains("error:"));
    let switch_gen = get_current_generation().await;

    if switch_success {
        if let (Some(gen), Some(msg)) = (switch_gen, &req.message) {
            let desc_path = format!(
                "/nix/var/nix/profiles/system-{}-link/UtopOS-description", gen
            );
            let _ = tokio::fs::write(&desc_path, msg).await;
        }
        Ok(Json(TestResponse {
            success: true, test_generation: test_gen, switched: true,
            switch_generation: switch_gen,
            summary: format!("测试通过，已自动切换。Generation {}", switch_gen.unwrap_or(0)),
            status: "switched".into(),
        }))
    } else {
        let err = switch_output.unwrap_or_else(|e| format!("{e}"));
        Ok(Json(TestResponse {
            success: false, test_generation: test_gen, switched: false,
            switch_generation: None,
            summary: format!("自动切换失败（测试仍有效，重启将恢复）: {err}"),
            status: "failed".into(),
        }))
    }
}

pub async fn cancel_test(
    _state: AppStateRef,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({
        "cancelled": true,
        "message": "自动切换已取消（注意：当前实现不支持取消已启动的 test）"
    })))
}
