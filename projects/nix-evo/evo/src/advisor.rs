use axum::{response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::cmd::{run_cmd, read_generation_description};
use crate::error::AppError;

/// A rollback candidate with scoring
#[derive(Debug, Clone, Serialize)]
pub struct RollbackCandidate {
    pub generation: u64,
    pub score: f64,       // 0.0 - 1.0 (higher = better)
    pub uptime_hours: f64,
    pub service_health: f64,  // 0.0 - 1.0
    pub age_hours: f64,
    pub description: String,
    pub nixos_version: String,
    pub reasons: Vec<String>,
}

/// Recommendation result
#[derive(Debug, Serialize)]
pub struct Recommendation {
    pub recommended_generation: u64,
    pub confidence: f64,
    pub candidates: Vec<RollbackCandidate>,
    pub current_generation: u64,
    pub analysis_summary: String,
}

#[derive(Debug, Deserialize)]
pub struct RecommendRequest {
    /// How many recent generations to consider (default: 10)
    pub lookback: Option<usize>,
    /// Specific services to prioritize
    pub critical_services: Option<Vec<String>>,
    /// Force re-analysis even if recent
    pub force: Option<bool>,
}

/// Get current generation number
async fn get_current_generation() -> Result<u64, AppError> {
    let output = run_cmd("nix-env", &["-p", "/nix/var/nix/profiles/system", "--list-generations"]).await?;
    let current = output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.contains("(current)") {
                trimmed.split_whitespace().next()?.parse::<u64>().ok()
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0);
    Ok(current)
}

/// List recent generations
async fn list_recent_generations(limit: usize) -> Result<Vec<(u64, String, String)>, AppError> {
    let output = run_cmd("nix-env", &["-p", "/nix/var/nix/profiles/system", "--list-generations"]).await?;
    let mut gens: Vec<(u64, String, String)> = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        let parts: Vec<&str> = trimmed.splitn(3, |c: char| c.is_whitespace()).collect();
        if parts.len() >= 2 {
            if let Ok(num) = parts[0].parse::<u64>() {
                let date = parts[1].to_string();
                let desc = if parts.len() >= 3 {
                    parts[2].trim().to_string()
                } else {
                    read_generation_description(num)
                };
                gens.push((num, date, desc));
            }
        }
    }

    gens.sort_by_key(|g| g.0);
    let start = gens.len().saturating_sub(limit);
    Ok(gens[start..].to_vec())
}

/// Check service health for a generation by examining its closure
async fn check_service_health(generation: u64, critical_services: &[String]) -> f64 {
    if critical_services.is_empty() {
        return 1.0;
    }

    let mut healthy_count = 0;
    let total = critical_services.len() as f64;

    for svc in critical_services {
        // Check if the service is configured in this generation
        let gen_path = format!("/nix/var/nix/profiles/system-{generation}-link");
        let svc_check = format!("ls {gen_path}/etc/systemd/system/{svc}.service 2>/dev/null");
        if let Ok(o) = tokio::process::Command::new("sh").arg("-c").arg(&svc_check).output().await {
            if o.status.success() {
                healthy_count += 1;
            }
        }
    }

    healthy_count / total
}

/// Estimate uptime based on generation age
fn estimate_uptime(generation: u64, current_gen: u64, date_str: &str) -> f64 {
    let age_gens = current_gen.saturating_sub(generation);
    // Rough estimate: each gen lasted until the next one was created
    // If this gen is N gens ago, and we assume ~1 day per gen on average
    let estimated_hours = (age_gens as f64) * 24.0;
    // Older generations that survived longer are more stable
    estimated_hours.min(720.0) // Cap at 30 days
}

