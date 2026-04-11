//! Backup and disaster recovery for NixOS configurations.
//!
//! Provides snapshot creation before config applies, backup rotation,
//! and restoration endpoints.

use axum::Json;
use serde::{Deserialize, Serialize};
use crate::error::AppError;
use super::{AppStateRef, run_cmd};
use std::time::{SystemTime, UNIX_EPOCH};

/// Metadata about a single backup
#[derive(Serialize, Clone)]
pub struct BackupInfo {
    pub id: String,
    pub label: String,
    pub created_at: u64,
    pub size_bytes: u64,
    pub file_count: usize,
    pub auto: bool,
}

const BACKUP_DIR: &str = "/var/lib/nix-evo/backups";
const MAX_AUTO_BACKUPS: usize = 20;
const MAX_MANUAL_BACKUPS: usize = 50;

// ─── GET /api/backups ────────────────────────────────────────────────────

pub async fn list_backups(
    _state: AppStateRef,
) -> Result<Json<ListBackupsResponse>, AppError> {
    let backups = scan_backup_dir().await?;
    Ok(Json(ListBackupsResponse { backups }))
}

#[derive(Serialize)]
pub struct ListBackupsResponse {
    pub backups: Vec<BackupInfo>,
}

// ─── POST /api/backup/create ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateBackupRequest {
    pub label: Option<String>,
}

#[derive(Serialize)]
pub struct CreateBackupResponse {
    pub backup: BackupInfo,
    pub summary: String,
}

pub async fn create_backup(
    _state: AppStateRef,
    Json(req): Json<CreateBackupRequest>,
) -> Result<Json<CreateBackupResponse>, AppError> {
    let label = req.label.unwrap_or_else(|| "手动备份".to_string());
    let backup = create_snapshot(&label, false).await?;
    let summary = format!("备份已创建: {} ({} 个文件, {} 字节)",
        backup.id, backup.file_count, backup.size_bytes);
    Ok(Json(CreateBackupResponse { backup, summary }))
}

// ─── POST /api/backup/restore ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RestoreRequest {
    pub backup_id: String,
    pub dry_run: Option<bool>,
}

#[derive(Serialize)]
pub struct RestoreResponse {
    pub success: bool,
    pub summary: String,
    pub files_restored: usize,
    pub dry_run_files: Option<Vec<String>>,
}

pub async fn restore_backup(
    state: AppStateRef,
    Json(req): Json<RestoreRequest>,
) -> Result<Json<RestoreResponse>, AppError> {
    let backup_path = format!("{}/{}", BACKUP_DIR, req.backup_id);

    if !tokio::fs::try_exists(&backup_path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            resource: format!("备份 {}", req.backup_id),
        });
    }

    let dry_run = req.dry_run.unwrap_or(false);

    if dry_run {
        let files = list_backup_files(&backup_path).await?;
        return Ok(Json(RestoreResponse {
            success: true,
            summary: format!("预览模式: {} 个文件将被恢复", files.len()),
            files_restored: 0,
            dry_run_files: Some(files),
        }));
    }

    // Safety backup before restore
    let safety_backup = create_snapshot("恢复前自动备份", true).await
        .map_err(|e| AppError::Internal {
            message: format!("无法创建安全备份: {e}"),
        })?;

    let output = run_cmd("rsync", &[
        "-av",
        "--delete",
        &format!("{}/", backup_path),
        &state.config.nixos_dir,
    ]).await?;

    let file_count = output.lines()
        .filter(|l| !l.is_empty() && !l.starts_with("sending") && !l.starts_with("sent"))
        .count();

    Ok(Json(RestoreResponse {
        success: true,
        summary: format!("已从备份 {} 恢复 {} 个文件\n安全备份: {}",
            req.backup_id, file_count, safety_backup.id),
        files_restored: file_count,
        dry_run_files: None,
    }))
}

// ─── POST /api/backup/rotate ─────────────────────────────────────────────

#[derive(Serialize)]
pub struct RotateResponse {
    pub removed: usize,
    pub kept: usize,
    pub summary: String,
}

