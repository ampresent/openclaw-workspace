use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::cmd::AppStateRef;
use crate::error::AppError;

/// A known issue pattern with its diagnosis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisEntry {
    pub id: String,
    pub patterns: Vec<String>,
    pub title: String,
    pub severity: String,
    pub category: String,
    pub description: String,
    pub solution: String,
    pub commands: Vec<String>,
    pub docs_url: Option<String>,
}

/// Request body for POST /api/doctor/diagnose
#[derive(Debug, Deserialize)]
pub struct DiagnoseRequest {
    pub error_message: String,
    pub context: Option<String>,
}

/// Single match result
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosisMatch {
    pub id: String,
    pub title: String,
    pub severity: String,
    pub category: String,
    pub description: String,
    pub solution: String,
    pub commands: Vec<String>,
    pub docs_url: Option<String>,
    pub confidence: f64,
    pub matched_patterns: Vec<String>,
}

/// Response for POST /api/doctor/diagnose
#[derive(Debug, Serialize)]
pub struct DiagnoseResponse {
    pub input: String,
    pub matches: Vec<DiagnosisMatch>,
    pub suggestions: Vec<String>,
    pub total_kb_entries: usize,
}

/// Built-in knowledge base of common NixOS issues
fn knowledge_base() -> &'static Vec<DiagnosisEntry> {
    static KB: OnceLock<Vec<DiagnosisEntry>> = OnceLock::new();
    KB.get_or_init(|| vec![
        DiagnosisEntry {
            id: "eval-error-attribute-missing".into(),
            patterns: vec![
                "attribute .* missing".into(),
                "error: attribute .* not found".into(),
                "The option .* is not defined".into(),
            ],
            title: "Missing attribute / option".into(),
            severity: "high".into(),
            category: "evaluation".into(),
            description: "A NixOS option or attribute is referenced but not defined. This usually means a typo in the option name, or a module is not imported.".into(),
            solution: "Check for typos in option names. Ensure the relevant module is imported in your configuration.".into(),
            commands: vec![
                "nixos-option <option-name>".into(),
                "grep -r 'option-name' /etc/nixos/".into(),
            ],
            docs_url: Some("https://search.nixos.org/options".into()),
        },
        DiagnosisEntry {
            id: "collision-between".into(),
            patterns: vec![
                "collision between".into(),
                "packages .* and .* have the same priority".into(),
            ],
            title: "Package collision".into(),
            severity: "medium".into(),
            category: "packages".into(),
            description: "Two packages provide the same file and have equal priority. NixOS cannot decide which to use.".into(),
            solution: "Use `lib.hiPrio` or `lib.lowPrio` to set relative priority, or use `environment.etc` to provide the file directly.".into(),
            commands: vec![
                "nix-env -q".into(),
            ],
            docs_url: None,
        },
        DiagnosisEntry {
            id: "out-of-disk".into(),
            patterns: vec![
                "No space left on device".into(),
                "cannot build .* disk full".into(),
                "error: writing to file: No space left".into(),
            ],
            title: "Disk space exhausted".into(),
            severity: "critical".into(),
            category: "system".into(),
            description: "The disk (likely /nix/store or /tmp) has run out of space. Nix builds require significant temporary space.".into(),
            solution: "Run garbage collection, delete old generations, or resize the disk.".into(),
            commands: vec![
                "nix-collect-garbage -d".into(),
                "nix-collect-garbage --delete-older-than 30d".into(),
                "sudo nix-store --optimise".into(),
                "df -h".into(),
            ],
            docs_url: Some("https://nixos.org/manual/nixos/stable/#sec-nix-gc".into()),
        },
        DiagnosisEntry {
            id: "hash-mismatch".into(),
            patterns: vec![
                "hash mismatch".into(),
                "fixed-output derivation .* has mismatching hash".into(),
                "got:.*expected:".into(),
            ],
            title: "Hash mismatch in fetch".into(),
            severity: "high".into(),
            category: "build".into(),
            description: "A fetched source (fetchurl, fetchFromGitHub, etc.) returned content that doesn't match the expected hash. The upstream source may have changed.".into(),
            solution: "Update the hash in the derivation to match the new content, or investigate if the source was tampered with.".into(),
            commands: vec![
                "nix-prefetch-url <url>".into(),
                "nix store prefetch-file --print-hashes <url>".into(),
            ],
            docs_url: None,
        },
        DiagnosisEntry {
            id: "infinite-recursion".into(),
            patterns: vec![
                "infinite recursion encountered".into(),
                "error: infinite recursion".into(),
            ],
            title: "Infinite recursion in Nix expression".into(),
            severity: "high".into(),
            category: "evaluation".into(),
            description: "A Nix expression references itself infinitely. Common causes: circular module imports, self-referencing let-bindings.".into(),
            solution: "Break the circular dependency. Use `lib.mkDefault` or restructure the config to avoid self-reference.".into(),
            commands: vec![
                "nix-instantiate --parse <file>".into(),
            ],
            docs_url: None,
        },
        DiagnosisEntry {
            id: "sandbox-failure".into(),
            patterns: vec![
                "sandboxing.*not supported".into(),
                "error: while setting up the build environment.*sandbox".into(),
                "cannot build in sandbox".into(),
            ],
            title: "Sandbox build failure".into(),
            severity: "medium".into(),
            category: "build".into(),
            description: "The Nix sandbox build is failing, possibly due to kernel restrictions or missing namespace support.".into(),
            solution: "Try disabling sandbox temporarily or ensure kernel supports user namespaces.".into(),
            commands: vec![
                "nix build --option sandbox false <derivation>".into(),
                "sysctl kernel.unprivileged_userns_clone=1".into(),
            ],
            docs_url: None,
        },
        DiagnosisEntry {
            id: "channel-conflict".into(),
            patterns: vec![
                "conflicting channel".into(),
                "error: file .* was not found in the Nix search path".into(),
                "<nixpkgs>".into(),
            ],
            title: "Channel or search path issue".into(),
            severity: "medium".into(),
            category: "configuration".into(),
            description: "Nix channels are not properly set up, or <nixpkgs> doesn't point to the expected location. This is common after switching to flakes.".into(),
            solution: "Update channels or use flake references instead. Run `nix-channel --list` to check current channels.".into(),
            commands: vec![
                "nix-channel --list".into(),
                "nix-channel --update".into(),
                "nix registry list | grep nixpkgs".into(),
            ],
            docs_url: Some("https://nixos.org/manual/nixos/stable/#sec-upgrading".into()),
        },
        DiagnosisEntry {
            id: "service-failed".into(),
            patterns: vec![
                "service.*failed".into(),
                "unit.*entered failed state".into(),
                "Main process exited.*code=exited.*status".into(),
            ],
            title: "Systemd service failure".into(),
            severity: "high".into(),
            category: "services".into(),
            description: "A systemd service has entered the failed state. Check service logs for the root cause.".into(),
            solution: "Inspect service logs and configuration. Common causes: missing config files, permission issues, port conflicts.".into(),
            commands: vec![
                "systemctl status <service>".into(),
                "journalctl -u <service> --no-pager -n 50".into(),
                "systemctl restart <service>".into(),
            ],
            docs_url: None,
        },
        DiagnosisEntry {
            id: "permission-denied".into(),
            patterns: vec![
                "Permission denied".into(),
                "error: opening file.*Permission denied".into(),
            ],
            title: "Permission denied".into(),
            severity: "high".into(),
            category: "system".into(),
            description: "A file or operation was denied due to insufficient permissions. Common for Nix store operations and service configs.".into(),
            solution: "Run with sudo if needed, or fix file ownership. Nix store files should be root-owned.".into(),
            commands: vec![
                "sudo nixos-rebuild switch".into(),
                "ls -la <path>".into(),
                "sudo chown root:nixbld <path>".into(),
            ],
            docs_url: None,
        },
        DiagnosisEntry {
            id: "broken-symlink".into(),
            patterns: vec![
                "broken symlink".into(),
                "symlink.*No such file or directory".into(),
            ],
            title: "Broken symlink in Nix store".into(),
            severity: "medium".into(),
            category: "system".into(),
            description: "A symlink in the Nix store or profile points to a path that no longer exists, likely due to garbage collection.".into(),
            solution: "Rebuild the system to regenerate symlinks, or delete the broken symlink manually.".into(),
            commands: vec![
                "sudo nixos-rebuild switch".into(),
                "find /nix/var/nix/profiles -xtype l -delete".into(),
            ],
            docs_url: None,
        },
        DiagnosisEntry {
            id: "nix-daemon-down".into(),
            patterns: vec![
                "nix daemon.*not running".into(),
                "error: cannot connect to daemon".into(),
                "could not connect to socket.*nix-daemon".into(),
            ],
            title: "Nix daemon not running".into(),
            severity: "critical".into(),
            category: "system".into(),
            description: "The Nix daemon (nix-daemon.service) is not running. All Nix operations require the daemon.".into(),
            solution: "Start the Nix daemon service.".into(),
            commands: vec![
                "sudo systemctl start nix-daemon".into(),
                "sudo systemctl enable nix-daemon".into(),
                "sudo systemctl status nix-daemon".into(),
            ],
            docs_url: None,
        },
        DiagnosisEntry {
            id: "gc-corruption".into(),
            patterns: vec![
                "store.*corrupt".into(),
                "hash.*does not match".into(),
                "integrity error".into(),
            ],
            title: "Nix store corruption".into(),
            severity: "critical".into(),
            category: "system".into(),
            description: "The Nix store has integrity errors. This can happen due to disk issues or interrupted builds.".into(),
            solution: "Verify and repair the Nix store. May require rebuilding affected paths.".into(),
            commands: vec![
                "nix-store --verify --check-contents".into(),
                "nix-store --repair-path <path>".into(),
            ],
            docs_url: None,
        },
    ])
}

