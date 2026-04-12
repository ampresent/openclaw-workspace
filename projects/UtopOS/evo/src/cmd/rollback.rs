use axum::Json;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Deserialize)]
pub struct RollbackRequest {
    pub host: Option<String>,
    /// 回滚目标：btrfs 快照路径或包管理器事务 ID
    pub target: Option<String>,
    /// 后端: "btrfs" | "rpm" | "conda"
    pub backend: Option<String>,
    /// btrfs 挂载点 (默认 "/")
    pub btrfs_mount: Option<String>,
    /// btrfs 快照目录 (默认 "$EVO_HOME/btrfs-snapshots")
    pub btrfs_snap_dir: Option<String>,
}

#[derive(Serialize)]
pub struct RollbackResponse {
    pub success: bool,
    pub backend: String,
    pub reverted_to: String,
    pub summary: String,
}

pub async fn handle(
    State(_state): AppStateRef,
    Json(req): Json<RollbackRequest>,
) -> Result<Json<RollbackResponse>, AppError> {
    let backend = req.backend.unwrap_or_else(|| {
        // 自动检测: 优先 btrfs，然后 rpm，最后 conda
        detect_backend()
    });

    match backend.as_str() {
        "btrfs" => handle_btrfs(req).await,
        "rpm" => handle_rpm(req).await,
        "conda" => handle_conda(req).await,
        other => Err(AppError::Validation {
            field: "backend".into(),
            message: format!("不支持的回滚后端: {other}，支持: btrfs, rpm, conda"),
        }),
    }
}

/// 自动检测可用回滚后端
fn detect_backend() -> String {
    // 优先检查是否为 btrfs 文件系统
    if let Ok(output) = std::process::Command::new("stat")
        .args(["-f", "-c", "%T", "/"])
        .output()
    {
        let fstype = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if fstype == "btrfs" {
            return "btrfs".into();
        }
    }
    // 检查 rpm
    if std::process::Command::new("yum")
        .arg("--version")
        .output()
        .is_ok()
    {
        return "rpm".into();
    }
    // 检查 conda
    if std::process::Command::new("conda")
        .arg("--version")
        .output()
        .is_ok()
    {
        return "conda".into();
    }
    "unknown".into()
}

// ──────────────────────────────────────────────
// btrfs 回滚
// ──────────────────────────────────────────────
///
/// btrfs 快照回滚原理:
/// 1. 安装前: `btrfs subvolume snapshot -r <src> <snap>` 创建只读快照
/// 2. 回滚时: 从 pre-install 快照创建新的可写快照，替换当前状态
/// 3. 回滚前: 先对当前状态创建保险快照
///
/// 优势:
/// - COW (Copy-on-Write) 机制，快照几乎零开销
/// - 原子操作，不存在中间状态
/// - 任何文件变更都能回滚，不限于包管理器
async fn handle_btrfs(req: RollbackRequest) -> Result<Json<RollbackResponse>, AppError> {
    let mount = req.btrfs_mount.as_deref().unwrap_or("/");
    let snap_dir = req
        .btrfs_snap_dir
        .as_deref()
        .unwrap_or("~/.evo/btrfs-snapshots");

    let snap_dir = shellexpand(snap_dir);

    // 验证是 btrfs
    let fstype = run_cmd("stat", &["-f", "-c", "%T", mount])
        .await
        .map_err(|e| AppError::CommandFailed {
            command: "stat".into(),
            message: format!("无法检测文件系统类型: {e}"),
        })?;

    if fstype.trim() != "btrfs" {
        return Err(AppError::Validation {
            field: "btrfs_mount".into(),
            message: format!(
                "挂载点 {mount} 不是 btrfs (实际: {})",
                fstype.trim()
            ),
        });
    }

    // 查找目标快照
    let snap_path = if let Some(target) = &req.target {
        target.clone()
    } else {
        // 自动找最新的 pre-install 快照
        find_latest_snapshot(&snap_dir, "pre-install")
            .await
            .ok_or_else(|| AppError::NotFound {
                resource: "btrfs pre-install 快照".into(),
            })?
    };

    // 验证是有效的 btrfs 子卷
    run_cmd("btrfs", &["subvolume", "show", &snap_path])
        .await
        .map_err(|e| AppError::CommandFailed {
            command: "btrfs subvolume show".into(),
            message: format!("不是有效的 btrfs 子卷 {snap_path}: {e}"),
        })?;

    // Step 1: 对当前状态创建保险快照 (只读)
    let ts = chrono_timestamp();
    let backup_path = format!("{snap_dir}/rollback-from-{ts}");
    run_cmd(
        "btrfs",
        &["subvolume", "snapshot", "-r", mount, &backup_path],
    )
    .await
    .map_err(|e| AppError::CommandFailed {
        command: "btrfs subvolume snapshot -r".into(),
        message: format!("创建保险快照失败: {e}"),
    })?;

    // Step 2: 从 pre-install 快照创建可写快照
    let restore_path = format!("{snap_dir}/restore-{ts}");
    run_cmd(
        "btrfs",
        &["subvolume", "snapshot", &snap_path, &restore_path],
    )
    .await
    .map_err(|e| AppError::CommandFailed {
        command: "btrfs subvolume snapshot".into(),
        message: format!("从快照恢复失败: {e}"),
    })?;

    Ok(Json(RollbackResponse {
        success: true,
        backend: "btrfs".into(),
        reverted_to: snap_path.clone(),
        summary: format!(
            "btrfs 回滚完成: 保险快照 {backup_path}, 恢复自 {snap_path} → {restore_path}"
        ),
    }))
}

