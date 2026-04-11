use axum::{response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::cmd::run_cmd;
use crate::error::AppError;

/// An optimization suggestion
#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub id: String,
    pub category: String,     // "performance", "security", "storage", "services"
    pub title: String,
    pub description: String,
    pub impact: String,       // "low", "medium", "high"
    pub effort: String,       // "trivial", "easy", "moderate", "hard"
    pub nix_snippet: Option<String>,
    pub reference_url: Option<String>,
}

/// Full optimization report
#[derive(Debug, Serialize)]
pub struct OptimizationReport {
    pub analyzed_at: String,
    pub total_suggestions: usize,
    pub quick_wins: usize,
    pub suggestions: Vec<Suggestion>,
}

/// Analyze the system and generate optimization suggestions
pub async fn analyze_system() -> Result<OptimizationReport, AppError> {
    let mut suggestions = Vec::new();

    // 1. Check for unnecessary services
    suggestions.extend(check_unused_services().await);

    // 2. Check for security hardening
    suggestions.extend(check_security().await);

    // 3. Check for storage optimization
    suggestions.extend(check_storage().await);

    // 4. Check for performance tuning
    suggestions.extend(check_performance().await);

    // 5. Check Nix store health
    suggestions.extend(check_nix_store().await);

    let quick_wins = suggestions.iter()
        .filter(|s| s.effort == "trivial" || s.effort == "easy")
        .count();

    Ok(OptimizationReport {
        analyzed_at: chrono_now(),
        total_suggestions: suggestions.len(),
        quick_wins,
        suggestions,
    })
}

async fn check_unused_services() -> Vec<Suggestion> {
    let mut out = Vec::new();

    // Check for services that might not be needed
    let candidates = [
        ("cups", "Print service — usually unnecessary on a server"),
        ("avahi-daemon", "mDNS — not needed unless you use .local discovery"),
        ("bluetooth", "Bluetooth — unnecessary on headless servers"),
    ];

    for (svc, desc) in &candidates {
        if let Ok(o) = tokio::process::Command::new("systemctl")
            .args(&["is-active", svc])
            .output()
            .await
        {
            let state = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if state == "active" {
                out.push(Suggestion {
                    id: format!("unused-{svc}"),
                    category: "services".into(),
                    title: format!("Disable unused service: {svc}"),
                    description: desc.to_string(),
                    impact: "low".into(),
                    effort: "trivial".into(),
                    nix_snippet: Some(format!("services.{svc}.enable = false;")),
                    reference_url: None,
                });
            }
        }
    }

    out
}

async fn check_security() -> Vec<Suggestion> {
    let mut out = Vec::new();

    // Check SSH config
    if let Ok(config) = tokio::fs::read_to_string("/etc/ssh/sshd_config").await {
        if config.contains("PermitRootLogin yes") || config.contains("PermitRootLogin without-password") {
            out.push(Suggestion {
                id: "ssh-root".into(),
                category: "security".into(),
                title: "Disable SSH root login".into(),
                description: "Root login via SSH is enabled. Use a regular user + sudo instead.".into(),
                impact: "high".into(),
                effort: "easy".into(),
                nix_snippet: Some("services.openssh.settings.PermitRootLogin = \"no\";".into()),
                reference_url: Some("https://wiki.nixos.org/wiki/SSH".into()),
            });
        }

        if !config.contains("PasswordAuthentication no") {
            out.push(Suggestion {
                id: "ssh-password".into(),
                category: "security".into(),
                title: "Disable SSH password authentication".into(),
                description: "Password auth is enabled. Use key-based auth only.".into(),
                impact: "high".into(),
                effort: "easy".into(),
                nix_snippet: Some("services.openssh.settings.PasswordAuthentication = false;".into()),
                reference_url: Some("https://wiki.nixos.org/wiki/SSH".into()),
            });
        }
    }

    // Check firewall
    if let Ok(config) = tokio::fs::read_to_string("/etc/nixos/configuration.nix").await {
        if !config.contains("networking.firewall") {
            out.push(Suggestion {
                id: "firewall".into(),
                category: "security".into(),
                title: "Enable NixOS firewall".into(),
                description: "No explicit firewall configuration found. Enable it to restrict inbound traffic.".into(),
                impact: "high".into(),
                effort: "easy".into(),
                nix_snippet: Some("networking.firewall.enable = true;\nnetworking.firewall.allowedTCPPorts = [ 22 80 443 ];".into()),
                reference_url: Some("https://wiki.nixos.org/wiki/Firewall".into()),
            });
        }
    }

    // Check for unattended upgrades
    if let Ok(config) = tokio::fs::read_to_string("/etc/nixos/configuration.nix").await {
        if !config.contains("system.autoUpgrade") {
            out.push(Suggestion {
                id: "auto-upgrade".into(),
                category: "security".into(),
                title: "Enable automatic security updates".into(),
                description: "No auto-upgrade configured. Enable for security patches.".into(),
                impact: "medium".into(),
                effort: "trivial".into(),
                nix_snippet: Some(
                    "system.autoUpgrade = {\n  enable = true;\n  allowReboot = false;\n  dates = \"04:00\";\n};".into()
                ),
                reference_url: Some("https://wiki.nixos.org/wiki/Automatic_system_upgrades".into()),
            });
        }
    }

    out
}

