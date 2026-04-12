use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::cmd::{run_cmd, AppStateRef};
use crate::error::AppError;

/// Healing rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealRule {
    /// Service name (e.g., "nginx.service")
    pub service: String,
    /// Number of failures before triggering action
    pub max_failures: u32,
    /// Time window in minutes for failure counting
    pub window_minutes: u32,
    /// Action to take: "restart" | "rollback"
    pub action: String,
    /// Cooldown in minutes before re-triggering
    pub cooldown_minutes: u32,
}

impl Default for HealRule {
    fn default() -> Self {
        Self {
            service: String::new(),
            max_failures: 3,
            window_minutes: 5,
            action: "restart".to_string(),
            cooldown_minutes: 10,
        }
    }
}

/// Tracked failure event
#[derive(Debug, Clone)]
struct FailureEvent {
    timestamp: Instant,
    description: String,
}

/// Service healing state
#[derive(Debug, Clone, Serialize)]
pub struct ServiceHealState {
    pub service: String,
    pub failure_count: u32,
    pub last_failure: Option<String>,
    pub last_action: Option<String>,
    pub last_action_time: Option<String>,
    pub healthy: bool,
}

/// Healer status for API
#[derive(Debug, Serialize)]
pub struct HealerStatus {
    pub running: bool,
    pub check_interval_secs: u64,
    pub rules: Vec<HealRule>,
    pub service_states: Vec<ServiceHealState>,
    pub total_heal_actions: u64,
    pub last_check: Option<String>,
}

/// Self-healer engine
pub struct Healer {
    rules: Vec<HealRule>,
    failure_history: RwLock<HashMap<String, Vec<FailureEvent>>>,
    action_history: RwLock<HashMap<String, Vec<HealAction>>>,
    heal_count: RwLock<u64>,
    last_check: RwLock<Option<String>>,
    running: RwLock<bool>,
}

#[derive(Debug, Clone)]
struct HealAction {
    timestamp: Instant,
    service: String,
    action: String,
    success: bool,
}

impl Healer {
    pub fn new(rules: Vec<HealRule>) -> Self {
        Self {
            rules,
            failure_history: RwLock::new(HashMap::new()),
            action_history: RwLock::new(HashMap::new()),
            heal_count: RwLock::new(0),
            last_check: RwLock::new(None),
            running: RwLock::new(false),
        }
    }

    /// Create default healer with common critical services
    pub fn with_defaults() -> Self {
        let rules = vec![
            HealRule {
                service: "nginx.service".to_string(),
                max_failures: 3,
                window_minutes: 5,
                action: "restart".to_string(),
                cooldown_minutes: 10,
            },
            HealRule {
                service: "sshd.service".to_string(),
                max_failures: 2,
                window_minutes: 3,
                action: "restart".to_string(),
                cooldown_minutes: 5,
            },
            HealRule {
                service: "phpfpm.service".to_string(),
                max_failures: 3,
                window_minutes: 5,
                action: "restart".to_string(),
                cooldown_minutes: 10,
            },
        ];
        Self::new(rules)
    }

    /// Check service health via systemctl
    async fn check_service(&self, service: &str) -> Result<bool, AppError> {
        let output = run_cmd("systemctl", &["is-active", "--no-pager", service]).await?;
        Ok(output.trim() == "active")
    }

    /// Record a failure event
    async fn record_failure(&self, service: &str, desc: &str) {
        let mut history = self.failure_history.write().await;
        let events = history.entry(service.to_string()).or_insert_with(Vec::new);
        events.push(FailureEvent {
            timestamp: Instant::now(),
            description: desc.to_string(),
        });
    }

    /// Count failures within the time window
    async fn count_recent_failures(&self, service: &str, window: Duration) -> u32 {
        let history = self.failure_history.read().await;
        let now = Instant::now();
        history
            .get(service)
            .map(|events| {
                events
                    .iter()
                    .filter(|e| now.duration_since(e.timestamp) < window)
                    .count() as u32
            })
            .unwrap_or(0)
    }

    /// Check if we're in cooldown for a service
    async fn in_cooldown(&self, service: &str, cooldown: Duration) -> bool {
        let actions = self.action_history.read().await;
        let now = Instant::now();
        actions
            .get(service)
            .map(|acts| {
                acts
                    .iter()
                    .any(|a| now.duration_since(a.timestamp) < cooldown)
            })
            .unwrap_or(false)
    }

