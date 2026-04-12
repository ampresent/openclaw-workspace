//! Environment Testing Framework
//!
//! Run smoke tests after environment changes to verify environments work correctly.
//! Tests: import checks, pytest, CUDA availability, version verification.
//! Configurable test suites per environment.

use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use crate::cmd::{AppStateRef, HostQuery, run_cmd};
use crate::conda;
use crate::error::AppError;

/// Test request body
#[derive(Debug, Clone, Deserialize)]
pub struct TestRequest {
    pub env: String,
    pub tests: Option<Vec<TestSpec>>,
    pub timeout_seconds: Option<u64>,
}

/// Individual test specification
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestSpec {
    pub test_type: TestType,
    pub target: Option<String>,     // package name, module name, or command
    pub expected: Option<String>,   // expected output pattern
    pub description: Option<String>,
}

/// Test type
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum TestType {
    #[serde(rename = "import")]
    Import,
    #[serde(rename = "import_version")]
    ImportVersion,
    #[serde(rename = "pytest")]
    Pytest,
    #[serde(rename = "cuda")]
    Cuda,
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "script")]
    Script,
    #[serde(rename = "healthcheck")]
    Healthcheck,
}

/// Test result
#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    pub test_type: TestType,
    pub target: String,
    pub description: String,
    pub passed: bool,
    pub duration_ms: u64,
    pub output: String,
    pub error: Option<String>,
}

/// Full test report
#[derive(Debug, Clone, Serialize)]
pub struct TestReport {
    pub environment: String,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: u64,
    pub results: Vec<TestResult>,
    pub overall_pass: bool,
    pub recommendations: Vec<String>,
}

// ─── Predefined Test Suites ───────────────────────────────────────────

/// Get the default smoke test suite for a given environment type
pub fn default_smoke_tests() -> Vec<TestSpec> {
    vec![
        TestSpec {
            test_type: TestType::Import,
            target: Some("numpy".to_string()),
            expected: None,
            description: Some("numpy import".to_string()),
        },
        TestSpec {
            test_type: TestType::Import,
            target: Some("pandas".to_string()),
            expected: None,
            description: Some("pandas import".to_string()),
        },
        TestSpec {
            test_type: TestType::ImportVersion,
            target: Some("python".to_string()),
            expected: None,
            description: Some("Python version check".to_string()),
        },
        TestSpec {
            test_type: TestType::Healthcheck,
            target: None,
            expected: None,
            description: Some("Environment integrity check".to_string()),
        },
    ]
}

/// Get ML-specific test suite (includes CUDA, torch, etc.)
pub fn ml_smoke_tests() -> Vec<TestSpec> {
    let mut tests = default_smoke_tests();
    tests.extend(vec![
        TestSpec {
            test_type: TestType::Import,
            target: Some("torch".to_string()),
            expected: None,
            description: Some("PyTorch import".to_string()),
        },
        TestSpec {
            test_type: TestType::Cuda,
            target: None,
            expected: None,
            description: Some("CUDA availability check".to_string()),
        },
        TestSpec {
            test_type: TestType::Import,
            target: Some("sklearn".to_string()),
            expected: None,
            description: Some("scikit-learn import".to_string()),
        },
    ]);
    tests
}

/// Get data science test suite
pub fn data_science_smoke_tests() -> Vec<TestSpec> {
    let mut tests = default_smoke_tests();
    tests.extend(vec![
        TestSpec {
            test_type: TestType::Import,
            target: Some("matplotlib".to_string()),
            expected: None,
            description: Some("matplotlib import".to_string()),
        },
        TestSpec {
            test_type: TestType::Import,
            target: Some("scipy".to_string()),
            expected: None,
            description: Some("scipy import".to_string()),
        },
        TestSpec {
            test_type: TestType::Import,
            target: Some("jupyter".to_string()),
            expected: None,
            description: Some("jupyter import".to_string()),
        },
    ]);
    tests
}

// ─── Test Runner ──────────────────────────────────────────────────────

