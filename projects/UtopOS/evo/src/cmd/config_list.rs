//! List NixOS configuration files and their import structure.

use axum::Json;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Deserialize)]
pub struct ListRequest {
    pub dir: Option<String>,
}

#[derive(Serialize)]
pub struct ListResponse {
    pub config_dir: String,
    pub files: Vec<ConfigFile>,
    pub imports: Vec<ImportInfo>,
    pub total_files: usize,
}

#[derive(Serialize)]
pub struct ConfigFile {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub is_nix: bool,
    pub last_modified: Option<String>,
}

#[derive(Serialize)]
pub struct ImportInfo {
    pub source: String,
    pub imports: Vec<String>,
}

pub async fn handle(
    State(state): AppStateRef,
    Query(query): Query<ListRequest>,
) -> Result<Json<ListResponse>, AppError> {
    let config_dir = query.dir.unwrap_or_else(|| state.config.nixos_dir.clone());

    // Validate: must be under /etc/nixos
    if !config_dir.starts_with("/etc/nixos") {
        return Err(AppError::Validation {
            field: "dir".into(),
            message: "配置目录必须在 /etc/nixos 下".into(),
        });
    }

    let output = run_cmd("find", &[&config_dir, "-maxdepth", "2", "-type", "f"]).await?;

    let mut files = Vec::new();
    let mut imports = Vec::new();

    for line in output.lines() {
        let path = line.trim();
        if path.is_empty() { continue; }

        let name = path.split('/').last().unwrap_or(path).to_string();
        let is_nix = name.ends_with(".nix");

        // Get file size
        let size_bytes = tokio::fs::metadata(path).await
            .map(|m| m.len())
            .unwrap_or(0);

        // Get last modified time
        let last_modified = tokio::fs::metadata(path).await
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let duration = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                let secs = duration.as_secs();
                // Simple date formatting: YYYY-MM-DD HH:MM
                // Days since epoch
                let days = secs / 86400;
                let remaining_secs = secs % 86400;
                let hours = remaining_secs / 3600;
                let minutes = (remaining_secs % 3600) / 60;
                // Approximate year/month/day (good enough for display)
                let year = 1970 + (days / 365);
                let day_of_year = days % 365;
                let month = (day_of_year / 30) + 1;
                let day = (day_of_year % 30) + 1;
                format!("{year:04}-{month:02}-{day:02} {hours:02}:{minutes:02}")
            });

        // Parse imports from .nix files
        if is_nix {
            if let Ok(content) = tokio::fs::read_to_string(path).await {
                let file_imports = parse_imports(&content);
                if !file_imports.is_empty() {
                    imports.push(ImportInfo {
                        source: path.to_string(),
                        imports: file_imports,
                    });
                }
            }
        }

        files.push(ConfigFile {
            path: path.to_string(),
            name,
            size_bytes,
            is_nix,
            last_modified,
        });
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));
    let total_files = files.len();

    Ok(Json(ListResponse {
        config_dir,
        files,
        imports,
        total_files,
    }))
}

/// Extract import paths from Nix source code.
fn parse_imports(content: &str) -> Vec<String> {
    let mut imports = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Match: imports = [ ... ];
        // Also match: imports = [ ./hardware-configuration.nix ];
        if trimmed.starts_with("imports") && trimmed.contains('[') {
            // Find all paths in the brackets
            let in_brackets = if let Some(start) = trimmed.find('[') {
                if let Some(end) = trimmed.find(']') {
                    &trimmed[start + 1..end]
                } else {
                    &trimmed[start + 1..]
                }
            } else {
                continue;
            };

            for part in in_brackets.split_whitespace() {
                let part = part.trim().trim_end_matches(';');
                if part.starts_with("./") || part.starts_with("../") || part.starts_with('/') {
                    imports.push(part.to_string());
                }
            }
        }

        // Also match: import ./path.nix
        if trimmed.starts_with("import ") {
            let rest = trimmed.trim_start_matches("import ").trim();
            if rest.starts_with("./") || rest.starts_with("../") || rest.starts_with('/') {
                let path = rest.split_whitespace().next().unwrap_or(rest);
                imports.push(path.to_string());
            }
        }
    }

    imports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_imports_bracket() {
        let content = r#"
{ config, pkgs, ... }:
{
  imports = [
    ./hardware-configuration.nix
    ./modules/nginx.nix
    ../shared/base.nix
  ];
}
"#;
        let imports = parse_imports(content);
        assert_eq!(imports.len(), 3);
        assert!(imports.contains(&"./hardware-configuration.nix".to_string()));
        assert!(imports.contains(&"./modules/nginx.nix".to_string()));
    }

    #[test]
    fn test_parse_imports_single() {
        let content = r#"import ./hardware-configuration.nix"#;
        let imports = parse_imports(content);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0], "./hardware-configuration.nix");
    }

    #[test]
    fn test_parse_imports_none() {
        let content = r#"{ services.nginx.enable = true; }"#;
        let imports = parse_imports(content);
        assert!(imports.is_empty());
    }
}
