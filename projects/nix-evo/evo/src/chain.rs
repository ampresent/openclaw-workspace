use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use sha2::{Sha256, Digest};

use crate::error::AppError;

// ─── Block: one config change in the chain ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: String,
    pub data: BlockData,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    pub action: String,           // "config_change", "service_restart", "package_install"
    pub description: String,
    pub config_snapshot: Option<String>,  // config content hash
    pub diff_summary: Option<String>,
    pub author: Option<String>,
    pub generation: Option<u64>,
}

impl Block {
    /// Compute SHA-256 hash of the block contents.
    fn compute_hash(index: u64, timestamp: &str, data: &BlockData, previous_hash: &str, nonce: u64) -> String {
        let content = format!(
            "{}:{}:{}:{}:{}",
            index,
            timestamp,
            serde_json::to_string(data).unwrap_or_default(),
            previous_hash,
            nonce,
        );
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify this block's hash is correct.
    pub fn verify_hash(&self) -> bool {
        let computed = Self::compute_hash(self.index, &self.timestamp, &self.data, &self.previous_hash, self.nonce);
        computed == self.hash
    }
}

// ─── Chain: the immutable audit trail ────────────────────────────────────

pub struct Chain {
    blocks: RwLock<Vec<Block>>,
    difficulty: usize,  // number of leading zeros required (simplified PoW)
}

impl Chain {
    pub fn new() -> Self {
        let genesis = Self::create_genesis_block();
        Self {
            blocks: RwLock::new(vec![genesis]),
            difficulty: 0, // no PoW for speed; set > 0 for real integrity
        }
    }

    fn create_genesis_block() -> Block {
        let timestamp = "2024-01-01T00:00:00Z".to_string();
        let data = BlockData {
            action: "genesis".into(),
            description: "NixOS config chain genesis block".into(),
            config_snapshot: None,
            diff_summary: None,
            author: Some("nix-evo".into()),
            generation: Some(0),
        };
        let previous_hash = "0".repeat(64);
        let hash = Block::compute_hash(0, &timestamp, &data, &previous_hash, 0);

        Block { index: 0, timestamp, data, previous_hash, hash, nonce: 0 }
    }

    /// Add a new block to the chain.
    pub async fn add_block(&self, data: BlockData) -> Block {
        let blocks = self.blocks.read().await;
        let previous = blocks.last().unwrap(); // genesis always exists
        let index = previous.index + 1;
        let timestamp = chrono::Utc::now().to_rfc3339();
        let previous_hash = previous.hash.clone();

        // Simple mining: find nonce that produces hash with required leading zeros
        let mut nonce = 0u64;
        let hash = loop {
            let h = Block::compute_hash(index, &timestamp, &data, &previous_hash, nonce);
            if self.difficulty == 0 || h.starts_with(&"0".repeat(self.difficulty)) {
                break h;
            }
            nonce += 1;
        };

        let block = Block { index, timestamp, data, previous_hash, hash, nonce };
        drop(blocks);

        let mut blocks = self.blocks.write().await;
        blocks.push(block.clone());
        block
    }

    /// Verify the entire chain's integrity.
    pub async fn verify(&self) -> VerificationResult {
        let blocks = self.blocks.read().await;
        let mut errors: Vec<String> = Vec::new();

        if blocks.is_empty() {
            return VerificationResult {
                valid: true,
                total_blocks: 0,
                verified_at: chrono::Utc::now().to_rfc3339(),
                errors: vec!["Empty chain".to_string()],
            };
        }

        // Check genesis
        if blocks[0].index != 0 {
            errors.push(format!("Genesis block has wrong index: {}", blocks[0].index));
        }

        // Verify each block
        for i in 0..blocks.len() {
            let block = &blocks[i];

            // Verify hash
            if !block.verify_hash() {
                errors.push(format!("Block {} has invalid hash", block.index));
            }

            // Verify chain linkage
            if i > 0 {
                let prev = &blocks[i - 1];
                if block.previous_hash != prev.hash {
                    errors.push(format!(
                        "Block {} previous_hash ({}) doesn't match block {} hash ({})",
                        block.index,
                        &block.previous_hash[..16],
                        prev.index,
                        &prev.hash[..16],
                    ));
                }
                if block.index != prev.index + 1 {
                    errors.push(format!("Block {} has wrong index (expected {})", block.index, prev.index + 1));
                }
            }
        }

        VerificationResult {
            valid: errors.is_empty(),
            total_blocks: blocks.len(),
            verified_at: chrono::Utc::now().to_rfc3339(),
            errors,
        }
    }