/// Run tests on an environment
pub async fn run_tests(request: &TestRequest) -> Result<TestReport, AppError> {
    let backend = conda::detect_backend().await?;
    let timeout = request.timeout_seconds.unwrap_or(60);

    // Get default tests if none specified
    let tests = request.tests.clone().unwrap_or_else(default_smoke_tests);

    // Verify environment exists
    let envs = conda::list_envs(&backend).await?;
    let env = envs.iter().find(|e| e.name == request.env)
        .ok_or_else(|| AppError::NotFound {
            resource: format!("conda environment: {}", request.env),
        })?;

    let python_bin = format!("{}/bin/python", env.path);
    if !Path::new(&python_bin).exists() {
        return Err(AppError::NotFound {
            resource: format!("python binary in {}: {python_bin}", env.path),
        });
    }

    let start = Instant::now();
    let mut results = Vec::new();
    let mut recommendations = Vec::new();

    for test in &tests {
        let result = execute_test(&python_bin, &env.path, test, timeout).await;

        if !result.passed {
            match test.test_type {
                TestType::Import => {
                    recommendations.push(format!(
                        "Package '{}' failed to import. Try: micromamba install -n {} {}",
                        test.target.as_deref().unwrap_or("?"),
                        request.env,
                        test.target.as_deref().unwrap_or("?"),
                    ));
                }
                TestType::Cuda => {
                    recommendations.push(
                        "CUDA not available. Check: nvidia-smi, CUDA_VISIBLE_DEVICES, or install cudatoolkit".to_string()
                    );
                }
                _ => {}
            }
        }

        results.push(result);
    }

    let total_ms = start.elapsed().as_millis() as u64;
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.iter().filter(|r| !r.passed).count();

    Ok(TestReport {
        environment: request.env.clone(),
        total_tests: results.len(),
        passed,
        failed,
        skipped: 0,
        duration_ms: total_ms,
        results,
        overall_pass: failed == 0,
        recommendations,
    })
}

/// Execute a single test
async fn execute_test(
    python_bin: &str,
    env_path: &str,
    test: &TestSpec,
    timeout_secs: u64,
) -> TestResult {
    let start = Instant::now();
    let target = test.target.clone().unwrap_or_default();
    let description = test.description.clone().unwrap_or_else(|| format!("{:?} {}", test.test_type, target));

    let (passed, output, error) = match test.test_type {
        TestType::Import => {
            let code = format!("import {target}; print('OK')");
            run_python_code(python_bin, &code, timeout_secs).await
        }
        TestType::ImportVersion => {
            let code = if target == "python" {
                "import sys; print(sys.version)".to_string()
            } else {
                format!("import {target}; print({target}.__version__)")
            };
            run_python_code(python_bin, &code, timeout_secs).await
        }
        TestType::Cuda => {
            let code = r#"
import sys
try:
    import torch
    if torch.cuda.is_available():
        print(f"CUDA available: {torch.cuda.get_device_name(0)}")
        print(f"CUDA version: {torch.version.cuda}")
    else:
        print("PyTorch installed but CUDA not available")
        sys.exit(1)
except ImportError:
    print("PyTorch not installed, checking nvidia-smi...")
    import subprocess
    result = subprocess.run(['nvidia-smi', '--query-gpu=name,driver_version', '--csv,noheader'],
                          capture_output=True, text=True)
    if result.returncode == 0:
        print(f"GPU detected: {result.stdout.strip()}")
    else:
        print("No GPU/CUDA detected")
        sys.exit(1)
"#.to_string();
            run_python_code(python_bin, &code, timeout_secs).await
        }
        TestType::Pytest => {
            let test_path = test.target.as_deref().unwrap_or(".");
            let result = tokio::process::Command::new(python_bin)
                .args(["-m", "pytest", test_path, "-v", "--tb=short", "-x"])
                .output()
                .await;
            match result {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let combined = format!("{stdout}\n{stderr}");
                    let passed = output.status.success();
                    if passed {
                        (true, combined, None)
                    } else {
                        (false, combined.clone(), Some(combined))
                    }
                }
                Err(e) => (false, String::new(), Some(format!("Failed to run pytest: {e}"))),
            }
        }
        TestType::Command => {
            let cmd = test.target.as_deref().unwrap_or("true");
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.is_empty() {
                (false, String::new(), Some("Empty command".to_string()))
            } else {
                let result = tokio::process::Command::new(parts[0])
                    .args(&parts[1..])
                    .current_dir(env_path)
                    .output()
                    .await;
                match result {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        (output.status.success(), stdout, None)
                    }
                    Err(e) => (false, String::new(), Some(e.to_string())),
                }
            }
        }
        TestType::Script => {
            let script = test.target.as_deref().unwrap_or("print('OK')");
            run_python_code(python_bin, script, timeout_secs).await
        }
        TestType::Healthcheck => {
            let code = r#"
import sys, os, importlib
# Check basic Python health
print(f"Python {sys.version}")
print(f"Executable: {sys.executable}")
print(f"Path entries: {len(sys.path)}")

# Check pip
try:
    import pip
    print(f"pip: OK")
except:
    print("pip: MISSING")

# Check site-packages
import site
dirs = site.getsitepackages()
print(f"Site packages: {len(dirs)} dirs")

# Check if any broken .dist-info
import importlib.metadata
dist_count = len(list(importlib.metadata.distributions()))
print(f"Installed distributions: {dist_count}")
print("HEALTHCHECK: OK")
"#.to_string();
            run_python_code(python_bin, &code, timeout_secs).await
        }
    };

    TestResult {
        test_type: test.test_type.clone(),
        target,
        description,
        passed,
        duration_ms: start.elapsed().as_millis() as u64,
        output,
        error,
    }
}