/// Match an error message against the knowledge base
fn match_error(error_msg: &str) -> Vec<DiagnosisMatch> {
    let kb = knowledge_base();
    let error_lower = error_msg.to_lowercase();
    let mut matches = Vec::new();

    for entry in kb {
        let mut matched_patterns = Vec::new();
        let mut total_score = 0.0;

        for pattern in &entry.patterns {
            let pattern_lower = pattern.to_lowercase();
            // Simple substring + regex-like matching
            let matched = if pattern_lower.contains(".*") {
                // Treat as a simple glob: split on .* and check all parts exist
                let parts: Vec<&str> = pattern_lower.split(".*").collect();
                parts.iter().all(|p| error_lower.contains(p.trim()))
            } else {
                error_lower.contains(&pattern_lower)
            };

            if matched {
                matched_patterns.push(pattern.clone());
                total_score += 1.0;
            }
        }

        if !matched_patterns.is_empty() {
            let confidence = (total_score / entry.patterns.len() as f64).min(1.0);
            // Boost confidence for more matches
            let confidence = (confidence * 100.0).round() / 100.0;

            matches.push(DiagnosisMatch {
                id: entry.id.clone(),
                title: entry.title.clone(),
                severity: entry.severity.clone(),
                category: entry.category.clone(),
                description: entry.description.clone(),
                solution: entry.solution.clone(),
                commands: entry.commands.clone(),
                docs_url: entry.docs_url.clone(),
                confidence,
                matched_patterns,
            });
        }
    }

    // Sort by confidence descending
    matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    matches
}

