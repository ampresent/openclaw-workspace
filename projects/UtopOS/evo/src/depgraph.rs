use axum::{extract::{Query, State}, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::cmd::{run_cmd, AppStateRef};
use crate::error::AppError;

/// Query parameters
#[derive(Deserialize)]
pub struct DepGraphQuery {
    /// Focus on a specific service (returns subgraph)
    pub focus: Option<String>,
    /// Max depth for dependency traversal
    pub depth: Option<u32>,
}

/// Service dependency node
#[derive(Debug, Clone, Serialize)]
pub struct ServiceNode {
    pub name: String,
    pub active: bool,
    pub unit_type: String,       // service, socket, timer, target
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub load_state: String,      // loaded, not-found, masked
    pub active_state: String,    // active, inactive, failed
}

/// Dependency graph result
#[derive(Debug, Serialize)]
pub struct DepGraphResult {
    pub nodes: Vec<ServiceNode>,
    pub edges: Vec<Edge>,
    pub critical_path: Vec<String>,
    pub failed_impact: Vec<String>,  // services affected by failures
    pub total_services: usize,
    pub circular_deps: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: String,  // requires, wants, after, before
}

/// Parse systemctl show output into a map
fn parse_systemctl_show(output: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in output.lines() {
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.to_string(), value.to_string());
        }
    }
    map
}

/// Extract unit names from a systemctl property (comma-separated or space-separated)
fn extract_unit_names(value: &str) -> Vec<String> {
    if value.trim().is_empty() || value.trim() == "n/a" {
        return vec![];
    }
    value
        .split_whitespace()
        .filter(|s| !s.is_empty() && s.ends_with(".service") || s.ends_with(".target") || s.ends_with(".socket") || s.ends_with(".timer"))
        .map(|s| s.to_string())
        .collect()
}

/// Get detailed info about a systemd unit
async fn get_unit_info(unit: &str) -> Result<HashMap<String, String>, AppError> {
    let output = run_cmd(
        "systemctl",
        &[
            "show",
            "--no-pager",
            "--property=Id,ActiveState,LoadState,SubState,Description,Requires,Wants,RequiredBy,WantedBy,After,Before,Type",
            unit,
        ],
    )
    .await?;
    Ok(parse_systemctl_show(&output))
}

/// List all loaded units
async fn list_all_units() -> Result<Vec<String>, AppError> {
    let output = run_cmd(
        "systemctl",
        &["list-units", "--type=service", "--all", "--no-pager", "--plain", "--no-legend"],
    )
    .await?;

    let units = output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                return None;
            }
            Some(parts[0].to_string())
        })
        .collect();

    Ok(units)
}

/// Build dependency graph
async fn build_dep_graph(focus: Option<&str>, max_depth: u32) -> Result<DepGraphResult, AppError> {
    // If focused, start from that service; otherwise collect common critical services
    let seed_units: Vec<String> = match focus {
        Some(svc) => vec![svc.to_string()],
        None => {
            // Core services to explore
            vec![
                "multi-user.target".into(),
                "network.target".into(),
                "sshd.service".into(),
                "nginx.service".into(),
                "docker.service".into(),
            ]
        }
    };

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();
    let mut nodes: HashMap<String, ServiceNode> = HashMap::new();
    let mut edges: Vec<Edge> = Vec::new();

    // BFS traversal
    for unit in seed_units {
        queue.push_back((unit, 0));
    }

    while let Some((unit, depth)) = queue.pop_front() {
        if visited.contains(&unit) || depth > max_depth {
            continue;
        }
        visited.insert(unit.clone());

        // Get unit info
        let info = match get_unit_info(&unit).await {
            Ok(i) => i,
            Err(_) => continue, // Skip units that can't be queried
        };

        let active = info.get("ActiveState").map(|s| s.as_str()) == Some("active");
        let unit_type = if unit.ends_with(".service") {
            "service"
        } else if unit.ends_with(".target") {
            "target"
        } else if unit.ends_with(".socket") {
            "socket"
        } else {
            "other"
        };

        let requires = extract_unit_names(info.get("Requires").map(|s| s.as_str()).unwrap_or(""));
        let wants = extract_unit_names(info.get("Wants").map(|s| s.as_str()).unwrap_or(""));
        let after = extract_unit_names(info.get("After").map(|s| s.as_str()).unwrap_or(""));
        let required_by = extract_unit_names(info.get("RequiredBy").map(|s| s.as_str()).unwrap_or(""));

        // Build dependency list
        let mut deps = requires.clone();
        deps.extend(wants.clone());
        deps.extend(after.clone());

        let dependents = required_by.clone();

        // Add edges
        for dep in &requires {
            edges.push(Edge { from: unit.clone(), to: dep.clone(), kind: "requires".into() });
        }
        for dep in &wants {
            edges.push(Edge { from: unit.clone(), to: dep.clone(), kind: "wants".into() });
        }
        for dep in &after {
            edges.push(Edge { from: unit.clone(), to: dep.clone(), kind: "after".into() });
        }

        // Add node
        nodes.insert(unit.clone(), ServiceNode {
            name: unit.clone(),
            active,
            unit_type: unit_type.to_string(),
            dependencies: deps.clone(),
            dependents,
            load_state: info.get("LoadState").cloned().unwrap_or_default(),
            active_state: info.get("ActiveState").cloned().unwrap_or_default(),
        });

        // Enqueue dependencies
        for dep in deps {
            if !visited.contains(&dep) {
                queue.push_back((dep, depth + 1));
            }
        }
    }

    // Detect circular dependencies
    let circular = detect_circular_deps(&nodes);

    // Find critical path (longest chain through active services)
    let critical_path = find_critical_path(&nodes, &edges);

    // Find failure impact (what fails if a service goes down)
    let failed_impact = match focus {
        Some(svc) => find_failure_impact(svc, &edges),
        None => vec![],
    };

    Ok(DepGraphResult {
        nodes: nodes.into_values().collect(),
        edges,
        critical_path,
        failed_impact,
        total_services: visited.len(),
        circular_deps: circular,
    })
}

