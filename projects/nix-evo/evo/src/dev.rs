use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use crate::error::AppError;

/// Dev mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevConfig {
    pub enabled: bool,
    pub mock_system: bool,      // Mock systemctl, nixos-rebuild, journalctl
    pub mock_data_dir: String,  // Where to store mock system state
    pub auto_reload: bool,      // Watch for config changes and reload
    pub verbose_logging: bool,
}

/// Mock system state for testing without real NixOS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSystemState {
    pub services: std::collections::HashMap<String, MockService>,
    pub generation: u64,
    pub nixos_version: String,
    pub hostname: String,
    pub config_content: String,
    pub generations: Vec<MockGeneration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockService {
    pub name: String,
    pub active_state: String,   // "active", "inactive", "failed"
    pub sub_state: String,      // "running", "dead", "exited"
    pub enabled: bool,
    pub description: String,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockGeneration {
    pub number: u64,
    pub date: String,
    pub description: String,
    pub current: bool,
}

/// POST /api/dev/mode — toggle dev mode
pub async fn toggle_dev_mode(
    State(_state): State<Arc<AppState>>,
    Json(config): Json<DevConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    if config.enabled {
        tracing::info!("DEV MODE ENABLED — using mock system commands");
        init_mock_system(&config.mock_data_dir)?;
    } else {
        tracing::info!("Dev mode disabled — using real system commands");
    }

    // Save dev config
    let config_path = format!("{}/dev-config.json", config.mock_data_dir);
    std::fs::create_dir_all(&config.mock_data_dir).map_err(|e| AppError::IoError {
        path: config.mock_data_dir.clone(),
        message: e.to_string(),
    })?;
    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap())
        .map_err(|e| AppError::IoError {
            path: config_path,
            message: e.to_string(),
        })?;

    Ok(Json(serde_json::json!({
        "dev_mode": config.enabled,
        "mock_system": config.mock_system,
        "data_dir": config.mock_data_dir,
        "message": if config.enabled {
            "开发模式已启用 — 使用模拟系统命令"
        } else {
            "开发模式已禁用 — 使用真实系统命令"
        }
    })))
}

/// GET /api/dev/status — dev mode status
pub async fn dev_status(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let dev_config = load_dev_config();
    let mock_state = if dev_config.enabled {
        Some(load_mock_state(&dev_config.mock_data_dir)?)
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "dev_mode": dev_config,
        "mock_state": mock_state,
        "real_system": !dev_config.enabled,
    })))
}

/// POST /api/dev/mock/service — set mock service state
pub async fn mock_service(
    State(_state): State<Arc<AppState>>,
    Json(svc): Json<MockService>,
) -> Result<Json<MockService>, AppError> {
    let config = load_dev_config();
    if !config.enabled {
        return Err(AppError::Validation {
            field: "dev_mode".into(),
            message: "请先启用开发模式".into(),
        });
    }

    let mut state = load_mock_state(&config.mock_data_dir)?;
    state.services.insert(svc.name.clone(), svc.clone());
    save_mock_state(&config.mock_data_dir, &state)?;

    Ok(Json(svc))
}

/// POST /api/dev/mock/generation — simulate a config apply (new generation)
pub async fn mock_apply(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<MockApplyRequest>,
) -> Result<Json<MockGeneration>, AppError> {
    let config = load_dev_config();
    if !config.enabled {
        return Err(AppError::Validation {
            field: "dev_mode".into(),
            message: "请先启用开发模式".into(),
        });
    }

    let mut state = load_mock_state(&config.mock_data_dir)?;
    state.generation += 1;

    if let Some(content) = req.config_content {
        state.config_content = content;
    }

    let gen = MockGeneration {
        number: state.generation,
        date: chrono_now(),
        description: req.description.unwrap_or_else(|| "模拟配置变更".into()),
        current: true,
    };

    // Mark previous generations as not current
    for g in &mut state.generations {
        g.current = false;
    }
    state.generations.push(gen.clone());

    save_mock_state(&config.mock_data_dir, &state)?;

    tracing::info!("Mock apply: generation {} created", gen.number);

    Ok(Json(gen))
}

#[derive(Debug, Deserialize)]
pub struct MockApplyRequest {
    pub description: Option<String>,
    pub config_content: Option<String>,
}

