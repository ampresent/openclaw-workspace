use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::error::AppError;

/// A dependency node in the config graph
#[derive(Debug, Clone, Serialize)]
pub struct DepNode {
    pub id: String,
    pub label: String,
    pub kind: String, // "service", "package", "module", "option"
    pub enabled: bool,
}

/// A directed edge: "from depends on to"
#[derive(Debug, Clone, Serialize)]
pub struct DepEdge {
    pub from: String,
    pub to: String,
    pub kind: String, // "requires", "uses", "imports", "wants"
}

/// Full dependency graph
#[derive(Debug, Clone, Serialize)]
pub struct DepGraph {
    pub nodes: Vec<DepNode>,
    pub edges: Vec<DepEdge>,
    pub dot: String,
}

#[derive(Debug, Deserialize)]
pub struct GraphQuery {
    pub format: Option<String>, // "json" or "dot"
    pub config_path: Option<String>,
    pub depth: Option<usize>,
}

/// Well-known NixOS service dependency map
fn known_dependencies() -> HashMap<&'static str, Vec<(&'static str, &'static str)>> {
    let mut deps: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();

    // Web servers
    deps.insert("nginx", vec![("openssl", "requires"), ("pcre", "requires"), ("zlib", "requires")]);
    deps.insert("httpd", vec![("openssl", "requires")]);

    // PHP stack
    deps.insert("phpfpm", vec![("php", "requires"), ("nginx", "uses")]);
    deps.insert("php", vec![("openssl", "requires"), ("libxml2", "requires")]);

    // Database
    deps.insert("mysql", vec![("openssl", "requires"), ("systemd", "uses")]);
    deps.insert("postgresql", vec![("openssl", "requires"), ("systemd", "uses"), ("zlib", "requires")]);
    deps.insert("redis", vec![("systemd", "uses")]);
    deps.insert("mongodb", vec![("openssl", "requires"), ("systemd", "uses")]);

    // Networking
    deps.insert("openssh", vec![("openssl", "requires"), ("systemd", "uses")]);
    deps.insert("wireguard", vec![("linux", "requires"), ("systemd", "uses")]);
    deps.insert("tailscale", vec![("systemd", "uses")]);
    deps.insert("networkmanager", vec![("systemd", "uses"), ("dbus", "requires")]);
    deps.insert("dnsmasq", vec![("systemd", "uses")]);

    // Containers
    deps.insert("docker", vec![("iptables", "uses"), ("systemd", "uses"), ("bridge-utils", "uses")]);
    deps.insert("podman", vec![("systemd", "uses"), ("conmon", "requires")]);
    deps.insert("kubernetes", vec![("docker", "uses"), ("etcd", "uses"), ("systemd", "uses")]);

    // Monitoring
    deps.insert("prometheus", vec![("systemd", "uses")]);
    deps.insert("grafana", vec![("systemd", "uses")]);
    deps.insert("loki", vec![("systemd", "uses")]);

    // Auth
    deps.insert("sssd", vec![("dbus", "requires"), ("systemd", "uses")]);
    deps.insert("keycloak", vec![("postgresql", "uses"), ("java", "requires")]);

    // Mail
    deps.insert("postfix", vec![("openssl", "requires"), ("cyrus_sasl", "uses")]);
    deps.insert("dovecot", vec![("openssl", "requires")]);

    deps
}

/// Extract service declarations from configuration.nix content
fn extract_services(config: &str) -> Vec<String> {
    let mut services = Vec::new();

    // Match patterns like: services.nginx.enable = true;
    for line in config.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("services.") {
            if let Some(dot_pos) = rest.find('.') {
                let service = &rest[..dot_pos];
                if rest[dot_pos..].contains("enable") && (rest.contains("= true") || rest.contains("=true")) {
                    if !services.contains(&service.to_string()) {
                        services.push(service.to_string());
                    }
                }
            }
        }

        // Also detect virtualisation.docker.enable, hardware.pulseaudio.enable, etc.
        for prefix in &["virtualisation.", "hardware.", "security.", "networking.", "programs."] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                if let Some(dot_pos) = rest.find('.') {
                    let service = &rest[..dot_pos];
                    if rest[dot_pos..].contains("enable") && rest.contains("true") {
                        let full_name = format!("{prefix}{service}");
                        if !services.contains(&full_name) {
                            services.push(full_name);
                        }
                    }
                }
            }
        }
    }

    services
}

