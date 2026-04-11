use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::cmd::{run_cmd, AppStateRef};
use crate::error::AppError;

/// Health factor with individual score
#[derive(Debug, Clone, Serialize)]
pub struct HealthFactor {
    pub name: String,
    pub score: f64,        // 0-100
    pub weight: f64,       // weight in overall score
    pub status: String,    // "good", "warning", "critical"
    pub details: String,
    pub recommendations: Vec<String>,
}

/// Historical score point
#[derive(Debug, Clone, Serialize)]
pub struct ScoreHistory {
    pub timestamp: String,
    pub score: f64,
}

/// Full health score response
#[derive(Debug, Serialize)]
pub struct HealthScoreResponse {
    pub overall_score: f64,
    pub grade: String,
    pub timestamp: String,
    pub factors: Vec<HealthFactor>,
    pub trend: Vec<ScoreHistory>,
    pub summary: HealthSummary,
}

#[derive(Debug, Serialize)]
pub struct HealthSummary {
    pub good_factors: usize,
    pub warning_factors: usize,
    pub critical_factors: usize,
    pub top_issue: Option<String>,
}

/// In-memory score history
static HISTORY: std::sync::OnceLock<Arc<RwLock<Vec<ScoreHistory>>>> = std::sync::OnceLock::new();

fn get_history() -> &'static Arc<RwLock<Vec<ScoreHistory>>> {
    HISTORY.get_or_init(|| Arc::new(RwLock::new(Vec::new())))
}

/// Score service health
async fn score_services() -> HealthFactor {
    let output = run_cmd("systemctl", &["list-units", "--type=service", "--state=running", "--no-pager", "--no-legend"]).await;
    let failed = run_cmd("systemctl", &["--failed", "--no-pager", "--no-legend"]).await;

    let running_count = output.map(|o| o.lines().filter(|l| !l.trim().is_empty()).count()).unwrap_or(0);
    let failed_count = failed.map(|o| o.lines().filter(|l| !l.trim().is_empty()).count()).unwrap_or(0);

    let (score, status, details) = if failed_count == 0 {
        (100.0, "good", format!("{} services running, 0 failed", running_count))
    } else if failed_count <= 2 {
        let s = 100.0 - (failed_count as f64 * 15.0);
        (s, "warning", format!("{} services running, {} failed", running_count, failed_count))
    } else {
        let s = (100.0 - (failed_count as f64 * 10.0)).max(20.0);
        (s, "critical", format!("{} services running, {} failed — needs attention", running_count, failed_count))
    };

    let mut recs = Vec::new();
    if failed_count > 0 {
        recs.push("Run 'systemctl --failed' to see failed services".into());
        recs.push("Check logs: journalctl -xe".into());
    }

    HealthFactor {
        name: "Services".into(),
        score,
        weight: 0.25,
        status: status.into(),
        details,
        recommendations: recs,
    }
}

/// Score disk health
async fn score_disk() -> HealthFactor {
    let output = run_cmd("df", &["--output=pcent", "/"]).await;
    let usage = output.ok()
        .and_then(|o| o.lines().nth(1).map(|l| l.trim().to_string()))
        .and_then(|s| s.trim_end_matches('%').parse::<f64>().ok())
        .unwrap_or(0.0);

    let (score, status) = if usage < 60.0 {
        (100.0, "good")
    } else if usage < 80.0 {
        (100.0 - (usage - 60.0) * 2.5, "good")
    } else if usage < 90.0 {
        (50.0 - (usage - 80.0) * 3.0, "warning")
    } else {
        ((100.0 - usage).max(0.0), "critical")
    };

    let mut recs = Vec::new();
    if usage > 80.0 {
        recs.push("Run nix-collect-garbage -d".into());
        recs.push("Check large files: du -sh /nix/store/* | sort -rh | head".into());
    }
    if usage > 90.0 {
        recs.push("URGENT: Disk critically low. Delete old generations immediately.".into());
    }

    HealthFactor {
        name: "Disk Space".into(),
        score: score.max(0.0),
        weight: 0.20,
        status: status.into(),
        details: format!("Root filesystem at {:.0}% usage", usage),
        recommendations: recs,
    }
}