/// Calculate a composite score for a rollback candidate
fn calculate_score(candidate: &RollbackCandidate) -> f64 {
    // Weights
    const W_SERVICE_HEALTH: f64 = 0.45;
    const W_UPTIME: f64 = 0.25;
    const W_RECENCY: f64 = 0.20;
    const W_AGE: f64 = 0.10;

    // Normalize uptime (0-720h → 0-1)
    let uptime_score = (candidate.uptime_hours / 720.0).min(1.0);

    // Recency: newer is generally better (inverted age)
    let recency_score = 1.0 - (candidate.age_hours / 720.0).min(1.0);

    // Age penalty: very new generations might not be stable yet
    let age_score = if candidate.age_hours < 1.0 {
        0.3 // Too new, might not be stable
    } else if candidate.age_hours < 24.0 {
        0.7
    } else {
        1.0
    };

    candidate.service_health * W_SERVICE_HEALTH
        + uptime_score * W_UPTIME
        + recency_score * W_RECENCY
        + age_score * W_AGE
}

/// Generate recommendation
pub async fn recommend_rollback(req: &RecommendRequest) -> Result<Recommendation, AppError> {
    let lookback = req.lookback.unwrap_or(10);
    let critical_services = req.critical_services.clone().unwrap_or_else(|| vec![
        "nginx".into(), "sshd".into(), "postgresql".into(),
    ]);

    let current_gen = get_current_generation().await?;
    let generations = list_recent_generations(lookback).await?;

    let mut candidates = Vec::new();

    for (gen_num, date, desc) in &generations {
        if *gen_num >= current_gen {
            continue; // Skip current and future
        }

        let service_health = check_service_health(*gen_num, &critical_services).await;
        let uptime_hours = estimate_uptime(*gen_num, current_gen, date);
        let age_hours = (current_gen.saturating_sub(*gen_num)) as f64 * 24.0;

        let mut reasons = Vec::new();
        if service_health >= 0.8 {
            reasons.push("Most critical services present".into());
        }
        if uptime_hours > 48.0 {
            reasons.push("Stable uptime history".into());
        }
        if age_hours > 24.0 && age_hours < 168.0 {
            reasons.push("Well-tested (1-7 days old)".into());
        }

        let candidate = RollbackCandidate {
            generation: *gen_num,
            score: 0.0, // Calculated below
            uptime_hours,
            service_health,
            age_hours,
            description: desc.clone(),
            nixos_version: String::new(),
            reasons,
        };

        let mut c = candidate;
        c.score = calculate_score(&c);
        candidates.push(c);
    }

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    let best = candidates.first();
    let recommended = best.map(|c| c.generation).unwrap_or(current_gen.saturating_sub(1));
    let confidence = best.map(|c| c.score).unwrap_or(0.5);

    let summary = if let Some(best) = best {
        format!(
            "Recommended generation #{gen} (score: {score:.2}). Service health: {health:.0%}, \
             estimated uptime: {uptime:.0}h. {reasons}",
            gen = best.generation,
            score = best.score,
            health = best.service_health,
            uptime = best.uptime_hours,
            reasons = best.reasons.join("; "),
        )
    } else {
        "No suitable rollback candidates found".into()
    };

    Ok(Recommendation {
        recommended_generation: recommended,
        confidence,
        candidates,
        current_generation: current_gen,
        analysis_summary: summary,
    })
}

// ─── HTTP Handlers ─────────────────────────────────────────────────────

/// POST /api/advisor/recommend
pub async fn handle_recommend(
    Json(body): Json<serde_json::Value>,
) -> Result<Json<Recommendation>, AppError> {
    let req: RecommendRequest = serde_json::from_value(body)
        .map_err(|e| AppError::Validation {
            field: "body".into(),
            message: e.to_string(),
        })?;
    let rec = recommend_rollback(&req).await?;
    Ok(Json(rec))
}

/// GET /api/advisor/status — quick status without full analysis
pub async fn handle_status() -> Result<Json<serde_json::Value>, AppError> {
    let current = get_current_generation().await?;
    Ok(Json(serde_json::json!({
        "current_generation": current,
        "status": "ready",
    })))
}
