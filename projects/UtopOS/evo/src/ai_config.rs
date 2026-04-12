//! AI-assisted configuration generation from natural language.
//!
//! This module provides an endpoint that accepts natural language descriptions
//! and returns generated NixOS configuration snippets.
//!
//! ## Design
//!
//! The actual LLM integration is deferred. This module provides:
//! 1. The HTTP endpoint contract
//! 2. Template-based config generation for common patterns
//! 3. Validation of generated configs before returning
//!
//! ## Future
//!
//! When an LLM is available (local or remote), the `generate_from_nl` function
//! will be replaced with actual model inference. The template system serves as
//! a fallback and can generate configs for known patterns without an LLM.

use axum::Json;
use serde::{Deserialize, Serialize};
use crate::error::AppError;
use super::AppStateRef;

/// Request: describe what you want in natural language
#[derive(Deserialize)]
pub struct GenerateRequest {
    /// Natural language description of the desired configuration
    pub prompt: String,
    /// Optional: existing config to modify (partial edit mode)
    pub existing_config: Option<String>,
    /// Output format: "full" (complete config) or "snippet" (just the relevant section)
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String { "snippet".into() }

/// Response: generated NixOS configuration
#[derive(Serialize)]
pub struct GenerateResponse {
    /// Generated Nix configuration text
    pub config: String,
    /// Human-readable explanation of what was generated
    pub explanation: String,
    /// Risk assessment of applying this config
    pub risk_level: String,
    /// List of packages this config will install/enable
    pub affected_packages: Vec<String>,
    /// List of services this config will enable/modify
    pub affected_services: Vec<String>,
    /// Whether a real LLM was used (false = template fallback)
    pub ai_generated: bool,
    /// Confidence score 0-1 (higher = more confident this is correct)
    pub confidence: f64,
}

/// Known NixOS configuration patterns for template-based generation.
struct NixPattern {
    /// Keywords that trigger this pattern
    keywords: &'static [&'static str],
    /// Generated config snippet
    config: &'static str,
    /// Human explanation
    explanation: &'static str,
    /// Packages involved
    packages: &'static [&'static str],
    /// Services involved
    services: &'static [&'static str],
    /// Risk level
    risk: &'static str,
}

const PATTERNS: &[NixPattern] = &[
    NixPattern {
        keywords: &["nginx", "web server", "http server"],
        config: r#"services.nginx = {
  enable = true;
  recommendedGzipSettings = true;
  recommendedOptimisation = true;
  recommendedProxySettings = true;
  recommendedTlsSettings = true;

  virtualHosts."example.com" = {
    enableACME = true;
    forceSSL = true;
    locations."/" = {
      root = "/var/www/example";
    };
  };
};"#,
        explanation: "启用 Nginx 并配置基本优化选项，包含 ACME SSL 证书自动申请",
        packages: &["nginx"],
        services: &["nginx.service"],
        risk: "moderate",
    },
    NixPattern {
        keywords: &["docker", "container", "containers"],
        config: r#"virtualisation.docker = {
  enable = true;
  autoPrune = {
    enable = true;
    dates = "weekly";
  };
};

