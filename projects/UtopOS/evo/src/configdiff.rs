use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::cmd::{run_cmd, AppStateRef};
use crate::error::AppError;

/// Request for config diff
#[derive(Debug, Deserialize)]
pub struct ConfigDiffRequest {
    /// First config content (or read from disk)
    pub config_a: Option<String>,
    /// Second config content
    pub config_b: Option<String>,
    /// Generation numbers to compare (alternative to content)
    pub gen_a: Option<u64>,
    pub gen_b: Option<u64>,
    /// Host for remote operations
    pub host: Option<String>,
}

/// Diff result
#[derive(Debug, Serialize)]
pub struct ConfigDiffResult {
    pub unified_diff: String,
    pub structured: StructuredDiff,
    pub risk_assessment: RiskAssessment,
    pub summary: String,
}

#[derive(Debug, Serialize)]
pub struct StructuredDiff {
    pub services_added: Vec<String>,
    pub services_removed: Vec<String>,
    pub services_modified: Vec<String>,
    pub packages_added: Vec<String>,
    pub packages_removed: Vec<String>,
    pub networking_changed: Vec<String>,
    pub security_changes: Vec<String>,
    pub lines_added: usize,
    pub lines_removed: usize,
}

#[derive(Debug, Serialize)]
pub struct RiskAssessment {
    pub level: String,       // safe, moderate, dangerous
    pub score: u32,          // 0-100
    pub factors: Vec<RiskFactor>,
    pub recommendation: String,
}

#[derive(Debug, Serialize)]
pub struct RiskFactor {
    pub category: String,
    pub severity: String,
    pub description: String,
}

/// Parse NixOS config into structured representation
fn parse_config_sections(content: &str) -> ConfigSections {
    let mut services = BTreeSet::new();
    let mut packages = BTreeSet::new();
    let mut networking = BTreeSet::new();
    let mut security = BTreeSet::new();
    let mut imports = BTreeSet::new();
    let mut other = BTreeSet::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        // Services
        if trimmed.starts_with("services.") {
            let section = extract_option_path(trimmed, "services");
            services.insert(section);
        }
        // Packages
        else if trimmed.contains("systemPackages") || trimmed.starts_with("environment.systemPackages") {
            // Extract package names from list
            for word in trimmed.split_whitespace() {
                if word.starts_with("pkgs.") && word.ends_with(|c: char| c.is_alphanumeric() || c == '"') {
                    let pkg = word.trim_start_matches("pkgs.").trim_end_matches('"').trim_end_matches(';');
                    if !pkg.is_empty() && pkg != "pkgs" {
                        packages.insert(pkg.to_string());
                    }
                }
            }
        }
        // Networking
        else if trimmed.starts_with("networking.") {
            let section = extract_option_path(trimmed, "networking");
            networking.insert(section);
        }
        // Security
        else if trimmed.starts_with("security.") || trimmed.starts_with("boot.") || trimmed.starts_with("firewall") {
            security.insert(trimmed.to_string());
        }
        // Imports
        else if trimmed.contains("imports") && trimmed.contains('=') {
            // Keep the line as-is for comparison
        }
    }

    ConfigSections { services, packages, networking, security, imports, other }
}

struct ConfigSections {
    services: BTreeSet<String>,
    packages: BTreeSet<String>,
    networking: BTreeSet<String>,
    security: BTreeSet<String>,
    imports: BTreeSet<String>,
    other: BTreeSet<String>,
}

/// Extract the option path prefix (e.g., "services.nginx.enable" -> "nginx")
fn extract_option_path(line: &str, prefix: &str) -> String {
    let after = line.trim_start_matches(&format!("{prefix}."));
    let section = after.split('.').next().unwrap_or(after);
    section.trim_end_matches(".enable").trim_end_matches(' ').to_string()
}

/// Compute structured diff between two configs
fn compute_structured_diff(sections_a: &ConfigSections, sections_b: &ConfigSections) -> StructuredDiff {
    let services_added: Vec<String> = sections_b.services.difference(&sections_a.services).cloned().collect();
    let services_removed: Vec<String> = sections_a.services.difference(&sections_b.services).cloned().collect();
    let services_modified: Vec<String> = sections_a.services.intersection(&sections_b.services).cloned().collect();

    let packages_added: Vec<String> = sections_b.packages.difference(&sections_a.packages).cloned().collect();
    let packages_removed: Vec<String> = sections_a.packages.difference(&sections_b.packages).cloned().collect();

    let networking_changed: Vec<String> = sections_b.networking.symmetric_difference(&sections_a.networking).cloned().collect();
    let security_changes: Vec<String> = sections_b.security.symmetric_difference(&sections_a.security).cloned().collect();

    // Count line differences (simple diff)
    let lines_a: BTreeSet<&str> = sections_a.services.iter().map(|s| s.as_str()).collect();
    let lines_b: BTreeSet<&str> = sections_b.services.iter().map(|s| s.as_str()).collect();
    let lines_added = lines_b.difference(&lines_a).count();
    let lines_removed = lines_a.difference(&lines_b).count();

    StructuredDiff {
        services_added,
        services_removed,
        services_modified,
        packages_added,
        packages_removed,
        networking_changed,
        security_changes,
        lines_added,
        lines_removed,
    }
}