/// Score memory health
async fn score_memory() -> HealthFactor {
    let output = run_cmd("free", &["-b"]).await;
    let (usage_pct, swap_pct) = if let Ok(out) = output {
        let mut mem_pct = 0.0;
        let mut swp_pct = 0.0;
        for line in out.lines() {
            if line.starts_with("Mem:") {
                let p: Vec<&str> = line.split_whitespace().collect();
                if p.len() >= 3 {
                    let t: f64 = p[1].parse().unwrap_or(1.0);
                    let u: f64 = p[2].parse().unwrap_or(0.0);
                    mem_pct = u / t * 100.0;
                }
            }
            if line.starts_with("Swap:") {
                let p: Vec<&str> = line.split_whitespace().collect();
                if p.len() >= 3 {
                    let t: f64 = p[1].parse().unwrap_or(1.0);
                    let u: f64 = p[2].parse().unwrap_or(0.0);
                    if t > 0.0 { swp_pct = u / t * 100.0; }
                }
            }
        }
        (mem_pct, swp_pct)
    } else {
        (0.0, 0.0)
    };

    let (score, status) = if usage_pct < 70.0 {
        (100.0, "good")
    } else if usage_pct < 85.0 {
        (100.0 - (usage_pct - 70.0) * 3.0, "warning")
    } else {
        ((100.0 - usage_pct).max(10.0), "critical")
    };

    let mut recs = Vec::new();
    if usage_pct > 80.0 {
        recs.push("Identify heavy processes: ps aux --sort=-%mem | head -10".into());
    }
    if swap_pct > 50.0 {
        recs.push("High swap usage indicates memory pressure. Consider adding RAM.".into());
    }

    HealthFactor {
        name: "Memory".into(),
        score: score.max(0.0),
        weight: 0.15,
        status: status.into(),
        details: format!("Memory at {:.0}%, Swap at {:.0}%", usage_pct, swap_pct),
        recommendations: recs,
    }
}

/// Score security posture
async fn score_security() -> HealthFactor {
    let mut score: f64 = 100.0;
    let mut issues = Vec::new();

    // Check firewall
    let fw = run_cmd("iptables", &["-L", "-n"]).await;
    if let Ok(output) = fw {
        if output.lines().count() < 10 {
            score -= 15.0;
            issues.push("Firewall rules seem minimal".into());
        }
    }

    // Check SSH config
    if let Ok(ssh_config) = tokio::fs::read_to_string("/etc/ssh/sshd_config").await {
        if ssh_config.contains("PermitRootLogin yes") {
            score -= 20.0;
            issues.push("SSH allows root login".into());
        }
        if ssh_config.contains("PasswordAuthentication yes") {
            score -= 10.0;
            issues.push("SSH allows password auth (prefer keys)".into());
        }
    }

    // Check for unattended upgrades
    let upgrades = run_cmd("systemctl", &["is-active", "unattended-upgrades"]).await;
    if upgrades.map(|o| o.trim().to_string()) != Ok("active".to_string()) {
        score -= 5.0;
        issues.push("Automatic updates not active".into());
    }

    let status = if score >= 80.0 { "good" } else if score >= 60.0 { "warning" } else { "critical" };

    let mut recs = Vec::new();
    if issues.iter().any(|i| i.contains("root")) {
        recs.push("Set PermitRootLogin to 'prohibit-password' or 'no'".into());
    }
    if issues.iter().any(|i| i.contains("password")) {
        recs.push("Disable password authentication: PasswordAuthentication no".into());
    }

    HealthFactor {
        name: "Security".into(),
        score: score.max(0.0),
        weight: 0.15,
        status: status.into(),
        details: if issues.is_empty() { "No security issues detected".into() } else { issues.join("; ") },
        recommendations: recs,
    }
}