/// Detect circular dependencies using DFS
fn detect_circular_deps(nodes: &HashMap<String, ServiceNode>) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut stack = Vec::new();
    let mut on_stack = HashSet::new();

    fn dfs(
        node: &str,
        nodes: &HashMap<String, ServiceNode>,
        visited: &mut HashSet<String>,
        stack: &mut Vec<String>,
        on_stack: &mut HashSet<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        if on_stack.contains(node) {
            // Found cycle
            if let Some(start) = stack.iter().position(|n| n == node) {
                let cycle: Vec<String> = stack[start..].to_vec();
                cycles.push(cycle);
            }
            return;
        }
        if visited.contains(node) {
            return;
        }

        visited.insert(node.to_string());
        on_stack.insert(node.to_string());
        stack.push(node.to_string());

        if let Some(svc) = nodes.get(node) {
            for dep in &svc.dependencies {
                dfs(dep, nodes, visited, stack, on_stack, cycles);
            }
        }

        stack.pop();
        on_stack.remove(node);
    }

    for node_name in nodes.keys() {
        if !visited.contains(node_name) {
            dfs(node_name, nodes, &mut visited, &mut stack, &mut on_stack, &mut cycles);
        }
    }

    cycles
}

/// Find the critical path (longest dependency chain)
fn find_critical_path(nodes: &HashMap<String, ServiceNode>, edges: &[Edge]) -> Vec<String> {
    // Build adjacency list (reverse — who depends on whom)
    let mut deps_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in edges {
        if edge.kind == "requires" {
            deps_of.entry(edge.to.as_str()).or_default().push(edge.from.as_str());
        }
    }

    // Find longest path from any leaf
    let mut longest = vec![];
    for node in nodes.keys() {
        let mut path = vec![node.as_str()];
        let mut current = node.as_str();
        while let Some(next_deps) = deps_of.get(current) {
            if let Some(next) = next_deps.first() {
                path.push(next);
                current = next;
            } else {
                break;
            }
        }
        if path.len() > longest.len() {
            longest = path;
        }
    }

    longest.into_iter().map(|s| s.to_string()).collect()
}

/// Find what services are impacted if a given service fails
fn find_failure_impact(service: &str, edges: &[Edge]) -> Vec<String> {
    let mut impacted = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(service);

    // Build "requires" reverse map: if A requires B, B failure impacts A
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in edges {
        if edge.kind == "requires" {
            dependents.entry(edge.to.as_str()).or_default().push(edge.from.as_str());
        }
    }

    while let Some(current) = queue.pop_front() {
        if let Some(deps) = dependents.get(current) {
            for dep in deps {
                if !impacted.contains(dep) {
                    impacted.insert(dep.to_string());
                    queue.push_back(dep);
                }
            }
        }
    }

    let mut result: Vec<String> = impacted.into_iter().collect();
    result.sort();
    result
}

