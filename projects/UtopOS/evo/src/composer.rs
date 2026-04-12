use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::cmd::AppStateRef;
use crate::error::AppError;

/// Service definition in a composition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDef {
    pub name: String,
    pub package: Option<String>,
    pub enable: bool,
    pub depends_on: Vec<String>,
    pub health_check: Option<HealthCheck>,
    pub restart_policy: Option<String>,
    pub scaling: Option<ScalingHint>,
    pub env: HashMap<String, String>,
    pub ports: Vec<PortMapping>,
    pub config_options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub check_type: String,  // "http", "tcp", "exec"
    pub target: String,
    pub interval_secs: u64,
    pub timeout_secs: u64,
    pub retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingHint {
    pub min_instances: u32,
    pub max_instances: u32,
    pub cpu_threshold_percent: Option<f64>,
    pub memory_threshold_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String,
}

/// A composition = multi-service deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Composition {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub services: Vec<ServiceDef>,
}

/// Service status at runtime
#[derive(Debug, Clone, Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub state: String,  // "running", "starting", "stopped", "failed", "healthy", "unhealthy"
    pub pid: Option<u32>,
    pub uptime_secs: Option<u64>,
    pub health: String,
    pub restart_count: u32,
    pub last_health_check: Option<String>,
}

/// Composition status
#[derive(Debug, Serialize)]
pub struct CompositionStatus {
    pub name: String,
    pub state: String,
    pub services: Vec<ServiceStatus>,
    pub started_at: Option<String>,
    pub total_restarts: u32,
}

/// Resolved startup order (topological sort)
#[derive(Debug, Serialize)]
pub struct StartupPlan {
    pub order: Vec<String>,
    pub layers: Vec<Vec<String>>,
    pub total_services: usize,
}

/// Global composition state
static COMPOSITIONS: std::sync::OnceLock<Arc<CompositionStore>> = std::sync::OnceLock::new();

pub struct CompositionStore {
    compositions: RwLock<HashMap<String, Composition>>,
    statuses: RwLock<HashMap<String, CompositionStatus>>,
}

impl CompositionStore {
    fn new() -> Self {
        Self {
            compositions: RwLock::new(HashMap::new()),
            statuses: RwLock::new(HashMap::new()),
        }
    }
}

fn get_store() -> &'static Arc<CompositionStore> {
    COMPOSITIONS.get_or_init(|| Arc::new(CompositionStore::new()))
}

/// Topological sort to determine startup order
fn resolve_startup_order(services: &[ServiceDef]) -> Result<StartupPlan, AppError> {
    let service_names: Vec<String> = services.iter().map(|s| s.name.clone()).collect();

    // Build adjacency: name -> list of dependencies
    let mut deps_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    for svc in services {
        deps_map.entry(svc.name.clone()).or_default();
        in_degree.entry(svc.name.clone()).or_insert(0);
        for dep in &svc.depends_on {
            if !service_names.contains(dep) {
                return Err(AppError::Validation {
                    field: "depends_on".into(),
                    message: format!("Service '{}' depends on '{}' which is not defined", svc.name, dep),
                });
            }
            deps_map.entry(dep.clone()).or_default().push(svc.name.clone());
            *in_degree.entry(svc.name.clone()).or_insert(0) += 1;
        }
    }

    // Kahn's algorithm
    let mut queue: Vec<String> = in_degree.iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(name, _)| name.clone())
        .collect();
    queue.sort();

    let mut order = Vec::new();
    let mut layers = Vec::new();

    while !queue.is_empty() {
        let layer = queue.clone();
        layers.push(layer.clone());
        queue.clear();

        for node in &layer {
            order.push(node.clone());
            if let Some(neighbors) = deps_map.get(node) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
        }
        queue.sort();
    }

    if order.len() != services.len() {
        return Err(AppError::Validation {
            field: "depends_on".into(),
            message: "Circular dependency detected in service composition".into(),
        });
    }

    Ok(StartupPlan {
        order,
        layers,
        total_services: services.len(),
    })
}

/// Generate NixOS config snippet from a composition
fn generate_nixos_config(composition: &Composition) -> String {
    let mut config = String::new();
    config.push_str(&format!("# Auto-generated from composition: {}\n", composition.name));
    config.push_str(&format!("# Version: {}\n\n", composition.version));
    config.push_str("{ config, pkgs, ... }:\n\n{\n");

    for svc in &composition.services {
        if !svc.enable { continue; }

        config.push_str(&format!("  # Service: {}\n", svc.name));

        // Map service name to NixOS option
        let nix_svc = match svc.name.as_str() {
            "nginx" => "services.nginx",
            "postgresql" | "postgres" => "services.postgresql",
            "redis" => "services.redis",
            "mysql" | "mariadb" => "services.mysql",
            "mongodb" => "services.mongodb",
            "openssh" | "ssh" => "services.openssh",
            "prometheus" => "services.prometheus",
            "grafana" => "services.grafana",
            "docker" => "virtualisation.docker",
            "caddy" => "services.caddy",
            _ => "",
        };

        if !nix_svc.is_empty() {
            config.push_str(&format!("  {}.enable = true;\n", nix_svc));
        } else {
            config.push_str(&format!("  # TODO: Add NixOS config for service '{}'\n", svc.name));
        }

        // Environment vars
        if !svc.env.is_empty() {
            config.push_str(&format!("  systemd.services.{}.environment = {{\n", svc.name));
            for (k, v) in &svc.env {
                config.push_str(&format!("    {} = \"{}\";\n", k, v));
            }
            config.push_str("  };\n");
        }

        // Dependencies
        if !svc.depends_on.is_empty() {
            config.push_str(&format!("  systemd.services.{}.after = [ {} ];\n",
                svc.name,
                svc.depends_on.iter().map(|d| format!("\"{}.service\"", d)).collect::<Vec<_>>().join(" ")
            ));
            config.push_str(&format!("  systemd.services.{}.requires = [ {} ];\n",
                svc.name,
                svc.depends_on.iter().map(|d| format!("\"{}.service\"", d)).collect::<Vec<_>>().join(" ")
            ));
        }

        // Restart policy
        if let Some(rp) = &svc.restart_policy {
            let systemd_policy = match rp.as_str() {
                "always" => "always",
                "on-failure" => "on-failure",
                "no" => "no",
                _ => "on-failure",
            };
            config.push_str(&format!("  systemd.services.{}.serviceConfig.Restart = \"{}\";\n", svc.name, systemd_policy));
        }

        config.push('\n');
    }

    config.push_str("}\n");
    config
}