/// Recursively resolve dependencies to a given depth
fn resolve_dependencies(
    services: &[String],
    depth: usize,
) -> (Vec<DepNode>, Vec<DepEdge>) {
    let known = known_dependencies();
    let mut nodes: HashMap<String, DepNode> = HashMap::new();
    let mut edges: Vec<DepEdge> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: Vec<(String, usize)> = services.iter().map(|s| (s.clone(), 0)).collect();

    // Add all top-level services as enabled nodes
    for svc in services {
        nodes.insert(svc.clone(), DepNode {
            id: svc.clone(),
            label: svc.clone(),
            kind: classify_node(svc),
            enabled: true,
        });
    }

    while let Some((current, level)) = queue.pop() {
        if visited.contains(&current) || level >= depth {
            continue;
        }
        visited.insert(current.clone());

        if let Some(deps) = known.get(current.as_str()) {
            for (dep_name, dep_kind) in deps {
                let dep_id = dep_name.to_string();

                edges.push(DepEdge {
                    from: current.clone(),
                    to: dep_id.clone(),
                    kind: dep_kind.to_string(),
                });

                if !nodes.contains_key(&dep_id) {
                    nodes.insert(dep_id.clone(), DepNode {
                        id: dep_id.clone(),
                        label: dep_id.clone(),
                        kind: classify_node(&dep_id),
                        enabled: false,
                    });
                }

                if !visited.contains(&dep_id) {
                    queue.push((dep_id, level + 1));
                }
            }
        }
    }

    let node_list: Vec<DepNode> = nodes.into_values().collect();
    (node_list, edges)
}

fn classify_node(name: &str) -> String {
    match name {
        "openssl" | "zlib" | "pcre" | "libxml2" | "conmon" | "cyrus_sasl" => "library".into(),
        "systemd" | "dbus" | "linux" | "java" | "php" => "runtime".into(),
        "iptables" | "bridge-utils" | "etcd" => "tool".into(),
        _ => "service".into(),
    }
}

/// Generate Graphviz DOT output
fn generate_dot(nodes: &[DepNode], edges: &[DepEdge]) -> String {
    let mut dot = String::from("digraph config_deps {\n");
    dot.push_str("  rankdir=LR;\n");
    dot.push_str("  node [shape=box, style=filled, fontname=\"Helvetica\"];\n");
    dot.push_str("  edge [fontname=\"Helvetica\", fontsize=10];\n\n");

    for node in nodes {
        let color = match node.kind.as_str() {
            "service" if node.enabled => "#4CAF50",
            "service" => "#90CAF9",
            "library" => "#FFCC80",
            "runtime" => "#CE93D8",
            "tool" => "#B0BEC5",
            _ => "#E0E0E0",
        };
        let shape = if node.enabled { "box" } else { "box" };
        let style = if node.enabled { "filled,bold" } else { "filled" };
        dot.push_str(&format!(
            "  \"{}\" [label=\"{}\", fillcolor=\"{}\", shape={}, style=\"{}\"];\n",
            node.id, node.label, color, shape, style
        ));
    }
    dot.push('\n');

    for edge in edges {
        let style = match edge.kind.as_str() {
            "requires" => "solid",
            "uses" => "dashed",
            "imports" => "dotted",
            _ => "solid",
        };
        dot.push_str(&format!(
            "  \"{}\" -> \"{}\" [style={}, label=\"{}\"];\n",
            edge.from, edge.to, style, edge.kind
        ));
    }

    dot.push_str("}\n");
    dot
}

/// Parse a configuration.nix file and build its dependency graph
pub async fn build_graph(config_content: &str, depth: usize) -> DepGraph {
    let services = extract_services(config_content);
    let (nodes, edges) = resolve_dependencies(&services, depth);
    let dot = generate_dot(&nodes, &edges);

    DepGraph { nodes, edges, dot }
}

// ─── HTTP Handlers ─────────────────────────────────────────────────────

/// GET /api/deps/graph?format=json&config_path=/etc/nixos/configuration.nix
pub async fn handle_graph(Query(query): Query<GraphQuery>) -> Result<impl IntoResponse, AppError> {
    let config_path = query.config_path
        .unwrap_or_else(|| "/etc/nixos/configuration.nix".into());
    let depth = query.depth.unwrap_or(5);

    let content = tokio::fs::read_to_string(&config_path).await
        .map_err(|e| AppError::IoError {
            path: config_path.clone(),
            message: e.to_string(),
        })?;

    let graph = build_graph(&content, depth).await;

    match query.format.as_deref() {
        Some("dot") => Ok((
            axum::http::StatusCode::OK,
            [("content-type", "text/vnd.graphviz")],
            graph.dot,
        )),
        _ => Ok((
            axum::http::StatusCode::OK,
            [("content-type", "application/json")],
            serde_json::to_string_pretty(&graph).unwrap_or_default(),
        )),
    }
}

/// GET /api/deps/graph/analyze — analyze inline config content
pub async fn handle_analyze(
    Json(body): Json<serde_json::Value>,
) -> Result<Json<DepGraph>, AppError> {
    let content = body.get("config_content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Validation {
            field: "config_content".into(),
            message: "Missing config_content field".into(),
        })?;
    let depth = body.get("depth").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

    let graph = build_graph(content, depth).await;
    Ok(Json(graph))
}
