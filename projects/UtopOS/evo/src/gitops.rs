/// GitOps Bridge — Watch a git repo for NixOS config changes
///
/// Receives webhooks, auto-pulls and validates on push,
/// tracks current commit, pending changes, and deploy status.

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use tokio::sync::RwLock;

use crate::cmd;
use crate::error::AppError;

// ─── Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsConfig {
    pub repo_url: String,
    pub branch: String,
    pub config_path: String,        // path within repo to configuration.nix
    pub auto_deploy: bool,
    pub deploy_command: String,
    pub webhook_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitOpsStatus {
    pub configured: bool,
    pub repo_url: Option<String>,
    pub branch: Option<String>,
    pub current_commit: Option<CommitInfo>,
    pub pending_commits: Vec<CommitInfo>,
    pub last_deploy: Option<DeployInfo>,
    pub last_webhook: Option<WebhookInfo>,
    pub deploy_state: DeployState,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub message: String,
    pub timestamp: String,
    pub files_changed: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployInfo {
    pub commit_hash: String,
    pub timestamp: String,
    pub success: bool,
    pub duration_secs: f64,
    pub output: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebhookInfo {
    pub event: String,
    pub timestamp: String,
    pub sender: String,
    pub branch: String,
    pub commits: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeployState {
    Idle,
    Pulling,
    Validating,
    Deploying,
    Success,
    Failed(String),
}

// ─── Global State ─────────────────────────────────────────────────────────

struct GitOpsState {
    config: RwLock<Option<GitOpsConfig>>,
    status: RwLock<GitOpsStatus>,
    webhook_count: AtomicU64,
    deploy_count: AtomicU64,
}

fn state() -> &'static GitOpsState {
    static STATE: OnceLock<GitOpsState> = OnceLock::new();
    STATE.get_or_init(|| GitOpsState {
        config: RwLock::new(None),
        status: RwLock::new(GitOpsStatus {
            configured: false,
            repo_url: None,
            branch: None,
            current_commit: None,
            pending_commits: Vec::new(),
            last_deploy: None,
            last_webhook: None,
            deploy_state: DeployState::Idle,
        }),
        webhook_count: AtomicU64::new(0),
        deploy_count: AtomicU64::new(0),
    })
}

// ─── Git Operations ───────────────────────────────────────────────────────

async fn get_current_commit(repo_path: &str) -> Option<CommitInfo> {
    let format = "%H%n%h%n%an%n%s%n%aI";
    let output = cmd::run_cmd("bash", &["-c", &format!("cd {repo_path} && git log -1 --format='{format}' 2>/dev/null")]).await.ok()?;
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() >= 5 {
        Some(CommitInfo {
            hash: lines[0].into(),
            short_hash: lines[1].into(),
            author: lines[2].into(),
            message: lines[3].into(),
            timestamp: lines[4].into(),
            files_changed: Vec::new(),
        })
    } else { None }
}

async fn get_pending_commits(repo_path: &str, branch: &str) -> Vec<CommitInfo> {
    let output = cmd::run_cmd("bash", &["-c",
        &format!("cd {repo_path} && git fetch origin {branch} 2>/dev/null && git log HEAD..origin/{branch} --oneline --format='%H%n%h%n%an%n%s%n%aI' 2>/dev/null")
    ]).await.unwrap_or_default();

    let mut commits = Vec::new();
    let lines: Vec<&str> = output.lines().collect();
    let mut i = 0;
    while i + 4 < lines.len() {
        commits.push(CommitInfo {
            hash: lines[i].into(),
            short_hash: lines[i+1].into(),
            author: lines[i+2].into(),
            message: lines[i+3].into(),
            timestamp: lines[i+4].into(),
            files_changed: Vec::new(),
        });
        i += 5;
    }
    commits
}

async fn pull_and_validate(config: &GitOpsConfig) -> Result<DeployInfo, AppError> {
    let start = std::time::Instant::now();
    let repo_path = "/tmp/nix-evo-gitops";

    // Clone or pull
    let pull_cmd = format!(
        "if [ -d {repo_path}/.git ]; then cd {repo_path} && git pull origin {branch}; else git clone --branch {branch} {url} {repo_path}; fi",
        repo_path = repo_path,
        branch = config.branch,
        url = config.repo_url,
    );
    let pull_output = cmd::run_cmd("bash", &["-c", &pull_cmd]).await
        .map_err(|e| AppError::CommandFailed { command: "git pull".into(), message: e.to_string() })?;

    let commit = get_current_commit(repo_path).await;

    // Validate the config
    let config_file = format!("{}/{}", repo_path, config.config_path);
    let validate_cmd = format!("nix-instantiate --parse {config_file} 2>&1");
    let validate_output = cmd::run_cmd("bash", &["-c", &validate_cmd]).await;

    let success = validate_output.is_ok();
    let duration = start.elapsed().as_secs_f64();

    let output = if success {
        format!("✅ 验证通过\nPull: {pull_output}\nCommit: {}", commit.as_ref().map(|c| c.short_hash.as_str()).unwrap_or("unknown"))
    } else {
        format!("❌ 验证失败\n{}", validate_output.unwrap_err().to_string())
    };

    // Auto-deploy if configured
    if success && config.auto_deploy {
        let deploy_output = cmd::run_cmd("bash", &["-c", &config.deploy_command]).await
            .unwrap_or_else(|e| format!("Deploy failed: {e}"));

        return Ok(DeployInfo {
            commit_hash: commit.map(|c| c.hash).unwrap_or_default(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            success: deploy_output.contains("success") || deploy_output.contains("done"),
            duration_secs: start.elapsed().as_secs_f64(),
            output: format!("{output}\nDeploy: {deploy_output}"),
        });
    }

    Ok(DeployInfo {
        commit_hash: commit.map(|c| c.hash).unwrap_or_default(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        success,
        duration_secs: duration,
        output,
    })
}

// ─── Webhook Processing ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub ref_field: Option<String>,
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    pub commits: Option<Vec<WebhookCommit>>,
    pub sender: Option<WebhookSender>,
    pub repository: Option<WebhookRepo>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookCommit {
    pub id: Option<String>,
    pub message: Option<String>,
    pub author: Option<WebhookAuthor>,
    pub modified: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookAuthor {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookSender {
    pub login: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookRepo {
    pub clone_url: Option<String>,
    pub default_branch: Option<String>,
}

pub async fn handle_webhook(Json(payload): Json<WebhookPayload>) -> Result<impl IntoResponse, AppError> {
    state().webhook_count.fetch_add(1, Ordering::Relaxed);

    let git_ref = payload.git_ref.or(payload.ref_field).unwrap_or_default();
    let branch = git_ref.strip_prefix("refs/heads/").unwrap_or(&git_ref);

    let commits = payload.commits.unwrap_or_default();
    let sender = payload.sender.and_then(|s| s.login).unwrap_or_default();

    // Update webhook info
    {
        let mut status = state().status.write().await;
        status.last_webhook = Some(WebhookInfo {
            event: "push".into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            sender: sender.clone(),
            branch: branch.to_string(),
            commits: commits.len(),
        });
    }

    // Check if we should auto-deploy
    let config = state().config.read().await;
    if let Some(ref cfg) = *config {
        if cfg.branch == branch {
            tracing::info!("GitOps: Received webhook for tracked branch {branch}, auto-deploying");

            {
                let mut status = state().status.write().await;
                status.deploy_state = DeployState::Pulling;
            }

            match pull_and_validate(cfg).await {
                Ok(deploy_info) => {
                    state().deploy_count.fetch_add(1, Ordering::Relaxed);
                    let mut status = state().status.write().await;
                    status.last_deploy = Some(deploy_info);
                    status.deploy_state = DeployState::Success;
                }
                Err(e) => {
                    let mut status = state().status.write().await;
                    status.deploy_state = DeployState::Failed(e.to_string());
                }
            }
        }
    }

    Ok(Json(serde_json::json!({
        "received": true,
        "branch": branch,
        "commits": commits.len(),
        "sender": sender,
    })))
}

// ─── API Handlers ─────────────────────────────────────────────────────────

pub async fn handle_status() -> impl IntoResponse {
    let status = state().status.read().await;
    Json(serde_json::to_value(&*status).unwrap())
}

#[derive(Debug, Deserialize)]
pub struct ConfigureBody {
    pub repo_url: String,
    pub branch: Option<String>,
    pub config_path: Option<String>,
    pub auto_deploy: Option<bool>,
    pub deploy_command: Option<String>,
    pub webhook_secret: Option<String>,
}

pub async fn handle_configure(Json(body): Json<ConfigureBody>) -> Result<impl IntoResponse, AppError> {
    let config = GitOpsConfig {
        repo_url: body.repo_url,
        branch: body.branch.unwrap_or_else(|| "main".into()),
        config_path: body.config_path.unwrap_or_else(|| "configuration.nix".into()),
        auto_deploy: body.auto_deploy.unwrap_or(false),
        deploy_command: body.deploy_command.unwrap_or_else(|| "nixos-rebuild switch".into()),
        webhook_secret: body.webhook_secret,
    };

    {
        let mut cfg = state().config.write().await;
        *cfg = Some(config.clone());
    }

    {
        let mut status = state().status.write().await;
        status.configured = true;
        status.repo_url = Some(config.repo_url.clone());
        status.branch = Some(config.branch.clone());
    }

    Ok(Json(serde_json::json!({
        "configured": true,
        "repo_url": config.repo_url,
        "branch": config.branch,
    })))
}

pub async fn handle_deploy() -> Result<impl IntoResponse, AppError> {
    let config = state().config.read().await;
    let cfg = config.as_ref().ok_or_else(|| AppError::Validation {
        field: "gitops".into(),
        message: "GitOps 未配置。请先 POST /api/gitops/configure".into(),
    })?;

    {
        let mut status = state().status.write().await;
        status.deploy_state = DeployState::Deploying;
    }

    let result = pull_and_validate(cfg).await;
    state().deploy_count.fetch_add(1, Ordering::Relaxed);

    match result {
        Ok(deploy_info) => {
            let mut status = state().status.write().await;
            status.deploy_state = if deploy_info.success { DeployState::Success } else { DeployState::Failed("验证失败".into()) };
            status.last_deploy = Some(deploy_info.clone());
            Ok(Json(serde_json::to_value(&deploy_info).unwrap()))
        }
        Err(e) => {
            let mut status = state().status.write().await;
            status.deploy_state = DeployState::Failed(e.to_string());
            Err(e)
        }
    }
}
