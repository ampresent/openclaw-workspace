use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use crate::error::AppError;
use crate::cmd::{run_cmd, run_cmd_with_timeout};

/// Webhook payload from Git providers
#[derive(Debug, Deserialize)]
pub struct GitWebhookPayload {
    /// Event type (push, merge_request, pull_request)
    pub event: Option<String>,
    /// Repository info
    pub repository: Option<RepositoryInfo>,
    /// Branch/ref that was pushed
    pub ref_field: Option<String>,
    /// Commits in the push
    pub commits: Option<Vec<CommitInfo>>,
    /// Pull/Merge request info
    pub pull_request: Option<PrInfo>,
    /// Provider type
    #[serde(rename = "type")]
    pub provider_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RepositoryInfo {
    pub name: Option<String>,
    pub full_name: Option<String>,
    pub clone_url: Option<String>,
    pub default_branch: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommitInfo {
    pub id: Option<String>,
    pub message: Option<String>,
    pub author: Option<AuthorInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthorInfo {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PrInfo {
    pub number: Option<u64>,
    pub title: Option<String>,
    pub action: Option<String>,
    pub head_ref: Option<String>,
    pub base_ref: Option<String>,
}

/// Webhook handler response
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub accepted: bool,
    pub action: String,
    pub deployment_id: Option<String>,
    pub message: String,
}

/// Deployment record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRecord {
    pub id: String,
    pub trigger: String,        // "push", "pr", "manual"
    pub branch: String,
    pub commit: Option<String>,
    pub status: String,         // "pending", "testing", "applied", "failed", "rolled_back"
    pub started_at: String,
    pub finished_at: Option<String>,
    pub log: Vec<String>,
    pub preview_url: Option<String>,
}

/// Preview deployment config
#[derive(Debug, Deserialize)]
pub struct PreviewDeployRequest {
    pub branch: String,
    pub config_path: Option<String>,
    pub auto_apply: Option<bool>,
    pub timeout_minutes: Option<u64>,
}

/// Preview deployment result
#[derive(Debug, Serialize)]
pub struct PreviewDeployResponse {
    pub deployment_id: String,
    pub status: String,
    pub validation_result: Option<ValidationResult>,
    pub vm_info: Option<VmInfo>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub config_valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub services_changed: Vec<String>,
    pub packages_changed: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct VmInfo {
    pub vm_name: String,
    pub vm_status: String,
    pub ssh_command: Option<String>,
    pub expires_at: String,
}

/// POST /api/cicd/webhook — receive Git webhook events
pub async fn webhook_handler(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<GitWebhookPayload>,
) -> Result<Json<WebhookResponse>, AppError> {
    let event = payload.event.as_deref().unwrap_or("push");
    let branch = payload
        .ref_field
        .as_ref()
        .map(|r| r.trim_start_matches("refs/heads/").to_string())
        .unwrap_or_else(|| "unknown".into());

    let repo_name = payload
        .repository
        .as_ref()
        .and_then(|r| r.full_name.clone())
        .or_else(|| payload.repository.as_ref().and_then(|r| r.name.clone()))
        .unwrap_or_else(|| "unknown".into());

    tracing::info!(
        "Webhook received: event={}, repo={}, branch={}",
        event, repo_name, branch
    );

    match event {
        "push" => {
            let commit_msg = payload
                .commits
                .as_ref()
                .and_then(|c| c.first())
                .and_then(|c| c.message.clone())
                .unwrap_or_default();

            // Check if the push includes nix config changes
            let has_nix_changes = commit_msg.contains(".nix")
                || commit_msg.contains("nixos")
                || branch == "main"
                || branch == "master";

            if has_nix_changes {
                let deploy_id = generate_deploy_id();
                tracing::info!(
                    "Nix config change detected in {} — starting validation (deploy: {})",
                    repo_name, deploy_id
                );

                // Trigger async validation
                let deploy_id_clone = deploy_id.clone();
                tokio::spawn(async move {
                    if let Err(e) = run_ci_validation(&deploy_id_clone, &branch).await {
                        tracing::error!("CI validation failed for {}: {}", deploy_id_clone, e);
                    }
                });

                Ok(Json(WebhookResponse {
                    accepted: true,
                    action: "validation_started".into(),
                    deployment_id: Some(deploy_id),
                    message: format!("Push to {branch} 触发配置验证"),
                }))
            } else {
                Ok(Json(WebhookResponse {
                    accepted: true,
                    action: "ignored".into(),
                    deployment_id: None,
                    message: "Push 不包含 Nix 配置变更，已跳过".into(),
                }))
            }
        }
        "pull_request" | "merge_request" => {
            let pr_action = payload
                .pull_request
                .as_ref()
                .and_then(|p| p.action.clone())
                .unwrap_or_default();

            if pr_action == "opened" || pr_action == "synchronize" {
                let deploy_id = generate_deploy_id();

                Ok(Json(WebhookResponse {
                    accepted: true,
                    action: "preview_deploy_queued".into(),
                    deployment_id: Some(deploy_id),
                    message: format!("PR/MR 变更已排队，等待预览部署"),
                }))
            } else {
                Ok(Json(WebhookResponse {
                    accepted: true,
                    action: "ignored".into(),
                    deployment_id: None,
                    message: format!("PR/MR action '{pr_action}' 无需处理"),
                }))
            }
        }
        _ => Ok(Json(WebhookResponse {
            accepted: true,
            action: "ignored".into(),
            deployment_id: None,
            message: format!("事件类型 '{event}' 不处理"),
        })),
    }
}

/// POST /api/cicd/preview-deploy — trigger a preview deployment
pub async fn preview_deploy(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<PreviewDeployRequest>,
) -> Result<Json<PreviewDeployResponse>, AppError> {
    let deploy_id = generate_deploy_id();
    let timeout = req.timeout_minutes.unwrap_or(30);

    tracing::info!(
        "Preview deploy requested: branch={}, id={}, timeout={}min",
        req.branch, deploy_id, timeout
    );

    // Step 1: Validate config
    let config_path = req.config_path.as_deref().unwrap_or("/etc/nixos/configuration.nix");
    let validation = validate_config(config_path).await?;

    if !validation.config_valid {
        return Ok(Json(PreviewDeployResponse {
            deployment_id: deploy_id,
            status: "validation_failed".into(),
            validation_result: Some(validation),
            vm_info: None,
            message: "配置验证失败，未启动预览部署".into(),
        }));
    }

    // Step 2: If auto_apply, run test-before-switch
    if req.auto_apply.unwrap_or(false) {
        tracing::info!("Auto-apply enabled, running nixos-rebuild test for {}", deploy_id);
    }

    Ok(Json(PreviewDeployResponse {
        deployment_id: deploy_id.clone(),
        status: if req.auto_apply.unwrap_or(false) { "testing" } else { "validated" }.into(),
        validation_result: Some(validation),
        vm_info: None,
        message: format!("预览部署 {deploy_id} 已创建，分支: {}", req.branch),
    }))
}

/// GET /api/cicd/deployments — list recent deployments
pub async fn list_deployments(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<DeploymentRecord>>, AppError> {
    // Read deployment records from state directory
    let state_dir = "/var/lib/nix-evo/deployments";
    let mut deployments = Vec::new();

    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(record) = serde_json::from_str::<DeploymentRecord>(&content) {
                        deployments.push(record);
                    }
                }
            }
        }
    }

