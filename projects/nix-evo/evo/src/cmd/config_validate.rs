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
) -> Result<Json<ValidateResponse>, AppError> {
    // Validate: config must not be empty
    if req.config.trim().is_empty() {
        return Err(AppError::Validation {
            field: "config".into(),
            message: "NixOS 配置内容不能为空".into(),
        });
    }

    // Write config to temp file for validation
    let tmp_path = "/tmp/nix-evo-validate.nix";
    tokio::fs::write(tmp_path, &req.config).await.map_err(|e| {
        AppError::IoError {
            path: tmp_path.into(),
            message: format!("无法写入临时配置文件: {e}"),
        }
    })?;

    // Run nixos-rebuild dry-build
    // Try multiple invocation patterns
    let dry_output = try_dry_build(&state.config.nixos_dir).await;

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

/// Try multiple dry-build invocation strategies
async fn try_dry_build(nixos_dir: &str) -> String {
    // Strategy 1: flake-based (modern NixOS)
    if std::path::Path::new(&format!("{}/flake.nix", nixos_dir)).exists() {
        if let Ok(o) = run_cmd(
            "nixos-rebuild",
            &["dry-build", "--fast", "--flake", &format!(".#{}", infer_hostname())],
        )
        .await
        {
            return o;
        }
    }

    // Strategy 2: try without flake
    if let Ok(o) = run_cmd("nixos-rebuild", &["dry-build", "--fast", "--flake", "false"]).await {
        return o;
    }

    // Strategy 3: basic invocation
    if let Ok(o) = run_cmd("nixos-rebuild", &["dry-build", "--fast"]).await {
        return o;
    }

    // Strategy 4: with impure flag (for some setups)
    if let Ok(o) = run_cmd("nixos-rebuild", &["dry-build", "--fast", "--impure"]).await {
        return o;
    }

    "所有 dry-build 策略均失败，请检查 NixOS 配置".to_string()
}

fn infer_hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "nixos".to_string())
}

/// Parse packages from nixos-rebuild dry-build output.
///
/// nixos-rebuild output has several formats:
/// 1. "will be built: /nix/store/hash-pkg-version.drv"
/// 2. "these derivations will be built:" followed by store paths
/// 3. "will be fetched:" for cached packages
/// 4. "/nix/store/... → /nix/store/..." (symlink changes)
/// 5. "building 'x.drv'..."
fn parse_dry_build_packages(output: &str) -> (Vec<String>, Vec<String>) {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut in_build_list = false;
    let mut in_fetch_list = false;

    for line in output.lines() {
        let trimmed = line.trim();

        // Detect section starts
        if trimmed.contains("will be built:") || trimmed.contains("will be built:") {
            in_build_list = true;
            in_fetch_list = false;
            continue;
        }
        if trimmed.contains("will be fetched:") || trimmed.contains("will be fetched:") {
            in_fetch_list = true;
            in_build_list = false;
            continue;
        }
        if trimmed.contains("these derivations will be built:") {
            in_build_list = true;
            in_fetch_list = false;
            continue;
        }
        if trimmed.contains("these paths will be fetched") {
            in_fetch_list = true;
            in_build_list = false;
            continue;
        }

        // Blank line or new section ends the list
        if trimmed.is_empty() || trimmed.starts_with("building") || trimmed.contains("will be") {
            if trimmed.contains("will be") && !trimmed.contains("built:") && !trimmed.contains("fetched:") {
                in_build_list = false;
                in_fetch_list = false;
            }
        }

        // Parse store paths
        if in_build_list || in_fetch_list {
            if trimmed.contains("/nix/store/") {
                if let Some(pkg) = extract_pkg_name(trimmed) {
                    if !added.contains(&pkg) {
                        added.push(pkg);
                    }
                }
            }
            continue;
        }

        // Look for removed packages (less common in dry-build)
        if trimmed.contains("removing") || trimmed.contains("will be removed") {
            if let Some(pkg) = extract_pkg_name(trimmed) {
                removed.push(pkg);
            }
        }

        // Arrow notation: old → new
        if trimmed.contains("→") && trimmed.contains("/nix/store/") {
            let parts: Vec<&str> = trimmed.split('→').collect();
            if parts.len() == 2 {
                if let Some(pkg) = extract_pkg_name(parts[1].trim()) {
                    if !added.contains(&pkg) {
                        added.push(pkg);
                    }
                }
            }
        }
    }

    (added, removed)
}

/// Extract package name from a store path or line containing one.
/// /nix/store/hash-pkg-version -> "pkg-version" (short form)
fn extract_pkg_name(line: &str) -> Option<String> {
    // Find /nix/store/ in the line
    let start = line.find("/nix/store/")?;
    let rest = &line[start + 11..]; // skip "/nix/store/"
    // Take until next space, slash, or end marker
    let end = rest.find(|c: char| c == ' ' || c == '\t' || c == '/' || c == ')').unwrap_or(rest.len());
    let store_entry = &rest[..end];
    // Strip hash prefix: "hash-pkg-version" -> "pkg-version"
    if let Some(dash_pos) = store_entry.find('-') {
        let name = &store_entry[dash_pos + 1..];
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    Some(store_entry.to_string())
}

fn parse_restarted_services(output: &str) -> Vec<String> {
    let mut services = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.contains("restarting") || trimmed.contains("reload") || trimmed.contains("restart of") {
            // Look for .service units
            for word in trimmed.split_whitespace() {
                if word.ends_with(".service") {
                    let svc = word.trim_end_matches('.');
                    if !services.contains(&svc.to_string()) {
                        services.push(svc.to_string());
                    }
                }
            }
        }
    }
    services
}

