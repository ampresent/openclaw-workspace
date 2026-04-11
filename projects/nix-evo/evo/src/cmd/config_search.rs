//! Search NixOS configuration files for patterns.

use axum::Json;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Deserialize)]
pub struct SearchRequest {
    /// Search pattern (grep-compatible)
    pub pattern: String,
    /// Optional: limit to specific file or directory
    pub path: Option<String>,
    /// Case insensitive search
    pub case_insensitive: Option<bool>,
    /// Max results
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub pattern: String,
    pub total_matches: usize,
    pub results: Vec<SearchMatch>,
}

#[derive(Serialize)]
pub struct SearchMatch {
    pub file: String,
    pub line_number: usize,
    pub line: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

pub async fn handle(
    State(state): AppStateRef,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, AppError> {
    if req.pattern.trim().is_empty() {
        return Err(AppError::Validation {
            field: "pattern".into(),
            message: "搜索模式不能为空".into(),
        });
    }

    // Validate: prevent searching outside config dir
    let search_path = req.path.as_deref().unwrap_or(&state.config.nixos_dir);
    if !search_path.starts_with("/etc/nixos") {
        return Err(AppError::Validation {
            field: "path".into(),
            message: "搜索路径必须在 /etc/nixos 下".into(),
        });
    }

    let limit = req.limit.unwrap_or(50).min(200);

    // Build grep command
    let mut args = vec![
        "-n",         // line numbers
        "--with-filename", // show filename
        "-r",         // recursive
        "-C", "1",    // 1 line context
    ];

    if req.case_insensitive.unwrap_or(false) {
        args.push("-i");
    }

    args.push(&req.pattern);
    args.push(search_path);

    // Restrict to .nix files
    args.push("--include");
    args.push("*.nix");

    let output = run_cmd("grep", &args).await.unwrap_or_default();

    let results = parse_grep_output(&output, limit);
    let total_matches = results.len();

    Ok(Json(SearchResponse {
        pattern: req.pattern,
        total_matches,
        results,
    }))
}

/// Parse grep -C output into structured results.
fn parse_grep_output(output: &str, limit: usize) -> Vec<SearchMatch> {
    let mut results = Vec::new();
    let mut current_file = String::new();
    let mut current_matches: Vec<(usize, String)> = Vec::new();
    let mut context_before: Vec<String> = Vec::new();
    let mut context_after: Vec<String> = Vec::new();
    let mut in_context_after = false;

    for line in output.lines() {
        if line.starts_with("--") {
            // Separator between match groups
            if let Some((line_num, content)) = current_matches.last() {
                results.push(SearchMatch {
                    file: current_file.clone(),
                    line_number: *line_num,
                    line: content.clone(),
                    context_before: std::mem::take(&mut context_before),
                    context_after: std::mem::take(&mut context_after),
                });
                if results.len() >= limit { break; }
            }
            current_matches.clear();
            context_before.clear();
            context_after.clear();
            in_context_after = false;
            continue;
        }

        // Parse "file:linenum:content" or "file-linenum-content" (context)
        if let Some(colon_pos) = line.find(':') {
            let file = &line[..colon_pos];
            let rest = &line[colon_pos + 1..];

            if let Some(colon2_pos) = rest.find(':') {
                // Match line (file:line:content)
                let line_num_str = &rest[..colon2_pos];
                let content = &rest[colon2_pos + 1..];

                if let Ok(num) = line_num_str.parse::<usize>() {
                    current_file = file.to_string();
                    if !current_matches.is_empty() {
                        // Push previous match
                        if let Some((prev_num, prev_content)) = current_matches.last() {
                            results.push(SearchMatch {
                                file: current_file.clone(),
                                line_number: *prev_num,
                                line: prev_content.clone(),
                                context_before: std::mem::take(&mut context_before),
                                context_after: std::mem::take(&mut context_after),
                            });
                            if results.len() >= limit { break; }
                        }
                        context_before.clear();
                        context_after.clear();
                    }
                    current_matches = vec![(num, content.to_string())];
                    in_context_after = true;
                } else {
                    // Context line before match
                    if !in_context_after {
                        context_before.push(rest[colon2_pos + 1..].to_string());
                    } else {
                        context_after.push(rest[colon2_pos + 1..].to_string());
                    }
                }
            }
        }
    }

    // Push last match
    if let Some((line_num, content)) = current_matches.last() {
        results.push(SearchMatch {
            file: current_file,
            line_number: *line_num,
            line: content.clone(),
            context_before,
            context_after,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_grep_output() {
        let output = "/etc/nixos/configuration.nix:15:  services.nginx.enable = true;\n/etc/nixos/configuration.nix-14-  networking.firewall.enable = true;\n/etc/nixos/configuration.nix-16-  \n--\n/etc/nixos/nginx.nix:3:services.nginx.virtualHosts.\"example.com\" = {\n/etc/nixos/nginx.nix-2-{\n/etc/nixos/nginx.nix-4-  enableACME = true;\n";
        let results = parse_grep_output(output, 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].line_number, 15);
        assert!(results[0].line.contains("nginx.enable"));
        assert_eq!(results[1].line_number, 3);
    }
}