/// Assess risk based on structured diff
fn assess_risk(diff: &StructuredDiff) -> RiskAssessment {
    let mut score: u32 = 0;
    let mut factors = Vec::new();

    // Service changes
    if !diff.services_removed.is_empty() {
        let s = diff.services_removed.len() as u32 * 15;
        score += s.min(30);
        factors.push(RiskFactor {
            category: "services".into(),
            severity: if diff.services_removed.len() > 2 { "high".into() } else { "medium".into() },
            description: format!("{} services will be removed", diff.services_removed.len()),
        });
    }

    if !diff.services_added.is_empty() {
        score += diff.services_added.len() as u32 * 5;
        factors.push(RiskFactor {
            category: "services".into(),
            severity: "low".into(),
            description: format!("{} new services will be enabled", diff.services_added.len()),
        });
    }

    // Package removals
    if !diff.packages_removed.is_empty() {
        let s = diff.packages_removed.len() as u32 * 10;
        score += s.min(25);
        factors.push(RiskFactor {
            category: "packages".into(),
            severity: "medium".into(),
            description: format!("{} packages will be removed", diff.packages_removed.len()),
        });
    }

    // Networking changes
    if !diff.networking_changed.is_empty() {
        score += 20;
        factors.push(RiskFactor {
            category: "networking".into(),
            severity: "high".into(),
            description: "Network configuration changes detected".into(),
        });
    }

    // Security changes
    if !diff.security_changes.is_empty() {
        score += 25;
        factors.push(RiskFactor {
            category: "security".into(),
            severity: "high".into(),
            description: "Security/boot configuration changes detected".into(),
        });
    }

    score = score.min(100);

    let level = if score >= 60 { "dangerous" } else if score >= 25 { "moderate" } else { "safe" }.to_string();

    let recommendation = match level.as_str() {
        "dangerous" => "此变更风险较高，建议在测试环境先验证，确认无误后再应用到生产环境。".to_string(),
        "moderate" => "此变更有一定风险，建议仔细检查变更内容，确认是否影响关键服务。".to_string(),
        _ => "此变更风险较低，可以安全应用。".to_string(),
    };

    RiskAssessment { level, score, factors, recommendation }
}

