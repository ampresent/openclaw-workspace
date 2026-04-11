use axum::Json;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Deserialize)]
pub struct GenQuery {
    pub host: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Serialize)]
pub struct GenResponse {
    pub current: u64,
    pub generations: Vec<Generation>,
    pub diff: Option<GenerationDiff>,
}

#[derive(Serialize)]
pub struct Generation {
    pub number: u64,
    pub date: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct GenerationDiff {
    pub packages_added: Vec<String>,
    pub packages_removed: Vec<String>,
    pub services_changed: Vec<String>,
    pub config_diff: String,
}

pub async fn handle(
    State(_state): AppStateRef,
    Query(query): Query<GenQuery>,
) -> Result<Json<GenResponse>, AppError> {
    let output = run_cmd(
        "nixos-rebuild",
        &["list-generations", "--no-pager"],
    )
    .await?;

    let mut generations = Vec::new();
    let mut current = 0u64;

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            if let Ok(num) = parts[0].parse::<u64>() {
                let date = format!("{} {}", parts[1], parts[2]);
                // Try nix-evo description first, then parse from list-generations output
                let desc = read_generation_description(num);
                let desc = if desc.is_empty() {
                    if parts.len() > 3 {
                        parts[3..].join(" ")
                    } else {
                        String::new()
                    }
                } else {
                    desc
                };
                generations.push(Generation {
                    number: num,
                    date,
                    description: desc,
                });
                current = current.max(num);
            }
        }
    }

    let diff = compute_diff(&query, current).await;

    Ok(Json(GenResponse {
        current,
        generations,
        diff,
    }))
}

async fn compute_diff(query: &GenQuery, current: u64) -> Option<GenerationDiff> {
    let to_gen = query
        .to
        .as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(current);
    let from_gen = query
        .from
        .as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(if to_gen > 0 { to_gen - 1 } else { 0 });

    if from_gen == to_gen {
        return None;
    }

    let from_profile = format!("/nix/var/nix/profiles/system-{}-link", from_gen);
    let to_profile = format!("/nix/var/nix/profiles/system-{}-link", to_gen);

    // Verify profiles exist
    if !std::path::Path::new(&from_profile).exists() {
        tracing::warn!("Generation {} profile not found at {}", from_gen, from_profile);
        return None;
    }
    if !std::path::Path::new(&to_profile).exists() {
        tracing::warn!("Generation {} profile not found at {}", to_gen, to_profile);
        return None;
    }

    let from_pkgs = run_cmd("nix-store", &["-qR", &from_profile])
        .await
        .unwrap_or_default();
    let to_pkgs = run_cmd("nix-store", &["-qR", &to_profile])
        .await
        .unwrap_or_default();

    let from_set: std::collections::HashSet<&str> = from_pkgs.lines().collect();
    let to_set: std::collections::HashSet<&str> = to_pkgs.lines().collect();

    let mut packages_added: Vec<String> = to_set
        .difference(&from_set)
        .map(|s| s.to_string())
        .collect();
    packages_added.sort();

    let mut packages_removed: Vec<String> = from_set
        .difference(&to_set)
        .map(|s| s.to_string())
        .collect();
    packages_removed.sort();

    // Diff systemd units between generations
    let from_systemd = format!("{}/etc/systemd/system", from_profile);
    let to_systemd = format!("{}/etc/systemd/system", to_profile);

    let config_diff = if std::path::Path::new(&from_systemd).exists()
        && std::path::Path::new(&to_systemd).exists()
    {
        run_cmd("diff", &["-u", "-r", &from_systemd, &to_systemd])
            .await
            .unwrap_or_else(|_| "(无服务变更)".to_string())
    } else {
        "(无法对比 systemd 目录)".to_string()
    };

    // Parse changed services from diff output
    let services_changed: Vec<String> = config_diff
        .lines()
        .filter(|l| l.starts_with("diff") || l.starts_with("---") || l.starts_with("+++"))
        .filter(|l| l.contains(".service"))
        .map(|l| {
            l.split('/')
                .last()
                .unwrap_or(l)
                .to_string()
        })
        .collect();

    Some(GenerationDiff {
        packages_added,
        packages_removed,
        services_changed,
        config_diff,
    })
}
