/// Interactive Config Builder — WebSocket-based wizard for NixOS configuration
///
/// State machine per connection:
/// Choose services → Configure ports → Set options → Preview → Apply

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::sync::broadcast;

use crate::cmd;

// ─── State Machine ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuilderStep {
    Welcome,
    SelectServices,
    ConfigurePorts,
    SetOptions,
    Review,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub enabled: bool,
    pub ports: Vec<u16>,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderState {
    pub step: BuilderStep,
    pub selected_services: Vec<String>,
    pub service_configs: HashMap<String, ServiceConfig>,
    pub hostname: String,
    pub preview: Option<String>,
    pub errors: Vec<String>,
}

impl Default for BuilderState {
    fn default() -> Self {
        Self {
            step: BuilderStep::Welcome,
            selected_services: Vec::new(),
            service_configs: HashMap::new(),
            hostname: "nixos".into(),
            preview: None,
            errors: Vec::new(),
        }
    }
}

// ─── Available Services ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ServiceOption {
    pub name: String,
    pub category: String,
    pub description: String,
    pub default_ports: Vec<u16>,
    pub options: Vec<OptionSpec>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptionSpec {
    pub key: String,
    pub description: String,
    pub default: String,
    pub option_type: String, // "bool", "string", "int", "select"
    pub choices: Option<Vec<String>>,
}

pub fn available_services() -> Vec<ServiceOption> {
    vec![
        ServiceOption {
            name: "nginx".into(),
            category: "Web Server".into(),
            description: "高性能 HTTP 和反向代理服务器".into(),
            default_ports: vec![80, 443],
            options: vec![
                OptionSpec { key: "enableACME".into(), description: "自动 Let's Encrypt 证书".into(), default: "false".into(), option_type: "bool".into(), choices: None },
                OptionSpec { key: "recommendedProxySettings".into(), description: "推荐代理设置".into(), default: "true".into(), option_type: "bool".into(), choices: None },
                OptionSpec { key: "recommendedOptimisation".into(), description: "推荐优化".into(), default: "true".into(), option_type: "bool".into(), choices: None },
            ],
        },
        ServiceOption {
            name: "postgresql".into(),
            category: "Database".into(),
            description: "高级开源关系型数据库".into(),
            default_ports: vec![5432],
            options: vec![
                OptionSpec { key: "package".into(), description: "PostgreSQL 版本".into(), default: "pkgs.postgresql_16".into(), option_type: "select".into(), choices: Some(vec!["pkgs.postgresql_15".into(), "pkgs.postgresql_16".into(), "pkgs.postgresql_17".into()]) },
                OptionSpec { key: "enableTCPIP".into(), description: "启用 TCP/IP 连接".into(), default: "false".into(), option_type: "bool".into(), choices: None },
            ],
        },
        ServiceOption {
            name: "redis".into(),
            category: "Cache".into(),
            description: "内存数据结构存储".into(),
            default_ports: vec![6379],
            options: vec![
                OptionSpec { key: "bind".into(), description: "监听地址".into(), default: "127.0.0.1".into(), option_type: "string".into(), choices: None },
            ],
        },
        ServiceOption {
            name: "openssh".into(),
            category: "Remote Access".into(),
            description: "SSH 远程访问服务".into(),
            default_ports: vec![22],
            options: vec![
                OptionSpec { key: "permitRootLogin".into(), description: "允许 root 登录".into(), default: "no".into(), option_type: "select".into(), choices: Some(vec!["yes".into(), "no".into(), "prohibit-password".into()]) },
                OptionSpec { key: "passwordAuthentication".into(), description: "密码认证".into(), default: "false".into(), option_type: "bool".into(), choices: None },
            ],
        },
        ServiceOption {
            name: "grafana".into(),
            category: "Monitoring".into(),
            description: "数据可视化和监控平台".into(),
            default_ports: vec![3000],
            options: vec![
                OptionSpec { key: "domain".into(), description: "域名".into(), default: "localhost".into(), option_type: "string".into(), choices: None },
            ],
        },
        ServiceOption {
            name: "prometheus".into(),
            category: "Monitoring".into(),
            description: "监控系统和时间序列数据库".into(),
            default_ports: vec![9090],
            options: vec![
                OptionSpec { key: "retentionTime".into(), description: "数据保留时间".into(), default: "15d".into(), option_type: "string".into(), choices: None },
            ],
        },
        ServiceOption {
            name: "docker".into(),
            category: "Virtualization".into(),
            description: "容器运行时".into(),
            default_ports: vec![],
            options: vec![
                OptionSpec { key: "enableOnBoot".into(), description: "开机启动 Docker".into(), default: "true".into(), option_type: "bool".into(), choices: None },
                OptionSpec { key: "rootless".into(), description: "Rootless 模式".into(), default: "false".into(), option_type: "bool".into(), choices: None },
            ],
        },
        ServiceOption {
            name: "caddy".into(),
            category: "Web Server".into(),
            description: "自动 HTTPS 的现代 Web 服务器".into(),
            default_ports: vec![80, 443],
            options: vec![
                OptionSpec { key: "email".into(), description: "ACME 邮箱".into(), default: "".into(), option_type: "string".into(), choices: None },
            ],
        },
        ServiceOption {
            name: "fail2ban".into(),
            category: "Security".into(),
            description: "入侵检测和防护".into(),
            default_ports: vec![],
            options: vec![
                OptionSpec { key: "maxretry".into(), description: "最大重试次数".into(), default: "5".into(), option_type: "int".into(), choices: None },
                OptionSpec { key: "bantime".into(), description: "封禁时间（秒）".into(), default: "3600".into(), option_type: "int".into(), choices: None },
            ],
        },
        ServiceOption {
            name: "mysql".into(),
            category: "Database".into(),
            description: "流行的关系型数据库".into(),
            default_ports: vec![3306],
            options: vec![
                OptionSpec { key: "package".into(), description: "数据库版本".into(), default: "pkgs.mariadb".into(), option_type: "select".into(), choices: Some(vec!["pkgs.mysql80".into(), "pkgs.mariadb".into()]) },
            ],
        },
    ]
}

