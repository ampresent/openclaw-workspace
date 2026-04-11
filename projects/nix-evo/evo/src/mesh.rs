use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::cmd::run_cmd;
use crate::error::AppError;

/// A discovered network service endpoint
#[derive(Debug, Clone, Serialize)]
pub struct ServiceEndpoint {
    pub name: String,
    pub pid: Option<u32>,
    pub listen_addresses: Vec<String>,
    pub protocol: String,    // "tcp", "udp", "unix"
    pub state: String,       // "LISTEN", "ESTABLISHED", etc.
    pub connections_in: usize,
    pub connections_out: usize,
}

/// A network connection between services
#[derive(Debug, Clone, Serialize)]
pub struct ServiceConnection {
    pub from: String,
    pub to: String,
    pub from_addr: String,
    pub to_addr: String,
    pub protocol: String,
    pub state: String,
}

/// Full mesh topology
#[derive(Debug, Serialize)]
pub struct MeshTopology {
    pub services: Vec<ServiceEndpoint>,
    pub connections: Vec<ServiceConnection>,
    pub internal_connections: usize,
    pub external_connections: usize,
    pub unix_socket_connections: usize,
    pub summary: MeshSummary,
}

#[derive(Debug, Serialize)]
pub struct MeshSummary {
    pub total_listeners: usize,
    pub total_connections: usize,
    pub services_with_external: Vec<String>,
    pub isolated_services: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct MeshQuery {
    pub include_connections: Option<bool>,
    pub protocol_filter: Option<String>,
}

/// Discover service mesh topology from /proc and ss
pub async fn discover_topology(query: &MeshQuery) -> Result<MeshTopology, AppError> {
    let mut services = discover_services().await;
    let mut connections = Vec::new();

    if query.include_connections.unwrap_or(true) {
        connections = discover_connections().await;
    }

    // Count connections per service
    for svc in &mut services {
        svc.connections_in = connections.iter()
            .filter(|c| c.to == svc.name)
            .count();
        svc.connections_out = connections.iter()
            .filter(|c| c.from == svc.name)
            .count();
    }

    // Calculate stats
    let internal = connections.iter()
        .filter(|c| is_internal_addr(&c.from_addr) && is_internal_addr(&c.to_addr))
        .count();
    let external = connections.iter()
        .filter(|c| !is_internal_addr(&c.from_addr) || !is_internal_addr(&c.to_addr))
        .count();
    let unix = connections.iter()
        .filter(|c| c.protocol == "unix")
        .count();

    // Find services with external connections
    let services_with_external: Vec<String> = services.iter()
        .filter(|s| connections.iter().any(|c| {
            (c.from == s.name || c.to == s.name) &&
            (!is_internal_addr(&c.from_addr) || !is_internal_addr(&c.to_addr))
        }))
        .map(|s| s.name.clone())
        .collect();

    // Find isolated services (listening but no connections)
    let isolated: Vec<String> = services.iter()
        .filter(|s| s.connections_in == 0 && s.connections_out == 0)
        .map(|s| s.name.clone())
        .collect();

    Ok(MeshTopology {
        services,
        connections,
        internal_connections: internal,
        external_connections: external,
        unix_socket_connections: unix,
        summary: MeshSummary {
            total_listeners: services.len(),
            total_connections: connections.len(),
            services_with_external,
            isolated_services: isolated,
        },
    })
}

/// Discover listening services from /proc/net/tcp and ss
async fn discover_services() -> Vec<ServiceEndpoint> {
    let mut endpoints = Vec::new();

    // Use ss to get listening sockets with process info
    let ss_output = match tokio::process::Command::new("ss")
        .args(&["-tlnp"])
        .output()
        .await
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => {
            // Fallback: parse /proc/net/tcp
            return discover_from_proc().await;
        }
    };

    for line in ss_output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 { continue; }

        let local_addr = parts[3].to_string();
        let state = parts[0].to_string();

        // Extract PID and process name from users field
        let (pid, name) = parse_process_info(parts.last().unwrap_or(&""));

        endpoints.push(ServiceEndpoint {
            name: name.unwrap_or_else(|| "unknown".into()),
            pid,
            listen_addresses: vec![local_addr.clone()],
            protocol: if line.contains("udp") { "udp".into() } else { "tcp".into() },
            state,
            connections_in: 0,
            connections_out: 0,
        });
    }

    // Merge endpoints with same process name
    merge_endpoints(endpoints)
}

async fn discover_from_proc() -> Vec<ServiceEndpoint> {
    let mut endpoints = Vec::new();

    for proto in &["tcp", "tcp6", "udp", "udp6"] {
        let path = format!("/proc/net/{proto}");
        if let Ok(content) = tokio::fs::read_to_string(&path).await {
            for line in content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 { continue; }

                let local_hex = parts[1];
                let state_hex = parts[3];

                if let Some(addr) = parse_hex_addr(local_hex) {
                    let state = parse_tcp_state(state_hex);
                    if state == "LISTEN" {
                        endpoints.push(ServiceEndpoint {
                            name: "unknown".into(),
                            pid: None,
                            listen_addresses: vec![addr],
                            protocol: if proto.contains("udp") { "udp".into() } else { "tcp".into() },
                            state,
                            connections_in: 0,
                            connections_out: 0,
                        });
                    }
                }
            }
        }
    }

    endpoints
}

