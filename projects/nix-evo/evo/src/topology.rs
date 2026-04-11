use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::cmd::run_cmd;
use crate::error::AppError;

// ─── Topology Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
    pub status: String, // "running", "stopped", "failed", "degraded"
    pub ports: Vec<PortBinding>,
    pub dependencies: Vec<String>, // node IDs this depends on
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Service,
    Database,
    ReverseProxy,
    LoadBalancer,
    Cache,
    Queue,
    Storage,
    Network,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortBinding {
    pub port: u16,
    pub protocol: String,
    pub bind_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String, // "depends", "connects", "proxies", "replicates"
    pub label: Option<String>,
    pub health: String, // "healthy", "degraded", "down"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topology {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
    pub generated_at: String,
    pub hostname: String,
}

// ─── Service Discovery ───────────────────────────────────────────────────

pub async fn discover_topology() -> Topology {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let hostname = run_cmd("hostname", &[]).await.unwrap_or_else(|_| "unknown".into()).trim().to_string();

    // Discover running services
    let services = discover_services().await;
    nodes.extend(services);

    // Discover listening ports
    let port_nodes = discover_ports().await;
    for pn in &port_nodes {
        if let Some(svc_node) = nodes.iter_mut().find(|n| n.name == pn.name) {
            svc_node.ports = pn.ports.clone();
        }
    }

    // Discover network connections
    let connections = discover_connections(&nodes).await;
    edges.extend(connections);

    // Infer dependencies from port connections
    infer_dependencies(&mut nodes, &edges);

    Topology {
        nodes,
        edges,
        generated_at: chrono::Utc::now().to_rfc3339(),
        hostname,
    }
}

async fn discover_services() -> Vec<TopologyNode> {
    let mut nodes = Vec::new();

    // Get running services from systemctl
    let output = run_cmd("systemctl", &["list-units", "--type=service", "--state=running", "--no-pager", "--plain"]).await.unwrap_or_default();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let unit = parts[0];
            let status = parts[2];

            if status == "running" {
                let name = unit.trim_end_matches(".service").to_string();
                let node_type = classify_service(&name);

                nodes.push(TopologyNode {
                    id: format!("svc-{}", name),
                    name: name.clone(),
                    node_type,
                    status: "running".into(),
                    ports: Vec::new(),
                    dependencies: Vec::new(),
                    metadata: HashMap::new(),
                });
            }
        }
    }

    nodes
}

fn classify_service(name: &str) -> NodeType {
    let lower = name.to_lowercase();
    if lower.contains("nginx") || lower.contains("apache") || lower.contains("caddy") || lower.contains("haproxy") {
        NodeType::ReverseProxy
    } else if lower.contains("mysql") || lower.contains("postgres") || lower.contains("mariadb") || lower.contains("mongo") || lower.contains("redis") || lower.contains("memcache") {
        if lower.contains("redis") || lower.contains("memcache") {
            NodeType::Cache
        } else {
            NodeType::Database
        }
    } else if lower.contains("rabbit") || lower.contains("kafka") || lower.contains("nats") {
        NodeType::Queue
    } else if lower.contains("docker") || lower.contains("containerd") || lower.contains("podman") {
        NodeType::Network
    } else if lower.contains("minio") || lower.contains("ceph") || lower.contains("gluster") {
        NodeType::Storage
    } else {
        NodeType::Service
    }
}

async fn discover_ports() -> Vec<TopologyNode> {
    let mut nodes = Vec::new();

    // Use ss to find listening ports
    let output = run_cmd("ss", &["-tlnp", "--no-header"]).await.unwrap_or_default();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            let local_addr = parts[3];
            let process_info = parts.last().unwrap_or(&"");

            // Parse port from address
            if let Some(port_str) = local_addr.rsplit(':').next() {
                if let Ok(port) = port_str.parse::<u16>() {
                    let bind = if local_addr.starts_with('*') || local_addr.starts_with("0.0.0.0") {
                        "0.0.0.0"
                    } else {
                        local_addr.rsplit(':').nth(1).unwrap_or("127.0.0.1")
                    };

                    let name = if process_info.contains("nginx") { "nginx" }
                        else if process_info.contains("sshd") { "sshd" }
                        else if process_info.contains("node") { "node-app" }
                        else { "unknown" };

                    nodes.push(TopologyNode {
                        id: format!("port-{}", port),
                        name: name.to_string(),
                        node_type: NodeType::Service,
                        status: "listening".into(),
                        ports: vec![PortBinding {
                            port,
                            protocol: "tcp".into(),
                            bind_address: bind.to_string(),
                        }],
                        dependencies: Vec::new(),
                        metadata: HashMap::new(),
                    });
                }
            }
        }
    }

    nodes
}

async fn discover_connections(nodes: &[TopologyNode]) -> Vec<TopologyEdge> {
    let mut edges = Vec::new();

    // Use ss to find established connections
    let output = run_cmd("ss", &["-tn", "state", "established", "--no-header"]).await.unwrap_or_default();

    let known_ports: HashMap<u16, &str> = nodes.iter()
        .filter_map(|n| n.ports.first().map(|p| (p.port, n.name.as_str())))
        .collect();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let local = parts[3];
            let remote = parts[4];

            if let Some(local_port) = local.rsplit(':').next().and_then(|p| p.parse::<u16>().ok()) {
                if let Some(remote_port) = remote.rsplit(':').next().and_then(|p| p.parse::<u16>().ok()) {
                    if let Some(from_svc) = known_ports.get(&remote_port) {
                        if let Some(to_svc) = known_ports.get(&local_port) {
                            edges.push(TopologyEdge {
                                from: from_svc.to_string(),
                                to: to_svc.to_string(),
                                edge_type: "connects".into(),
                                label: Some(format!("{}→{}", remote_port, local_port)),
                                health: "healthy".into(),
                            });
                        }
                    }
                }
            }
        }
    }

    edges
}

fn infer_dependencies(nodes: &mut [TopologyNode], edges: &[TopologyEdge]) {
    for edge in edges {
        if let Some(node) = nodes.iter_mut().find(|n| n.name == edge.to) {
            let from_id = format!("svc-{}", edge.from);
            if !node.dependencies.contains(&from_id) {
                node.dependencies.push(from_id);
            }
        }
    }
}

// ─── API types ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TopologyQuery {
    pub format: Option<String>, // "json" or "svg"
}

// ─── API handlers ────────────────────────────────────────────────────────

/// GET /api/topology — Get service topology
pub async fn handle_topology(Query(q): Query<TopologyQuery>) -> impl IntoResponse {
    let topology = discover_topology().await;
    Json(serde_json::to_value(&topology).unwrap())
}

/// GET /api/topology/services — List services only
pub async fn handle_services() -> impl IntoResponse {
    let topology = discover_topology().await;
    Json(serde_json::json!({
        "hostname": topology.hostname,
        "service_count": topology.nodes.len(),
        "services": topology.nodes,
    }))
}

/// GET /api/topology/connections — List connections only
pub async fn handle_connections() -> impl IntoResponse {
    let topology = discover_topology().await;
    Json(serde_json::json!({
        "connection_count": topology.edges.len(),
        "connections": topology.edges,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_service() {
        assert!(matches!(classify_service("nginx"), NodeType::ReverseProxy));
        assert!(matches!(classify_service("postgresql"), NodeType::Database));
        assert!(matches!(classify_service("redis"), NodeType::Cache));
        assert!(matches!(classify_service("docker"), NodeType::Network));
        assert!(matches!(classify_service("sshd"), NodeType::Service));
    }
}