// ─── Nix Config Generator ─────────────────────────────────────────────────

pub fn generate_config(state: &BuilderState) -> String {
    let mut config = String::new();
    config.push_str(&format!(
        "# Generated by nix-evo Config Builder\n# Hostname: {}\n\n{{ config, pkgs, ... }}:\n\n{{\n",
        state.hostname
    ));

    config.push_str(&format!("  networking.hostName = \"{}\";\n\n", state.hostname));

    // Firewall
    let all_ports: Vec<u16> = state.service_configs.values()
        .flat_map(|s| s.ports.iter().copied())
        .collect();
    if !all_ports.is_empty() {
        config.push_str("  networking.firewall.enable = true;\n");
        config.push_str(&format!("  networking.firewall.allowedTCPPorts = [ {} ];\n\n",
            all_ports.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(" ")));
    }

    // Services
    for (name, svc) in &state.service_configs {
        if !svc.enabled { continue; }
        config.push_str(&format!("  services.{}.enable = true;\n", name));
        for (key, val) in &svc.options {
            if val == "true" || val == "false" {
                config.push_str(&format!("  services.{}.{} = {};\n", name, key, val));
            } else if val.parse::<i64>().is_ok() {
                config.push_str(&format!("  services.{}.{} = {};\n", name, key, val));
            } else if !val.is_empty() {
                config.push_str(&format!("  services.{}.{} = \"{}\";\n", name, key, val));
            }
        }
        config.push('\n');
    }

    config.push_str("}\n");
    config
}

// ─── WebSocket Handler ────────────────────────────────────────────────────

static ACTIVE_CONNECTIONS: AtomicU64 = AtomicU64::new(0);
static BROADCAST_TX: OnceLock<broadcast::Sender<String>> = OnceLock::new();

fn get_broadcast() -> broadcast::Sender<String> {
    BROADCAST_TX.get_or_init(|| broadcast::channel(100).0).clone()
}

pub async fn handle_ws(ws: WebSocketUpgrade, Query(q): Query<HashMap<String, String>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_connection(socket, q))
}

async fn handle_connection(mut socket: WebSocket, _q: HashMap<String, String>) {
    ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
    let mut state = BuilderState::default();

    // Send welcome
    let welcome = serde_json::json!({
        "type": "welcome",
        "message": "🎮 nix-evo Interactive Config Builder",
        "steps": ["welcome", "select_services", "configure_ports", "set_options", "review", "done"],
        "services": available_services(),
    });
    let _ = socket.send(Message::Text(welcome.to_string())).await;

    let mut rx = get_broadcast().subscribe();

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(&text) {
                            let response = process_command(&mut state, &cmd).await;
                            if socket.send(Message::Text(response.to_string())).await.is_err() { break; }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            Ok(broadcast_msg) = rx.recv() => {
                let _ = socket.send(Message::Text(broadcast_msg)).await;
            }
        }
    }

    ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
}

