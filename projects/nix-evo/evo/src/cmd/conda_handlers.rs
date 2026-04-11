//! HTTP handlers for conda environment management

use axum::Json;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Deserialize)]
pub struct CondaEnvQuery {
    pub host: Option<String>,
    pub env: Option<String>,
}

#[derive(Deserialize)]
pub struct CondaInstallBody {
    pub env: String,
    pub packages: Vec<String>,
}

#[derive(Deserialize)]
pub struct CondaCreateBody {
    pub name: String,
    pub python_version: Option<String>,
    pub packages: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct CondaExportQuery {
    pub host: Option<String>,
    pub env: String,
    pub explicit: Option<bool>,
}

#[derive(Deserialize)]
pub struct CondaFromYmlBody {
    pub path: String,
}

// GET /api/conda/envs
pub async fn list_envs_handler(
    State(_state): AppStateRef,
    Query(query): Query<HostQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = crate::conda::detect_backend().await?;
    let envs = crate::conda::list_envs(&backend).await?;

    // Enrich each env with package count
    let mut enriched = Vec::new();
    for env in &envs {
        let pkgs = crate::conda::list_packages(&backend, &env.name).await.unwrap_or_default();
        let mut env_json = serde_json::to_value(env).unwrap();
        env_json["package_count"] = serde_json::json!(pkgs.len());
        enriched.push(env_json);
    }

    Ok(Json(serde_json::json!({
        "backend": backend,
        "environments": enriched
    })))
}

// GET /api/conda/packages?env=<name>
pub async fn list_packages_handler(
    State(_state): AppStateRef,
    Query(query): Query<CondaEnvQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let env_name = query.env.ok_or_else(|| AppError::Validation {
        field: "env".to_string(),
        message: "env parameter is required".to_string(),
    })?;
    let backend = crate::conda::detect_backend().await?;
    let packages = crate::conda::list_packages(&backend, &env_name).await?;
    Ok(Json(serde_json::json!({
        "environment": env_name,
        "packages": packages,
        "count": packages.len()
    })))
}

// POST /api/conda/create
pub async fn create_env_handler(
    State(_state): AppStateRef,
    Json(body): Json<CondaCreateBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = crate::conda::detect_backend().await?;
    let pkg_refs: Option<Vec<&str>> = body.packages.as_ref().map(|v| v.iter().map(|s| s.as_str()).collect());
    let result = crate::conda::create_env(
        &backend,
        &body.name,
        body.python_version.as_deref(),
        pkg_refs.as_deref(),
    ).await?;
    Ok(Json(serde_json::to_value(result).unwrap()))
}

// POST /api/conda/install
pub async fn install_handler(
    State(_state): AppStateRef,
    Json(body): Json<CondaInstallBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = crate::conda::detect_backend().await?;
    let pkg_refs: Vec<&str> = body.packages.iter().map(|s| s.as_str()).collect();
    let result = crate::conda::install_packages(&backend, &body.env, &pkg_refs).await?;
    Ok(Json(serde_json::to_value(result).unwrap()))
}

// POST /api/conda/remove
pub async fn remove_handler(
    State(_state): AppStateRef,
    Json(body): Json<CondaInstallBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = crate::conda::detect_backend().await?;
    let pkg_refs: Vec<&str> = body.packages.iter().map(|s| s.as_str()).collect();
    let result = crate::conda::remove_packages(&backend, &body.env, &pkg_refs).await?;
    Ok(Json(serde_json::to_value(result).unwrap()))
}

// GET /api/conda/export?env=<name>
pub async fn export_handler(
    State(_state): AppStateRef,
    Query(query): Query<CondaExportQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = crate::conda::detect_backend().await?;
    let explicit = query.explicit.unwrap_or(false);
    let yml = if explicit {
        crate::conda::export_env_explicit(&backend, &query.env).await?
    } else {
        crate::conda::export_env(&backend, &query.env).await?
    };
    Ok(Json(serde_json::json!({
        "environment": query.env,
        "format": if explicit { "explicit" } else { "environment_yml" },
        "content": yml
    })))
}

// POST /api/conda/create-from-yml
pub async fn create_from_yml_handler(
    State(_state): AppStateRef,
    Json(body): Json<CondaFromYmlBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = crate::conda::detect_backend().await?;
    let result = crate::conda::create_from_yml(&backend, &body.path).await?;
    Ok(Json(serde_json::to_value(result).unwrap()))
}

// DELETE /api/conda/envs/<name>
pub async fn remove_env_handler(
    State(_state): AppStateRef,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = crate::conda::detect_backend().await?;
    let result = crate::conda::remove_env(&backend, &name).await?;
    Ok(Json(serde_json::json!({
        "removed": result,
        "environment": name
    })))
}