/// Score config quality
async fn score_config() -> HealthFactor {
    let mut score: f64 = 100.0;
    let mut notes = Vec::new();

    // Check if configuration.nix exists
    if !std::path::Path::new("/etc/nixos/configuration.nix").exists() {
        score -= 30.0;
        notes.push("No /etc/nixos/configuration.nix found".into());
    }

    // Check if using flakes
    if std::path::Path::new("/etc/nixos/flake.nix").exists() {
        notes.push("Using flakes (modern)".into());
    }

    // Check number of generations
    let gens = run_cmd("nix-env", &["-p", "/nix/var/nix/profiles/system", "--list-generations"]).await;
    let gen_count = gens.map(|o| o.lines().count()).unwrap_or(0);
    if gen_count > 100 {
        score -= 10.0;
        notes.push(format!("{} generations (consider cleanup)", gen_count));
    }

    // Check for syntax errors
    let check = run_cmd("nix-instantiate", &["--parse", "/etc/nixos/configuration.nix"]).await;
    if check.is_err() {
        score -= 25.0;
        notes.push("Configuration has parse errors".into());
    }

    let status = if score >= 80.0 { "good" } else if score >= 60.0 { "warning" } else { "critical" };

    HealthFactor {
        name: "Config Quality".into(),
        score: score.max(0.0),
        weight: 0.10,
        status: status.into(),
        details: notes.join("; "),
        recommendations: Vec::new(),
    }
}

/// Score update freshness
async fn score_updates() -> HealthFactor {
    let mut score: f64 = 100.0;
    let mut details = Vec::new();

    // Check last system build time
    let link = std::fs::read_link("/run/current-system").ok();
    if let Some(path) = link {
        if let Ok(meta) = std::fs::metadata(&path) {
            if let Ok(modified) = meta.modified() {
                let age = std::time::SystemTime::now().duration_since(modified).unwrap_or_default();
                let days = age.as_secs() / 86400;
                details.push(format!("System built {} days ago", days));
                if days > 30 {
                    score -= 20.0;
                    details.push("System is more than 30 days old".into());
                }
                if days > 90 {
                    score -= 20.0;
                    details.push("System is severely outdated".into());
                }
            }
        }
    }

    // Check channel update time
    let channels = run_cmd("nix-channel", &["--list"]).await;
    if let Ok(ch) = channels {
        if ch.trim().is_empty() {
            details.push("No channels configured (may be using flakes)".into());
        }
    }

    let status = if score >= 80.0 { "good" } else if score >= 60.0 { "warning" } else { "critical" };

    HealthFactor {
        name: "Update Freshness".into(),
        score: score.max(0.0),
        weight: 0.15,
        status: status.into(),
        details: details.join("; "),
        recommendations: if score < 80.0 { vec!["Consider updating: nixos-rebuild switch --upgrade".into()] } else { vec![] },
    }
}

/// GET /api/health/score — composite system health score
pub async fn handle_score(
    State(_state): AppStateRef,
) -> Result<impl IntoResponse, AppError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let timestamp = format!("{}", now.as_secs());

    // Collect all factors
    let factors = vec![
        score_services().await,
        score_disk().await,
        score_memory().await,
        score_security().await,
        score_config().await,
        score_updates().await,
    ];

    // Weighted average
    let total_weight: f64 = factors.iter().map(|f| f.weight).sum();
    let weighted_score: f64 = factors.iter().map(|f| f.score * f.weight).sum::<f64>() / total_weight;
    let overall_score = (weighted_score * 10.0).round() / 10.0;

    let grade = match overall_score as i32 {
        90..=100 => "A",
        80..=89 => "B",
        70..=79 => "C",
        60..=69 => "D",
        _ => "F",
    };

    let good = factors.iter().filter(|f| f.status == "good").count();
    let warning = factors.iter().filter(|f| f.status == "warning").count();
    let critical = factors.iter().filter(|f| f.status == "critical").count();

    let top_issue = factors.iter()
        .filter(|f| f.status != "good")
        .min_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
        .map(|f| format!("{}: {}", f.name, f.details.clone()));

    // Record to history
    {
        let history = get_history();
        let mut h = history.write().await;
        h.push(ScoreHistory { timestamp: timestamp.clone(), score: overall_score });
        // Keep last 100 points
        if h.len() > 100 {
            h.drain(0..h.len() - 100);
        }
    }

    let trend = get_history().read().await.clone();

    Ok(Json(serde_json::to_value(HealthScoreResponse {
        overall_score,
        grade: grade.into(),
        timestamp,
        factors,
        trend,
        summary: HealthSummary {
            good_factors: good,
            warning_factors: warning,
            critical_factors: critical,
            top_issue,
        },
    }).unwrap_or_default()))
}
