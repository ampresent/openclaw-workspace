use axum::{extract::{Query, State}, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::timeout;

use crate::error::AppError;

/// Cluster node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNode {
    pub name: String,
    pub url: String,
    pub token: Option<String>,
    pub ssh_tunnel: Option<String>,
}

/// Cluster state management
pub struct ClusterManager {
    nodes: RwLock<HashMap<String, ClusterNode>>,
    deploy_state: RwLock<DeployState>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DeployState {
    pub active: bool,
    pub strategy: String,
    pub total_nodes: usize,
    pub completed: usize,
    pub failed: usize,
    pub results: Vec<NodeDeployResult>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDeployResult {
    pub node: String,
    pub success: bool,
    pub duration_ms: u64,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeployRequest {
    /// Command to run on all nodes (e.g., "nixos-rebuild switch")
    pub command: String,
    /// Strategy: "fan-out", "fan-in", "rolling"
    pub strategy: Option<String>,
    /// Specific nodes (None = all)
    pub nodes: Option<Vec<String>>,
    /// Rolling: stop on first failure
    pub stop_on_failure: Option<bool>,
    /// Timeout per node in seconds
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ClusterStatus {
    pub node_count: usize,
    pub nodes: Vec<NodeStatus>,
    pub last_deploy: Option<DeployState>,
}

#[derive(Debug, Serialize)]
pub struct NodeStatus {
    pub name: String,
    pub url: String,
    pub reachable: bool,
    pub last_check: String,
    pub latency_ms: Option<u64>,
}

impl ClusterManager {
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            deploy_state: RwLock::new(DeployState::default()),
        }
    }

    pub async fn add_node(&self, node: ClusterNode) {
        let mut nodes = self.nodes.write().await;
        nodes.insert(node.name.clone(), node);
    }

    pub async fn remove_node(&self, name: &str) -> bool {
        let mut nodes = self.nodes.write().await;
        nodes.remove(name).is_some()
    }

    pub async fn get_nodes(&self) -> Vec<ClusterNode> {
        self.nodes.read().await.values().cloned().collect()
    }

    pub async fn get_status(&self) -> ClusterStatus {
        let nodes = self.nodes.read().await;
        let deploy = self.deploy_state.read().await;

        let mut node_statuses = Vec::new();
        for (name, node) in nodes.iter() {
            let start = Instant::now();
            let reachable = check_node_reachable(&node.url, node.token.as_deref()).await;
            let latency = start.elapsed().as_millis() as u64;

            node_statuses.push(NodeStatus {
                name: name.clone(),
                url: node.url.clone(),
                reachable,
                last_check: chrono_now(),
                latency_ms: if reachable { Some(latency) } else { None },
            });
        }

        ClusterStatus {
            node_count: nodes.len(),
            nodes: node_statuses,
            last_deploy: if deploy.active || deploy.finished_at.is_some() {
                Some(deploy.clone())
            } else {
                None
            },
        }
    }

    pub async fn deploy(&self, req: DeployRequest) -> Result<DeployState, AppError> {
        // Check no deploy is active
        {
            let state = self.deploy_state.read().await;
            if state.active {
                return Err(AppError::Validation {
                    field: "deploy".into(),
                    message: "A deploy is already in progress".into(),
                });
            }
        }

        let strategy = req.strategy.unwrap_or_else(|| "fan-out".into());
        let stop_on_failure = req.stop_on_failure.unwrap_or(true);
        let timeout_dur = Duration::from_secs(req.timeout_secs.unwrap_or(300));
        let command = req.command.clone();

        // Get target nodes
        let all_nodes = self.get_nodes().await;
        let targets: Vec<ClusterNode> = match &req.nodes {
            Some(names) => all_nodes
                .into_iter()
                .filter(|n| names.contains(&n.name))
                .collect(),
            None => all_nodes,
        };

        if targets.is_empty() {
            return Err(AppError::Validation {
                field: "nodes".into(),
                message: "No cluster nodes available".into(),
            });
        }

        // Initialize deploy state
        {
            let mut state = self.deploy_state.write().await;
            *state = DeployState {
                active: true,
                strategy: strategy.clone(),
                total_nodes: targets.len(),
                completed: 0,
                failed: 0,
                results: Vec::new(),
                started_at: Some(chrono_now()),
                finished_at: None,
            };
        }

        let mut results = Vec::new();

        match strategy.as_str() {
            "rolling" => {
                for node in &targets {
                    let result = execute_on_node(node, &command, timeout_dur).await;
                    let success = result.success;

                    let mut state = self.deploy_state.write().await;
                    state.completed += 1;
                    if !success {
                        state.failed += 1;
                    }
                    state.results.push(result.clone());
                    results.push(result);

                    if !success && stop_on_failure {
                        tracing::warn!(
                            "Rolling deploy stopped at node {} due to failure",
                            node.name
                        );
                        break;
                    }
                }
            }
            _ => {
                // Default: fan-out/fan-in (parallel)
                let mut handles = Vec::new();
                for node in &targets {
                    let node = node.clone();
                    let cmd = command.clone();
                    handles.push(tokio::spawn(async move {
                        execute_on_node(&node, &cmd, timeout_dur).await
                    }));
                }

                for handle in handles {
                    match handle.await {
                        Ok(result) => {
                            let mut state = self.deploy_state.write().await;
                            state.completed += 1;
                            if !result.success {
                                state.failed += 1;
                            }
                            state.results.push(result.clone());
                            results.push(result);
                        }
                        Err(e) => {
                            tracing::error!("Task join error: {e}");
                        }
                    }
                }
            }
        }

        // Finalize
        let mut state = self.deploy_state.write().await;
        state.active = false;
        state.finished_at = Some(chrono_now());
        Ok(state.clone())
    }
}

async fn execute_on_node(node: &ClusterNode, command: &str, timeout_dur: Duration) -> NodeDeployResult {
    let start = Instant::now();

    if let Some(tunnel) = &node.ssh_tunnel {
        // Execute via SSH
        let ssh_cmd = format!(
            "ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no {} '{}'",
            tunnel, command.replace('\'', "'\\''")
        );
        match timeout(timeout_dur, async {
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&ssh_cmd)
                .output()
                .await
        })
        .await
        {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                NodeDeployResult {
                    node: node.name.clone(),
                    success: output.status.success(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    output: stdout.to_string(),
                    error: if output.status.success() { None } else { Some(stderr.to_string()) },
                }
            }
            Ok(Err(e)) => NodeDeployResult {
                node: node.name.clone(),
                success: false,
                duration_ms: start.elapsed().as_millis() as u64,
                output: String::new(),
                error: Some(format!("SSH execution error: {e}")),
            },
            Err(_) => NodeDeployResult {
                node: node.name.clone(),
                success: false,
                duration_ms: start.elapsed().as_millis() as u64,
                output: String::new(),
                error: Some(format!("Timeout after {}s", timeout_dur.as_secs())),
            },
        }
    } else {
        // Use agent API
        let client = reqwest::Client::new();
        let url = format!("{}/api/config/apply", node.url.trim_end_matches('/'));
        let mut req = client.post(&url).json(&serde_json::json!({ "command": command }));
        if let Some(token) = &node.token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        match timeout(timeout_dur, req.send()).await {
            Ok(Ok(resp)) => {
                let success = resp.status().is_success();
                let body = resp.text().await.unwrap_or_default();
                NodeDeployResult {
                    node: node.name.clone(),
                    success,
                    duration_ms: start.elapsed().as_millis() as u64,
                    output: body,
                    error: if success { None } else { Some("API returned error".into()) },
                }
            }
            Ok(Err(e)) => NodeDeployResult {
                node: node.name.clone(),
                success: false,
                duration_ms: start.elapsed().as_millis() as u64,
                output: String::new(),
                error: Some(e.to_string()),
            },
            Err(_) => NodeDeployResult {
                node: node.name.clone(),
                success: false,
                duration_ms: start.elapsed().as_millis() as u64,
                output: String::new(),
                error: Some(format!("Timeout after {}s", timeout_dur.as_secs())),
            },
        }
    }
}

async fn check_node_reachable(url: &str, _token: Option<&str>) -> bool {
    let health_url = format!("{}/health", url.trim_end_matches('/'));
    match timeout(Duration::from_secs(5), async {
        reqwest::Client::new().get(&health_url).send().await
    })
    .await
    {
        Ok(Ok(resp)) => resp.status().is_success(),
        _ => false,
    }
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_default()
}

// ─── Cluster singleton ─────────────────────────────────────────────────

use std::sync::OnceLock;
pub static CLUSTER: OnceLock<ClusterManager> = OnceLock::new();

pub fn cluster_manager() -> &'static ClusterManager {
    CLUSTER.get_or_init(ClusterManager::new)
}

// ─── HTTP Handlers ─────────────────────────────────────────────────────

/// POST /api/cluster/deploy
pub async fn handle_deploy(Json(req): Json<DeployRequest>) -> Result<Json<DeployState>, AppError> {
    let manager = cluster_manager();
    let state = manager.deploy(req).await?;
    Ok(Json(state))
}

/// GET /api/cluster/status
pub async fn handle_status() -> Json<ClusterStatus> {
    let manager = cluster_manager();
    Json(manager.get_status().await)
}

/// POST /api/cluster/nodes
pub async fn handle_add_node(Json(node): Json<ClusterNode>) -> Json<serde_json::Value> {
    let manager = cluster_manager();
    manager.add_node(node.clone()).await;
    Json(serde_json::json!({
        "added": node.name,
        "total": manager.get_nodes().await.len()
    }))
}

/// DELETE /api/cluster/nodes
pub async fn handle_remove_node(
    Query(params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let manager = cluster_manager();
    let name = params.get("name").cloned().unwrap_or_default();
    let removed = manager.remove_node(&name).await;
    Json(serde_json::json!({
        "removed": removed,
        "name": name
    }))
}
