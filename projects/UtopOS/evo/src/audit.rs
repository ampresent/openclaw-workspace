use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::cmd::AppStateRef;
use crate::error::AppError;

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub action: String,
    pub method: String,
    pub path: String,
    pub params_hash: String,
    pub client_ip: String,
    pub result: String,
    pub duration_ms: u64,
}

/// Audit log writer — thread-safe JSONL file appender
pub struct AuditLog {
    path: std::path::PathBuf,
}

impl AuditLog {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let dir = std::path::Path::new(&home).join(".nix-evo");
        std::fs::create_dir_all(&dir).ok();
        Self {
            path: dir.join("audit.log"),
        }
    }

    /// Append an entry to the JSONL audit log
    pub fn write(&self, entry: &AuditEntry) -> Result<(), AppError> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| AppError::IoError {
                path: self.path.display().to_string(),
                message: e.to_string(),
            })?;

        let line = serde_json::to_string(entry).map_err(|e| AppError::Internal {
            message: format!("audit serialize failed: {e}"),
        })?;

        writeln!(file, "{line}").map_err(|e| AppError::IoError {
            path: self.path.display().to_string(),
            message: e.to_string(),
        })?;

        Ok(())
    }

    /// Query entries, optionally filtered by action, last N entries
    pub fn query(&self, filter: &AuditQuery) -> Result<Vec<AuditEntry>, AppError> {
        let file = std::fs::File::open(&self.path).map_err(|e| AppError::IoError {
            path: self.path.display().to_string(),
            message: e.to_string(),
        })?;

        let reader = std::io::BufReader::new(file);
        let mut entries: Vec<AuditEntry> = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| AppError::IoError {
                path: self.path.display().to_string(),
                message: e.to_string(),
            })?;

            if line.trim().is_empty() {
                continue;
            }

            let entry: AuditEntry = serde_json::from_str(&line).map_err(|e| AppError::Internal {
                message: format!("audit parse failed: {e}"),
            })?;

            // Filter by action
            if let Some(ref action) = filter.action {
                if !entry.action.contains(action) {
                    continue;
                }
            }

            // Filter by path
            if let Some(ref path) = filter.path {
                if !entry.path.contains(path) {
                    continue;
                }
            }

            entries.push(entry);
        }

        // Apply limit (most recent first)
        let limit = filter.limit.unwrap_or(100).min(1000);
        let offset = entries.len().saturating_sub(limit);
        Ok(entries[offset..].to_vec())
    }

    /// Get the path for diagnostics
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Count total entries
    pub fn count(&self) -> Result<u64, AppError> {
        let file = std::fs::File::open(&self.path).map_err(|e| AppError::IoError {
            path: self.path.display().to_string(),
            message: e.to_string(),
        })?;
        let reader = std::io::BufReader::new(file);
        Ok(reader.lines().filter_map(|l| l.ok()).filter(|l| !l.trim().is_empty()).count() as u64)
    }
}

/// Global audit log instance
static AUDIT_LOG: std::sync::OnceLock<AuditLog> = std::sync::OnceLock::new();

pub fn get_audit_log() -> &'static AuditLog {
    AUDIT_LOG.get_or_init(|| AuditLog::new())
}

/// Hash parameters for audit (privacy-preserving — stores hash, not raw data)
pub fn hash_params(params: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    params.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Log an API call to the audit trail
pub fn log_api_call(
    method: &str,
    path: &str,
    params: &str,
    result: &str,
    client_ip: &str,
    duration_ms: u64,
) {
    let entry = AuditEntry {
        timestamp: chrono_now(),
        action: extract_action(path),
        method: method.to_string(),
        path: path.to_string(),
        params_hash: hash_params(params),
        client_ip: client_ip.to_string(),
        result: result.to_string(),
        duration_ms,
    };

    if let Err(e) = get_audit_log().write(&entry) {
        tracing::error!("Failed to write audit log: {e}");
    }
}

/// Extract a human-readable action from the API path
fn extract_action(path: &str) -> String {
    match path {
        p if p.contains("/snapshot") => "system_snapshot".to_string(),
        p if p.contains("/logs") => "service_logs".to_string(),
        p if p.contains("/config/validate") => "config_validate".to_string(),
        p if p.contains("/config/apply") => "config_apply".to_string(),
        p if p.contains("/config") => "config_read".to_string(),
        p if p.contains("/package") => "package_info".to_string(),
        p if p.contains("/generations") => "generation_diff".to_string(),
        p if p.contains("/rollback") => "rollback".to_string(),
        p if p.contains("/dashboard") => "dashboard".to_string(),
        p if p.contains("/audit") => "audit_query".to_string(),
        p if p.contains("/healer") => "healer_status".to_string(),
        p if p.contains("/flake") => "flake_convert".to_string(),
        _ => "unknown".to_string(),
    }
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_secs())
}

// ─── API Handlers ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AuditQuery {
    pub action: Option<String>,
    pub path: Option<String>,
    pub limit: Option<usize>,
}

/// GET /api/audit — query audit logs
pub async fn handle_query(
    Query(query): Query<AuditQuery>,
    State(_state): AppStateRef,
) -> Result<impl IntoResponse, AppError> {
    let log = get_audit_log();
    let entries = log.query(&query)?;
    let total = log.count().unwrap_or(0);

    Ok(Json(serde_json::json!({
        "total": total,
        "returned": entries.len(),
        "log_path": log.path().display().to_string(),
        "entries": entries,
    })))
}

/// GET /api/audit/stats — audit log statistics
pub async fn handle_stats(
    State(_state): AppStateRef,
) -> Result<impl IntoResponse, AppError> {
    let log = get_audit_log();
    let entries = log.query(&AuditQuery { action: None, path: None, limit: Some(1000) })?;

    // Compute stats
    let mut action_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut total_duration = 0u64;
    let mut error_count = 0u64;

    for entry in &entries {
        *action_counts.entry(entry.action.clone()).or_insert(0) += 1;
        total_duration += entry.duration_ms;
        if entry.result.contains("error") || entry.result.contains("Error") {
            error_count += 1;
        }
    }

    let avg_duration = if entries.is_empty() { 0 } else { total_duration / entries.len() as u64 };

    Ok(Json(serde_json::json!({
        "total_entries": entries.len(),
        "error_count": error_count,
        "avg_duration_ms": avg_duration,
        "action_counts": action_counts,
        "log_path": log.path().display().to_string(),
    })))
}
