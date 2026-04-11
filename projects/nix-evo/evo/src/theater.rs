use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::AppError;

// ─── Scene: a single config change event ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub act: u64,                    // "act" = grouping of scenes
    pub scene_number: u64,           // sequential within the timeline
    pub timestamp: String,
    pub description: String,
    pub diff: ConfigDiff,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub applied: bool,               // was this scene actually applied?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDiff {
    pub added: Vec<ConfigEntry>,
    pub removed: Vec<ConfigEntry>,
    pub modified: Vec<ConfigChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    pub key: String,
    pub old_value: String,
    pub new_value: String,
}

// ─── Branch: alternative timeline ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub id: String,
    pub name: String,
    pub description: String,
    pub fork_scene_id: String,       // scene where this branch diverges
    pub scenes: Vec<Scene>,
    pub created_at: String,
    pub active: bool,
}

// ─── Timeline: the complete history ──────────────────────────────────────

pub struct Theater {
    scenes: RwLock<Vec<Scene>>,
    branches: RwLock<HashMap<String, Branch>>,
    undo_stack: RwLock<Vec<Scene>>,
    next_id: std::sync::atomic::AtomicU64,
}

impl Theater {
    pub fn new() -> Self {
        Self {
            scenes: RwLock::new(Vec::new()),
            branches: RwLock::new(HashMap::new()),
            undo_stack: RwLock::new(Vec::new()),
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> String {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("scene-{}", id)
    }

    /// Record a new scene.
    pub async fn record(&self, description: String, diff: ConfigDiff, author: Option<String>, tags: Vec<String>) -> Scene {
        let mut scenes = self.scenes.write().await;
        let scene_number = scenes.len() as u64 + 1;
        let act = scene_number / 10 + 1; // every 10 scenes = new act

        let scene = Scene {
            id: self.next_id(),
            act,
            scene_number,
            timestamp: chrono::Utc::now().to_rfc3339(),
            description,
            diff,
            author,
            tags,
            applied: true,
        };

        scenes.push(scene.clone());
        scene
    }

    /// Get all scenes in chronological order.
    pub async fn get_scenes(&self) -> Vec<Scene> {
        self.scenes.read().await.clone()
    }

    /// Replay: return scenes from a specific point or all.
    pub async fn replay(&self, from_scene: Option<u64>, to_scene: Option<u64>) -> ReplayResult {
        let scenes = self.scenes.read().await;
        let from = from_scene.unwrap_or(1);
        let to = to_scene.unwrap_or(scenes.len() as u64);

        let filtered: Vec<&Scene> = scenes.iter()
            .filter(|s| s.scene_number >= from && s.scene_number <= to)
            .collect();

        let total_diff = self.compute_cumulative_diff(&filtered);

        ReplayResult {
            total_scenes: filtered.len(),
            from_scene: from,
            to_scene: to,
            scenes: filtered.into_iter().cloned().collect(),
            cumulative_diff: total_diff,
            duration_ms: 0, // would compute from timestamps in real impl
        }
    }

    /// Undo a single scene by ID (not just rollback to previous).
    pub async fn undo_scene(&self, scene_id: &str) -> Result<UndoResult, String> {
        let mut scenes = self.scenes.write().await;
        let mut undo_stack = self.undo_stack.write().await;

        let idx = scenes.iter().position(|s| s.id == scene_id)
            .ok_or_else(|| format!("Scene '{}' not found", scene_id))?;

        let scene = scenes.remove(idx);

        // Generate inverse diff for undo
        let inverse_diff = ConfigDiff {
            added: scene.diff.removed.clone(),
            removed: scene.diff.added.clone(),
            modified: scene.diff.modified.iter().map(|m| ConfigChange {
                key: m.key.clone(),
                old_value: m.new_value.clone(),
                new_value: m.old_value.clone(),
            }).collect(),
        };

        undo_stack.push(scene.clone());

        Ok(UndoResult {
            undone_scene: scene,
            inverse_diff,
            remaining_scenes: scenes.len(),
        })
    }

    /// Create an alternative branch from a specific scene.
    pub async fn branch(&self, fork_scene_id: &str, branch_name: String, description: String) -> Result<Branch, String> {
        let scenes = self.scenes.read().await;
        let fork_idx = scenes.iter().position(|s| s.id == fork_scene_id)
            .ok_or_else(|| format!("Fork scene '{}' not found", fork_scene_id))?;

        // Take scenes up to and including the fork point
        let base_scenes: Vec<Scene> = scenes[..=fork_idx].to_vec();

        let branch = Branch {
            id: format!("branch-{}", chrono::Utc::now().timestamp_millis()),
            name: branch_name,
            description,
            fork_scene_id: fork_scene_id.to_string(),
            scenes: base_scenes,
            created_at: chrono::Utc::now().to_rfc3339(),
            active: true,
        };

        let mut branches = self.branches.write().await;
        branches.insert(branch.id.clone(), branch.clone());

        Ok(branch)
    }

    /// Add a scene to a branch.
    pub async fn add_to_branch(&self, branch_id: &str, description: String, diff: ConfigDiff) -> Result<Scene, String> {
        let mut branches = self.branches.write().await;
        let branch = branches.get_mut(branch_id)
            .ok_or_else(|| format!("Branch '{}' not found", branch_id))?;

        let scene_number = branch.scenes.len() as u64 + 1;
        let scene = Scene {
            id: self.next_id(),
            act: scene_number / 10 + 1,
            scene_number,
            timestamp: chrono::Utc::now().to_rfc3339(),
            description,
            diff,
            author: None,
            tags: vec!["branch".to_string()],
            applied: false,
        };

        branch.scenes.push(scene.clone());
        Ok(scene)
    }

    /// Compare two branches or a branch vs main timeline.
    pub async fn compare(&self, branch_id_a: Option<&str>, branch_id_b: Option<&str>) -> ComparisonResult {
        let scenes = self.scenes.read().await;
        let branches = self.branches.read().await;

        let timeline_a = if let Some(id) = branch_id_a {
            branches.get(id).map(|b| b.scenes.clone()).unwrap_or_default()
        } else {
            scenes.clone()
        };

        let timeline_b = if let Some(id) = branch_id_b {
            branches.get(id).map(|b| b.scenes.clone()).unwrap_or_default()
        } else {
            scenes.clone()
        };

        ComparisonResult {
            timeline_a_scenes: timeline_a.len(),
            timeline_b_scenes: timeline_b.len(),
            shared_ancestor: None, // simplified
            divergence_point: None,
            unique_to_a: timeline_a.len().saturating_sub(timeline_b.len()),
            unique_to_b: timeline_b.len().saturating_sub(timeline_a.len()),
        }
    }

    /// Get all branches.
    pub async fn get_branches(&self) -> Vec<Branch> {
        self.branches.read().await.values().cloned().collect()
    }

    /// Compute cumulative diff across multiple scenes.
    fn compute_cumulative_diff(&self, scenes: &[&Scene]) -> ConfigDiff {
        let mut added: Vec<ConfigEntry> = Vec::new();
        let mut removed: Vec<ConfigEntry> = Vec::new();
        let mut modified: Vec<ConfigChange> = Vec::new();

        for scene in scenes {
            for entry in &scene.diff.added {
                if !added.iter().any(|a| a.key == entry.key) {
                    added.push(entry.clone());
                }
            }
            for entry in &scene.diff.removed {
                if !removed.iter().any(|r| r.key == entry.key) {
                    removed.push(entry.clone());
                }
            }
            for change in &scene.diff.modified {
                if let Some(existing) = modified.iter_mut().find(|m| m.key == change.key) {
                    existing.new_value = change.new_value.clone();
                } else {
                    modified.push(change.clone());
                }
            }
        }

        ConfigDiff { added, removed, modified }
    }
}

// ─── API types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ReplayResult {
    pub total_scenes: usize,
    pub from_scene: u64,
    pub to_scene: u64,
    pub scenes: Vec<Scene>,
    pub cumulative_diff: ConfigDiff,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct UndoResult {
    pub undone_scene: Scene,
    pub inverse_diff: ConfigDiff,
    pub remaining_scenes: usize,
}

#[derive(Debug, Serialize)]
pub struct ComparisonResult {
    pub timeline_a_scenes: usize,
    pub timeline_b_scenes: usize,
    pub shared_ancestor: Option<String>,
    pub divergence_point: Option<String>,
    pub unique_to_a: usize,
    pub unique_to_b: usize,
}

#[derive(Debug, Deserialize)]
pub struct RecordRequest {
    pub description: String,
    pub diff: ConfigDiff,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ReplayQuery {
    pub from: Option<u64>,
    pub to: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UndoRequest {
    pub scene_id: String,
}

#[derive(Debug, Deserialize)]
pub struct BranchRequest {
    pub fork_scene_id: String,
    pub name: String,
    pub description: Option<String>,
}

// ─── Global theater ──────────────────────────────────────────────────────

use std::sync::LazyLock;
static THEATER: LazyLock<Arc<Theater>> = LazyLock::new(|| Arc::new(Theater::new()));

/// POST /api/theater/record — Record a config change as a scene
pub async fn handle_record(Json(req): Json<RecordRequest>) -> Result<impl IntoResponse, AppError> {
    let theater = THEATER.clone();
    let scene = theater.record(
        req.description, req.diff, req.author,
        req.tags.unwrap_or_default(),
    ).await;
    Ok(Json(serde_json::to_value(&scene).unwrap()))
}

/// GET /api/theater/replay — Replay scenes
pub async fn handle_replay(Query(q): Query<ReplayQuery>) -> impl IntoResponse {
    let theater = THEATER.clone();
    let result = theater.replay(q.from, q.to).await;
    Json(serde_json::to_value(&result).unwrap())
}

/// POST /api/theater/undo — Undo a single scene
pub async fn handle_undo(Json(req): Json<UndoRequest>) -> Result<impl IntoResponse, AppError> {
    let theater = THEATER.clone();
    match theater.undo_scene(&req.scene_id).await {
        Ok(result) => Ok(Json(serde_json::to_value(&result).unwrap())),
        Err(e) => Err(AppError::BadRequest(e)),
    }
}

/// POST /api/theater/branch — Create alternative timeline
pub async fn handle_branch(Json(req): Json<BranchRequest>) -> Result<impl IntoResponse, AppError> {
    let theater = THEATER.clone();
    match theater.branch(&req.fork_scene_id, req.name, req.description.unwrap_or_default()).await {
        Ok(branch) => Ok(Json(serde_json::to_value(&branch).unwrap())),
        Err(e) => Err(AppError::BadRequest(e)),
    }
}

/// GET /api/theater/branches — List all branches
pub async fn handle_branches() -> impl IntoResponse {
    let theater = THEATER.clone();
    let branches = theater.get_branches().await;
    Json(serde_json::json!({ "branches": branches }))
}
