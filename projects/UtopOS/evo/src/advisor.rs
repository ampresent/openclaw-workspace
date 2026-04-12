use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use crate::error::AppError;
use crate::cmd::{run_cmd, run_cmd_with_timeout, read_generation_description};

/// A rollback candidate with scoring
#[derive(Debug, Clone, Serialize)]
pub struct RollbackCandidate {
    pub generation: u64,
    pub score: f64,            // 0.0 - 1.0 (higher = better rollback target)
    pub age_hours: f64,
    pub description: String,
    pub reasons: Vec<String>,
}

/// Rollback recommendation
#[derive(Debug, Serialize)]
pub struct RollbackRecommendation {
    pub recommended_generation: u64,
    pub confidence: f64,
    pub candidates: Vec<RollbackCandidate>,
    pub current_generation: u64,
    pub current_issues: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Deserialize)]
pub struct RecommendRequest {
    pub lookback: Option<usize>,
    pub critical_services: Option<Vec<String>>,
}

/// Capacity report
#[derive(Debug, Serialize)]
pub struct CapacityReport {
    pub disk: Vec<DiskForecast>,
    pub memory: MemoryForecast,
    pub cpu: CpuForecast,
    pub nix_store_gb: f64,
    pub gc_savings_gb: Option<f64>,
    pub recommendations: Vec<CapacityRecommendation>,
}

#[derive(Debug, Serialize)]
pub struct DiskForecast {
    pub mount: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub usage_percent: f64,
    pub risk_level: String,   // "low", "medium", "high", "critical"
    pub days_until_full: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct MemoryForecast {
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub usage_percent: f64,
    pub swap_total_gb: f64,
    pub swap_used_gb: f64,
    pub risk_level: String,
}

#[derive(Debug, Serialize)]
pub struct CpuForecast {
    pub cores: u32,
    pub load_1m: f64,
    pub load_5m: f64,
    pub load_15m: f64,
    pub load_per_core: f64,
    pub risk_level: String,
}

#[derive(Debug, Serialize)]
pub struct CapacityRecommendation {
    pub resource: String,
    pub severity: String,
    pub action: String,
    pub details: String,
}

// ============================================================
// Rollback Advisor
// ============================================================

/// POST /api/advisor/rollback — get smart rollback recommendation
pub async fn rollback_recommendation(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<RecommendRequest>,
) -> Result<Json<RollbackRecommendation>, AppError> {
    let current_gen = get_current_generation().await?;
    let lookback = req.lookback.unwrap_or(10);
    let critical_services = req.critical_services.unwrap_or_default();

    // Detect current issues
    let current_issues = detect_issues(&critical_services).await;

    // List recent generations
    let generations = list_generations(lookback).await?;

    // Score each generation
    let mut candidates: Vec<RollbackCandidate> = Vec::new();
    for (gen_num, date_str, desc) in &generations {
        if *gen_num >= current_gen {
            continue; // Don't recommend current or future gens
        }

        let (score, reasons) = score_generation(
            *gen_num,
            current_gen,
            &current_issues,
            &critical_services,
            desc,
        ).await;

        let age_hours = parse_date_to_hours(&date_str);

        candidates.push(RollbackCandidate {
            generation: *gen_num,
            score,
            age_hours,
            description: desc.clone(),
            reasons,
        });
    }

    // Sort by score descending
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    let recommended = candidates.first().map(|c| c.generation).unwrap_or(current_gen.saturating_sub(1));
    let confidence = candidates.first().map(|c| c.score).unwrap_or(0.0);

    let summary = if current_issues.is_empty() {
        "系统运行正常，无需回滚".into()
    } else {
        format!(
            "检测到 {} 个问题，推荐回滚到第 {} 代 (置信度: {:.0}%)",
            current_issues.len(), recommended, confidence * 100.0
        )
    };

    Ok(Json(RollbackRecommendation {
        recommended_generation: recommended,
        confidence,
        candidates,
        current_generation: current_gen,
        current_issues,
        summary,
    }))
}

async fn get_current_generation() -> Result<u64, AppError> {
    let output = run_cmd("nix-env", &["-p", "/nix/var/nix/profiles/system", "--list-generations"]).await?;
    let current = output
        .lines()
        .filter(|l| l.contains("(current)"))
        .filter_map(|l| l.split_whitespace().next().and_then(|s| s.parse().ok()))
        .max()
        .unwrap_or(0);
    Ok(current)
}

async fn list_generations(limit: usize) -> Result<Vec<(u64, String, String)>, AppError> {
    let output = run_cmd("nix-env", &["-p", "/nix/var/nix/profiles/system", "--list-generations"]).await?;
    let mut gens: Vec<(u64, String, String)> = output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let parts: Vec<&str> = trimmed.splitn(3, |c: char| c.is_whitespace()).collect();
            if parts.len() >= 2 {
                let num = parts[0].parse::<u64>().ok()?;
                let date = parts[1].to_string();
                let desc = if parts.len() >= 3 {
                    parts[2].trim().to_string()
                } else {
                    read_generation_description(num)
                };
                Some((num, date, desc))
            } else {
                None
            }
        })
        .collect();

    gens.sort_by_key(|g| g.0);
    let start = gens.len().saturating_sub(limit);
    Ok(gens[start..].to_vec())
}