async fn check_storage() -> Vec<Suggestion> {
    let mut out = Vec::new();

    // Check Nix store size
    if let Ok(o) = tokio::process::Command::new("du")
        .args(&["-sh", "/nix/store"])
        .output()
        .await
    {
        let size_str = String::from_utf8_lossy(&o.stdout);
        if let Some(size) = size_str.split_whitespace().next() {
            if size.contains("G") {
                if let Some(gb) = size.trim_end_matches('G').parse::<f64>().ok() {
                    if gb > 50.0 {
                        out.push(Suggestion {
                            id: "store-cleanup".into(),
                            category: "storage".into(),
                            title: "Clean up Nix store".into(),
                            description: format!("Nix store is {size}. Run garbage collection to reclaim space."),
                            impact: "medium".into(),
                            effort: "trivial".into(),
                            nix_snippet: Some("nix.gc = {\n  automatic = true;\n  dates = \"weekly\";\n  options = \"--delete-older-than 30d\";\n};".into()),
                            reference_url: Some("https://wiki.nixos.org/wiki/NixOS:_Clean_up_old_system_generations".into()),
                        });
                    }
                }
            }
        }
    }

    // Check /tmp usage
    if let Ok(o) = tokio::process::Command::new("df")
        .args(&["-h", "/tmp"])
        .output()
        .await
    {
        let output = String::from_utf8_lossy(&o.stdout);
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(use_pct) = parts.get(4) {
                if let Some(pct) = use_pct.trim_end_matches('%').parse::<u32>().ok() {
                    if pct > 80 {
                        out.push(Suggestion {
                            id: "tmp-cleanup".into(),
                            category: "storage".into(),
                            title: "Clean up /tmp".into(),
                            description: format!("/tmp is {pct}% full."),
                            impact: "medium".into(),
                            effort: "trivial".into(),
                            nix_snippet: Some("boot.tmp.cleanOnBoot = true;".into()),
                            reference_url: None,
                        });
                    }
                }
            }
        }
    }

    out
}

async fn check_performance() -> Vec<Suggestion> {
    let mut out = Vec::new();

    // Check if swap is being used heavily
    if let Ok(content) = tokio::fs::read_to_string("/proc/meminfo").await {
        for line in content.lines() {
            if line.starts_with("SwapTotal:") || line.starts_with("SwapFree:") {
                // Parse values
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let (Some(label), Some(kb_str)) = (parts.first(), parts.get(1)) {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        let gb = kb as f64 / 1048576.0;
                        if label.contains("Total") && gb > 4.0 {
                            out.push(Suggestion {
                                id: "swap-large".into(),
                                category: "performance".into(),
                                title: "Large swap detected".into(),
                                description: format!("Swap is {:.1}GB. Consider reducing if RAM is sufficient.", gb),
                                impact: "low".into(),
                                effort: "moderate".into(),
                                nix_snippet: Some("swapDevices = [];".into()),
                                reference_url: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Check kernel params
    if let Ok(cmdline) = tokio::fs::read_to_string("/proc/cmdline").await {
        if !cmdline.contains("transparent_hugepage") {
            out.push(Suggestion {
                id: "thp".into(),
                category: "performance".into(),
                title: "Consider configuring Transparent Huge Pages".into(),
                description: "THP not explicitly configured. For databases, disable it; for general workloads, madvise is recommended.".into(),
                impact: "low".into(),
                effort: "easy".into(),
                nix_snippet: Some("boot.kernel.sysctl.\"transparent_hugepage\" = \"madvise\";".into()),
                reference_url: Some("https://www.kernel.org/doc/html/latest/admin-guide/mm/transhuge.html".into()),
            });
        }
    }

    out
}

async fn check_nix_store() -> Vec<Suggestion> {
    let mut out = Vec::new();

    // Check number of generations
    if let Ok(output) = run_cmd("nix-env", &["-p", "/nix/var/nix/profiles/system", "--list-generations"]).await {
        let count = output.lines().count();
        if count > 50 {
            out.push(Suggestion {
                id: "gen-cleanup".into(),
                category: "storage".into(),
                title: format!("{} NixOS generations accumulated", count),
                description: "Old generations consume disk space. Delete generations older than 30 days.".into(),
                impact: "medium".into(),
                effort: "trivial".into(),
                nix_snippet: Some(format!(
                    "# Delete old generations:\nnix-env -p /nix/var/nix/profiles/system --delete-generations old\n\n# Or auto-cleanup:\nnix.gc = {{ automatic = true; dates = \"weekly\"; }};"
                )),
                reference_url: None,
            });
        }
    }

    out
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_default()
}

// ─── HTTP Handler ──────────────────────────────────────────────────────

/// GET /api/optimizer/analyze
pub async fn handle_analyze() -> Result<Json<OptimizationReport>, AppError> {
    let report = analyze_system().await?;
    Ok(Json(report))
}