fn parse_stopped_services(output: &str) -> Vec<String> {
    let mut services = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.contains("stopping") || trimmed.contains("stop of") {
            for word in trimmed.split_whitespace() {
                if word.ends_with(".service") {
                    let svc = word.trim_end_matches('.');
                    if !services.contains(&svc.to_string()) {
                        services.push(svc.to_string());
                    }
                }
            }
        }
    }
    services
}

/// Risk assessment heuristic with detailed scoring.
///
/// Scoring:
/// - Package removal: +3
/// - Firewall/iptables/nftables change: +3
/// - Boot loader change: +3
/// - Disk/filesystem change: +3
/// - Network config change: +2
/// - Core service restart (nginx/sshd/network): +2 each
/// - Any service restart/stop: +1
/// - Any package addition: +1
///
/// Levels:
/// - safe: score 0-1
/// - moderate: score 2-4
/// - dangerous: score 5+
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

    let output_lower = output.to_lowercase();

    if output_lower.contains("firewall") || output_lower.contains("iptables") || output_lower.contains("nftables") {
        score += 3;
        reasons.push("涉及防火墙规则变更".into());
    }

    if output_lower.contains("boot") || output_lower.contains("grub") || output_lower.contains("systemd-boot") || output_lower.contains("loader") {
        score += 3;
        reasons.push("涉及引导加载器变更".into());
    }

    if output_lower.contains("disk") || output_lower.contains("parted") || output_lower.contains("filesystem") || output_lower.contains("fs") || output_lower.contains("lvm") || output_lower.contains("zfs") {
        score += 3;
        reasons.push("涉及磁盘/文件系统变更".into());
    }

    if output_lower.contains("network") || output_lower.contains("interfaces") || output_lower.contains("interfaces") || output_lower.contains("dhcp") {
        score += 2;
        reasons.push("涉及网络配置变更".into());
    }

    // Core services are high-risk to restart
    let core_services = ["nginx", "sshd", "network", "firewall", "docker"];
    for svc in restart.iter().chain(stop.iter()) {
        for core in &core_services {
            if svc.contains(core) {
                score += 2;
                reasons.push(format!("将重启核心服务: {svc}"));
                break;
            }
        }
    }

    if !restart.is_empty() || !stop.is_empty() {
        let total = restart.len() + stop.len();
        score += 1;
        reasons.push(format!("将重启 {total} 个服务"));
    }

    if !added.is_empty() {
        score += 1;
        // Don't add reason for just adding packages — it's the normal case
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dry_build_packages() {
        let output = r#"
these derivations will be built:
  /nix/store/abc-nginx-1.24.2.drv
  /nix/store/def-php83-8.3.0.drv
building '/nix/store/abc-nginx-1.24.2.drv'...
building '/nix/store/def-php83-8.3.0.drv'...
"#;
        let (added, _removed) = parse_dry_build_packages(output);
        assert!(added.iter().any(|p| p.contains("nginx")));
        assert!(added.iter().any(|p| p.contains("php83")));
    }

    #[test]
    fn test_parse_dry_build_fetch() {
        let output = r#"
these paths will be fetched (23.45 MiB download, 89.12 MiB unpacked):
  /nix/store/aaa-nixpkgs-24.05
  /nix/store/bbb-somepackage-1.0
"#;
        let (added, _removed) = parse_dry_build_packages(output);
        assert_eq!(added.len(), 2);
    }

    #[test]
    fn test_assess_risk_safe() {
        let (level, reasons) = assess_risk(
            &["pkg-1.0".into()],
            &[],
            &[],
            &[],
            "building something",
        );
        assert_eq!(level, "safe");
        assert!(reasons.iter().any(|r| r.contains("无破坏性")));
    }

    #[test]
    fn test_assess_risk_dangerous() {
        let (level, reasons) = assess_risk(
            &[],
            &["old-pkg".into()],
            &["nginx.service".into()],
            &[],
            "firewall rules changed",
        );
        assert_eq!(level, "dangerous");
        assert!(reasons.iter().any(|r| r.contains("删除")));
        assert!(reasons.iter().any(|r| r.contains("防火墙")));
        assert!(reasons.iter().any(|r| r.contains("nginx")));
    }

    #[test]
    fn test_assess_risk_moderate() {
        let (level, _reasons) = assess_risk(
            &[],
            &[],
            &["someapp.service".into()],
            &[],
            "just a service restart",
        );
        assert_eq!(level, "moderate");
    }

    #[test]
    fn test_extract_pkg_name() {
        assert_eq!(
            extract_pkg_name("/nix/store/abc123-nginx-1.24.2"),
            Some("nginx-1.24.2".into())
        );
        assert_eq!(
            extract_pkg_name("  /nix/store/abc123-my-pkg-2.0 (45.6 MiB)"),
            Some("my-pkg-2.0".into())
        );
    }
}
