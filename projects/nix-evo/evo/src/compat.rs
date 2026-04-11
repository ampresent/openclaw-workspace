use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::cmd::AppStateRef;
use crate::error::AppError;

/// Translation request
#[derive(Debug, Deserialize)]
pub struct TranslateRequest {
    pub nixos_config: String,
    pub target_distro: String,  // "ubuntu", "debian", "fedora", "arch", "alpine"
    pub services: Option<Vec<String>>,
}

/// Translation result
#[derive(Debug, Serialize)]
pub struct TranslateResponse {
    pub source: String,
    pub target_distro: String,
    pub translated_configs: Vec<TranslatedConfig>,
    pub package_mapping: Vec<PackageMapping>,
    pub warnings: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TranslatedConfig {
    pub service: String,
    pub config_type: String,  // "systemd-unit", "config-file", "script"
    pub filename: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct PackageMapping {
    pub nixos_package: String,
    pub target_package: String,
    pub match_type: String,  // "exact", "alias", "alternative", "manual"
}

/// NixOS service → target distro mapping
struct ServiceMapping {
    nixos_option: &'str,
    target_pkg: &'str,
    systemd_unit: &'str,
    config_files: &'str,
}

/// Known service mappings
fn service_mappings() -> Vec<ServiceMapping> {
    vec![
        ServiceMapping {
            nixos_option: "services.nginx",
            target_pkg: "nginx",
            systemd_unit: "nginx.service",
            config_files: "/etc/nginx/nginx.conf",
        },
        ServiceMapping {
            nixos_option: "services.postgresql",
            target_pkg: "postgresql",
            systemd_unit: "postgresql.service",
            config_files: "/etc/postgresql/*/main/postgresql.conf",
        },
        ServiceMapping {
            nixos_option: "services.redis",
            target_pkg: "redis-server",
            systemd_unit: "redis.service",
            config_files: "/etc/redis/redis.conf",
        },
        ServiceMapping {
            nixos_option: "services.openssh",
            target_pkg: "openssh-server",
            systemd_unit: "sshd.service",
            config_files: "/etc/ssh/sshd_config",
        },
        ServiceMapping {
            nixos_option: "services.mysql",
            target_pkg: "mysql-server",
            systemd_unit: "mysql.service",
            config_files: "/etc/mysql/mysql.conf.d/mysqld.cnf",
        },
        ServiceMapping {
            nixos_option: "services.caddy",
            target_pkg: "caddy",
            systemd_unit: "caddy.service",
            config_files: "/etc/caddy/Caddyfile",
        },
        ServiceMapping {
            nixos_option: "services.docker",
            target_pkg: "docker.io",
            systemd_unit: "docker.service",
            config_files: "/etc/docker/daemon.json",
        },
        ServiceMapping {
            nixos_option: "services.prometheus",
            target_pkg: "prometheus",
            systemd_unit: "prometheus.service",
            config_files: "/etc/prometheus/prometheus.yml",
        },
        ServiceMapping {
            nixos_option: "services.grafana",
            target_pkg: "grafana",
            systemd_unit: "grafana-server.service",
            config_files: "/etc/grafana/grafana.ini",
        },
    ]
}

/// Package name mappings per distro
fn pkg_name_map(distro: &str) -> HashMap<&str, &str> {
    let mut map = HashMap::new();
    match distro {
        "ubuntu" | "debian" => {
            map.insert("nginx", "nginx");
            map.insert("postgresql_16", "postgresql-16");
            map.insert("postgresql", "postgresql");
            map.insert("redis", "redis-server");
            map.insert("openssh", "openssh-server");
            map.insert("mysql80", "mysql-server-8.0");
            map.insert("mariadb", "mariadb-server");
            map.insert("caddy", "caddy");
            map.insert("docker", "docker.io");
            map.insert("grafana", "grafana");
            map.insert("prometheus", "prometheus");
            map.insert("nodejs", "nodejs");
            map.insert("python3", "python3");
            map.insert("git", "git");
            map.insert("vim", "vim");
            map.insert("htop", "htop");
            map.insert("curl", "curl");
            map.insert("jq", "jq");
            map.insert("tmux", "tmux");
        }
        "fedora" => {
            map.insert("nginx", "nginx");
            map.insert("postgresql_16", "postgresql-server");
            map.insert("redis", "redis");
            map.insert("openssh", "openssh-server");
            map.insert("mysql80", "mysql-server");
            map.insert("caddy", "caddy");
            map.insert("docker", "docker-ce");
            map.insert("grafana", "grafana");
            map.insert("prometheus", "golang-github-prometheus");
        }
        "arch" => {
            map.insert("nginx", "nginx");
            map.insert("postgresql_16", "postgresql");
            map.insert("redis", "redis");
            map.insert("openssh", "openssh");
            map.insert("mysql80", "mysql");
            map.insert("caddy", "caddy");
            map.insert("docker", "docker");
            map.insert("grafana", "grafana");
            map.insert("prometheus", "prometheus");
        }
        "alpine" => {
            map.insert("nginx", "nginx");
            map.insert("postgresql_16", "postgresql16");
            map.insert("redis", "redis");
            map.insert("openssh", "openssh");
            map.insert("caddy", "caddy");
            map.insert("docker", "docker");
        }
        _ => {}
    }
    map
}

/// Parse NixOS service names from config text
fn parse_services(config: &str) -> Vec<String> {
    let mut services = Vec::new();
    let known = ["nginx", "postgresql", "redis", "openssh", "mysql", "caddy", "docker", "prometheus", "grafana"];

    for svc in known {
        let pattern = format!("services.{}.enable = true", svc);
        if config.contains(&pattern) {
            services.push(svc.to_string());
        }
    }
    // Check for enable = true pattern generically
    for line in config.lines() {
        let trimmed = line.trim();
        if trimmed.contains("enable") && trimmed.contains("true") {
            for svc in known {
                if trimmed.contains(svc) && !services.contains(&svc.to_string()) {
                    services.push(svc.to_string());
                }
            }
        }
    }
    services
}

/// Generate a systemd unit file for a service
fn generate_systemd_unit(service: &str, distro: &str) -> String {
    let mapping = service_mappings().into_iter().find(|m| m.nixos_option.contains(service));
    let pkg = mapping.as_ref().map(|m| m.target_pkg).unwrap_or(service);

    format!(
r#"[Unit]
Description={service} service
After=network.target

[Service]
Type=forking
ExecStart=/usr/bin/systemctl start {pkg}
ExecReload=/usr/bin/systemctl reload {pkg}
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
"#,
        service = service,
        pkg = pkg,
    )
}

/// Generate install script for target distro
fn generate_install_script(services: &[String], distro: &str) -> String {
    let pkg_map = pkg_name_map(distro);
    let (pkg_mgr, install_cmd) = match distro {
        "ubuntu" | "debian" => ("apt", "apt-get install -y"),
        "fedora" => ("dnf", "dnf install -y"),
        "arch" => ("pacman", "pacman -S --noconfirm"),
        "alpine" => ("apk", "apk add"),
        _ => ("apt", "apt-get install -y"),
    };

    let packages: Vec<&str> = services.iter()
        .filter_map(|s| pkg_map.get(s.as_str()).copied())
        .collect();

    if packages.is_empty() {
        return format!("# No known packages for services: {}", services.join(", "));
    }

    let mut script = format!("#!/bin/bash\n# Auto-generated install script for {}\n", distro);
    script.push_str(&format!("# Equivalent packages for: {}\n\n", services.join(", ")));

    match distro {
        "ubuntu" | "debian" => script.push_str("apt-get update\n"),
        "fedora" => script.push_str("dnf check-update || true\n"),
        _ => {}
    }

    script.push_str(&format!("{} {}\n\n", install_cmd, packages.join(" ")));
    script.push_str("# Enable services\n");
    for svc in services {
        let pkg = pkg_map.get(svc.as_str()).copied().unwrap_or(svc.as_str());
        let unit = if pkg.ends_with("-server") { format!("{}.service", pkg.replace("-server", "")) } else { format!("{}.service", pkg) };
        script.push_str(&format!("systemctl enable {}\nsystemctl start {}\n", unit, unit));
    }

    script
}

/// POST /api/compat/translate — translate NixOS config to another distro
pub async fn handle_translate(
    State(_state): AppStateRef,
    Json(req): Json<TranslateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let valid_distros = ["ubuntu", "debian", "fedora", "arch", "alpine"];
    if !valid_distros.contains(&req.target_distro.as_str()) {
        return Err(AppError::Validation {
            field: "target_distro".into(),
            message: format!("Unsupported distro '{}'. Supported: {}", req.target_distro, valid_distros.join(", ")),
        });
    }

    // Parse services from config or use explicit list
    let services = req.services.unwrap_or_else(|| parse_services(&req.nixos_config));

    let mut translated_configs = Vec::new();
    let mut package_mapping = Vec::new();
    let mut warnings = Vec::new();
    let mut notes = Vec::new();

    let pkg_map = pkg_name_map(&req.target_distro);

    for svc in &services {
        // Generate systemd unit
        let unit_content = generate_systemd_unit(svc, &req.target_distro);
        translated_configs.push(TranslatedConfig {
            service: svc.clone(),
            config_type: "systemd-unit".into(),
            filename: format!("/etc/systemd/system/{}.service", svc),
            content: unit_content,
        });

        // Map packages
        let target_pkg = pkg_map.get(svc.as_str()).copied().unwrap_or(svc.as_str());
        let match_type = if pkg_map.contains_key(svc.as_str()) { "alias" } else { "manual" };
        package_mapping.push(PackageMapping {
            nixos_package: svc.clone(),
            target_package: target_pkg.to_string(),
            match_type: match_type.to_string(),
        });

        if match_type == "manual" {
            warnings.push(format!("No known package mapping for '{}' on {}. Manual lookup needed.", svc, req.target_distro));
        }
    }

    // Generate install script
    let install_script = generate_install_script(&services, &req.target_distro);
    translated_configs.push(TranslatedConfig {
        service: "_install".into(),
        config_type: "script".into(),
        filename: "install.sh".into(),
        content: install_script,
    });

    notes.push(format!("NixOS declarative config cannot be 1:1 translated to {}. These are approximations.", req.target_distro));
    notes.push("NixOS rollbacks (generation-based) have no equivalent on traditional distros.".into());
    notes.push("Consider using Ansible/Puppet/Chef for declarative management on traditional distros.".into());

    Ok(Json(serde_json::to_value(TranslateResponse {
        source: "nixos".into(),
        target_distro: req.target_distro,
        translated_configs,
        package_mapping,
        warnings,
        notes,
    }).unwrap_or_default()))
}