/// Validate a composition
fn validate_composition(comp: &Composition) -> Vec<String> {
    let mut warnings = Vec::new();
    let names: Vec<&str> = comp.services.iter().map(|s| s.name.as_str()).collect();

    for svc in &comp.services {
        for dep in &svc.depends_on {
            if !names.contains(&dep.as_str()) {
                warnings.push(format!("Service '{}' depends on '{}' which is not in the composition", svc.name, dep));
            }
        }

        if svc.ports.len() > 1 {
            warnings.push(format!("Service '{}' exposes multiple ports — consider if this is intentional", svc.name));
        }

        if svc.health_check.is_none() {
            warnings.push(format!("Service '{}' has no health check configured", svc.name));
        }
    }

    // Check for duplicate names
    let mut seen = std::collections::HashSet::new();
    for name in &names {
        if !seen.insert(name) {
            warnings.push(format!("Duplicate service name: '{}'", name));
        }
    }

    warnings
}

// ─── Request/Response types ───────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ComposeRequest {
    pub composition: Composition,
    pub action: Option<String>,  // "plan", "deploy", "validate"
}

#[derive(Debug, Serialize)]
pub struct ComposeResponse {
    pub name: String,
    pub action: String,
    pub startup_plan: Option<StartupPlan>,
    pub nixos_config: Option<String>,
    pub validation_warnings: Vec<String>,
    pub status: Option<CompositionStatus>,
}

// ─── Handlers ─────────────────────────────────────────────────────

/// POST /api/compose — create/update/validate/deploy a composition
pub async fn handle_compose(
    State(_state): AppStateRef,
    Json(req): Json<ComposeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let action = req.action.as_deref().unwrap_or("plan");
    let comp = req.composition;

    // Validate
    let warnings = validate_composition(&comp);

    // Resolve startup order
    let startup_plan = resolve_startup_order(&comp.services)?;

    // Generate NixOS config
    let nixos_config = generate_nixos_config(&comp);

    match action {
        "deploy" => {
            let store = get_store();

            // Create status
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let status = CompositionStatus {
                name: comp.name.clone(),
                state: "running".into(),
                services: comp.services.iter().map(|s| ServiceStatus {
                    name: s.name.clone(),
                    state: if s.enable { "running".into() } else { "stopped".into() },
                    pid: None,
                    uptime_secs: Some(0),
                    health: "healthy".into(),
                    restart_count: 0,
                    last_health_check: Some(format!("{}", now.as_secs())),
                }).collect(),
                started_at: Some(format!("{}", now.as_secs())),
                total_restarts: 0,
            };

            store.compositions.write().await.insert(comp.name.clone(), comp.clone());
            store.statuses.write().await.insert(comp.name.clone(), status.clone());

            Ok(Json(serde_json::to_value(ComposeResponse {
                name: comp.name,
                action: "deploy".into(),
                startup_plan: Some(startup_plan),
                nixos_config: Some(nixos_config),
                validation_warnings: warnings,
                status: Some(status),
            }).unwrap_or_default()))
        }
        "validate" => {
            Ok(Json(serde_json::to_value(ComposeResponse {
                name: comp.name,
                action: "validate".into(),
                startup_plan: Some(startup_plan),
                nixos_config: Some(nixos_config),
                validation_warnings: warnings,
                status: None,
            }).unwrap_or_default()))
        }
        _ => {
            // "plan" or default
            Ok(Json(serde_json::to_value(ComposeResponse {
                name: comp.name,
                action: "plan".into(),
                startup_plan: Some(startup_plan),
                nixos_config: Some(nixos_config),
                validation_warnings: warnings,
                status: None,
            }).unwrap_or_default()))
        }
    }
}

/// GET /api/compose/status — get status of all compositions
pub async fn handle_status(
    State(_state): AppStateRef,
) -> Result<impl IntoResponse, AppError> {
    let store = get_store();
    let statuses: Vec<CompositionStatus> = store.statuses.read().await.values().cloned().collect();

    Ok(Json(serde_json::json!({
        "compositions": statuses,
        "total": statuses.len(),
    })))
}