/// GET /api/deps — get service dependency graph
pub async fn handle_deps(
    Query(query): Query<DepGraphQuery>,
    State(_state): AppStateRef,
) -> Result<impl IntoResponse, AppError> {
    let max_depth = query.depth.unwrap_or(3).min(10);
    let graph = build_dep_graph(query.focus.as_deref(), max_depth).await?;
    Ok(Json(serde_json::to_value(&graph).unwrap()))
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_systemctl_show() {
        let output = "Id=nginx.service
ActiveState=active
LoadState=loaded
Requires=network.target
";
        let map = parse_systemctl_show(output);
        assert_eq!(map.get("Id").unwrap(), "nginx.service");
        assert_eq!(map.get("ActiveState").unwrap(), "active");
        assert_eq!(map.get("Requires").unwrap(), "network.target");
    }

    #[test]
    fn test_extract_unit_names_empty() {
        assert!(extract_unit_names("").is_empty());
        assert!(extract_unit_names("n/a").is_empty());
    }

    #[test]
    fn test_extract_unit_names_mixed() {
        let names = extract_unit_names("network.target basic.target systemd-journald.socket");
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"network.target".to_string()));
        assert!(names.contains(&"systemd-journald.socket".to_string()));
    }

    #[test]
    fn test_detect_circular_deps_no_cycles() {
        let mut nodes = HashMap::new();
        nodes.insert("a.service".into(), ServiceNode {
            name: "a.service".into(), active: true, unit_type: "service".into(),
            dependencies: vec!["b.service".into()], dependents: vec![],
            load_state: "loaded".into(), active_state: "active".into(),
        });
        nodes.insert("b.service".into(), ServiceNode {
            name: "b.service".into(), active: true, unit_type: "service".into(),
            dependencies: vec![], dependents: vec![],
            load_state: "loaded".into(), active_state: "active".into(),
        });

        let cycles = detect_circular_deps(&nodes);
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_detect_circular_deps_with_cycle() {
        let mut nodes = HashMap::new();
        nodes.insert("a.service".into(), ServiceNode {
            name: "a.service".into(), active: true, unit_type: "service".into(),
            dependencies: vec!["b.service".into()], dependents: vec![],
            load_state: "loaded".into(), active_state: "active".into(),
        });
        nodes.insert("b.service".into(), ServiceNode {
            name: "b.service".into(), active: true, unit_type: "service".into(),
            dependencies: vec!["a.service".into()], dependents: vec![],
            load_state: "loaded".into(), active_state: "active".into(),
        });

        let cycles = detect_circular_deps(&nodes);
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_find_failure_impact() {
        let edges = vec![
            Edge { from: "nginx.service".into(), to: "network.target".into(), kind: "requires".into() },
            Edge { from: "app.service".into(), to: "nginx.service".into(), kind: "requires".into() },
        ];

        let impact = find_failure_impact("network.target", &edges);
        assert!(impact.contains(&"nginx.service".to_string()));
        assert!(impact.contains(&"app.service".to_string()));
    }

    #[test]
    fn test_find_failure_impact_leaf() {
        let edges = vec![
            Edge { from: "nginx.service".into(), to: "network.target".into(), kind: "requires".into() },
        ];

        let impact = find_failure_impact("nginx.service", &edges);
        assert!(impact.is_empty()); // No one depends on nginx
    }

    #[test]
    fn test_edge_serialization() {
        let edge = Edge {
            from: "a.service".into(),
            to: "b.service".into(),
            kind: "requires".into(),
        };
        let json = serde_json::to_string(&edge).unwrap();
        assert!(json.contains(""from":"a.service""));
        assert!(json.contains("\"kind\":\"requires\""));
    }

    #[test]
    fn test_parse_systemctl_show_empty() {
        let map = parse_systemctl_show("");
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_systemctl_show_no_equals() {
        let map = parse_systemctl_show("some random text\nno keys here");
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_systemctl_show_multiline() {
        let output = "Id=mysql.service\nActiveState=inactive\nLoadState=loaded\nSubState=dead\n";
        let map = parse_systemctl_show(output);
        assert_eq!(map.len(), 4);
        assert_eq!(map.get("SubState").unwrap(), "dead");
    }

    #[test]
    fn test_extract_unit_names_services() {
        let names = extract_unit_names("nginx.service postgres.service");
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"nginx.service".to_string()));
    }

    #[test]
    fn test_extract_unit_names_timer() {
        let names = extract_unit_names("backup.timer");
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "backup.timer");
    }

    #[test]
    fn test_extract_unit_names_ignores_plain_text() {
        let names = extract_unit_names("some description text here");
        assert!(names.is_empty());
    }

    #[test]
    fn test_detect_circular_deps_three_node_cycle() {
        let mut nodes = HashMap::new();
        nodes.insert("a.service".into(), ServiceNode {
            name: "a.service".into(), active: true, unit_type: "service".into(),
            dependencies: vec!["b.service".into()], dependents: vec![],
            load_state: "loaded".into(), active_state: "active".into(),
        });
        nodes.insert("b.service".into(), ServiceNode {
            name: "b.service".into(), active: true, unit_type: "service".into(),
            dependencies: vec!["c.service".into()], dependents: vec![],
            load_state: "loaded".into(), active_state: "active".into(),
        });
        nodes.insert("c.service".into(), ServiceNode {
            name: "c.service".into(), active: true, unit_type: "service".into(),
            dependencies: vec!["a.service".into()], dependents: vec![],
            load_state: "loaded".into(), active_state: "active".into(),
        });
        let cycles = detect_circular_deps(&nodes);
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_detect_circular_deps_empty() {
        let nodes = HashMap::new();
        let cycles = detect_circular_deps(&nodes);
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_find_failure_impact_chain() {
        let edges = vec![
            Edge { from: "c.service".into(), to: "b.service".into(), kind: "requires".into() },
            Edge { from: "b.service".into(), to: "a.service".into(), kind: "requires".into() },
        ];
        let impact = find_failure_impact("a.service", &edges);
        assert!(impact.contains(&"b.service".to_string()));
        assert!(impact.contains(&"c.service".to_string()));
    }

    #[test]
    fn test_find_failure_impact_empty_edges() {
        let edges = vec![];
        let impact = find_failure_impact("anything.service", &edges);
        assert!(impact.is_empty());
    }
}
