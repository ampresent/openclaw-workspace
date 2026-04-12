use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use crate::error::AppError;
use crate::cmd::{run_cmd, run_cmd_with_timeout};

/// A running Docker container
#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: String,
    pub ports: Vec<String>,
    pub created: String,
    /// If this container image has a NixOS-native alternative
    pub nixos_alternative: Option<NixOsAlternative>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NixOsAlternative {
    pub nixos_service: String,
    pub nixos_package: String,
    pub description: String,
    pub migration_difficulty: String, // easy / moderate / hard
}

/// A Docker Compose stack
#[derive(Debug, Serialize, Deserialize)]
pub struct ComposeStack {
    pub name: String,
    pub project_dir: String,
    pub services: Vec<ComposeService>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComposeService {
    pub name: String,
    pub image: String,
    pub state: String,
    pub ports: Vec<String>,
}

/// Full Docker status response
#[derive(Debug, Serialize)]
pub struct DockerStatusResponse {
    pub docker_available: bool,
    pub docker_version: Option<String>,
    pub containers: Vec<ContainerInfo>,
    pub compose_stacks: Vec<ComposeStack>,
    pub total_containers: usize,
    pub running_containers: usize,
    pub nixos_suggestions: Vec<NixOsSuggestion>,
}

#[derive(Debug, Serialize)]
pub struct NixOsSuggestion {
    pub container_name: String,
    pub image: String,
    pub suggestion: String,
    pub nixos_config_snippet: String,
}

/// Known Docker images that have NixOS-native service alternatives
fn get_nixos_alternatives() -> Vec<(&'static str, NixOsAlternative)> {
    vec![
        ("nginx", NixOsAlternative {
            nixos_service: "services.nginx".into(),
            nixos_package: "pkgs.nginx".into(),
            description: "NixOS has first-class nginx support with declarative config".into(),
            migration_difficulty: "easy".into(),
        }),
        ("postgres", NixOsAlternative {
            nixos_service: "services.postgresql".into(),
            nixos_package: "pkgs.postgresql".into(),
            description: "NixOS PostgreSQL module with extensions, backups, and auth".into(),
            migration_difficulty: "moderate".into(),
        }),
        ("redis", NixOsAlternative {
            nixos_service: "services.redis".into(),
            nixos_package: "pkgs.redis".into(),
            description: "NixOS Redis module with persistence and clustering".into(),
            migration_difficulty: "easy".into(),
        }),
        ("grafana", NixOsAlternative {
            nixos_service: "services.grafana".into(),
            nixos_package: "pkgs.grafana".into(),
            description: "NixOS Grafana module with declarative datasource/dashboards".into(),
            migration_difficulty: "moderate".into(),
        }),
        ("prometheus", NixOsAlternative {
            nixos_service: "services.prometheus".into(),
            nixos_package: "pkgs.prometheus".into(),
            description: "NixOS Prometheus with declarative scrape configs and alerting".into(),
            migration_difficulty: "moderate".into(),
        }),
        ("caddy", NixOsAlternative {
            nixos_service: "services.caddy".into(),
            nixos_package: "pkgs.caddy".into(),
            description: "NixOS Caddy module with automatic HTTPS".into(),
            migration_difficulty: "easy".into(),
        }),
        ("mysql", NixOsAlternative {
            nixos_service: "services.mysql".into(),
            nixos_package: "pkgs.mariadb".into(),
            description: "NixOS MySQL/MariaDB module".into(),
            migration_difficulty: "moderate".into(),
        }),
        ("mongodb", NixOsAlternative {
            nixos_service: "services.mongodb".into(),
            nixos_package: "pkgs.mongodb".into(),
            description: "NixOS MongoDB module".into(),
            migration_difficulty: "moderate".into(),
        }),
        ("nextcloud", NixOsAlternative {
            nixos_service: "services.nextcloud".into(),
            nixos_package: "pkgs.nextcloud".into(),
            description: "NixOS Nextcloud module with declarative config, auto-SSL".into(),
            migration_difficulty: "hard".into(),
        }),
        ("jellyfin", NixOsAlternative {
            nixos_service: "services.jellyfin".into(),
            nixos_package: "pkgs.jellyfin".into(),
            description: "NixOS Jellyfin media server module".into(),
            migration_difficulty: "easy".into(),
        }),
        ("gitea", NixOsAlternative {
            nixos_service: "services.gitea".into(),
            nixos_package: "pkgs.gitea".into(),
            description: "NixOS Gitea/Gitea Actions module".into(),
            migration_difficulty: "easy".into(),
        }),
        ("vaultwarden", NixOsAlternative {
            nixos_service: "services.vaultwarden".into(),
            nixos_package: "pkgs.vaultwarden".into(),
            description: "NixOS Vaultwarden (Bitwarden compatible) module".into(),
            migration_difficulty: "easy".into(),
        }),
        ("minio", NixOsAlternative {
            nixos_service: "services.minio".into(),
            nixos_package: "pkgs.minio".into(),
            description: "NixOS MinIO S3-compatible object storage".into(),
            migration_difficulty: "easy".into(),
        }),
    ]
}

/// Match a Docker image name against known NixOS alternatives
fn match_nixos_alternative(image: &str) -> Option<NixOsAlternative> {
    let image_lower = image.to_lowercase();
    let image_base = image_lower.split(':').next().unwrap_or(&image_lower);
    // Also check last path component for registry images
    let short_name = image_base.split('/').last().unwrap_or(image_base);

    for (keyword, alt) in get_nixos_alternatives() {
        if short_name.contains(keyword) {
            return Some(alt);
        }
    }
    None
}

/// GET /api/docker/status — full Docker environment status
pub async fn docker_status(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<DockerStatusResponse>, AppError> {
    // Check if Docker is available
    let docker_version = match run_cmd_with_timeout("docker", &["version", "--format", "{{.Server.Version}}"], 10).await {
        Ok(v) => Some(v.trim().to_string()),
        Err(_) => None,
    };

    let docker_available = docker_version.is_some();

    if !docker_available {
        return Ok(Json(DockerStatusResponse {
            docker_available: false,
            docker_version: None,
            containers: vec![],
            compose_stacks: vec![],
            total_containers: 0,
            running_containers: 0,
            nixos_suggestions: vec![],
        }));
    }

    // List containers
    let containers = list_containers().await.unwrap_or_default();
    let total = containers.len();
    let running = containers.iter().filter(|c| c.state == "running").count();

    // List compose stacks
    let compose_stacks = list_compose_stacks().await.unwrap_or_default();

    // Generate NixOS suggestions for containers with known alternatives
    let nixos_suggestions: Vec<NixOsSuggestion> = containers
        .iter()
        .filter_map(|c| {
            match_nixos_alternative(&c.image).map(|alt| NixOsSuggestion {
                container_name: c.name.clone(),
                image: c.image.clone(),
                suggestion: format!(
                    "容器 '{}' 使用 {}，NixOS 原生替代: {} (迁移难度: {})",
                    c.name, c.image, alt.nixos_service, alt.migration_difficulty
                ),
                nixos_config_snippet: format!("{} = {{ enable = true; }};", alt.nixos_service),
            })
        })
        .collect();

    Ok(Json(DockerStatusResponse {
        docker_available,
        docker_version,
        containers,
        compose_stacks,
        total_containers: total,
        running_containers: running,
        nixos_suggestions,
    }))
}

/// List all Docker containers with detailed info
async fn list_containers() -> Result<Vec<ContainerInfo>, AppError> {
    let format = "{{.ID}}\\t{{.Names}}\\t{{.Image}}\\t{{.Status}}\\t{{.State}}\\t{{.Ports}}\\t{{.CreatedAt}}";
    let output = run_cmd("docker", &["ps", "-a", "--format", format]).await?;

    let containers = output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 7 {
                return None;
            }
            let image = parts[2].to_string();
            Some(ContainerInfo {
                id: parts[0].to_string(),
                name: parts[1].to_string(),
                nixos_alternative: match_nixos_alternative(&image),
                image,
                status: parts[3].to_string(),
                state: parts[4].to_string(),
                ports: parts[5]
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                created: parts[6].to_string(),
            })
        })
        .collect();

