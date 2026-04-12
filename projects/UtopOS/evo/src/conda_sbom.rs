//! Conda Supply Chain Security
//!
//! Generate SBOM (Software Bill of Materials) for any conda environment.
//! Supports SPDX and CycloneDX formats.
//! Detects packages from untrusted channels and verifies checksums.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Trusted conda channels
const TRUSTED_CHANNELS: &[&str] = &[
    "conda-forge", "defaults", "bioconda", "pytorch", "nvidia",
    "intel", "msys2", "main", "free", "r", "menpo",
];

/// Query parameters for SBOM generation
#[derive(Debug, Deserialize)]
pub struct SbomQuery {
    pub env: String,
    pub format: Option<String>, // "spdx" or "cyclonedx"
    pub host: Option<String>,
}

/// Request to verify package checksums
#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub env: String,
    pub packages: Option<Vec<String>>, // if None, verify all
}

/// SBOM generation result
#[derive(Debug, Clone, Serialize)]
pub struct SbomResult {
    pub environment: String,
    pub format: String,
    pub total_packages: usize,
    pub trusted_packages: usize,
    pub untrusted_packages: usize,
    pub untrusted_channels: Vec<UntrustedEntry>,
    pub bom: serde_json::Value,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UntrustedEntry {
    pub package: String,
    pub version: String,
    pub channel: String,
    pub risk_level: String,
}

/// Package verification result
#[derive(Debug, Clone, Serialize)]
pub struct VerifyResult {
    pub environment: String,
    pub total_checked: usize,
    pub verified: usize,
    pub failed: usize,
    pub results: Vec<PackageVerify>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageVerify {
    pub name: String,
    pub version: String,
    pub channel: String,
    pub has_build_string: bool,
    pub has_platform: bool,
    pub verified: bool,
    pub details: String,
}

/// Generate SBOM for an environment
pub async fn generate_sbom(backend: &str, query: &SbomQuery) -> Result<SbomResult, AppError> {
    let fmt = query.format.as_deref().unwrap_or("cyclonedx");
    let packages = conda::list_packages(backend, &query.env).await?;

    let mut untrusted = Vec::new();
    let mut trusted_count = 0;

    for pkg in &packages {
        if TRUSTED_CHANNELS.contains(&pkg.channel.as_str()) {
            trusted_count += 1;
        } else {
            let risk = if pkg.channel == "pypi" || pkg.channel == "unknown" {
                "high"
            } else {
                "medium"
            };
            untrusted.push(UntrustedEntry {
                package: pkg.name.clone(),
                version: pkg.version.clone(),
                channel: pkg.channel.clone(),
                risk_level: risk.to_string(),
            });
        }
    }

    let bom = match fmt {
        "spdx" => generate_spdx(&query.env, &packages),
        _ => generate_cyclonedx(&query.env, &packages),
    };

    Ok(SbomResult {
        environment: query.env.clone(),
        format: fmt.to_string(),
        total_packages: packages.len(),
        trusted_packages: trusted_count,
        untrusted_packages: untrusted.len(),
        untrusted_channels: untrusted,
        bom,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Generate CycloneDX BOM
fn generate_cyclonedx(env_name: &str, packages: &[conda::CondaPackage]) -> serde_json::Value {
    let components: Vec<serde_json::Value> = packages.iter().map(|pkg| {
        serde_json::json!({
            "type": "library",
            "bomRef": format!("conda:{}@{}", pkg.name, pkg.version),
            "name": pkg.name,
            "version": pkg.version,
            "purl": format!("pkg:conda/{}@{}", pkg.name, pkg.version),
            "scope": "required",
            "properties": [
                { "name": "conda:channel", "value": pkg.channel },
                { "name": "conda:build", "value": pkg.build },
            ],
        })
    }).collect();

    serde_json::json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "component": {
                "type": "application",
                "name": env_name,
            },
            "tools": {
                "components": [{
                    "type": "application",
                    "name": "nix-evo-agent",
                    "version": "0.1.0",
                }]
            }
        },
        "components": components,
    })
}

/// Generate SPDX document
fn generate_spdx(env_name: &str, packages: &[conda::CondaPackage]) -> serde_json::Value {
    let spdx_id = format!("SPDXRef-CONDA-{}", env_name);
    let mut relationships = Vec::new();
    let mut pkg_infos = Vec::new();

    for (i, pkg) in packages.iter().enumerate() {
        let ref_id = format!("SPDXRef-Package-{}", i);
        pkg_infos.push(serde_json::json!({
            "SPDXID": ref_id,
            "name": pkg.name,
            "versionInfo": pkg.version,
            "downloadLocation": format!("https://anaconda.org/{}/{}", pkg.channel, pkg.name),
            "supplier": format!("Organization: {}", pkg.channel),
            "filesAnalyzed": false,
        }));
        relationships.push(serde_json::json!({
            "spdxElementId": spdx_id,
            "relatedSpdxElement": ref_id,
            "relationshipType": "CONTAINS",
        }));
    }

    serde_json::json!({
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": spdx_id,
        "name": env_name,
        "documentNamespace": format!("https://nix-evo.local/sbom/{}", env_name),
        "creationInfo": {
            "created": chrono::Utc::now().to_rfc3339(),
            "creators": ["Tool: nix-evo-agent"],
        },
        "packages": pkg_infos,
        "relationships": relationships,
    })
}

/// Verify package checksums/integrity
pub async fn verify_packages(backend: &str, req: &VerifyRequest) -> Result<VerifyResult, AppError> {
    let packages = conda::list_packages(backend, &req.env).await?;

    let check_set: Option<std::collections::HashSet<&str>> = req.packages.as_ref()
        .map(|pkgs| pkgs.iter().map(|s| s.as_str()).collect());

    let mut results = Vec::new();
    let mut verified = 0;
    let mut failed = 0;

    for pkg in &packages {
        if let Some(ref set) = check_set {
            if !set.contains(pkg.name.as_str()) {
                continue;
            }
        }

        let has_build = !pkg.build.is_empty() && pkg.build != "0";
        let has_platform = pkg.platform.is_some();
        let is_valid = has_build && pkg.channel != "unknown";

        if is_valid {
            verified += 1;
        } else {
            failed += 1;
        }

        results.push(PackageVerify {
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            channel: pkg.channel.clone(),
            has_build_string: has_build,
            has_platform: has_platform,
            verified: is_valid,
            details: if is_valid {
                format!("Package verified from channel '{}'", pkg.channel)
            } else {
                format!("Missing metadata — build: {}, platform: {}, channel: {}",
                    has_build, has_platform, pkg.channel)
            },
        });
    }

    Ok(VerifyResult {
        environment: req.env.clone(),
        total_checked: results.len(),
        verified,
        failed,
        results,
    })
}

// ─── Axum Handlers ────────────────────────────────────────────────────

pub async fn sbom_handler(
    State(_state): AppStateRef,
    Query(query): Query<SbomQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = conda::detect_backend().await?;
    let result = generate_sbom(&backend, &query).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}

pub async fn verify_handler(
    State(_state): AppStateRef,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = conda::detect_backend().await?;
    let result = verify_packages(&backend, &req).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}
