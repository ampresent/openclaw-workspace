use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap, BTreeSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::AppError;

// ─── Sync Node ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncNode {
    pub id: String,
    pub name: String,
    pub url: String,
    pub last_sync: Option<String>,
    pub config_hash: Option<String>,
    pub status: String, // "online", "offline", "diverged", "syncing"
}

// ─── Config Version (CRDT-inspired) ─────────────────────────────────────

/// A version vector entry for causal ordering.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionVector {
    /// node_id → highest seen sequence number
    pub entries: HashMap<String, u64>,
}

impl VersionVector {
    pub fn increment(&mut self, node_id: &str) {
        let entry = self.entries.entry(node_id.to_string()).or_insert(0);
        *entry += 1;
    }

    pub fn merge(&mut self, other: &VersionVector) {
        for (node, seq) in &other.entries {
            let entry = self.entries.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(*seq);
        }
    }

    pub fn dominates(&self, other: &VersionVector) -> bool {
        for (node, seq) in &other.entries {
            if self.entries.get(node).copied().unwrap_or(0) < *seq {
                return false;
            }
        }
        true
    }
}

/// A single config operation in the log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOp {
    pub id: String,
    pub node_id: String,
    pub timestamp: String,
    pub version: VersionVector,
    pub op_type: OpType,
    pub path: String,       // e.g. "services.nginx.enable"
    pub value: Option<String>,
    pub parent_value: Option<String>, // for merge resolution
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpType {
    Set,
    Delete,
    Append,
}

// ─── Merge Result ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct MergeResult {
    pub success: bool,
    pub merged_ops: usize,
    pub conflicts: Vec<MergeConflict>,
    pub final_config: BTreeMap<String, String>,
    pub version: VersionVector,
}

#[derive(Debug, Clone, Serialize)]
pub struct MergeConflict {
    pub path: String,
    pub local_value: String,
    pub remote_value: String,
    pub resolution: String, // "last-write-wins", "manual-required"
}

// ─── Sync State ──────────────────────────────────────────────────────────