    Ok(containers)
}

/// List Docker Compose stacks
async fn list_compose_stacks() -> Result<Vec<ComposeStack>, AppError> {
    // Try docker compose ls (v2) or docker-compose ls (v1)
    let output = match run_cmd_with_timeout(
        "docker",
        &["compose", "ls", "--format", "json"],
        10,
    )
    .await
    {
        Ok(o) => o,
        Err(_) => match run_cmd_with_timeout("docker-compose", &["ls", "--format", "json"], 10).await {
            Ok(o) => o,
            Err(_) => return Ok(vec![]),
        },
    };

    // Parse JSON array output
    let stacks: Vec<ComposeStack> = serde_json::from_str(&output).unwrap_or_else(|_| {
        // Fallback: parse table output
        output
            .lines()
            .skip(1) // skip header
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }
                Some(ComposeStack {
                    name: parts[0].to_string(),
                    project_dir: parts.get(2).unwrap_or(&"").to_string(),
                    services: vec![],
                    status: parts.get(1).unwrap_or(&"unknown").to_string(),
                })
            })
            .collect()
    });

    Ok(stacks)
}

/// Request body for compose validation
#[derive(Debug, Deserialize)]
pub struct ComposeValidateRequest {
    /// Path to docker-compose.yml or inline YAML content
    pub content: String,
    /// Whether `content` is a file path (true) or inline YAML (false)
    pub is_file_path: bool,
}

/// Validation result
#[derive(Debug, Serialize)]
pub struct ComposeValidateResponse {
    pub valid: bool,
    pub services: Vec<ValidatedService>,
    pub warnings: Vec<String>,
    pub nixos_alternatives: Vec<NixOsSuggestion>,
}