/// Generate unified diff between two config strings
fn generate_unified_diff(a: &str, b: &str) -> String {
    let a_lines: Vec<&str> = a.lines().collect();
    let b_lines: Vec<&str> = b.lines().collect();
    let mut diff = String::new();

    diff.push_str("--- configuration (before)
");
    diff.push_str("+++ configuration (after)
");

    // Simple line-by-line diff
    let max_len = a_lines.len().max(b_lines.len());
    let mut in_hunk = false;
    let mut hunk_start_a = 0;
    let mut hunk_start_b = 0;

    for i in 0..max_len {
        let line_a = a_lines.get(i).copied();
        let line_b = b_lines.get(i).copied();

        match (line_a, line_b) {
            (Some(a), Some(b)) if a == b => {
                if in_hunk {
                    diff.push_str(&format!(" {a}
"));
                }
            }
            (Some(a), Some(b)) => {
                if !in_hunk {
                    hunk_start_a = i + 1;
                    hunk_start_b = i + 1;
                    in_hunk = true;
                }
                diff.push_str(&format!("-{a}
"));
                diff.push_str(&format!("+{b}
"));
            }
            (Some(a), None) => {
                if !in_hunk {
                    hunk_start_a = i + 1;
                    hunk_start_b = i + 1;
                    in_hunk = true;
                }
                diff.push_str(&format!("-{a}
"));
            }
            (None, Some(b)) => {
                if !in_hunk {
                    hunk_start_a = i + 1;
                    hunk_start_b = i + 1;
                    in_hunk = true;
                }
                diff.push_str(&format!("+{b}
"));
            }
            (None, None) => {}
        }
    }

    if diff.is_empty() {
        diff.push_str("(no differences)
");
    }

    diff
}

/// POST /api/config/diff — compare two configurations
pub async fn handle_diff(
    State(state): AppStateRef,
    Json(req): Json<ConfigDiffRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Get config A
    let config_a = match &req.config_a {
        Some(c) => c.clone(),
        None => {
            let path = format!("{}/configuration.nix", state.config.nixos_dir);
            tokio::fs::read_to_string(&path).await.map_err(|e| AppError::IoError {
                path: path.clone(),
                message: e.to_string(),
            })?
        }
    };

    // Get config B
    let config_b = match &req.config_b {
        Some(c) => c.clone(),
        None => {
            return Err(AppError::Validation {
                field: "config_b".into(),
                message: "config_b is required when not comparing generations".into(),
            });
        }
    };

    // Parse and diff
    let sections_a = parse_config_sections(&config_a);
    let sections_b = parse_config_sections(&config_b);
    let structured = compute_structured_diff(&sections_a, &sections_b);
    let risk = assess_risk(&structured);
    let unified = generate_unified_diff(&config_a, &config_b);

    // Summary
    let mut summary_parts = Vec::new();
    if !structured.services_added.is_empty() {
        summary_parts.push(format!("+{} services", structured.services_added.len()));
    }
    if !structured.services_removed.is_empty() {
        summary_parts.push(format!("-{} services", structured.services_removed.len()));
    }
    if !structured.packages_added.is_empty() {
        summary_parts.push(format!("+{} packages", structured.packages_added.len()));
    }
    if !structured.packages_removed.is_empty() {
        summary_parts.push(format!("-{} packages", structured.packages_removed.len()));
    }

    let summary = if summary_parts.is_empty() {
        "No significant changes detected".to_string()
    } else {
        format!("Changes: {}", summary_parts.join(", "))
    };

    Ok(Json(serde_json::to_value(&ConfigDiffResult {
        unified_diff: unified,
        structured,
        risk_assessment: risk,
        summary,
    }).unwrap()))
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_services() {
        let config = r#"
{
  services.nginx.enable = true;
  services.postgresql.enable = true;
}
"#;
        let sections = parse_config_sections(config);
        assert!(sections.services.contains("nginx"));
        assert!(sections.services.contains("postgresql"));
    }

    #[test]
    fn test_diff_services_added() {
        let a = r#"{ services.nginx.enable = true; }"#;
        let b = r#"{ services.nginx.enable = true; services.redis.enable = true; }"#;

        let sa = parse_config_sections(a);
        let sb = parse_config_sections(b);
        let diff = compute_structured_diff(&sa, &sb);

        assert!(diff.services_added.contains(&"redis".to_string()));
        assert!(diff.services_removed.is_empty());
    }

    #[test]
    fn test_diff_services_removed() {
        let a = r#"{ services.nginx.enable = true; services.redis.enable = true; }"#;
        let b = r#"{ services.nginx.enable = true; }"#;

        let sa = parse_config_sections(a);
        let sb = parse_config_sections(b);
        let diff = compute_structured_diff(&sa, &sb);

        assert!(diff.services_removed.contains(&"redis".to_string()));
        assert!(diff.services_added.is_empty());
    }

    #[test]
    fn test_risk_safe_no_changes() {
        let diff = StructuredDiff {
            services_added: vec![], services_removed: vec![], services_modified: vec![],
            packages_added: vec![], packages_removed: vec![],
            networking_changed: vec![], security_changes: vec![],
            lines_added: 0, lines_removed: 0,
        };
        let risk = assess_risk(&diff);
        assert_eq!(risk.level, "safe");
        assert_eq!(risk.score, 0);
    }

    #[test]
    fn test_risk_dangerous_security_change() {
        let diff = StructuredDiff {
            services_added: vec![], services_removed: vec!["nginx".into()], services_modified: vec![],
            packages_added: vec![], packages_removed: vec!["vim".into(), "git".into()],
            networking_changed: vec!["firewall".into()],
            security_changes: vec!["boot.loader".into()],
            lines_added: 1, lines_removed: 3,
        };
        let risk = assess_risk(&diff);
        assert_eq!(risk.level, "dangerous");
        assert!(risk.score >= 60);
    }

    #[test]
    fn test_risk_moderate() {
        let diff = StructuredDiff {
            services_added: vec!["redis".into()],
            services_removed: vec![],
            services_modified: vec![],
            packages_added: vec!["jq".into()],
            packages_removed: vec!["tree".into()],
            networking_changed: vec![],
            security_changes: vec![],
            lines_added: 2, lines_removed: 1,
        };
        let risk = assess_risk(&diff);
        assert_eq!(risk.level, "moderate");
    }

    #[test]
    fn test_unified_diff_basic() {
        let a = "line1
line2
line3
";
        let b = "line1
modified
line3
";
        let diff = generate_unified_diff(a, b);
        assert!(diff.contains("-line2"));
        assert!(diff.contains("+modified"));
    }

    #[test]
    fn test_unified_diff_no_changes() {
        let a = "same
";
        let b = "same
";
        let diff = generate_unified_diff(a, b);
        assert!(diff.contains("no differences"));
    }

    #[test]
    fn test_extract_option_path() {
        assert_eq!(extract_option_path("services.nginx.enable = true;", "services"), "nginx");
        assert_eq!(extract_option_path("services.postgresql.package = pkgs.postgresql_15;", "services"), "postgresql");
        assert_eq!(extract_option_path("networking.hostName = "test";", "networking"), "hostName");
    }
}