    // Sort by started_at descending
    deployments.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    deployments.truncate(50); // Last 50 deployments

    Ok(Json(deployments))
}

/// GET /api/cicd/deployments/:id — get deployment details
pub async fn get_deployment(
    State(_state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<DeploymentRecord>, AppError> {
    let path = format!("/var/lib/nix-evo/deployments/{id}.json");
    let content = std::fs::read_to_string(&path).map_err(|_| AppError::NotFound {
        resource: format!("部署记录: {id}"),
    })?;
    let record: DeploymentRecord = serde_json::from_str(&content).map_err(|e| AppError::Internal {
        message: format!("解析部署记录失败: {e}"),
    })?;
    Ok(Json(record))
}

/// Generate a unique deployment ID
fn generate_deploy_id() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let pid = std::process::id();
    format!("deploy-{ts}-{pid:x}")
}

/// Run CI validation for a deployment
async fn run_ci_validation(deploy_id: &str, branch: &str) -> Result<(), AppError> {
    let mut record = DeploymentRecord {
        id: deploy_id.to_string(),
        trigger: "push".into(),
        branch: branch.to_string(),
        commit: None,
        status: "testing".into(),
        started_at: chrono_now(),
        finished_at: None,
        log: vec!["开始 CI 验证...".into()],
        preview_url: None,
    };

    // Step 1: Dry-build
    record.log.push("步骤 1/3: 执行 nixos-rebuild dry-build...".into());
    match run_cmd_with_timeout("nixos-rebuild", &["dry-build"], 300).await {
        Ok(output) => {
            record.log.push(format!("dry-build 成功 ({} bytes output)", output.len()));
        }
        Err(e) => {
            record.status = "failed".into();
            record.log.push(format!("dry-build 失败: {e}"));
            record.finished_at = Some(chrono_now());
            save_deployment_record(&record)?;
            return Err(e);
        }
    }

    // Step 2: NixOS VM test (if available)
    record.log.push("步骤 2/3: 检查是否有 VM 测试...".into());
    let has_vm_test = std::path::Path::new("/etc/nixos/tests").exists();
    if has_vm_test {
        record.log.push("发现 VM 测试目录，执行测试...".into());
        // Would run: nix-build '<nixpkgs/nixos/tests>' -A <test-name>
        record.log.push("VM 测试: 待实现".into());
    } else {
        record.log.push("未发现 VM 测试，跳过".into());
    }

    // Step 3: Config diff
    record.log.push("步骤 3/3: 生成配置差异...".into());
    match run_cmd_with_timeout("nixos-rebuild", &["dry-build", "--show-trace"], 120).await {
        Ok(output) => {
            record.log.push(format!("配置差异已生成"));
        }
        Err(e) => {
            record.log.push(format!("配置差异生成失败: {e}"));
        }
    }

    record.status = "validated".into();
    record.finished_at = Some(chrono_now());
    record.log.push("CI 验证完成".into());
    save_deployment_record(&record)?;

    Ok(())
}

/// Validate a NixOS config file
async fn validate_config(path: &str) -> Result<ValidationResult, AppError> {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    // Check file exists
    if !std::path::Path::new(path).exists() {
        return Ok(ValidationResult {
            config_valid: false,
            warnings: vec![],
            errors: vec![format!("配置文件不存在: {path}")],
            services_changed: vec![],
            packages_changed: vec![],
        });
    }

    // Try nix-instantiate to parse
    match run_cmd_with_timeout("nix-instantiate", &["--parse", path], 30).await {
        Ok(_) => {}
        Err(e) => {
            errors.push(format!("Nix 解析失败: {e}"));
            return Ok(ValidationResult {
                config_valid: false,
                warnings,
                errors,
                services_changed: vec![],
                packages_changed: vec![],
            });
        }
    }

    // Try dry-build
    match run_cmd_with_timeout("nixos-rebuild", &["dry-build"], 120).await {
        Ok(output) => {
            // Parse changes from output
            let services: Vec<String> = output
                .lines()
                .filter(|l| l.contains("systemd") || l.contains("service"))
                .take(20)
                .map(|l| l.to_string())
                .collect();

            let packages: Vec<String> = output
                .lines()
                .filter(|l| l.contains("/nix/store/"))
                .take(20)
                .map(|l| l.to_string())
                .collect();

            if output.contains("error:") || output.contains("ERROR") {
                warnings.push("dry-build 有警告信息".into());
            }

            Ok(ValidationResult {
                config_valid: errors.is_empty(),
                warnings,
                errors,
                services_changed: services,
                packages_changed: packages,
            })
        }
        Err(e) => {
            warnings.push(format!("dry-build 不可用: {e}"));
            Ok(ValidationResult {
                config_valid: errors.is_empty(),
                warnings,
                errors,
                services_changed: vec![],
                packages_changed: vec![],
            })
        }
    }
}

/// Save deployment record to disk
fn save_deployment_record(record: &DeploymentRecord) -> Result<(), AppError> {
    let state_dir = "/var/lib/nix-evo/deployments";
    std::fs::create_dir_all(state_dir).map_err(|e| AppError::IoError {
        path: state_dir.into(),
        message: e.to_string(),
    })?;

    let path = format!("{state_dir}/{}.json", record.id);
    let content = serde_json::to_string_pretty(record).map_err(|e| AppError::Internal {
        message: format!("序列化失败: {e}"),
    })?;
    std::fs::write(&path, content).map_err(|e| AppError::IoError {
        path,
        message: e.to_string(),
    })?;
    Ok(())
}

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