#[derive(Debug, Serialize)]
pub struct ValidatedService {
    pub name: String,
    pub image: Option<String>,
    pub has_nixos_alternative: bool,
    pub nixos_service: Option<String>,
}

/// POST /api/docker/compose-validate — validate a compose file and check for NixOS alternatives
pub async fn compose_validate(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ComposeValidateRequest>,
) -> Result<Json<ComposeValidateResponse>, AppError> {
    let mut warnings = Vec::new();
    let mut nixos_alternatives = Vec::new();
    let mut services = Vec::new();

    if req.is_file_path {
        // Validate it's a reasonable path
        let path = std::path::Path::new(&req.content);
        if !path.exists() {
            return Err(AppError::NotFound {
                resource: format!("Compose 文件: {}", req.content),
            });
        }
        // Use docker compose config to validate
        let output = run_cmd_with_timeout(
            "docker",
            &["compose", "-f", &req.content, "config", "--format", "json"],
            15,
        )
        .await?;

        parse_compose_config(&output, &mut services, &mut warnings, &mut nixos_alternatives);
    } else {
        // Write inline content to temp file and validate
        let tmp_path = "/tmp/UtopOS-compose-validate.yml";
        std::fs::write(tmp_path, &req.content).map_err(|e| AppError::IoError {
            path: tmp_path.into(),
            message: e.to_string(),
        })?;

        let output = match run_cmd_with_timeout(
            "docker",
            &["compose", "-f", tmp_path, "config", "--format", "json"],
            15,
        )
        .await
        {
            Ok(o) => o,
            Err(e) => {
                let _ = std::fs::remove_file(tmp_path);
                return Ok(Json(ComposeValidateResponse {
                    valid: false,
                    services: vec![],
                    warnings: vec![format!("Compose 验证失败: {e}")],
                    nixos_alternatives: vec![],
                }));
            }
        };

        let _ = std::fs::remove_file(tmp_path);
        parse_compose_config(&output, &mut services, &mut warnings, &mut nixos_alternatives);
    }

    // Check for port conflicts
    let mut port_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for svc in &services {
        // This is a simplified check — real implementation would parse ports from compose config
        if svc.has_nixos_alternative {
            warnings.push(format!(
                "服务 '{}' 有 NixOS 原生替代 ({}), 考虑迁移以获得更好的系统集成",
                svc.name,
                svc.nixos_service.as_deref().unwrap_or("unknown")
            ));
        }
    }

    Ok(Json(ComposeValidateResponse {
        valid: true,
        services,
        warnings,
        nixos_alternatives,
    }))
}

fn parse_compose_config(
    json_str: &str,
    services: &mut Vec<ValidatedService>,
    warnings: &mut Vec<String>,
    nixos_alternatives: &mut Vec<NixOsSuggestion>,
) {
    if let Ok(config) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(svc_obj) = config.get("services").and_then(|s| s.as_object()) {
            for (name, svc_config) in svc_obj {
                let image = svc_config
                    .get("image")
                    .and_then(|i| i.as_str())
                    .map(|s| s.to_string());

                let alt = image.as_ref().and_then(|img| match_nixos_alternative(img));

                if let (Some(img), Some(ref alt)) = (&image, &alt) {
                    nixos_alternatives.push(NixOsSuggestion {
                        container_name: name.clone(),
                        image: img.clone(),
                        suggestion: format!(
                            "服务 '{}' 可以迁移到 NixOS 原生服务 {}",
                            name, alt.nixos_service
                        ),
                        nixos_config_snippet: format!("{} = {{ enable = true; }};", alt.nixos_service),
                    });
                }

                services.push(ValidatedService {
                    name: name.clone(),
                    image,
                    has_nixos_alternative: alt.is_some(),
                    nixos_service: alt.map(|a| a.nixos_service),
                });
            }
        }
    } else {
        warnings.push("无法解析 Compose 配置 JSON".into());
    }
}

/// Check if Docker daemon is running on this NixOS system
pub async fn check_docker_nixos_integration() -> Result<Json<serde_json::Value>, AppError> {
    let docker_enabled = std::path::Path::new("/run/current-system/sw/bin/docker").exists();
    let docker_running = run_cmd_with_timeout("systemctl", &["is-active", "docker"], 5)
        .await
        .map(|s| s.trim() == "active")
        .unwrap_or(false);

    let podman_enabled = std::path::Path::new("/run/current-system/sw/bin/podman").exists();

    Ok(Json(serde_json::json!({
        "docker_in_nixos": docker_enabled,
        "docker_running": docker_running,
        "podman_available": podman_enabled,
        "nixos_config_hint": if docker_enabled {
            "Docker 已通过 NixOS 安装。在 configuration.nix 中使用 virtualisation.docker.enable = true;"
        } else {
            "建议使用 NixOS 原生 Docker 模块: virtualisation.docker.enable = true;"
        },
    })))
}
