use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade, Message},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use futures::{StreamExt, SinkExt};

// ─── Operational Transformation Types ────────────────────────────────────

/// An operation represents a single edit action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum Operation {
    /// Insert text at position
    Insert { pos: usize, text: String },
    /// Delete text at position
    Delete { pos: usize, len: usize },
    /// Cursor/selection update
    Cursor { pos: usize, sel_start: Option<usize>, sel_end: Option<usize> },
}

/// An operation with metadata for transformation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaggedOp {
    pub id: String,
    pub client_id: String,
    pub revision: u64,
    pub operation: Operation,
    pub timestamp: String,
}

/// A cursor/selection from a remote peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerCursor {
    pub client_id: String,
    pub client_name: String,
    pub pos: usize,
    pub sel_start: Option<usize>,
    pub sel_end: Option<usize>,
    pub color: String,
}

// ─── Operational Transformation Engine ───────────────────────────────────

pub struct OTEngine;

impl OTEngine {
    /// Transform two concurrent operations so they can be applied in any order.
    pub fn transform(a: &Operation, b: &Operation) -> (Operation, Operation) {
        match (a, b) {
            // Insert vs Insert
            (Operation::Insert { pos: pa, text: ta }, Operation::Insert { pos: pb, text: tb }) => {
                if pa <= pb {
                    (
                        Operation::Insert { pos: *pa, text: ta.clone() },
                        Operation::Insert { pos: pb + ta.len(), text: tb.clone() },
                    )
                } else {
                    (
                        Operation::Insert { pos: pa + tb.len(), text: ta.clone() },
                        Operation::Insert { pos: *pb, text: tb.clone() },
                    )
                }
            }
            // Insert vs Delete
            (Operation::Insert { pos: pa, text: ta }, Operation::Delete { pos: pb, len: lb }) => {
                if pa <= pb {
                    (
                        Operation::Insert { pos: *pa, text: ta.clone() },
                        Operation::Delete { pos: pb + ta.len(), len: *lb },
                    )
                } else if pa > pb + lb {
                    (
                        Operation::Insert { pos: pa - lb, text: ta.clone() },
                        Operation::Delete { pos: *pb, len: *lb },
                    )
                } else {
                    // Insert falls within deleted range
                    (
                        Operation::Insert { pos: *pb, text: ta.clone() },
                        Operation::Delete { pos: *pb, len: *lb },
                    )
                }
            }
            // Delete vs Insert — swap and transform
            (Operation::Delete { .. }, Operation::Insert { .. }) => {
                let (b2, a2) = Self::transform(b, a);
                (a2, b2)
            }
            // Delete vs Delete
            (Operation::Delete { pos: pa, len: la }, Operation::Delete { pos: pb, len: lb }) => {
                if pa + la <= *pb {
                    (
                        Operation::Delete { pos: *pa, len: *la },
                        Operation::Delete { pos: pb - la, len: *lb },
                    )
                } else if pb + lb <= *pa {
                    (
                        Operation::Delete { pos: pa - lb, len: *la },
                        Operation::Delete { pos: *pb, len: *lb },
                    )
                } else {
                    // Overlapping deletes
                    let start = (*pa).min(*pb);
                    let end = (pa + la).max(pb + lb);
                    let overlap_start = (*pa).max(*pb);
                    let overlap_end = (pa + la).min(pb + lb);
                    let overlap = if overlap_end > overlap_start { overlap_end - overlap_start } else { 0 };

                    (
                        Operation::Delete { pos: start, len: (end - start).saturating_sub(overlap) },
                        Operation::Delete { pos: start, len: 0 }, // no-op: already deleted
                    )
                }
            }
            // Cursor ops don't transform with edits — just pass through
            (Operation::Cursor { .. }, _) => (a.clone(), b.clone()),
            (_, Operation::Cursor { .. }) => (a.clone(), b.clone()),
        }
    }

    /// Apply an operation to a text buffer.
    pub fn apply(text: &str, op: &Operation) -> Result<String, String> {
        match op {
            Operation::Insert { pos, text: ins } => {
                if *pos > text.len() {
                    return Err(format!("Insert position {} out of bounds (len={})", pos, text.len()));
                }
                let mut result = text.to_string();
                result.insert_str(*pos, ins);
                Ok(result)
            }
            Operation::Delete { pos, len } => {
                if pos + len > text.len() {
                    return Err(format!("Delete range {}..{} out of bounds (len={})", pos, pos + len, text.len()));
                }
                let mut result = text.to_string();
                result.replace_range(pos..pos + len, "");
                Ok(result)
            }
            Operation::Cursor { .. } => Ok(text.to_string()), // cursor ops don't change text
        }
    }
}

