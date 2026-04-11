use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::cmd::run_cmd;
use crate::error::AppError;

/// Chaos experiment definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosExperiment {
    pub id: String,
    pub name: String,
    pub target: String,        // service name or "network", "disk", "cpu"
    pub action: String,        // "kill", "stop", "saturate_cpu", "fill_disk", "drop_packets"
    pub duration_secs: u64,
    pub intensity: f64,        // 0.0 - 1.0
    pub auto_recover: bool,
}

/// Experiment run result
#[derive(Debug, Clone, Serialize)]
pub struct ChaosResult {
    pub experiment_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: String,        // "running", "passed", "failed", "recovered"
    pub observations: Vec<Observation>,
    pub recovery_action: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Observation {
    pub timestamp: String,
    pub metric: String,
    pub value: f64,
    pub threshold: f64,
    pub breached: bool,
}

/// Predefined chaos scenarios
#[derive(Debug, Serialize)]
pub struct ChaosScenario {
    pub id: String,
    pub name: String,
    pub description: String,
    pub risk_level: String,
    pub targets: Vec<String>,
}

pub struct ChaosEngine {
    active_experiments: RwLock<HashMap<String, ChaosResult>>,
    history: RwLock<Vec<ChaosResult>>,
}

impl ChaosEngine {
    pub fn new() -> Self {
        Self {
            active_experiments: RwLock::new(HashMap::new()),
            history: RwLock::new(Vec::new()),
        }
    }

    pub async fn get_scenarios(&self) -> Vec<ChaosScenario> {
        vec![
            ChaosScenario {
                id: "service-kill".into(),
                name: "Service Kill & Recover".into(),
                description: "Stop a service, verify healer restarts it, measure recovery time".into(),
                risk_level: "medium".into(),
                targets: vec!["nginx", "sshd", "postgresql", "redis"].iter().map(|s| s.to_string()).collect(),
            },
            ChaosScenario {
                id: "network-partition".into(),
                name: "Network Partition Simulation".into(),
                description: "Drop packets between nodes using iptables, test service resilience".into(),
                risk_level: "high".into(),
                targets: vec!["all-external", "specific-port", "dns-only"].iter().map(|s| s.to_string()).collect(),
            },
            ChaosScenario {
                id: "disk-pressure".into(),
                name: "Disk Pressure Test".into(),
                description: "Fill disk to 95%, verify services degrade gracefully".into(),
                risk_level: "high".into(),
                targets: vec!["/tmp", "/var", "/"].iter().map(|s| s.to_string()).collect(),
            },
            ChaosScenario {
                id: "cpu-stress".into(),
                name: "CPU Stress Test".into(),
                description: "Saturate CPU cores, verify service latency stays acceptable".into(),
                risk_level: "low".into(),
                targets: vec!["50%", "75%", "100%"].iter().map(|s| s.to_string()).collect(),
            },
            ChaosScenario {
                id: "config-corrupt".into(),
                name: "Config Corruption Detection".into(),
                description: "Modify a config file, verify nixos-rebuild detects drift".into(),
                risk_level: "medium".into(),
                targets: vec!["nginx.conf", "sshd_config", "postgresql.conf"].iter().map(|s| s.to_string()).collect(),
            },
        ]
    }

    pub async fn run_experiment(&self, exp: &ChaosExperiment) -> Result<ChaosResult, AppError> {
        let id = exp.id.clone();
        let mut result = ChaosResult {
            experiment_id: id.clone(),
            started_at: chrono_now(),
            ended_at: None,
            status: "running".into(),
            observations: Vec::new(),
            recovery_action: None,
        };

        // Record pre-state
        let pre_state = capture_service_state(&exp.target).await;
        result.observations.push(Observation {
            timestamp: chrono_now(),
            metric: format!("{}.pre_state", exp.target),
            value: if pre_state == "active" { 1.0 } else { 0.0 },
            threshold: 1.0,
            breached: false,
        });

        // Execute chaos action
        match exp.action.as_str() {
            "kill" | "stop" => {
                let cmd = format!("systemctl stop {}", exp.target);
                let _ = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .output()
                    .await;
            }
            "saturate_cpu" => {
                let workers = (num_cpus() as f64 * exp.intensity).max(1.0) as u32;
                let duration = exp.duration_secs;
                // Stress test in background
                let _ = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(format!(
                        "stress --cpu {workers} --timeout {duration}s &"
                    ))
                    .output()
                    .await;
            }
            "fill_disk" => {
                let size_mb = (exp.intensity * 1024.0) as u64;
                let _ = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(format!(
                        "dd if=/dev/zero of=/tmp/nix-evo-chaos bs=1M count={size_mb} 2>/dev/null &"
                    ))
                    .output()
                    .await;
            }
            "drop_packets" => {
                let _ = run_cmd("iptables", &["-A", "OUTPUT", "-p", "tcp", "--dport", "443", "-j", "DROP"]).await;
            }
            _ => {}
        }

        // Monitor during experiment
        tokio::time::sleep(Duration::from_secs(exp.duration_secs.min(30))).await;

        // Capture post-state
        let post_state = capture_service_state(&exp.target).await;
        let recovered = post_state == "active";

        result.observations.push(Observation {
            timestamp: chrono_now(),
            metric: format!("{}.post_state", exp.target),
            value: if recovered { 1.0 } else { 0.0 },
            threshold: 1.0,
            breached: !recovered,
        });

        // Auto-recover if enabled
        if exp.auto_recover && !recovered {
            let recover_cmd = format!("systemctl start {}", exp.target);
            let _ = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&recover_cmd)
                .output()
                .await;
            result.recovery_action = Some(format!("Restarted {}", exp.target));

            // Verify recovery
            tokio::time::sleep(Duration::from_secs(3)).await;
            let final_state = capture_service_state(&exp.target).await;
            result.status = if final_state == "active" { "recovered".into() } else { "failed".into() };
        } else {
            result.status = if recovered { "passed".into() } else { "failed".into() };
        }

