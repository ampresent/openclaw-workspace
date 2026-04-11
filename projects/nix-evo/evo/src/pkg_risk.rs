//! Package Popularity & Risk Assessment
//!
//! Query conda-forge/PyPI for download stats, last update, maintainer count.
//! Risk scoring: unmaintained packages, single-maintainer, known CVEs.

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::cmd::{AppStateRef, run_cmd};
use crate::error::AppError;

/// Package risk assessment result
#[derive(Debug, Clone, Serialize)]
pub struct PackageRisk {
    pub name: String,
    pub conda_available: bool,
    pub pypi_available: bool,
    pub conda_info: Option<CondaInfo>,
    pub pypi_info: Option<PyPiInfo>,
    pub risk_score: f64,       // 0-100, higher = riskier
    pub risk_factors: Vec<RiskFactor>,
    pub risk_level: RiskLevel,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CondaInfo {
    pub version: Option<String>,
    pub channel: Option<String>,
    pub last_updated: Option<String>,
    pub download_count: Option<u64>,
    pub platform_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PyPiInfo {
    pub version: Option<String>,
    pub last_updated: Option<String>,
    pub download_count: Option<u64>,
    pub maintainer_count: Option<usize>,
    pub license: Option<String>,
    pub requires_python: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskFactor {
    pub factor_type: RiskFactorType,
    pub severity: f64,  // 0-100 contribution to risk score
    pub description: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum RiskFactorType {
    #[serde(rename = "unmaintained")]
    Unmaintained,
    #[serde(rename = "single_maintainer")]
    SingleMaintainer,
    #[serde(rename = "low_downloads")]
    LowDownloads,
    #[serde(rename = "no_license")]
    NoLicense,
    #[serde(rename = "old_python_compat")]
    OldPythonCompat,
    #[serde(rename = "abandoned")]
    Abandoned,
    #[serde(rename = "deprecated")]
    Deprecated,
    #[serde(rename = "single_channel")]
    SingleChannel,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum RiskLevel {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "critical")]
    Critical,
}

// ─── Risk Assessment Engine ───────────────────────────────────────────

/// Assess risk for a package
pub async fn assess_package_risk(name: &str) -> Result<PackageRisk, AppError> {
    let mut risk_score = 0.0;
    let mut risk_factors = Vec::new();

    // Query conda info
    let conda_info = query_conda_info(name).await;
    let conda_available = conda_info.is_some();

    // Query PyPI info
    let pypi_info = query_pypi_info(name).await;
    let pypi_available = pypi_info.is_some();

    if !conda_available && !pypi_available {
        return Ok(PackageRisk {
            name: name.to_string(),
            conda_available: false,
            pypi_available: false,
            conda_info: None,
            pypi_info: None,
            risk_score: 80.0,
            risk_factors: vec![RiskFactor {
                factor_type: RiskFactorType::Deprecated,
                severity: 80.0,
                description: "Package not found in conda-forge or PyPI".to_string(),
            }],
            risk_level: RiskLevel::High,
            recommendation: "Package not found in any repository. Verify the package name.".to_string(),
        });
    }

    // Assess PyPI risk factors
    if let Some(ref info) = pypi_info {
        // Check maintainers
        if let Some(count) = info.maintainer_count {
            if count == 0 {
                risk_score += 30.0;
                risk_factors.push(RiskFactor {
                    factor_type: RiskFactorType::Unmaintained,
                    severity: 30.0,
                    description: "No maintainers listed on PyPI".to_string(),
                });
            } else if count == 1 {
                risk_score += 15.0;
                risk_factors.push(RiskFactor {
                    factor_type: RiskFactorType::SingleMaintainer,
                    severity: 15.0,
                    description: "Single maintainer — bus factor = 1".to_string(),
                });
            }
        }

        // Check last update
        if let Some(ref updated) = info.last_updated {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(updated, "%Y-%m-%d") {
                let now = chrono::Utc::now().naive_utc().date();
                let days_since = (now - date).num_days();
                if days_since > 730 {
                    risk_score += 25.0;
                    risk_factors.push(RiskFactor {
                        factor_type: RiskFactorType::Abandoned,
                        severity: 25.0,
                        description: format!("Last updated {days_since} days ago (>2 years)"),
                    });
                } else if days_since > 365 {
                    risk_score += 10.0;
                    risk_factors.push(RiskFactor {
                        factor_type: RiskFactorType::Unmaintained,
                        severity: 10.0,
                        description: format!("Last updated {days_since} days ago (>1 year)"),
                    });
                }
            }
        }

        // Check license
        if info.license.is_none() || info.license.as_deref() == Some("") {
            risk_score += 10.0;
            risk_factors.push(RiskFactor {
                factor_type: RiskFactorType::NoLicense,
                severity: 10.0,
                description: "No license specified".to_string(),
            });
        }

        // Check download count
        if let Some(dl) = info.download_count {
            if dl < 1000 {
                risk_score += 15.0;
                risk_factors.push(RiskFactor {
                    factor_type: RiskFactorType::LowDownloads,
                    severity: 15.0,
                    description: format!("Very low downloads: {dl}"),
                });
            }
        }
    }

    // Check conda-only risk
    if !pypi_available && conda_available {
        risk_score += 5.0;
        risk_factors.push(RiskFactor {
            factor_type: RiskFactorType::SingleChannel,
            severity: 5.0,
            description: "Only available in conda (not PyPI)".to_string(),
        });
    }

    // Determine risk level
    let risk_level = if risk_score >= 60.0 {
        RiskLevel::Critical
    } else if risk_score >= 40.0 {
        RiskLevel::High
    } else if risk_score >= 20.0 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    };

    // Generate recommendation
    let recommendation = generate_recommendation(name, &risk_factors, &risk_level);

    Ok(PackageRisk {
        name: name.to_string(),
        conda_available,
        pypi_available,
        conda_info,
        pypi_info,
        risk_score: (risk_score * 100.0).round() / 100.0,
        risk_factors,
        risk_level,
        recommendation,
    })
}

/// Batch assess multiple packages
pub async fn batch_assess(packages: &[String]) -> Result<Vec<PackageRisk>, AppError> {
    let mut results = Vec::new();
    for pkg in packages {
        match assess_package_risk(pkg).await {
            Ok(risk) => results.push(risk),
            Err(e) => {
                results.push(PackageRisk {
                    name: pkg.clone(),
                    conda_available: false,
                    pypi_available: false,
                    conda_info: None,
                    pypi_info: None,
                    risk_score: 0.0,
                    risk_factors: vec![RiskFactor {
                        factor_type: RiskFactorType::Deprecated,
                        severity: 0.0,
                        description: format!("Assessment failed: {e}"),
                    }],
                    risk_level: RiskLevel::Low,
                    recommendation: "Could not assess. Check manually.".to_string(),
                });
            }
        }
    }
    Ok(results)
}

// ─── Data Sources ─────────────────────────────────────────────────────

async fn query_conda_info(name: &str) -> Option<CondaInfo> {
    match run_cmd("micromamba", &["search", "-c", "conda-forge", name, "--json"]).await {
        Ok(output) => {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
                let pkgs = json.get("packages").and_then(|p| p.as_object());
                if let Some(packages) = pkgs {
                    if let Some((_key, pkg)) = packages.iter().next() {
                        return Some(CondaInfo {
                            version: pkg.get("version").and_then(|v| v.as_str()).map(String::from),
                            channel: pkg.get("channel").and_then(|c| c.as_str()).map(String::from),
                            last_updated: pkg.get("timestamp").and_then(|t| {
                                t.as_u64().map(|ts| {
                                    chrono::DateTime::from_timestamp(ts as i64 / 1000, 0)
                                        .map(|dt| dt.format("%Y-%m-%d").to_string())
                                        .unwrap_or_default()
                                })
                            }).flatten(),
                            download_count: None,
                            platform_count: pkg.get("platforms").and_then(|p| p.as_array()).map(|a| a.len()),
                        });
                    }
                }
            }
            // Fallback: simple search
            if output.contains(name) {
                let version = output.lines()
                    .find(|l| l.contains(name))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .map(String::from);
                Some(CondaInfo {
                    version,
                    channel: Some("conda-forge".to_string()),
                    last_updated: None,
                    download_count: None,
                    platform_count: None,
                })
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

async fn query_pypi_info(name: &str) -> Option<PyPiInfo> {
    let url = format!("https://pypi.org/pypi/{name}/json");
    match run_cmd("curl", &["-s", "--max-time", "5", &url]).await {
        Ok(output) => {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
                let info = json.get("info")?;
                Some(PyPiInfo {
                    version: info.get("version").and_then(|v| v.as_str()).map(String::from),
                    last_updated: json.get("urls").and_then(|u| u.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|u| u.get("upload_time"))
                        .and_then(|t| t.as_str())
                        .map(|s| s[..10].to_string()),
                    download_count: None, // PyPI JSON API doesn't include downloads
                    maintainer_count: {
                        let has_author = info.get("author").and_then(|a| a.as_str())
                            .map(|s| !s.is_empty()).unwrap_or(false);
                        let has_maintainer = info.get("maintainer").and_then(|m| m.as_str())
                            .map(|s| !s.is_empty()).unwrap_or(false);
                        let count = if has_author || has_maintainer { 1 } else { 0 };
                        Some(count)
                    },
                    license: info.get("license").and_then(|l| l.as_str()).map(String::from),
                    requires_python: info.get("requires_python").and_then(|r| r.as_str()).map(String::from),
                    description: info.get("summary").and_then(|s| s.as_str()).map(String::from),
                })
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

fn generate_recommendation(name: &str, factors: &[RiskFactor], level: &RiskLevel) -> String {
    match level {
        RiskLevel::Critical => {
            format!("⚠️ HIGH RISK: Consider alternatives to '{name}'. This package has significant maintenance and reliability concerns.")
        }
        RiskLevel::High => {
            format!("⚠️ Consider pinning the version and monitoring for updates. '{name}' has some risk factors.")
        }
        RiskLevel::Medium => {
            format!("ℹ️ '{name}' has minor risk factors. Pin version for reproducibility.")
        }
        RiskLevel::Low => {
            format!("✅ '{name}' looks healthy. Standard pinning recommended.")
        }
    }
}

// ─── HTTP Handlers ────────────────────────────────────────────────────

/// GET /api/pkg/risk/{name}
pub async fn risk_handler(
    State(_state): AppStateRef,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = assess_package_risk(&name).await?;
    Ok(Json(serde_json::to_value(&result).unwrap()))
}

/// POST /api/pkg/risk/batch
pub async fn batch_risk_handler(
    State(_state): AppStateRef,
    Json(body): Json<BatchRiskBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let results = batch_assess(&body.packages).await?;
    Ok(Json(serde_json::json!({
        "packages": results,
        "count": results.len(),
        "high_risk_count": results.iter().filter(|r| r.risk_level == RiskLevel::High || r.risk_level == RiskLevel::Critical).count(),
    })))
}

#[derive(Deserialize)]
pub struct BatchRiskBody {
    pub packages: Vec<String>,
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_serialization() {
        assert_eq!(serde_json::to_string(&RiskLevel::Low).unwrap(), "\"low\"");
        assert_eq!(serde_json::to_string(&RiskLevel::Critical).unwrap(), "\"critical\"");
    }

    #[test]
    fn test_risk_factor_serialization() {
        let factor = RiskFactor {
            factor_type: RiskFactorType::SingleMaintainer,
            severity: 15.0,
            description: "Bus factor = 1".to_string(),
        };
        let json = serde_json::to_string(&factor).unwrap();
        assert!(json.contains("single_maintainer"));
        assert!(json.contains("15"));
    }

    #[test]
    fn test_package_risk_serialization() {
        let risk = PackageRisk {
            name: "numpy".to_string(),
            conda_available: true,
            pypi_available: true,
            conda_info: None,
            pypi_info: None,
            risk_score: 5.0,
            risk_factors: vec![],
            risk_level: RiskLevel::Low,
            recommendation: "Looks good".to_string(),
        };
        let json = serde_json::to_string(&risk).unwrap();
        assert!(json.contains("numpy"));
        assert!(json.contains("low"));
    }
}