pub async fn rotate_backups(
    _state: AppStateRef,
) -> Result<Json<RotateResponse>, AppError> {
    let backups = scan_backup_dir().await?;
    let (auto, manual): (Vec<_>, Vec<_>) = backups.iter().partition(|b| b.auto);
    let mut removed = 0;

    if auto.len() > MAX_AUTO_BACKUPS {
        for backup in auto.iter().take(auto.len() - MAX_AUTO_BACKUPS) {
            let _ = tokio::fs::remove_dir_all(format!("{}/{}", BACKUP_DIR, backup.id)).await;
            removed += 1;
        }
    }

    if manual.len() > MAX_MANUAL_BACKUPS {
        for backup in manual.iter().take(manual.len() - MAX_MANUAL_BACKUPS) {
            let _ = tokio::fs::remove_dir_all(format!("{}/{}", BACKUP_DIR, backup.id)).await;
            removed += 1;
        }
    }

    let remaining = scan_backup_dir().await?.len();
    Ok(Json(RotateResponse {
        removed,
        kept: remaining,
        summary: format!("清理完成: 删除 {} 个旧备份，保留 {} 个", removed, remaining),
    }))
}

// ─── Internal helpers ────────────────────────────────────────────────────

async fn create_snapshot(label: &str, auto: bool) -> Result<BackupInfo, AppError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let backup_id = format!("{}-{}", timestamp, if auto { "auto" } else { "manual" });
    let backup_path = format!("{}/{}", BACKUP_DIR, backup_id);

    tokio::fs::create_dir_all(&backup_path).await.map_err(|e| {
        AppError::IoError {
            path: backup_path.clone(),
            message: format!("无法创建备份目录: {e}"),
        }
    })?;

    run_cmd("cp", &["-a", "/etc/nixos/.", &backup_path]).await.map_err(|e| {
        AppError::CommandFailed {
            command: "cp".into(),
            message: format!("备份复制失败: {e}"),
        }
    })?;

    let label_path = format!("{}/.nix-evo-label", backup_path);
    let _ = tokio::fs::write(&label_path, label).await;

    let (size_bytes, file_count) = get_dir_stats(&backup_path).await;

    Ok(BackupInfo {
        id: backup_id,
        label: label.to_string(),
        created_at: timestamp,
        size_bytes,
        file_count,
        auto,
    })
}

async fn scan_backup_dir() -> Result<Vec<BackupInfo>, AppError> {
    let mut backups = Vec::new();
    let mut entries = tokio::fs::read_dir(BACKUP_DIR).await.map_err(|_| {
        AppError::NotFound { resource: format!("备份目录 {BACKUP_DIR}") }
    })?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        if !path.is_dir() { continue; }

        let (created_at, auto) = if let Some(ts_str) = name.split('-').next() {
            (ts_str.parse::<u64>().unwrap_or(0), name.ends_with("-auto"))
        } else {
            (0, false)
        };

        let label_path = format!("{}/.nix-evo-label", path.display());
        let label = tokio::fs::read_to_string(&label_path).await.unwrap_or_default().trim().to_string();

        let (size_bytes, file_count) = get_dir_stats(&path.display().to_string()).await;

        backups.push(BackupInfo {
            id: name,
            label: if label.is_empty() {
                if auto { "自动备份".into() } else { "手动备份".into() }
            } else { label },
            created_at,
            size_bytes,
            file_count,
            auto,
        });
    }

    backups.sort_by_key(|b| b.created_at);
    Ok(backups)
}

async fn get_dir_stats(path: &str) -> (u64, usize) {
    let output = run_cmd("du", &["-sb", path]).await.unwrap_or_default();
    let size = output.split_whitespace().next()
        .and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    let count_output = run_cmd("find", &[path, "-type", "f"]).await.unwrap_or_default();
    let count = count_output.lines().count();
    (size, count)
}

async fn list_backup_files(backup_path: &str) -> Result<Vec<String>, AppError> {
    let output = run_cmd("find", &[backup_path, "-type", "f"]).await?;
    Ok(output.lines()
        .filter_map(|l| l.strip_prefix(backup_path).map(|s| s.trim_start_matches('/').to_string()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_id_format() {
        let ts: u64 = 1712900000;
        let auto_id = format!("{}-{}", ts, "auto");
        let manual_id = format!("{}-{}", ts, "manual");
        assert!(auto_id.ends_with("-auto"));
        assert!(manual_id.ends_with("-manual"));
        let parsed_ts: u64 = auto_id.split('-').next().unwrap().parse().unwrap();
        assert_eq!(parsed_ts, ts);
    }

    #[test]
    fn test_backup_info_serialization() {
        let info = BackupInfo {
            id: "1712900000-auto".into(),
            label: "测试备份".into(),
            created_at: 1712900000,
            size_bytes: 102400,
            file_count: 42,
            auto: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("测试备份"));
        assert!(json.contains("\"auto\":true"));
    }
}