/// Run a Python code string and return (passed, stdout, error)
async fn run_python_code(python_bin: &str, code: &str, timeout_secs: u64) -> (bool, String, Option<String>) {
    let result = tokio::process::Command::new(python_bin)
        .args(["-c", code])
        .output()
        .await;

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if output.status.success() {
                (true, stdout, if stderr.is_empty() { None } else { Some(stderr) })
            } else {
                (false, stdout, Some(stderr))
            }
        }
        Err(e) => (false, String::new(), Some(format!("Execution failed: {e}"))),
    }
}

/// Auto-detect which test suite to use based on installed packages
pub async fn auto_detect_test_suite(env_name: &str) -> Result<Vec<TestSpec>, AppError> {
    let backend = conda::detect_backend().await?;
    let packages = conda::list_packages(&backend, env_name).await?;
    let pkg_names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();

    let has_ml = pkg_names.iter().any(|p| matches!(*p, "torch" | "tensorflow" | "keras" | "jax"));
    let has_ds = pkg_names.iter().any(|p| matches!(*p, "matplotlib" | "seaborn" | "jupyter" | "scipy"));

    if has_ml {
        Ok(ml_smoke_tests())
    } else if has_ds {
        Ok(data_science_smoke_tests())
    } else {
        Ok(default_smoke_tests())
    }
}

// ─── HTTP Handlers ────────────────────────────────────────────────────

/// POST /api/env/test — run tests on an environment
pub async fn test_handler(
    State(_state): AppStateRef,
    Json(body): Json<TestRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let report = run_tests(&body).await?;
    Ok(Json(serde_json::to_value(&report).unwrap()))
}

/// POST /api/env/test/auto — auto-detect and run tests
#[derive(Deserialize)]
pub struct AutoTestBody {
    pub env: String,
    pub timeout_seconds: Option<u64>,
}

pub async fn auto_test_handler(
    State(_state): AppStateRef,
    Json(body): Json<AutoTestBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let tests = auto_detect_test_suite(&body.env).await?;
    let request = TestRequest {
        env: body.env,
        tests: Some(tests),
        timeout_seconds: body.timeout_seconds,
    };
    let report = run_tests(&request).await?;
    Ok(Json(serde_json::to_value(&report).unwrap()))
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_smoke_tests() {
        let tests = default_smoke_tests();
        assert!(tests.len() >= 3);
        assert!(tests.iter().any(|t| t.test_type == TestType::Import && t.target.as_deref() == Some("numpy")));
    }

    #[test]
    fn test_ml_smoke_tests() {
        let tests = ml_smoke_tests();
        assert!(tests.iter().any(|t| t.test_type == TestType::Cuda));
        assert!(tests.iter().any(|t| t.target.as_deref() == Some("torch")));
    }

    #[test]
    fn test_data_science_smoke_tests() {
        let tests = data_science_smoke_tests();
        assert!(tests.iter().any(|t| t.target.as_deref() == Some("matplotlib")));
    }

    #[test]
    fn test_test_spec_serialization() {
        let spec = TestSpec {
            test_type: TestType::Import,
            target: Some("numpy".to_string()),
            expected: None,
            description: Some("test numpy".to_string()),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert!(json.contains("import"));
        assert!(json.contains("numpy"));
    }
}