// ─── Collaborative Session ───────────────────────────────────────────────

pub struct CollabSession {
    /// Current document content
    document: RwLock<String>,
    /// Revision counter
    revision: RwLock<u64>,
    /// Operation history (for OT)
    op_history: RwLock<Vec<TaggedOp>>,
    /// Connected peers
    peers: RwLock<HashMap<String, PeerCursor>>,
    /// Broadcast channel for real-time updates
    tx: broadcast::Sender<CollabMessage>,
    /// Color palette for cursors
    colors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum CollabMessage {
    Operation { op: TaggedOp },
    CursorUpdate { cursor: PeerCursor },
    PeerJoin { client_id: String, name: String },
    PeerLeave { client_id: String },
    Sync { content: String, revision: u64 },
    Ack { op_id: String, revision: u64 },
}

impl CollabSession {
    pub fn new(initial_content: String) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            document: RwLock::new(initial_content),
            revision: RwLock::new(0),
            op_history: RwLock::new(Vec::new()),
            peers: RwLock::new(HashMap::new()),
            tx,
            colors: vec![
                "#e74c3c".into(), "#3498db".into(), "#2ecc71".into(),
                "#f39c12".into(), "#9b59b6".into(), "#1abc9c".into(),
                "#e67e22".into(), "#34495e".into(),
            ],
        }
    }

    /// Apply an operation and broadcast.
    pub async fn apply_operation(&self, mut op: TaggedOp) -> Result<u64, String> {
        let mut doc = self.document.write().await;
        let mut rev = self.revision.write().await;
        let mut history = self.op_history.write().await;

        // Transform against concurrent ops
        for prev_op in history.iter().rev() {
            if prev_op.revision >= op.revision && prev_op.client_id != op.client_id {
                let (transformed, _) = OTEngine::transform(&op.operation, &prev_op.operation);
                op.operation = transformed;
            }
        }

        // Apply
        let new_doc = OTEngine::apply(&doc, &op.operation)?;
        *doc = new_doc;
        *rev += 1;

        let ack_rev = *rev;
        op.revision = ack_rev;
        history.push(op.clone());

        // Broadcast
        let _ = self.tx.send(CollabMessage::Operation { op: op.clone() });
        let _ = self.tx.send(CollabMessage::Ack { op_id: op.id, revision: ack_rev });

        Ok(ack_rev)
    }

    /// Update a peer's cursor position.
    pub async fn update_cursor(&self, cursor: PeerCursor) {
        let mut peers = self.peers.write().await;
        peers.insert(cursor.client_id.clone(), cursor.clone());
        let _ = self.tx.send(CollabMessage::CursorUpdate { cursor });
    }

    /// Add a peer to the session.
    pub async fn join(&self, client_id: String, name: String) -> (String, u64, Vec<PeerCursor>) {
        let doc = self.document.read().await;
        let rev = self.revision.read().await;
        let peers = self.peers.read().await;

        let color_idx = peers.len() % self.colors.len();

        let cursor = PeerCursor {
            client_id: client_id.clone(),
            client_name: name.clone(),
            pos: 0,
            sel_start: None,
            sel_end: None,
            color: self.colors[color_idx].clone(),
        };

        let existing_peers: Vec<PeerCursor> = peers.values().cloned().collect();
        drop(peers);

        let mut peers_mut = self.peers.write().await;
        peers_mut.insert(client_id.clone(), cursor);

        let _ = self.tx.send(CollabMessage::PeerJoin {
            client_id: client_id.clone(),
            name,
        });

        (doc.clone(), *rev, existing_peers)
    }

    /// Remove a peer from the session.
    pub async fn leave(&self, client_id: &str) {
        let mut peers = self.peers.write().await;
        peers.remove(client_id);
        let _ = self.tx.send(CollabMessage::PeerLeave {
            client_id: client_id.to_string(),
        });
    }

    /// Subscribe to updates.
    pub fn subscribe(&self) -> broadcast::Receiver<CollabMessage> {
        self.tx.subscribe()
    }

    /// Get current document content.
    pub async fn get_content(&self) -> (String, u64) {
        let doc = self.document.read().await;
        let rev = self.revision.read().await;
        (doc.clone(), *rev)
    }

    /// Get all connected peers.
    pub async fn get_peers(&self) -> Vec<PeerCursor> {
        self.peers.read().await.values().cloned().collect()
    }
}