    /// Get the full chain history.
    pub async fn get_history(&self, limit: Option<usize>) -> Vec<Block> {
        let blocks = self.blocks.read().await;
        let start = if let Some(l) = limit {
            blocks.len().saturating_sub(l)
        } else {
            0
        };
        blocks[start..].to_vec()
    }

    /// Get the latest block.
    pub async fn get_latest(&self) -> Option<Block> {
        self.blocks.read().await.last().cloned()
    }

    /// Get chain statistics.
    pub async fn stats(&self) -> ChainStats {
        let blocks = self.blocks.read().await;
        let mut action_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        for block in blocks.iter() {
            *action_counts.entry(block.data.action.clone()).or_insert(0) += 1;
        }

        ChainStats {
            total_blocks: blocks.len(),
            first_block: blocks.first().map(|b| b.timestamp.clone()),
            last_block: blocks.last().map(|b| b.timestamp.clone()),
            action_counts,
        }
    }
}

// ─── API types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct VerificationResult {
    pub valid: bool,
    pub total_blocks: usize,
    pub verified_at: String,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ChainStats {
    pub total_blocks: usize,
    pub first_block: Option<String>,
    pub last_block: Option<String>,
    pub action_counts: std::collections::HashMap<String, u64>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<usize>,
    pub action: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddBlockRequest {
    pub action: String,
    pub description: String,
    pub config_snapshot: Option<String>,
    pub diff_summary: Option<String>,
    pub author: Option<String>,
    pub generation: Option<u64>,
}

// ─── Global chain ────────────────────────────────────────────────────────

use std::sync::LazyLock;
static CHAIN: LazyLock<Arc<Chain>> = LazyLock::new(|| Arc::new(Chain::new()));

/// GET /api/chain/verify — Verify chain integrity
pub async fn handle_verify() -> impl IntoResponse {
    let chain = CHAIN.clone();
    let result = chain.verify().await;
    Json(serde_json::to_value(&result).unwrap())
}

/// GET /api/chain/history — Get chain history
pub async fn handle_history(Query(q): Query<HistoryQuery>) -> impl IntoResponse {
    let chain = CHAIN.clone();
    let mut blocks = chain.get_history(q.limit).await;

    if let Some(ref action) = q.action {
        blocks.retain(|b| b.data.action == *action);
    }

    let stats = chain.stats().await;
    Json(serde_json::json!({
        "blocks": blocks,
        "stats": stats,
    }))
}

/// POST /api/chain/add — Add a block (internal use)
pub async fn handle_add_block(Json(req): Json<AddBlockRequest>) -> impl IntoResponse {
    let chain = CHAIN.clone();
    let data = BlockData {
        action: req.action,
        description: req.description,
        config_snapshot: req.config_snapshot,
        diff_summary: req.diff_summary,
        author: req.author,
        generation: req.generation,
    };
    let block = chain.add_block(data).await;
    Json(serde_json::to_value(&block).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_genesis_block() {
        let chain = Chain::new();
        let blocks = chain.get_history(None).await;
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].index, 0);
    }

    #[tokio::test]
    async fn test_add_and_verify() {
        let chain = Chain::new();
        let data = BlockData {
            action: "config_change".into(),
            description: "Enable nginx".into(),
            config_snapshot: Some("abc123".into()),
            diff_summary: Some("+ services.nginx.enable = true".into()),
            author: Some("admin".into()),
            generation: Some(1),
        };
        let block = chain.add_block(data).await;
        assert_eq!(block.index, 1);
        assert!(block.verify_hash());

        let verification = chain.verify().await;
        assert!(verification.valid);
        assert_eq!(verification.total_blocks, 2);
    }

    #[tokio::test]
    async fn test_tamper_detection() {
        let chain = Chain::new();
        let data = BlockData {
            action: "config_change".into(),
            description: "Test".into(),
            config_snapshot: None, diff_summary: None, author: None, generation: None,
        };
        chain.add_block(data).await;

        // Tamper with a block
        {
            let mut blocks = chain.blocks.write().await;
            blocks[1].data.description = "TAMPERED".into();
        }

        let verification = chain.verify().await;
        assert!(!verification.valid);
        assert!(!verification.errors.is_empty());
    }
}