async fn process_command(state: &mut BuilderState, cmd: &serde_json::Value) -> serde_json::Value {
    let action = cmd.get("action").and_then(|a| a.as_str()).unwrap_or("");

    match action {
        "start" => {
            state.step = BuilderStep::SelectServices;
            serde_json::json!({
                "type": "step",
                "step": "select_services",
                "message": "选择需要启用的服务（发送 {\"action\":\"select\",\"services\":[\"nginx\",\"openssh\"]}）",
                "services": available_services(),
            })
        }
        "select" => {
            if let Some(services) = cmd.get("services").and_then(|s| s.as_array()) {
                state.selected_services = services.iter()
                    .filter_map(|s| s.as_str().map(String::from))
                    .collect();
                for svc_name in &state.selected_services {
                    if let Some(svc_info) = available_services().iter().find(|s| &s.name == svc_name) {
                        let mut options = HashMap::new();
                        for opt in &svc_info.options {
                            options.insert(opt.key.clone(), opt.default.clone());
                        }
                        state.service_configs.insert(svc_name.clone(), ServiceConfig {
                            name: svc_name.clone(),
                            enabled: true,
                            ports: svc_info.default_ports.clone(),
                            options,
                        });
                    }
                }
                state.step = BuilderStep::ConfigurePorts;
                serde_json::json!({
                    "type": "step",
                    "step": "configure_ports",
                    "message": "配置端口（发送 {\"action\":\"set_ports\",\"service\":\"nginx\",\"ports\":[80,443]}）",
                    "configs": state.service_configs,
                })
            } else {
                serde_json::json!({"type": "error", "message": "Missing 'services' array"})
            }
        }
        "set_ports" => {
            if let (Some(svc), Some(ports)) = (cmd.get("service").and_then(|s| s.as_str()),
                cmd.get("ports").and_then(|p| p.as_array())) {
                if let Some(config) = state.service_configs.get_mut(svc) {
                    config.ports = ports.iter().filter_map(|p| p.as_u64().map(|n| n as u16)).collect();
                }
            }
            serde_json::json!({"type": "ack", "message": "Ports updated", "configs": state.service_configs})
        }
        "set_option" => {
            if let (Some(svc), Some(key), Some(val)) = (
                cmd.get("service").and_then(|s| s.as_str()),
                cmd.get("key").and_then(|k| k.as_str()),
                cmd.get("value").and_then(|v| v.as_str()),
            ) {
                if let Some(config) = state.service_configs.get_mut(svc) {
                    config.options.insert(key.to_string(), val.to_string());
                }
            }
            serde_json::json!({"type": "ack", "message": "Option updated"})
        }
        "next" => {
            state.step = match state.step {
                BuilderStep::SelectServices => BuilderStep::ConfigurePorts,
                BuilderStep::ConfigurePorts => BuilderStep::SetOptions,
                BuilderStep::SetOptions => BuilderStep::Review,
                BuilderStep::Review => BuilderStep::Done,
                _ => state.step.clone(),
            };
            serde_json::json!({
                "type": "step",
                "step": format!("{:?}", state.step).to_lowercase(),
                "configs": state.service_configs,
            })
        }
        "preview" => {
            let config = generate_config(state);
            state.preview = Some(config.clone());
            serde_json::json!({
                "type": "preview",
                "config": config,
                "message": "配置预览生成完成。确认无误后发送 {\"action\":\"apply\"}",
            })
        }
        "apply" => {
            if let Some(config) = &state.preview {
                let backup = cmd::run_cmd("bash", &["-c",
                    &format!("cp /etc/nixos/configuration.nix /etc/nixos/configuration.nix.bak.{}", chrono::Utc::now().timestamp())
                ]).await;

                state.step = BuilderStep::Done;
                serde_json::json!({
                    "type": "applied",
                    "success": backup.is_ok(),
                    "message": if backup.is_ok() { "配置已备份，新配置已生成" } else { "备份失败，请手动操作" },
                    "config": config,
                })
            } else {
                serde_json::json!({"type": "error", "message": "请先预览配置（发送 {\"action\":\"preview\"}）"})
            }
        }
        "reset" => {
            *state = BuilderState::default();
            serde_json::json!({"type": "reset", "message": "已重置，发送 {\"action\":\"start\"} 重新开始"})
        }
        _ => serde_json::json!({
            "type": "help",
            "actions": ["start", "select", "set_ports", "set_option", "next", "preview", "apply", "reset"],
            "message": "可用操作列表",
        })
    }
}

pub fn active_connections() -> u64 {
    ACTIVE_CONNECTIONS.load(Ordering::Relaxed)
}