// ─── WebSocket Handler ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct WsInit {
    client_id: String,
    client_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    #[serde(rename = "init")]
    Init { client_id: String, client_name: Option<String> },
    #[serde(rename = "operation")]
    Operation { op: TaggedOp },
    #[serde(rename = "cursor")]
    Cursor { pos: usize, sel_start: Option<usize>, sel_end: Option<usize> },
}

use std::sync::LazyLock;
static SESSION: LazyLock<Arc<CollabSession>> = LazyLock::new(|| {
    Arc::new(CollabSession::new(
        "# NixOS Configuration\n# Edit collaboratively!\n\n{ config, pkgs, ... }:\n{\n  # Your config here\n}\n".into()
    ))
});

/// WS /api/collab/ws — Collaborative editing WebSocket
pub async fn handle_ws(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let session = SESSION.clone();
    let (mut sender, mut receiver) = socket.split();
    let mut rx = session.subscribe();

    let client_id: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));

    // Forward broadcast messages to this client
    let cid_fwd = client_id.clone();
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap_or_default();
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // Process incoming messages
    let cid_recv = client_id.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                    match ws_msg {
                        WsMessage::Init { client_id: id, client_name } => {
                            let name = client_name.unwrap_or_else(|| "Anonymous".into());
                            let (content, rev, peers) = session.join(id.clone(), name).await;
                            *cid_recv.write().await = Some(id.clone());

                            // Send sync
                            let sync = CollabMessage::Sync { content, revision: rev };
                            let _ = session.tx.send(sync);
                        }
                        WsMessage::Operation { op } => {
                            let _ = session.apply_operation(op).await;
                        }
                        WsMessage::Cursor { pos, sel_start, sel_end } => {
                            let cid = cid_recv.read().await;
                            if let Some(ref id) = *cid {
                                let cursor = PeerCursor {
                                    client_id: id.clone(),
                                    client_name: "".into(),
                                    pos,
                                    sel_start,
                                    sel_end,
                                    color: "#3498db".into(),
                                };
                                session.update_cursor(cursor).await;
                            }
                        }
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }

    // Cleanup
    let cid = client_id.read().await;
    if let Some(ref id) = *cid {
        session.leave(id).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ot_insert_insert() {
        let a = Operation::Insert { pos: 3, text: "X".into() };
        let b = Operation::Insert { pos: 5, text: "Y".into() };
        let (a2, b2) = OTEngine::transform(&a, &b);

        // Apply a then b2
        let text = "abcdef";
        let t1 = OTEngine::apply(text, &a).unwrap();
        let t2 = OTEngine::apply(&t1, &b2).unwrap();

        // Apply b then a2
        let t3 = OTEngine::apply(text, &b).unwrap();
        let t4 = OTEngine::apply(&t3, &a2).unwrap();

        assert_eq!(t2, t4, "OT convergence failed");
    }

    #[test]
    fn test_ot_insert_delete() {
        let a = Operation::Insert { pos: 2, text: "XX".into() };
        let b = Operation::Delete { pos: 4, len: 2 };
        let (a2, b2) = OTEngine::transform(&a, &b);

        let text = "abcdef";
        let t1 = OTEngine::apply(text, &a).unwrap();
        let t2 = OTEngine::apply(&t1, &b2).unwrap();

        let t3 = OTEngine::apply(text, &b).unwrap();
        let t4 = OTEngine::apply(&t3, &a2).unwrap();

        assert_eq!(t2, t4, "OT insert/delete convergence failed");
    }

    #[tokio::test]
    async fn test_session_join_leave() {
        let session = CollabSession::new("hello".into());
        let (content, rev, peers) = session.join("client1".into(), "Alice".into()).await;
        assert_eq!(content, "hello");
        assert_eq!(rev, 0);
        assert!(peers.is_empty());

        let peer_list = session.get_peers().await;
        assert_eq!(peer_list.len(), 1);

        session.leave("client1").await;
        let peer_list = session.get_peers().await;
        assert!(peer_list.is_empty());
    }

    #[tokio::test]
    async fn test_collaborative_edit() {
        let session = CollabSession::new("hello world".into());
        session.join("c1".into(), "Alice".into()).await;
        session.join("c2".into(), "Bob".into()).await;

        let op1 = TaggedOp {
            id: "op1".into(),
            client_id: "c1".into(),
            revision: 0,
            operation: Operation::Insert { pos: 5, text: " beautiful".into() },
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        session.apply_operation(op1).await.unwrap();

        let (content, rev) = session.get_content().await;
        assert_eq!(content, "hello beautiful world");
        assert_eq!(rev, 1);
    }
}