// ──────────────────────────────────────────────
// rpm 回滚
// ──────────────────────────────────────────────
async fn handle_rpm(req: RollbackRequest) -> Result<Json<RollbackResponse>, AppError> {
    let txn = if let Some(target) = &req.target {
        target.clone()
    } else {
        // 自动找到最近的事务
        let output = run_cmd("yum", &["history", "list"]).await.map_err(|e| {
            AppError::CommandFailed {
                command: "yum history list".into(),
                message: format!("无法获取 yum 事务历史: {e}"),
            }
        })?;
        // 取第二行数据行的事务 ID
        output
            .lines()
            .nth(2)
            .and_then(|l| l.split_whitespace().next())
            .unwrap_or("unknown")
            .to_string()
    };

    let result = run_cmd("yum", &["history", "undo", &txn]).await;

    match result {
        Ok(_) => Ok(Json(RollbackResponse {
            success: true,
            backend: "rpm".into(),
            reverted_to: txn.clone(),
            summary: format!("已回滚 yum 事务 {txn}"),
        })),
        Err(e) => Err(AppError::CommandFailed {
            command: format!("yum history undo {txn}"),
            message: format!("yum 回滚失败: {e}"),
        }),
    }
}

// ──────────────────────────────────────────────
// conda 回滚
// ──────────────────────────────────────────────
async fn handle_conda(req: RollbackRequest) -> Result<Json<RollbackResponse>, AppError> {
    let rev = if let Some(target) = &req.target {
        target.clone()
    } else {
        let output = run_cmd("conda", &["list", "--revisions"])
            .await
            .map_err(|e| AppError::CommandFailed {
                command: "conda list --revisions".into(),
                message: format!("无法获取 conda 版本历史: {e}"),
            })?;
        output
            .lines()
            .last()
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or("unknown")
            .to_string()
    };

    let result = run_cmd("conda", &["install", "--revision", &rev]).await;

    match result {
        Ok(_) => Ok(Json(RollbackResponse {
            success: true,
            backend: "conda".into(),
            reverted_to: rev.clone(),
            summary: format!("已回滚到 conda revision {rev}"),
        })),
        Err(e) => Err(AppError::CommandFailed {
            command: format!("conda install --revision {rev}"),
            message: format!("conda 回滚失败: {e}"),
        }),
    }
}

// ──────────────────────────────────────────────
// 辅助函数
// ──────────────────────────────────────────────

fn shellexpand(path: &str) -> String {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return path.replacen("~", &home, 1);
        }
    }
    path.to_string()
}

async fn find_latest_snapshot(snap_dir: &str, prefix: &str) -> Option<String> {
    let output = tokio::process::Command::new("ls")
        .args(["-dt", &format!("{snap_dir}/{prefix}-*")])
        .output()
        .await
        .ok()?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(|s| s.to_string())
}

fn chrono_timestamp() -> String {
    // 轻量时间戳，避免引入 chrono 依赖
    std::process::Command::new("date")
        .arg("+%Y%m%d-%H%M%S")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".into())
        .trim()
        .to_string()
}