/// Generate generic suggestions when no matches found
fn generate_suggestions(error_msg: &str) -> Vec<String> {
    let mut suggestions = Vec::new();
    let lower = error_msg.to_lowercase();

    if lower.contains("error") {
        suggestions.push("Run with --show-trace to get more context: nixos-rebuild switch --show-trace".into());
    }
    if lower.contains("build") || lower.contains("derivation") {
        suggestions.push("Try building with verbose output: nix build -v".into());
    }
    if lower.contains("service") || lower.contains("unit") {
        suggestions.push("Check service logs: journalctl -xe".into());
    }
    if suggestions.is_empty() {
        suggestions.push("Search the NixOS Discourse or Matrix for similar errors".into());
        suggestions.push("Check https://nixos.org/manual/nixos/stable/ for documentation".into());
    }
    suggestions
}

/// POST /api/doctor/diagnose — diagnose a NixOS error message
pub async fn handle_diagnose(
    State(_state): AppStateRef,
    Json(req): Json<DiagnoseRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.error_message.trim().is_empty() {
        return Err(AppError::Validation {
            field: "error_message".into(),
            message: "Error message cannot be empty".into(),
        });
    }

    let mut matches = match_error(&req.error_message);

    // Also match against context if provided
    if let Some(ctx) = &req.context {
        let ctx_matches = match_error(ctx);
        for cm in ctx_matches {
            if !matches.iter().any(|m| m.id == cm.id) {
                matches.push(cm);
            }
        }
    }

    let suggestions = if matches.is_empty() {
        generate_suggestions(&req.error_message)
    } else {
        vec![
            "If none of the diagnoses match, try searching the error message online.".into(),
            "Consider adding new patterns to the knowledge base via plugins.".into(),
        ]
    };

    let total_kb = knowledge_base().len();

    Ok(Json(serde_json::to_value(DiagnoseResponse {
        input: req.error_message,
        matches,
        suggestions,
        total_kb_entries: total_kb,
    }).unwrap_or_default()))
}

/// GET /api/doctor/knowledge — list all knowledge base entries
pub async fn handle_knowledge(
    State(_state): AppStateRef,
) -> Result<impl IntoResponse, AppError> {
    let kb = knowledge_base();
    let entries: Vec<serde_json::Value> = kb.iter().map(|e| {
        serde_json::json!({
            "id": e.id,
            "title": e.title,
            "severity": e.severity,
            "category": e.category,
            "patterns_count": e.patterns.len(),
        })
    }).collect();

    Ok(Json(serde_json::json!({
        "total": entries.len(),
        "entries": entries,
    })))
}
