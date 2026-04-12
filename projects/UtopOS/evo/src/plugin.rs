/// Plugin System — Dynamic plugin loading via shared libraries (.so)
///
/// Plugin interface: init, handle_request, health_check
/// Discovery: scan ~/.nix-evo/plugins/
/// Each plugin is a .so file with exported C functions.

use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

use crate::error::AppError;

// ─── Plugin Interface (C ABI) ─────────────────────────────────────────────
//
// Each plugin .so must export these functions:
//
//   fn nix_evo_plugin_init() -> *const c_char  // returns plugin name
//   fn nix_evo_plugin_version() -> *const c_char
//   fn nix_evo_plugin_handle_request(method: *const c_char, path: *const c_char, body: *const c_char) -> *const c_char
//   fn nix_evo_plugin_health_check() -> *const c_char  // "ok" or error message
//   fn nix_evo_plugin_cleanup()

// ─── Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub path: String,
    pub status: PluginStatus,
    pub endpoints: Vec<PluginEndpoint>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginStatus {
    Loaded,
    Failed(String),
    Unloaded,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginEndpoint {
    pub method: String,
    pub path: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct PluginList {
    pub plugins: Vec<PluginInfo>,
    pub plugin_dir: String,
    pub total: usize,
    pub loaded: usize,
    pub failed: usize,
}

#[derive(Debug, Serialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub endpoints: Vec<PluginEndpoint>,
    pub min_api_version: String,
}

// ─── Loaded Plugin ────────────────────────────────────────────────────────

struct LoadedPlugin {
    name: String,
    version: String,
    path: String,
    // We store the library handle; function pointers would go here in a real impl
    healthy: bool,
    last_error: Option<String>,
}

// ─── Plugin Manager ───────────────────────────────────────────────────────

struct PluginManager {
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
    plugin_dir: PathBuf,
}

fn manager() -> &'static PluginManager {
    static MANAGER: OnceLock<PluginManager> = OnceLock::new();
    MANAGER.get_or_init(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        PluginManager {
            plugins: RwLock::new(HashMap::new()),
            plugin_dir: PathBuf::from(format!("{home}/.nix-evo/plugins")),
        }
    })
}

/// Discover and load all plugins from the plugin directory
pub async fn discover_plugins() -> Result<Vec<PluginInfo>, AppError> {
    let plugin_dir = &manager().plugin_dir;

    // Create plugin directory if it doesn't exist
    tokio::fs::create_dir_all(plugin_dir).await.ok();

    let mut entries = tokio::fs::read_dir(plugin_dir).await
        .map_err(|e| AppError::IoError {
            path: plugin_dir.display().to_string(),
            message: format!("Cannot read plugin directory: {e}"),
        })?;

    let mut discovered = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "so" || e == "dylib") {
            match load_plugin(&path).await {
                Ok(info) => {
                    tracing::info!("Loaded plugin: {} v{}", info.name, info.version);
                    discovered.push(info);
                }
                Err(e) => {
                    tracing::warn!("Failed to load plugin {}: {}", path.display(), e);
                    discovered.push(PluginInfo {
                        name: path.file_stem().unwrap().to_string_lossy().into(),
                        version: "unknown".into(),
                        path: path.display().to_string(),
                        status: PluginStatus::Failed(e.to_string()),
                        endpoints: Vec::new(),
                        last_error: Some(e.to_string()),
                    });
                }
            }
        }
    }

    // Also check for plugin manifests (.json alongside .so)
    let mut entries2 = tokio::fs::read_dir(plugin_dir).await.unwrap();
    while let Ok(Some(entry)) = entries2.next_entry().await {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") {
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                if let Ok(_manifest) = serde_json::from_str::<PluginManifest>(&content) {
                    tracing::info!("Found plugin manifest: {}", path.display());
                }
            }
        }
    }

    Ok(discovered)
}

async fn load_plugin(path: &std::path::Path) -> Result<PluginInfo, String> {
    // In a real implementation, we'd use libloading to dlopen the .so
    // For this prototype, we validate the .so and create a placeholder entry
    let metadata = tokio::fs::metadata(path).await
        .map_err(|e| format!("Cannot read plugin file: {e}"))?;

    if metadata.len() == 0 {
        return Err("Plugin file is empty".into());
    }

    let name = path.file_stem()
        .ok_or("No filename")?
        .to_string_lossy()
        .to_string();

    // In production, this would do:
    //   let lib = Library::new(path).map_err(|e| e.to_string())?;
    //   let init: Symbol<fn() -> *const c_char> = lib.get(b"nix_evo_plugin_init").map_err(|e| e.to_string())?;
    //   let name = unsafe { CStr::from_ptr(init()).to_string_lossy().into_owned() };

    Ok(PluginInfo {
        name: name.clone(),
        version: "0.1.0".into(), // would call nix_evo_plugin_version()
        path: path.display().to_string(),
        status: PluginStatus::Loaded,
        endpoints: vec![
            PluginEndpoint {
                method: "GET".into(),
                path: format!("/api/plugins/{name}/status"),
                description: "Plugin health check".into(),
            },
        ],
        last_error: None,
    })
}

/// Handle a request routed to a plugin
pub async fn handle_plugin_request(
    plugin_name: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<serde_json::Value, AppError> {
    let plugins = manager().plugins.read().await;

    if !plugins.contains_key(plugin_name) {
        return Err(AppError::NotFound {
            resource: format!("Plugin '{plugin_name}' not found"),
        });
    }

    // In production, this would call:
    //   let handler: Symbol<fn(*const c_char, *const c_char, *const c_char) -> *const c_char>
    //       = lib.get(b"nix_evo_plugin_handle_request").unwrap();
    //   let result = unsafe { CStr::from_ptr(handler(method_c.as_ptr(), path_c.as_ptr(), body_c.as_ptr())) };

    Ok(serde_json::json!({
        "plugin": plugin_name,
        "method": method,
        "path": path,
        "result": "Plugin handler invoked (simulated)",
    }))
}

/// Check health of all loaded plugins
pub async fn health_check_all() -> Vec<(String, bool, Option<String>)> {
    let plugins = manager().plugins.read().await;
    let mut results = Vec::new();

    for (name, plugin) in plugins.iter() {
        // In production, call nix_evo_plugin_health_check()
        results.push((name.clone(), plugin.healthy, plugin.last_error.clone()));
    }

    results
}

// ─── API Handlers ─────────────────────────────────────────────────────────

pub async fn handle_list() -> Result<impl IntoResponse, AppError> {
    let discovered = discover_plugins().await?;

    let loaded = discovered.iter().filter(|p| matches!(p.status, PluginStatus::Loaded)).count();
    let failed = discovered.iter().filter(|p| matches!(p.status, PluginStatus::Failed(_))).count();

    Ok(Json(PluginList {
        total: discovered.len(),
        loaded,
        failed,
        plugin_dir: manager().plugin_dir.display().to_string(),
        plugins: discovered,
    }))
}

pub async fn handle_health() -> impl IntoResponse {
    let checks = health_check_all().await;
    let results: Vec<_> = checks.into_iter().map(|(name, healthy, error)| {
        serde_json::json!({
            "name": name,
            "healthy": healthy,
            "error": error,
        })
    }).collect();

    Json(serde_json::json!({
        "plugins": results,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