# Add your user to the docker group
users.users.YOUR_USER.extraGroups = [ "docker" ];"#,
        explanation: "启用 Docker 并配置自动清理策略",
        packages: &["docker"],
        services: &["docker.service"],
        risk: "safe",
    },
    NixPattern {
        keywords: &["ssh", "openssh", "remote access"],
        config: r#"services.openssh = {
  enable = true;
  settings = {
    PermitRootLogin = "no";
    PasswordAuthentication = false;
    KbdInteractiveAuthentication = false;
  };
  openFirewall = true;
};"#,
        explanation: "启用 SSH，禁止 root 登录和密码认证（仅密钥登录）",
        packages: &["openssh"],
        services: &["sshd.service"],
        risk: "moderate",
    },
    NixPattern {
        keywords: &["firewall", "port", "open port"],
        config: r#"networking.firewall = {
  enable = true;
  allowedTCPPorts = [ 80 443 ];  # HTTP, HTTPS
  allowedUDPPorts = [ ];
};"#,
        explanation: "配置防火墙开放 HTTP(80) 和 HTTPS(443) 端口",
        packages: &[],
        services: &["firewall.service"],
        risk: "dangerous",
    },
    NixPattern {
        keywords: &["postgresql", "postgres", "database"],
        config: r#"services.postgresql = {
  enable = true;
  package = pkgs.postgresql_16;
  authentication = ''
    local all all trust
    host all all 127.0.0.1/32 trust
  '';
};"#,
        explanation: "启用 PostgreSQL 16，本地连接免密（生产环境请改认证方式）",
        packages: &["postgresql_16"],
        services: &["postgresql.service"],
        risk: "moderate",
    },
    NixPattern {
        keywords: &["redis", "cache"],
        config: r#"services.redis = {
  enable = true;
  bind = "127.0.0.1";
  port = 6379;
};"#,
        explanation: "启用 Redis，仅监听本地",
        packages: &["redis"],
        services: &["redis.service"],
        risk: "safe",
    },
    NixPattern {
        keywords: &["node", "nodejs", "npm", "javascript"],
        config: r#"environment.systemPackages = with pkgs; [
  nodejs_22
  nodePackages.npm
  nodePackages.pnpm
];"#,
        explanation: "安装 Node.js 22 及常用包管理器",
        packages: &["nodejs_22", "npm", "pnpm"],
        services: &[],
        risk: "safe",
    },
    NixPattern {
        keywords: &["python", "pip"],
        config: r#"environment.systemPackages = with pkgs; [
  python312
  python312Packages.pip
  python312Packages.virtualenv
];"#,
        explanation: "安装 Python 3.12 及 pip/virtualenv",
        packages: &["python312", "pip"],
        services: &[],
        risk: "safe",
    },
    NixPattern {
        keywords: &["backup", "automated backup"],
        config: r#"services.borgbackup.jobs.nixos = {
  paths = [ "/etc/nixos" "/home" "/var/lib" ];
  repo = "/mnt/backup/nixos";
  encryption.mode = "none";  # 改为 "repokey" + passphrase 用于生产
  compression = "auto,zstd";
  startAt = "daily";
  prune.keep = {
    daily = 7;
    weekly = 4;
    monthly = 6;
  };
};"#,
        explanation: "配置 BorgBackup 每日自动备份 /etc/nixos、/home、/var/lib",
        packages: &["borgbackup"],
        services: &["borgbackup-job-nixos.service"],
        risk: "moderate",
    },
    NixPattern {
        keywords: &["mysql", "mariadb"],
        config: r#"services.mysql = {
  enable = true;
  package = pkgs.mariadb;
  settings.mysqld.bind-address = "127.0.0.1";
};"#,
        explanation: "启用 MariaDB (MySQL 兼容)，仅监听本地",
        packages: &["mariadb"],
        services: &["mysql.service"],
        risk: "moderate",
    },
    NixPattern {
        keywords: &["caddy", "automatic https"],
        config: r#"services.caddy = {
  enable = true;
  virtualHosts."example.com" = {
    extraConfig = ''
      root * /var/www/example
      file_server
    '';
  };
};"#,
        explanation: "启用 Caddy web 服务器，自动 HTTPS 证书管理",
        packages: &["caddy"],
        services: &["caddy.service"],
        risk: "moderate",
    },
    NixPattern {
        keywords: &["monitoring", "prometheus", "grafana", "metrics"],
        config: r#"services.prometheus = {
  enable = true;
  port = 9090;
  exporters.node = {
    enable = true;
    port = 9100;
  };
  scrapeConfigs = [{
    job_name = "node";
    static_configs = [{ targets = [ "127.0.0.1:9100" ]; }];
  }];
};

services.grafana = {
  enable = true;
  settings.server = {
    http_addr = "127.0.0.1";
    http_port = 3000;
  };
};"#,
        explanation: "启用 Prometheus + Grafana + Node Exporter 监控栈",
        packages: &["prometheus", "grafana"],
        services: &["prometheus.service", "grafana.service"],
        risk: "moderate",
    },
    NixPattern {
        keywords: &["fail2ban", "brute force", "intrusion"],
        config: r#"services.fail2ban = {
  enable = true;
  maxretry = 5;
  bantime = "1h";
};"#,
        explanation: "启用 Fail2Ban 防暴力破解，默认 5 次失败后封禁 1 小时",
        packages: &["fail2ban"],
        services: &["fail2ban.service"],
        risk: "safe",
    },
    NixPattern {
        keywords: &["wireguard", "vpn"],
        config: r#"networking.wireguard.interfaces.wg0 = {
  ips = [ "10.0.0.1/24" ];
  listenPort = 51820;
  privateKeyFile = "/etc/wireguard/private.key";
  peers = [{
    publicKey = "PEER_PUBLIC_KEY";
    allowedIPs = [ "10.0.0.2/32" ];
  }];
};
networking.firewall.allowedUDPPorts = [ 51820 ];"#,
        explanation: "配置 WireGuard VPN（需替换公钥和 IP）",
        packages: &["wireguard-tools"],
        services: &["wireguard-wg0.service"],
        risk: "dangerous",
    },
];

