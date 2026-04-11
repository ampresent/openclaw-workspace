use axum::Json;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub host: Option<String>,
    pub config: String,
}

#[derive(Serialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub dry_run_output: String,
    pub summary: ValidationSummary,
}

#[derive(Serialize)]
pub struct ValidationSummary {
    pub packages_added: Vec<String>,
    pub packages_removed: Vec<String>,
    pub services_restart: Vec<String>,
    pub services_stop: Vec<String>,
    pub risk_level: String,
    pub risk_reasons: Vec<String>,
}

pub async fn handle(
    State(state): AppStateRef,
    Json(req): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, String> {
    // Write config to temp file for validation
    let tmp_path = "/tmp/nix-evo-validate.nix";
    tokio::fs::write(tmp_path, &req.config)
        .await
        .map_err(|e| format!("failed to write temp config: {e}"))?;

    // Run nixos-rebuild dry-build
    let dry_output = run_cmd(
        "nixos-rebuild",
        &["dry-build", "--fast", "--flake", "false"],
    )
    .await;

    // Also try without flake
    let dry_output = match dry_output {
        Ok(o) => o,
        Err(_) => run_cmd("nixos-rebuild", &["dry-build", "--fast"])
            .await
            .unwrap_or_else(|e| format!("dry-build failed: {e}")),
    };

    let valid = !dry_output.contains("error:") && !dry_output.contains("trace:");

    let (packages_added, packages_removed) = parse_dry_build_packages(&dry_output);
    let services_restart = parse_restarted_services(&dry_output);
    let services_stop = parse_stopped_services(&dry_output);
    let (risk_level, risk_reasons) = assess_risk(
        &packages_added,
        &packages_removed,
        &services_restart,
        &services_stop,
        &dry_output,
    );

    Ok(Json(ValidateResponse {
        valid,
        dry_run_output: dry_output,
        summary: ValidationSummary {
            packages_added,
            packages_removed,
            services_restart,
            services_stop,
            risk_level,
            risk_reasons,
        },
    }))
}

fn parse_dry_build_packages(output: &str) -> (Vec<String>, Vec<String>) {
    let mut added = Vec::new();
    let mut removed = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("building") || line.starts_with("will be") {
            continue;
        }
        // Look for "these N derivations will be built"
        if line.contains("will be built") {
            // Extract package names from following lines
            continue;
        }
        // Look for removed packages
        if line.contains("will be removed") || line.contains("removing") {
            if let Some(pkg) = line.split(':').last() {
                removed.push(pkg.trim().to_string());
            }
        }
        // New store paths
        if line.contains("/nix/store/") && line.contains("→") {
            let parts: Vec<&str> = line.split("→").collect();
            if parts.len() == 2 {
                added.push(parts[1].trim().to_string());
            }
        }
    }

    (added, removed)
}

fn parse_restarted_services(output: &str) -> Vec<String> {
    let mut services = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.contains("restarting") || line.contains("reload") {
            if let Some(svc) = line.split_whitespace().find(|w| w.ends_with(".service")) {
                services.push(svc.to_string());
            }
        }
    }
    services
}

fn parse_stopped_services(output: &str) -> Vec<String> {
    let mut services = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.contains("stopping") {
            if let Some(svc) = line.split_whitespace().find(|w| w.ends_with(".service")) {
                services.push(svc.to_string());
            }
        }
    }
    services
}

fn assess_risk(
    added: &[String],
    removed: &[String],
    restart: &[String],
    stop: &[String],
    output: &str,
) -> (String, Vec<String>) {
    let mut reasons = Vec::new();
    let mut score = 0;

    if !removed.is_empty() {
        score += 3;
        reasons.push(format!("将删除 {} 个包", removed.len()));
    }

    if output.contains("firewall") || output.contains("iptables") || output.contains("nftables") {
        score += 3;
        reasons.push("涉及防火墙规则变更".into());
    }

    if output.contains("boot") || output.contains("grub") || output.contains("systemd-boot") {
        score += 3;
        reasons.push("涉及引导加载器变更".into());
    }

    if output.contains("disk") || output.contains("parted") || output.contains("fs") {
        score += 3;
        reasons.push("涉及磁盘/文件系统变更".into());
    }

    if output.contains("network") || output.contains("interfaces") {
        score += 2;
        reasons.push("涉及网络配置变更".into());
    }

    for svc in restart.iter().chain(stop.iter()) {
        if svc.contains("nginx") || svc.contains("sshd") || svc.contains("network") {
            score += 2;
            reasons.push(format!("将重启核心服务: {svc}"));
        }
    }

    if !restart.is_empty() || !stop.is_empty() {
        score += 1;
        reasons.push(format!("将重启 {} 个服务", restart.len() + stop.len()));
    }

    if !added.is_empty() {
        score += 1;
    }

    let level = if score >= 5 {
        "dangerous"
    } else if score >= 2 {
        "moderate"
    } else {
        "safe"
    };

    if reasons.is_empty() {
        reasons.push("仅添加配置项，无破坏性变更".into());
    }

    (level.to_string(), reasons)
}
