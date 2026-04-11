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
) -> Result<Json<GenResponse>, String> {
    let output = run_cmd(
        "nixos-rebuild",
        &["list-generations", "--no-pager"],
    )
    .await
    .map_err(|e| format!("nixos-rebuild list-generations failed: {e}"))?;

    let mut generations = Vec::new();
    let mut current = 0u64;

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            if let Ok(num) = parts[0].parse::<u64>() {
                let date = format!("{} {}", parts[1], parts[2]);
                let desc = if parts.len() > 3 {
                    parts[3..].join(" ")
                } else {
                    String::new()
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

    // Compute diff if from/to specified or between last two
    let diff = compute_diff(&query, current, &generations).await;

    Ok(Json(GenResponse {
        current,
        generations,
        diff,
    }))
}

async fn compute_diff(
    query: &GenQuery,
    current: u64,
    gens: &[Generation],
) -> Option<GenerationDiff> {
    let to_gen = query
        .to
        .as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(current);
    let from_gen = query
        .from
        .as_ref()
        .and_then(|s| s.parse::<u64>().ok()
        )
        .unwrap_or(if to_gen > 0 { to_gen - 1 } else { 0 });

    if from_gen == to_gen {
        return None;
    }

    let from_profile = format!("/nix/var/nix/profiles/system-{}-link", from_gen);
    let to_profile = format!("/nix/var/nix/profiles/system-{}-link", to_gen);

    let from_pkgs = run_cmd("nix-store", &["-qR", &from_profile])
        .await
        .unwrap_or_default();
    let to_pkgs = run_cmd("nix-store", &["-qR", &to_profile])
        .await
        .unwrap_or_default();

    let from_set: std::collections::HashSet<&str> = from_pkgs.lines().collect();
    let to_set: std::collections::HashSet<&str> = to_pkgs.lines().collect();

    let packages_added: Vec<String> = to_set
        .difference(&from_set)
        .map(|s| s.to_string())
        .collect();
    let packages_removed: Vec<String> = from_set
        .difference(&to_set)
        .map(|s| s.to_string())
        .collect();

    // Diff system profile closures (services)
    let config_diff = run_cmd(
        "diff",
        &[
            "-u",
            &format!("{}/etc/systemd/system", from_profile),
            &format!("{}/etc/systemd/system", to_profile),
        ],
    )
    .await
    .unwrap_or_else(|_| String::from("(no service changes)"));

    Some(GenerationDiff {
        packages_added,
        packages_removed,
        services_changed: vec![], // TODO: parse from config_diff
        config_diff,
    })
}