    /// Execute healing action
    async fn execute_heal(&self, rule: &HealRule) -> Result<String, AppError> {
        match rule.action.as_str() {
            "restart" => {
                tracing::warn!("Healer: restarting {}", rule.service);
                let output = run_cmd("systemctl", &["restart", &rule.service]).await?;
                Ok(format!("Restarted {}", rule.service))
            }
            "rollback" => {
                tracing::warn!("Healer: rolling back due to {}", rule.service);
                // Trigger rollback to previous generation
                let output = run_cmd("nixos-rebuild", &["switch", "--rollback"]).await?;
                Ok(format!("Rolled back system due to {}", rule.service))
            }
            _ => Err(AppError::Validation {
                field: "action".to_string(),
                message: format!("Unknown heal action: {}", rule.action),
            }),
        }
    }

    /// Run one check cycle
    pub async fn check_cycle(&self) {
        for rule in &self.rules {
            // Check if healthy
            match self.check_service(&rule.service).await {
                Ok(true) => {
                    // Service is healthy, skip
                    continue;
                }
                Ok(false) => {
                    // Service is unhealthy — record failure
                    self.record_failure(&rule.service, "Service not active").await;
                }
                Err(e) => {
                    // Error checking — count as failure
                    self.record_failure(&rule.service, &format!("Check failed: {e}")).await;
                }
            }

            // Check failure threshold
            let window = Duration::from_secs(rule.window_minutes as u64 * 60);
            let failures = self.count_recent_failures(&rule.service, window).await;

            if failures >= rule.max_failures {
                // Check cooldown
                let cooldown = Duration::from_secs(rule.cooldown_minutes as u64 * 60);
                if self.in_cooldown(&rule.service, cooldown).await {
                    tracing::info!(
                        "Healer: {} hit threshold but in cooldown",
                        rule.service
                    );
                    continue;
                }

                // Execute healing action
                let result = self.execute_heal(rule).await;
                let success = result.is_ok();
                let desc = result.unwrap_or_else(|e| format!("Failed: {e}"));

                // Record action
                {
                    let mut actions = self.action_history.write().await;
                    actions
                        .entry(rule.service.clone())
                        .or_insert_with(Vec::new)
                        .push(HealAction {
                            timestamp: Instant::now(),
                            service: rule.service.clone(),
                            action: rule.action.clone(),
                            success,
                        });
                }

                // Increment heal count
                {
                    let mut count = self.heal_count.write().await;
                    *count += 1;
                }

                tracing::warn!("Healer action: {desc}");

                // Clear failure history after action
                {
                    let mut history = self.failure_history.write().await;
                    history.remove(&rule.service);
                }
            }
        }

        // Update last check time
        {
            let mut last = self.last_check.write().await;
            *last = Some(chrono_now());
        }
    }

    /// Get current status
    pub async fn status(&self) -> HealerStatus {
        let mut service_states = Vec::new();

        for rule in &self.rules {
            let healthy = self.check_service(&rule.service).await.unwrap_or(false);
            let failures = self
                .count_recent_failures(
                    &rule.service,
                    Duration::from_secs(rule.window_minutes as u64 * 60),
                )
                .await;

            let actions = self.action_history.read().await;
            let last = actions.get(&rule.service).and_then(|a| a.last());

            service_states.push(ServiceHealState {
                service: rule.service.clone(),
                failure_count: failures,
                last_failure: None,
                last_action: last.map(|a| a.action.clone()),
                last_action_time: last.map(|_| chrono_now()),
                healthy,
            });
        }

        HealerStatus {
            running: *self.running.read().await,
            check_interval_secs: 30,
            rules: self.rules.clone(),
            service_states,
            total_heal_actions: *self.heal_count.read().await,
            last_check: self.last_check.read().await.clone(),
        }
    }
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_secs())
}

/// Global healer instance
static HEALER: std::sync::OnceLock<Arc<Healer>> = std::sync::OnceLock::new();

pub fn get_healer() -> &'static Arc<Healer> {
    HEALER.get_or_init(|| Arc::new(Healer::with_defaults()))
}

/// Start the background healing loop
pub fn start_background_task() {
    let healer = get_healer().clone();

    tokio::spawn(async move {
        {
            let mut running = healer.running.write().await;
            *running = true;
        }

        tracing::info!("Self-healer started (check interval: 30s)");
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;
            healer.check_cycle().await;
        }
    });
}

/// GET /api/healer/status — return healer status
pub async fn handle_status(
    State(_state): AppStateRef,
) -> Result<impl IntoResponse, AppError> {
    let healer = get_healer();
    let status = healer.status().await;
    Ok(Json(serde_json::to_value(&status).unwrap_or_default()))
}