pub struct SyncState {
    nodes: RwLock<HashMap<String, SyncNode>>,
    op_log: RwLock<Vec<ConfigOp>>,
    version: RwLock<VersionVector>,
    configs: RwLock<BTreeMap<String, BTreeMap<String, String>>>, // node_id → config
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            op_log: RwLock::new(Vec::new()),
            version: RwLock::new(VersionVector::default()),
            configs: RwLock::new(BTreeMap::new()),
        }
    }

    /// Initialize a sync group.
    pub async fn init(&self, node: SyncNode) -> Result<SyncInitResult, AppError> {
        let mut nodes = self.nodes.write().await;
        let node_id = node.id.clone();
        nodes.insert(node_id.clone(), node);

        Ok(SyncInitResult {
            node_id: node_id.clone(),
            peer_count: nodes.len() - 1,
            peers: nodes.keys().filter(|k| **k != node_id).cloned().collect(),
        })
    }

    /// Push config changes from a node.
    pub async fn push(
        &self,
        node_id: &str,
        ops: Vec<ConfigOp>,
    ) -> Result<MergeResult, AppError> {
        let mut op_log = self.op_log.write().await;
        let mut version = self.version.write().await;
        let mut configs = self.configs.write().await;
        let mut conflicts = Vec::new();

        // Get or create node config
        let node_config = configs
            .entry(node_id.to_string())
            .or_insert_with(BTreeMap::new);

        let mut merged_count = 0;

        for op in &ops {
            // Check for conflicts with other nodes
            let other_ops: Vec<&ConfigOp> = op_log
                .iter()
                .filter(|o| o.path == op.path && o.node_id != op.node_id)
                .collect();

            if let Some(conflicting) = other_ops.last() {
                // Last-write-wins based on version vector
                if !op.version.dominates(&conflicting.version) {
                    conflicts.push(MergeConflict {
                        path: op.path.clone(),
                        local_value: op.value.clone().unwrap_or_default(),
                        remote_value: conflicting.value.clone().unwrap_or_default(),
                        resolution: "last-write-wins".into(),
                    });
                }
            }

            // Apply operation
            match op.op_type {
                OpType::Set => {
                    node_config.insert(op.path.clone(), op.value.clone().unwrap_or_default());
                }
                OpType::Delete => {
                    node_config.remove(&op.path);
                }
                OpType::Append => {
                    let existing = node_config.entry(op.path.clone())
                        .or_insert_with(String::new);
                    if let Some(ref v) = op.value {
                        if !existing.is_empty() {
                            existing.push('\n');
                        }
                        existing.push_str(v);
                    }
                }
            }

            version.merge(&op.version);
            merged_count += 1;
        }

        // Add to log
        op_log.extend(ops);

        // Update node status
        if let Some(node) = self.nodes.write().await.get_mut(node_id) {
            node.status = "online".into();
            node.last_sync = Some(chrono_now());
        }

        // Build merged config (union of all node configs)
        let mut final_config = BTreeMap::new();
        for (_nid, cfg) in configs.iter() {
            for (k, v) in cfg {
                final_config.insert(k.clone(), v.clone());
            }
        }

        Ok(MergeResult {
            success: conflicts.is_empty(),
            merged_ops: merged_count,
            conflicts,
            final_config,
            version: version.clone(),
        })
    }

    /// Get the merged config for the fleet.
    pub async fn get_merged_config(&self) -> BTreeMap<String, String> {
        let configs = self.configs.read().await;
        let mut merged = BTreeMap::new();
        for (_nid, cfg) in configs.iter() {
            for (k, v) in cfg {
                merged.insert(k.clone(), v.clone());
            }
        }
        merged
    }

    /// Get status of all nodes.
    pub async fn get_status(&self) -> SyncStatus {
        let nodes = self.nodes.read().await;
        let version = self.version.read().await;
        let configs = self.configs.read().await;

        // Detect divergences
        let mut config_hashes: Vec<(String, String)> = Vec::new();
        for (nid, cfg) in configs.iter() {
            let hash = format!("{:016x}", hash_config(cfg));
            config_hashes.push((nid.clone(), hash));
        }

        let all_same_hash = config_hashes.windows(2).all(|w| w[0].1 == w[1].1);

        SyncStatus {
            node_count: nodes.len(),
            nodes: nodes.values().cloned().collect(),
            version: version.clone(),
            config_hash: config_hashes.first().map(|(_, h)| h.clone()),
            all_in_sync: all_same_hash,
            op_count: 0, // filled by caller if needed
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SyncInitResult {
    pub node_id: String,
    pub peer_count: usize,
    pub peers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncStatus {
    pub node_count: usize,
    pub nodes: Vec<SyncNode>,
    pub version: VersionVector,
    pub config_hash: Option<String>,
    pub all_in_sync: bool,
    pub op_count: usize,
}

fn hash_config(config: &BTreeMap<String, String>) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    for (k, v) in config {
        k.hash(&mut hasher);
        v.hash(&mut hasher);
    }
    hasher.finish()
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_default()
}

// ─── Singleton ───────────────────────────────────────────────────────────

use std::sync::OnceLock;
pub static STATE: OnceLock<SyncState> = OnceLock::new();

pub fn state() -> &'static SyncState {
    STATE.get_or_init(SyncState::new)
}

// ─── HTTP Handlers ───────────────────────────────────────────────────────

/// POST /api/sync/init
#[derive(Debug, Deserialize)]
pub struct SyncInitRequest {
    pub id: String,
    pub name: String,
    pub url: String,
}

pub async fn handle_init(
    Json(req): Json<SyncInitRequest>,
) -> Result<Json<SyncInitResult>, AppError> {
    let node = SyncNode {
        id: req.id.clone(),
        name: req.name,
        url: req.url,
        last_sync: None,
        config_hash: None,
        status: "online".into(),
    };
    let result = state().init(node).await?;
    Ok(Json(result))
}

/// POST /api/sync/push
#[derive(Debug, Deserialize)]
pub struct SyncPushRequest {
    pub node_id: String,
    pub ops: Vec<ConfigOp>,
}

pub async fn handle_push(
    Json(req): Json<SyncPushRequest>,
) -> Result<Json<MergeResult>, AppError> {
    let result = state().push(&req.node_id, req.ops).await?;
    Ok(Json(result))
}

/// GET /api/sync/status
pub async fn handle_status() -> Json<SyncStatus> {
    let mut status = state().get_status().await;
    let op_log = state().op_log.read().await;
    status.op_count = op_log.len();
    Json(status)
}

/// GET /api/sync/config
pub async fn handle_config() -> Json<serde_json::Value> {
    let config = state().get_merged_config().await;
    Json(serde_json::json!({
        "entries": config.len(),
        "config": config,
    }))
}