/// POST /api/dev/mock/reset — reset mock system to defaults
pub async fn mock_reset(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let config = load_dev_config();
    if !config.enabled {
        return Err(AppError::Validation {
            field: "dev_mode".into(),
            message: "请先启用开发模式".into(),
        });
    }

    init_mock_system(&config.mock_data_dir)?;

    Ok(Json(serde_json::json!({
        "message": "模拟系统已重置为默认状态",
        "generation": 1,
    })))
}

/// GET /api/dev/mock/snapshot — get mock system snapshot (replaces real system_snapshot)
pub async fn mock_snapshot(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let config = load_dev_config();
    if !config.enabled {
        return Err(AppError::Validation {
            field: "dev_mode".into(),
            message: "开发模式未启用".into(),
        });
    }

    let state = load_mock_state(&config.mock_data_dir)?;

    let services: Vec<serde_json::Value> = state.services.values().map(|s| {
        serde_json::json!({
            "name": s.name,
            "active_state": s.active_state,
            "sub_state": s.sub_state,
            "enabled": s.enabled,
            "description": s.description,
            "memory_bytes": s.memory_bytes,
        })
    }).collect();

    let failed: Vec<&MockService> = state.services.values()
        .filter(|s| s.active_state == "failed")
        .collect();

    Ok(Json(serde_json::json!({
        "hostname": state.hostname,
        "nixos_version": state.nixos_version,
        "generation": state.generation,
        "services": services,
        "failed_services": failed.iter().map(|s| &s.name).collect::<Vec<_>>(),
        "uptime_hours": 42,
        "memory": {
            "total_mb": 16384,
            "used_mb": 8192,
            "available_mb": 8192,
        },
        "disk": {
            "total_gb": 500,
            "used_gb": 150,
            "available_gb": 350,
        },
        "_mock": true,
    })))
}

// ============================================================
// Internal helpers
// ============================================================

fn load_dev_config() -> DevConfig {
    let default_dir = "/tmp/nix-evo-dev".to_string();
    let config_path = format!("{default_dir}/dev-config.json");
    std::fs::read_to_string(&config_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(DevConfig {
            enabled: false,
            mock_system: true,
            mock_data_dir: default_dir,
            auto_reload: false,
            verbose_logging: true,
        })
}

fn load_mock_state(dir: &str) -> Result<MockSystemState, AppError> {
    let path = format!("{dir}/mock-state.json");
    let content = std::fs::read_to_string(&path).map_err(|e| AppError::IoError {
        path: path.clone(),
        message: format!("无法读取模拟状态: {e}"),
    })?;
    serde_json::from_str(&content).map_err(|e| AppError::Internal {
        message: format!("解析模拟状态失败: {e}"),
    })
}

fn save_mock_state(dir: &str, state: &MockSystemState) -> Result<(), AppError> {
    let path = format!("{dir}/mock-state.json");
    let content = serde_json::to_string_pretty(state).map_err(|e| AppError::Internal {
        message: format!("序列化失败: {e}"),
    })?;
    std::fs::write(&path, content).map_err(|e| AppError::IoError {
        path,
        message: e.to_string(),
    })?;
    Ok(())
}

fn init_mock_system(dir: &str) -> Result<(), AppError> {
    std::fs::create_dir_all(dir).map_err(|e| AppError::IoError {
        path: dir.into(),
        message: e.to_string(),
    })?;

    let mut services = std::collections::HashMap::new();
    for svc_name in &["sshd", "nginx", "postgresql", "docker", "grafana-server", "prometheus"] {
        services.insert(svc_name.to_string(), MockService {
            name: svc_name.to_string(),
            active_state: "active".into(),
            sub_state: "running".into(),
            enabled: true,
            description: format!("{svc_name} service"),
            memory_bytes: 128 * 1024 * 1024, // 128MB each
        });
    }

    let state = MockSystemState {
        services,
        generation: 1,
        nixos_version: "25.05.20260412.abc1234 (Warbler)".into(),
        hostname: "nix-evo-dev".into(),
        config_content: "# Default NixOS configuration\n{ config, pkgs, ... }:\n{\n  imports = [ ./hardware-configuration.nix ];\n  boot.loader.systemd-boot.enable = true;\n  networking.hostName = \"nix-evo\";\n  services.openssh.enable = true;\n}\n".into(),
        generations: vec![MockGeneration {
            number: 1,
            date: chrono_now(),
            description: "初始配置".into(),
            current: true,
        }],
    };

    save_mock_state(dir, &state)?;
    Ok(())
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
