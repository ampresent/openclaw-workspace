//! Config diff: show what would change before applying.

use axum::Json;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Deserialize)]
pub struct DiffRequest {
    pub config: String,
}

#[derive(Serialize)]
pub struct DiffResponse {
    pub has_changes: bool,
    pub diff: String,
    pub files_changed: Vec<FileChange>,
    pub summary: String,
}

#[derive(Serialize)]
pub struct FileChange {
    pub path: String,
    pub status: String, // "modified", "added", "removed"
    pub diff_lines: usize,
}

pub async fn handle(
    State(state): AppStateRef,
    Json(req): Json<DiffRequest>,
) -> Result<Json<DiffResponse>, AppError> {
    if req.config.trim().is_empty() {
        return Err(AppError::Validation {
            field: "config".into(),
            message: "配置内容不能为空".into(),
        });
    }

    let config_dir = &state.config.nixos_dir;
    let target = format!("{config_dir}/configuration.nix");

    // Read current config
    let current = tokio::fs::read_to_string(&target).await.unwrap_or_default();

    // Compute diff using unified diff algorithm
    let diff = compute_diff(&current, &req.config);
    let has_changes = !diff.is_empty();

    let diff_lines = diff.lines().count();
    let files_changed = if has_changes {
        vec![FileChange {
            path: target.clone(),
            status: "modified".to_string(),
            diff_lines,
        }]
    } else {
        vec![]
    };

    let summary = if has_changes {
        let additions = diff.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
        let deletions = diff.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
        format!("将修改 {target}: +{additions} -{deletions} 行")
    } else {
        "配置无变更".to_string()
    };

    Ok(Json(DiffResponse {
        has_changes,
        diff,
        files_changed,
        summary,
    }))
}

/// Simple unified diff between two strings.
/// Uses line-by-line comparison.
fn compute_diff(old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    if old_lines == new_lines {
        return String::new();
    }

    let mut result = Vec::new();
    result.push("--- a/configuration.nix".to_string());
    result.push("+++ b/configuration.nix".to_string());

    // Simple LCS-based diff
    let lcs = longest_common_subsequence(&old_lines, &new_lines);
    let mut oi = 0;
    let mut ni = 0;
    let mut li = 0;

    while oi < old_lines.len() || ni < new_lines.len() {
        if li < lcs.len() && oi < old_lines.len() && old_lines[oi] == lcs[li] {
            // Check if new_lines[ni] also matches
            if ni < new_lines.len() && new_lines[ni] == lcs[li] {
                result.push(format!(" {}", old_lines[oi]));
                oi += 1;
                ni += 1;
                li += 1;
            } else {
                // new has extra lines before the match
                result.push(format!("+{}", new_lines[ni]));
                ni += 1;
            }
        } else if li < lcs.len() && ni < new_lines.len() && new_lines[ni] == lcs[li] {
            // old has extra lines before the match
            result.push(format!("-{}", old_lines[oi]));
            oi += 1;
        } else if oi < old_lines.len() {
            result.push(format!("-{}", old_lines[oi]));
            oi += 1;
        } else if ni < new_lines.len() {
            result.push(format!("+{}", new_lines[ni]));
            ni += 1;
        }
    }

    result.join("\n")
}

fn longest_common_subsequence<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<&'a str> {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to find LCS
    let mut result = Vec::new();
    let mut i = m;
    let mut j = n;
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push(a[i - 1]);
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_diff_no_changes() {
        let d = compute_diff("hello\nworld", "hello\nworld");
        assert!(d.is_empty());
    }

    #[test]
    fn test_compute_diff_with_addition() {
        let d = compute_diff("hello", "hello\nworld");
        assert!(d.contains("+world"));
    }

    #[test]
    fn test_compute_diff_with_deletion() {
        let d = compute_diff("hello\nworld", "hello");
        assert!(d.contains("-world"));
    }

    #[test]
    fn test_lcs() {
        let a = vec!["a", "b", "c", "d"];
        let b = vec!["a", "c", "d", "e"];
        let lcs = longest_common_subsequence(&a, &b);
        assert_eq!(lcs, vec!["a", "c", "d"]);
    }
}