async fn detect_issues(critical_services: &[String]) -> Vec<String> {
    let mut issues = Vec::new();

    // Check failed services
    if let Ok(output) = run_cmd_with_timeout("systemctl", &["--failed", "--no-legend"], 10).await {
        for line in output.lines().filter(|l| !l.trim().is_empty()) {
            issues.push(format!("服务失败: {}", line.trim()));
        }
    }

    // Check critical services
    for svc in critical_services {
        if let Ok(state) = run_cmd_with_timeout("systemctl", &["is-active", svc], 5).await {
            if state.trim() != "active" {
                issues.push(format!("关键服务 {svc} 非活跃状态: {}", state.trim()));
            }
        }
    }

    // Check disk
    if let Ok(output) = run_cmd("df", &["--output=pcent", "/"]).await {
        for line in output.lines().skip(1) {
            if let Some(pct) = line.trim().trim_end_matches('%').parse::<f64>().ok() {
                if pct > 95.0 {
                    issues.push(format!("根分区使用率过高: {pct}%"));
                }
            }
        }
    }

    issues
}

async fn score_generation(
    gen: u64,
    current_gen: u64,
    current_issues: &[String],
    critical_services: &[String],
    description: &str,
) -> (f64, Vec<String>) {
    let mut score = 0.5; // base score
    let mut reasons = Vec::new();

    // Prefer recent generations (closer = better)
    let age = (current_gen - gen) as f64;
    let recency_score = 1.0 / (1.0 + age * 0.1);
    score += recency_score * 0.2;
    if age <= 2.0 {
        reasons.push("最近的生成".into());
    }

    // Prefer generations that were stable (long uptime)
    let gen_path = format!("/nix/var/nix/profiles/system-{gen}-link");
    if std::path::Path::new(&gen_path).exists() {
        score += 0.1;
        reasons.push("生成存在且可访问".into());
    }

    // Check if this generation's config had critical services
    for svc in critical_services {
        let svc_path = format!("{gen_path}/etc/systemd/system/{svc}.service");
        if std::path::Path::new(&svc_path).exists() {
            score += 0.05;
            reasons.push(format!("包含关键服务 {svc}"));
        }
    }

    // Prefer stable descriptions (avoid "test", "wip", "temp")
    let desc_lower = description.to_lowercase();
    if desc_lower.contains("test") || desc_lower.contains("wip") || desc_lower.contains("temp") {
        score -= 0.15;
        reasons.push("标记为测试/临时配置".into());
    } else if !description.is_empty() {
        score += 0.05;
        reasons.push(format!("有描述: {description}"));
    }

    // If current system has issues, boost older stable generations
    if !current_issues.is_empty() {
        score += 0.1;
        reasons.push("当前系统有问题，此生成可能更稳定".into());
    }

    (score.max(0.0).min(1.0), reasons)
}