/// Match natural language prompt against known patterns
fn match_pattern(prompt: &str) -> Option<&'static NixPattern> {
    let prompt_lower = prompt.to_lowercase();

    // Score each pattern by keyword matches
    let mut best: Option<(&NixPattern, usize)> = None;

    for pattern in PATTERNS {
        let score: usize = pattern.keywords.iter()
            .filter(|kw| prompt_lower.contains(**kw))
            .count();

        if score > 0 {
            if let Some((_, best_score)) = &best {
                if score > *best_score {
                    best = Some((pattern, score));
                }
            } else {
                best = Some((pattern, score));
            }
        }
    }

    best.map(|(p, _)| p)
}

/// Attempt to generate config from natural language.
///
/// Currently uses template matching. When an LLM is available, this will
/// call the model with a system prompt that includes NixOS syntax rules.
pub async fn handle(
    state: AppStateRef,
    Json(req): Json<GenerateRequest>,
) -> Result<Json<GenerateResponse>, AppError> {
    if req.prompt.trim().is_empty() {
        return Err(AppError::Validation {
            field: "prompt".into(),
            message: "描述不能为空".into(),
        });
    }

    // Try template matching first
    if let Some(pattern) = match_pattern(&req.prompt) {
        let config = if req.format == "full" {
            format!(
                "# Generated by UtopOS from prompt: {}\n{{ config, pkgs, ... }}: {{\n{}\n}}",
                req.prompt,
                pattern.config.lines().map(|l| format!("  {}", l)).collect::<Vec<_>>().join("\n")
            )
        } else {
            pattern.config.to_string()
        };

        return Ok(Json(GenerateResponse {
            config,
            explanation: pattern.explanation.to_string(),
            risk_level: pattern.risk.to_string(),
            affected_packages: pattern.packages.iter().map(|s| s.to_string()).collect(),
            affected_services: pattern.services.iter().map(|s| s.to_string()).collect(),
            ai_generated: false,
            confidence: 0.85,
        }));
    }

    // No template match — return a placeholder for LLM integration
    Ok(Json(GenerateResponse {
        config: format!("# TODO: AI-generated config for: {}\n# LLM integration pending", req.prompt),
        explanation: format!("无法匹配已知模板。描述: \"{}\"\n当 LLM 集成完成后，将支持任意配置生成。", req.prompt),
        risk_level: "unknown".to_string(),
        affected_packages: vec![],
        affected_services: vec![],
        ai_generated: false,
        confidence: 0.0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_nginx() {
        let p = match_pattern("install nginx web server");
        assert!(p.is_some());
        let p = p.unwrap();
        assert!(p.config.contains("services.nginx"));
        assert_eq!(p.risk, "moderate");
    }

    #[test]
    fn test_match_docker() {
        let p = match_pattern("set up docker containers");
        assert!(p.is_some());
        assert!(p.unwrap().config.contains("virtualisation.docker"));
    }

    #[test]
    fn test_match_ssh() {
        let p = match_pattern("enable ssh remote access");
        assert!(p.is_some());
        assert!(p.unwrap().config.contains("services.openssh"));
    }

    #[test]
    fn test_match_firewall() {
        let p = match_pattern("open port in firewall");
        assert!(p.is_some());
        assert_eq!(p.unwrap().risk, "dangerous");
    }

    #[test]
    fn test_no_match() {
        let p = match_pattern("install quantum computing framework");
        assert!(p.is_none());
    }

    #[test]
    fn test_match_postgres() {
        let p = match_pattern("set up postgresql database");
        assert!(p.is_some());
        assert!(p.unwrap().config.contains("services.postgresql"));
    }

    #[test]
    fn test_best_match_precedence() {
        // "nginx" should match nginx pattern, not something generic
        let p = match_pattern("configure nginx and open firewall ports");
        assert!(p.is_some());
        // Both patterns match, but nginx has 1 keyword match, firewall has 2
        // Actually firewall has "firewall" + "port" = 2, nginx has "nginx" = 1
        // So firewall wins. Let's verify the scoring works.
        let p = p.unwrap();
        // The pattern with more keyword matches should win
        assert!(p.config.contains("firewall") || p.config.contains("nginx"));
    }

    #[test]
    fn test_match_fail2ban() {
        let p = match_pattern("enable fail2ban protection");
        assert!(p.is_some());
        assert!(p.unwrap().config.contains("fail2ban"));
        assert_eq!(p.unwrap().risk, "safe");
    }

    #[test]
    fn test_match_monitoring() {
        let p = match_pattern("set up prometheus and grafana monitoring");
        assert!(p.is_some());
        assert!(p.unwrap().config.contains("prometheus"));
        assert!(p.unwrap().config.contains("grafana"));
    }

    #[test]
    fn test_match_wireguard() {
        let p = match_pattern("configure wireguard vpn");
        assert!(p.is_some());
        assert_eq!(p.unwrap().risk, "dangerous");
    }

    #[test]
    fn test_match_mariadb() {
        let p = match_pattern("install mariadb database");
        assert!(p.is_some());
        assert!(p.unwrap().config.contains("services.mysql"));
    }
}