/// Discover active connections
async fn discover_connections() -> Vec<ServiceConnection> {
    let mut connections = Vec::new();

    let ss_output = match tokio::process::Command::new("ss")
        .args(&["-tnp"])
        .output()
        .await
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return connections,
    };

    for line in ss_output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 { continue; }

        let state = parts[0].to_string();
        let local = parts[3].to_string();
        let remote = parts[4].to_string();
        let (_, proc_name) = parse_process_info(parts.last().unwrap_or(&""));

        connections.push(ServiceConnection {
            from: proc_name.clone().unwrap_or_else(|| "unknown".into()),
            to: resolve_service_name(&remote),
            from_addr: local,
            to_addr: remote,
            protocol: "tcp".into(),
            state,
        });
    }

    // Also check unix sockets
    if let Ok(o) = tokio::process::Command::new("ss")
        .args(&["-xlnp"])
        .output()
        .await
    {
        if o.status.success() {
            let output = String::from_utf8_lossy(&o.stdout);
            for line in output.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let socket_path = parts[4].to_string();
                    let (_, proc_name) = parse_process_info(parts.last().unwrap_or(&""));
                    connections.push(ServiceConnection {
                        from: proc_name.unwrap_or_else(|| "unknown".into()),
                        to: "unix".into(),
                        from_addr: socket_path,
                        to_addr: "unix".into(),
                        protocol: "unix".into(),
                        state: "LISTEN".into(),
                    });
                }
            }
        }
    }

    connections
}

fn merge_endpoints(endpoints: Vec<ServiceEndpoint>) -> Vec<ServiceEndpoint> {
    let mut map: HashMap<String, ServiceEndpoint> = HashMap::new();
    for ep in endpoints {
        if let Some(existing) = map.get_mut(&ep.name) {
            existing.listen_addresses.extend(ep.listen_addresses);
        } else {
            map.insert(ep.name.clone(), ep);
        }
    }
    map.into_values().collect()
}

fn parse_process_info(raw: &str) -> (Option<u32>, Option<String>) {
    // Format: users:(("nginx",pid=1234,fd=6))
    if let Some(inner) = raw.strip_prefix("users:((\"").and_then(|s| s.strip_suffix("\"))")) {
        let parts: Vec<&str> = inner.split("\",").collect();
        let name = parts.first().map(|s| s.to_string());
        let pid = parts.get(1)
            .and_then(|s| s.strip_prefix("pid="))
            .and_then(|s| s.parse().ok());
        (pid, name)
    } else {
        (None, None)
    }
}

fn parse_hex_addr(hex: &str) -> Option<String> {
    let parts: Vec<&str> = hex.split(':').collect();
    if parts.len() != 2 { return None; }
    let port = u16::from_str_radix(parts[1], 16).ok()?;
    let ip_hex = parts[0];
    if ip_hex.len() == 8 {
        // IPv4
        let mut bytes = [0u8; 4];
        for i in 0..4 {
            bytes[i] = u8::from_str_radix(&ip_hex[i*2..i*2+2], 16).ok()?;
        }
        Some(format!("{}.{}.{}.{}:{}", bytes[0], bytes[1], bytes[2], bytes[3], port))
    } else {
        Some(format!("{}:{}", ip_hex, port))
    }
}

fn parse_tcp_state(hex: &str) -> String {
    match hex {
        "0A" => "LISTEN".into(),
        "01" => "ESTABLISHED".into(),
        "06" => "TIME_WAIT".into(),
        "08" => "CLOSE_WAIT".into(),
        _ => format!("UNKNOWN({hex})"),
    }
}

fn is_internal_addr(addr: &str) -> bool {
    addr.starts_with("127.") || addr.starts_with("::1") || addr.starts_with("[::1]")
        || addr == "unix" || addr.starts_with("/run/") || addr.starts_with("/var/run/")
}

fn resolve_service_name(addr: &str) -> String {
    // Map well-known ports to services
    let port = addr.rsplit(':').next().unwrap_or("");
    match port {
        "22" => "sshd".into(),
        "80" => "nginx".into(),
        "443" => "nginx".into(),
        "3306" => "mysql".into(),
        "5432" => "postgresql".into(),
        "6379" => "redis".into(),
        "8080" => "http-alt".into(),
        "9090" => "prometheus".into(),
        "3000" => "grafana".into(),
        _ => addr.to_string(),
    }
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_default()
}

// ─── HTTP Handlers ─────────────────────────────────────────────────────

/// GET /api/mesh/topology
pub async fn handle_topology(Query(query): Query<MeshQuery>) -> Result<Json<MeshTopology>, AppError> {
    let topology = discover_topology(&query).await?;
    Ok(Json(topology))
}
