use axum::{
    extract::{ws::{Message, WebSocket}, State, WebSocketUpgrade},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::cmd::AppStateRef;

/// A config file change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    pub event_type: String,  // "created", "modified", "deleted", "moved"
    pub path: String,
    pub timestamp: String,
    pub size: Option<u64>,
    pub git_commit: Option<GitCommitInfo>,
    pub diff: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub message: String,
    pub timestamp: String,
}

/// WS message from client
#[derive(Debug, Deserialize)]
pub struct StreamCommand {
    pub action: String,  // "subscribe", "unsubscribe", "history"
    pub paths: Option<Vec<String>>,
    pub limit: Option<usize>,
}

/// WS message to client
#[derive(Debug, Serialize)]
pub struct StreamEvent {
    pub event_type: String,
    pub data: serde_json::Value,
}

/// Global broadcast channel for config changes
static CHANNEL: std::sync::OnceLock<broadcast::Sender<ConfigChangeEvent>> = std::sync::OnceLock::new();

fn get_channel() -> &'static broadcast::Sender<ConfigChangeEvent> {
    CHANNEL.get_or_init(|| {
        let (tx, _) = broadcast::channel(1024);
        tx
    })
}

/// Start the background file watcher
pub fn start_file_watcher() {
    let tx = get_channel().clone();

    tokio::spawn(async move {
        let watch_paths = vec![
            "/etc/nixos".to_string(),
            "/etc/nix".to_string(),
        ];

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        let mut last_mtimes: std::collections::HashMap<String, std::time::SystemTime> = std::collections::HashMap::new();

        loop {
            interval.tick().await;

            for base_path in &watch_paths {
                let entries = match std::fs::read_dir(base_path) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                for entry in entries.flatten() {
                    let path = entry.path();
                    let path_str = path.to_string_lossy().to_string();

                    // Only watch .nix files and known config files
                    if !path_str.ends_with(".nix") && !path_str.ends_with(".conf") &&
                       !path_str.ends_with("configuration.nix") && !path_str.ends_with("hardware-configuration.nix") {
                        continue;
                    }

                    let metadata = match std::fs::metadata(&path) {
                        Ok(m) => m,
                        Err(_) => continue,
                    };

                    let modified = match metadata.modified() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };

                    let is_new = !last_mtimes.contains_key(&path_str);
                    let changed = last_mtimes.get(&path_str).map_or(true, |&last| modified > last);

                    if changed {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default();

                        let event_type = if is_new { "discovered" } else { "modified" };

                        // Try to get git info
                        let git_commit = get_git_info(&path_str).await;

                        // Get a small diff preview
                        let diff = if !is_new { get_file_diff_preview(&path_str).await } else { None };

                        let event = ConfigChangeEvent {
                            event_type: event_type.to_string(),
                            path: path_str.clone(),
                            timestamp: format!("{}", now.as_secs()),
                            size: Some(metadata.len()),
                            git_commit,
                            diff,
                        };

                        let _ = tx.send(event);
                        last_mtimes.insert(path_str, modified);
                    }
                }
            }
        }
    });
}

/// Try to get git commit info for a file
async fn get_git_info(path: &str) -> Option<GitCommitInfo> {
    let dir = std::path::Path::new(path).parent()?;
    let output = tokio::process::Command::new("git")
        .args(["log", "-1", "--format=%H%n%h%n%an%n%s%n%ci", "--", path])
        .current_dir(dir)
        .output()
        .await
        .ok()?;

    if !output.status.success() { return None; }

    let text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = text.trim().lines().collect();
    if lines.len() < 5 { return None; }

    Some(GitCommitInfo {
        hash: lines[0].to_string(),
        short_hash: lines[1].to_string(),
        author: lines[2].to_string(),
        message: lines[3].to_string(),
        timestamp: lines[4].to_string(),
    })
}

/// Get a short diff preview for a file (compared to git HEAD)
async fn get_file_diff_preview(path: &str) -> Option<String> {
    let dir = std::path::Path::new(path).parent()?;
    let output = tokio::process::Command::new("git")
        .args(["diff", "--no-color", "-U3", "HEAD", "--", path])
        .current_dir(dir)
        .output()
        .await
        .ok()?;

    if !output.status.success() { return None; }

    let diff = String::from_utf8_lossy(&output.stdout);
    if diff.is_empty() { return None; }

    // Truncate to first 500 chars
    let preview = if diff.len() > 500 {
        format!("{}...\n(truncated)", &diff[..500])
    } else {
        diff.to_string()
    };

    Some(preview)
}

/// WS /api/stream/config — WebSocket endpoint for real-time config changes
pub async fn handle_ws(
    ws: WebSocketUpgrade,
    State(_state): AppStateRef,
) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = get_channel().subscribe();

    // Send initial connection event
    let welcome = StreamEvent {
        event_type: "connected".into(),
        data: serde_json::json!({
            "watch_paths": ["/etc/nixos", "/etc/nix"],
            "check_interval_secs": 5,
            "message": "Watching for NixOS config file changes"
        }),
    };
    let _ = sender.send(Message::Text(serde_json::to_string(&welcome).unwrap_or_default().into())).await;

    // Spawn task to forward broadcast events to WS client
    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let ws_event = StreamEvent {
                event_type: "config_change".into(),
                data: serde_json::to_value(&event).unwrap_or_default(),
            };
            if sender.send(Message::Text(serde_json::to_string(&ws_event).unwrap_or_default().into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming commands from client
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(cmd) = serde_json::from_str::<StreamCommand>(&text) {
                    match cmd.action.as_str() {
                        "ping" => {
                            // Respond to keepalive
                        }
                        "history" => {
                            // TODO: implement history from git log
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }
}
