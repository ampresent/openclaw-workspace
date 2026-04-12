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

#[cfg(test)]
mod tests {
    use super::*;

    // ─── service_mappings ────────────────────────────────────────────

    #[test]
    fn test_service_mappings_non_empty() {
        let mappings = service_mappings();
        assert!(!mappings.is_empty());
    }

    #[test]
    fn test_service_mappings_contain_nginx() {
        let mappings = service_mappings();
        let nginx = mappings.iter().find(|m| m.nixos_option.contains("nginx"));
        assert!(nginx.is_some());
        let nginx = nginx.unwrap();
        assert_eq!(nginx.target_pkg, "nginx");
        assert_eq!(nginx.systemd_unit, "nginx.service");
    }

    #[test]
    fn test_service_mappings_contain_postgresql() {
        let mappings = service_mappings();
        let pg = mappings.iter().find(|m| m.nixos_option.contains("postgresql"));
        assert!(pg.is_some());
        assert_eq!(pg.unwrap().target_pkg, "postgresql");
    }

    #[test]
    fn test_service_mappings_contain_redis() {
        let mappings = service_mappings();
        let redis = mappings.iter().find(|m| m.nixos_option.contains("redis"));
        assert!(redis.is_some());
        assert_eq!(redis.unwrap().target_pkg, "redis-server");
    }

    // ─── pkg_name_map ───────────────────────────────────────────────

    #[test]
    fn test_pkg_map_ubuntu_has_nginx() {
        let map = pkg_name_map("ubuntu");
        assert_eq!(map.get("nginx"), Some(&"nginx"));
    }

    #[test]
    fn test_pkg_map_ubuntu_has_redis() {
        let map = pkg_name_map("ubuntu");
        assert_eq!(map.get("redis"), Some(&"redis-server"));
    }

    #[test]
    fn test_pkg_map_fedora_has_docker() {
        let map = pkg_name_map("fedora");
        assert_eq!(map.get("docker"), Some(&"docker-ce"));
    }

    #[test]
    fn test_pkg_map_arch_has_nginx() {
        let map = pkg_name_map("arch");
        assert_eq!(map.get("nginx"), Some(&"nginx"));
    }

    #[test]
    fn test_pkg_map_alpine_has_nginx() {
        let map = pkg_name_map("alpine");
        assert_eq!(map.get("nginx"), Some(&"nginx"));
    }

    #[test]
    fn test_pkg_map_unknown_distro_empty() {
        let map = pkg_name_map("windows");
        assert!(map.is_empty());
    }

    #[test]
    fn test_pkg_map_debian_same_as_ubuntu() {
        let ubuntu = pkg_name_map("ubuntu");
        let debian = pkg_name_map("debian");
        assert_eq!(ubuntu.get("redis"), debian.get("redis"));
        assert_eq!(ubuntu.get("nginx"), debian.get("nginx"));
    }

    // ─── parse_services ─────────────────────────────────────────────

    #[test]
    fn test_parse_services_nginx() {
        let config = "{ services.nginx.enable = true; }";
        let services = parse_services(config);
        assert!(services.contains(&"nginx".to_string()));
    }

    #[test]
    fn test_parse_services_multiple() {
        let config = r#"{
            services.nginx.enable = true;
            services.postgresql.enable = true;
            services.redis.enable = true;
        }"#;
        let services = parse_services(config);
        assert!(services.contains(&"nginx".to_string()));
        assert!(services.contains(&"postgresql".to_string()));
        assert!(services.contains(&"redis".to_string()));
    }

    #[test]
    fn test_parse_services_empty_config() {
        let config = "{ environment.systemPackages = [ pkgs.vim ]; }";
        let services = parse_services(config);
        assert!(services.is_empty());
    }

    #[test]
    fn test_parse_services_disabled() {
        let config = "{ services.nginx.enable = false; }";
        let services = parse_services(config);
        assert!(!services.contains(&"nginx".to_string()));
    }

    // ─── generate_systemd_unit ──────────────────────────────────────

    #[test]
    fn test_generate_unit_contains_service_name() {
        let unit = generate_systemd_unit("nginx", "ubuntu");
        assert!(unit.contains("nginx.service"));
        assert!(unit.contains("[Unit]"));
        assert!(unit.contains("[Service]"));
        assert!(unit.contains("[Install]"));
    }

    #[test]
    fn test_generate_unit_has_network_target() {
        let unit = generate_systemd_unit("redis", "debian");
        assert!(unit.contains("After=network.target"));
    }

    #[test]
    fn test_generate_unit_has_restart() {
        let unit = generate_systemd_unit("postgresql", "fedora");
        assert!(unit.contains("Restart=on-failure"));
    }

    // ─── generate_install_script ────────────────────────────────────

    #[test]
    fn test_install_script_ubuntu_uses_apt() {
        let services = vec!["nginx".to_string()];
        let script = generate_install_script(&services, "ubuntu");
        assert!(script.contains("apt-get install -y"));
        assert!(script.contains("nginx"));
    }

    #[test]
    fn test_install_script_fedora_uses_dnf() {
        let services = vec!["nginx".to_string()];
        let script = generate_install_script(&services, "fedora");
        assert!(script.contains("dnf install -y"));
    }

    #[test]
    fn test_install_script_arch_uses_pacman() {
        let services = vec!["nginx".to_string()];
        let script = generate_install_script(&services, "arch");
        assert!(script.contains("pacman -S --noconfirm"));
    }

    #[test]
    fn test_install_script_alpine_uses_apk() {
        let services = vec!["nginx".to_string()];
        let script = generate_install_script(&services, "alpine");
        assert!(script.contains("apk add"));
    }

    #[test]
    fn test_install_script_empty_services() {
        let services: Vec<String> = vec![];
        let script = generate_install_script(&services, "ubuntu");
        assert!(script.contains("# No known packages"));
    }

    #[test]
    fn test_install_script_enables_services() {
        let services = vec!["nginx".to_string()];
        let script = generate_install_script(&services, "ubuntu");
        assert!(script.contains("systemctl enable"));
        assert!(script.contains("systemctl start"));
    }

    #[test]
    fn test_install_script_apt_update() {
        let services = vec!["nginx".to_string()];
        let script = generate_install_script(&services, "ubuntu");
        assert!(script.contains("apt-get update"));
    }
}