fn parse_date_to_hours(date_str: &str) -> f64 {
    // Try to parse various date formats
    // NixOS generations usually list dates like "2026-04-10 14:30"
    // For simplicity, just return a placeholder
    24.0
}

// ============================================================
// Capacity Planning
// ============================================================

/// GET /api/advisor/capacity — system capacity analysis
pub async fn capacity_report(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<CapacityReport>, AppError> {
    let disk = collect_disk_forecast().await?;
    let memory = collect_memory_forecast().await?;
    let cpu = collect_cpu_forecast().await?;
    let nix_store_gb = get_nix_store_size().await;
    let gc_savings = estimate_gc_savings().await;

    let mut recommendations = Vec::new();

    // Disk recommendations
    for d in &disk {
        if d.risk_level == "critical" {
            recommendations.push(CapacityRecommendation {
                resource: format!("磁盘 {}", d.mount),
                severity: "critical".into(),
                action: "立即释放空间".into(),
                details: format!("使用率 {:.0}%，仅剩 {:.1} GB", d.usage_percent, d.available_gb),
            });
        } else if d.risk_level == "high" {
            recommendations.push(CapacityRecommendation {
                resource: format!("磁盘 {}", d.mount),
                severity: "high".into(),
                action: "计划扩容或清理".into(),
                details: format!("使用率 {:.0}%", d.usage_percent),
            });
        }
    }

    // Nix store GC recommendation
    if nix_store_gb > 20.0 {
        let savings = gc_savings.unwrap_or(0.0);
        if savings > 2.0 {
            recommendations.push(CapacityRecommendation {
                resource: "Nix Store".into(),
                severity: "medium".into(),
                action: "运行 nix-collect-garbage".into(),
                details: format!("Nix Store {:.1} GB，可回收约 {:.1} GB", nix_store_gb, savings),
            });
        }
    }

    // Memory recommendations
    if memory.risk_level == "critical" {
        recommendations.push(CapacityRecommendation {
            resource: "内存".into(),
            severity: "critical".into(),
            action: "检查内存泄漏或增加内存".into(),
            details: format!("使用率 {:.0}%，可用 {:.1} GB", memory.usage_percent, memory.available_gb),
        });
    }

    // CPU recommendations
    if cpu.risk_level == "critical" {
        recommendations.push(CapacityRecommendation {
            resource: "CPU".into(),
            severity: "high".into(),
            action: "检查高负载进程".into(),
            details: format!("负载 {:.2}/核 ({}核)", cpu.load_per_core, cpu.cores),
        });
    }

    if recommendations.is_empty() {
        recommendations.push(CapacityRecommendation {
            resource: "系统整体".into(),
            severity: "low".into(),
            action: "无需操作".into(),
            details: "所有资源使用在正常范围内".into(),
        });
    }

    Ok(Json(CapacityReport {
        disk,
        memory,
        cpu,
        nix_store_gb,
        gc_savings_gb: gc_savings,
        recommendations,
    }))
}

async fn collect_disk_forecast() -> Result<Vec<DiskForecast>, AppError> {
    let output = run_cmd("df", &["-BG", "--output=target,size,used,avail,pcent", "/", "/tmp", "/nix/store"]).await?;
    let mut disks = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        let mount = parts[0].to_string();
        if !seen.insert(mount.clone()) {
            continue;
        }
        let total: f64 = parts[1].trim_end_matches('G').parse().unwrap_or(0.0);
        let used: f64 = parts[2].trim_end_matches('G').parse().unwrap_or(0.0);
        let avail: f64 = parts[3].trim_end_matches('G').parse().unwrap_or(0.0);
        let pct: f64 = parts[4].trim_end_matches('%').parse().unwrap_or(0.0);

        let risk = if pct >= 95.0 { "critical" }
            else if pct >= 85.0 { "high" }
            else if pct >= 70.0 { "medium" }
            else { "low" };

        disks.push(DiskForecast {
            mount, total_gb: total, used_gb: used, available_gb: avail,
            usage_percent: pct, risk_level: risk.into(), days_until_full: None,
        });
    }
    Ok(disks)
}