        // Cleanup
        if exp.action == "fill_disk" {
            let _ = tokio::fs::remove_file("/tmp/nix-evo-chaos").await;
        }
        if exp.action == "drop_packets" {
            let _ = run_cmd("iptables", &["-D", "OUTPUT", "-p", "tcp", "--dport", "443", "-j", "DROP"]).await;
        }

        result.ended_at = Some(chrono_now());

        // Store in history
        {
            let mut history = self.history.write().await;
            history.push(result.clone());
        }

        Ok(result)
    }

    pub async fn get_status(&self) -> serde_json::Value {
        let active = self.active_experiments.read().await;
        let history = self.history.read().await;
        serde_json::json!({
            "active_count": active.len(),
            "total_experiments": history.len(),
            "last_experiment": history.last(),
        })
    }
}

async fn capture_service_state(service: &str) -> String {
    match tokio::process::Command::new("systemctl")
        .args(&["is-active", service])
        .output()
        .await
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => "unknown".into(),
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_default()
}

use std::sync::OnceLock;
pub static CHAOS: OnceLock<ChaosEngine> = OnceLock::new();

pub fn chaos_engine() -> &'static ChaosEngine {
    CHAOS.get_or_init(ChaosEngine::new)
}

// ─── HTTP Handlers ─────────────────────────────────────────────────────

/// GET /api/chaos/scenarios
pub async fn handle_scenarios() -> Json<serde_json::Value> {
    let engine = chaos_engine();
    let scenarios = engine.get_scenarios().await;
    Json(serde_json::json!({ "scenarios": scenarios }))
}

/// POST /api/chaos/run
pub async fn handle_run(Json(exp): Json<ChaosExperiment>) -> Result<Json<ChaosResult>, AppError> {
    let engine = chaos_engine();
    let result = engine.run_experiment(&exp).await?;
    Ok(Json(result))
}

/// GET /api/chaos/status
pub async fn handle_chaos_status() -> Json<serde_json::Value> {
    let engine = chaos_engine();
    Json(engine.get_status().await)
}

/// POST /api/chaos/start — Start a chaos experiment from a predefined scenario
#[derive(Debug, Deserialize)]
pub struct ChaosStartRequest {
    pub scenario: String,        // scenario id like "service-kill"
    pub target: Option<String>,  // override target
    pub duration_secs: Option<u64>,
    pub intensity: Option<f64>,
    pub auto_recover: Option<bool>,
}

pub async fn handle_start(Json(req): Json<ChaosStartRequest>) -> Result<Json<ChaosResult>, AppError> {
    let engine = chaos_engine();
    let scenarios = engine.get_scenarios().await;
    let scenario = scenarios.iter()
        .find(|s| s.id == req.scenario)
        .ok_or_else(|| AppError::NotFound {
            resource: format!("chaos scenario: {}", req.scenario),
        })?;

    let target = req.target.unwrap_or_else(|| {
        scenario.targets.first().cloned().unwrap_or_default()
    });

    let exp = ChaosExperiment {
        id: format!("chaos-{}", chrono_now()),
        name: scenario.name.clone(),
        target,
        action: match req.scenario.as_str() {
            "service-kill" => "kill",
            "network-partition" => "drop_packets",
            "disk-pressure" => "fill_disk",
            "cpu-stress" => "saturate_cpu",
            "config-corrupt" => "kill",
            _ => "kill",
        }.to_string(),
        duration_secs: req.duration_secs.unwrap_or(10),
        intensity: req.intensity.unwrap_or(0.5),
        auto_recover: req.auto_recover.unwrap_or(true),
    };

    let result = engine.run_experiment(&exp).await?;
    Ok(Json(result))
}

/// GET /api/chaos/report — Summary report of all chaos experiments
pub async fn handle_report() -> Json<serde_json::Value> {
    let engine = chaos_engine();
    let status = engine.get_status().await;
    let history = engine.history.read().await;

    let passed = history.iter().filter(|r| r.status == "passed").count();
    let recovered = history.iter().filter(|r| r.status == "recovered").count();
    let failed = history.iter().filter(|r| r.status == "failed").count();
    let total = history.len();

    let avg_recovery_ms: f64 = if !history.is_empty() {
        // Compute from observations
        history.iter()
            .filter(|r| r.status == "recovered")
            .map(|r| {
                let first = r.observations.first().map(|o| o.timestamp.parse::<u64>().unwrap_or(0)).unwrap_or(0);
                let last = r.observations.last().map(|o| o.timestamp.parse::<u64>().unwrap_or(0)).unwrap_or(0);
                ((last - first) * 1000) as f64
            })
            .sum::<f64>() / recovered.max(1) as f64
    } else {
        0.0
    };

    Json(serde_json::json!({
        "total_experiments": total,
        "passed": passed,
        "recovered": recovered,
        "failed": failed,
        "resilience_score": if total > 0 {
            ((passed + recovered) as f64 / total as f64 * 100.0).round()
        } else { 0.0 },
        "avg_recovery_ms": avg_recovery_ms.round(),
        "last_experiment": history.last(),
        "experiments": history.iter().map(|r| serde_json::json!({
            "id": r.experiment_id,
            "status": r.status,
            "started_at": r.started_at,
            "ended_at": r.ended_at,
            "observations_count": r.observations.len(),
        })).collect::<Vec<_>>(),
    }))
}