async fn collect_memory_forecast() -> Result<MemoryForecast, AppError> {
    let output = run_cmd("free", &["-b"]).await?;
    let mut total = 0u64;
    let mut used = 0u64;
    let mut available = 0u64;
    let mut swap_total = 0u64;
    let mut swap_used = 0u64;

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if line.starts_with("Mem:") && parts.len() >= 7 {
            total = parts[1].parse().unwrap_or(0);
            used = parts[2].parse().unwrap_or(0);
            available = parts[6].parse().unwrap_or(0);
        } else if line.starts_with("Swap:") && parts.len() >= 3 {
            swap_total = parts[1].parse().unwrap_or(0);
            swap_used = parts[2].parse().unwrap_or(0);
        }
    }

    let to_gb = |b: u64| (b as f64) / (1024.0 * 1024.0 * 1024.0);
    let pct = if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 };
    let risk = if pct >= 95.0 { "critical" } else if pct >= 85.0 { "high" } else if pct >= 70.0 { "medium" } else { "low" };

    Ok(MemoryForecast {
        total_gb: to_gb(total),
        used_gb: to_gb(used),
        available_gb: to_gb(available),
        usage_percent: pct,
        swap_total_gb: to_gb(swap_total),
        swap_used_gb: to_gb(swap_used),
        risk_level: risk.into(),
    })
}

async fn collect_cpu_forecast() -> Result<CpuForecast, AppError> {
    let output = run_cmd("cat", &["/proc/loadavg"]).await?;
    let parts: Vec<&str> = output.split_whitespace().collect();
    let load_1m: f64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let load_5m: f64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let load_15m: f64 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);

    let cores = num_cpus();
    let per_core = if cores > 0 { load_1m / cores as f64 } else { load_1m };
    let risk = if per_core > 2.0 { "critical" } else if per_core > 1.0 { "high" } else if per_core > 0.7 { "medium" } else { "low" };

    Ok(CpuForecast {
        cores,
        load_1m,
        load_5m,
        load_15m,
        load_per_core: per_core,
        risk_level: risk.into(),
    })
}

async fn get_nix_store_size() -> f64 {
    match run_cmd_with_timeout("du", &["-sh", "/nix/store"], 30).await {
        Ok(output) => parse_size_to_gb(output.split_whitespace().next().unwrap_or("0")),
        Err(_) => 0.0,
    }
}

async fn estimate_gc_savings() -> Option<f64> {
    match run_cmd_with_timeout("nix-collect-garbage", &["--dry-run"], 30).await {
        Ok(output) => {
            // Look for "would free X MiB/GiB" or "deleting X generations"
            for line in output.lines() {
                if line.contains("would free") || line.contains("freed") {
                    return Some(parse_size_to_gb(line));
                }
            }
            None
        }
        Err(_) => None,
    }
}

fn parse_size_to_gb(s: &str) -> f64 {
    let s = s.trim();
    if let Some(num_str) = s.strip_suffix("GiB").or_else(|| s.strip_suffix("G")) {
        num_str.trim().parse().unwrap_or(0.0)
    } else if let Some(num_str) = s.strip_suffix("MiB").or_else(|| s.strip_suffix("M")) {
        num_str.trim().parse::<f64>().unwrap_or(0.0) / 1024.0
    } else if let Some(num_str) = s.strip_suffix("KiB").or_else(|| s.strip_suffix("K")) {
        num_str.trim().parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0)
    } else {
        s.parse().unwrap_or(0.0)
    }
}

fn num_cpus() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1)
}
