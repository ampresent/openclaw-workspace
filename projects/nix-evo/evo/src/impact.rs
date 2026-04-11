use axum::{response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::AppError;

// ─── Impact Analysis Types ───────────────────────────────────────────────

/// A proposed config change to analyze.
#[derive(Debug, Deserialize)]
pub struct ImpactRequest {
    pub changes: Vec<ProposedChange>,
    pub config_source: Option<String>, // "file" or inline nix
}

#[derive(Debug, Deserialize)]
pub struct ProposedChange {
    pub option: String,       // e.g. "services.nginx.listen.port"
    pub old_value: Option<String>,
    pub new_value: String,
}

/// Full impact analysis result.
#[derive(Debug, Serialize)]
pub struct ImpactReport {
    pub summary: String,
    pub risk_level: String,
    pub direct_impacts: Vec<Impact>,
    pub transitive_impacts: Vec<Impact>,
    pub required_changes: Vec<RequiredChange>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
    pub dependency_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Impact {
    pub target: String,
    pub impact_type: String,   // "port_conflict", "service_restart", "config_rewrite", "dns_change"
    pub severity: String,      // "info", "warning", "breaking"
    pub description: String,
    pub remediation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RequiredChange {
    pub option: String,
    pub current_value: String,
    pub suggested_value: String,
    pub reason: String,
}

// ─── Dependency Graph ────────────────────────────────────────────────────

/// Known NixOS option dependencies and their transitive effects.
struct ImpactGraph {
    edges: HashMap<String, Vec<Edge>>,
}

struct Edge {
    target: String,
    impact_type: String,
    description: String,
    severity: String,
}

impl ImpactGraph {
    fn new() -> Self {
        let mut edges: HashMap<String, Vec<Edge>> = HashMap::new();

        // nginx port changes
        edges.insert("services.nginx.listen.port".into(), vec![
            Edge { target: "networking.firewall.allowedTCPPorts".into(), impact_type: "firewall".into(), description: "Firewall must allow the new port".into(), severity: "breaking".into() },
            Edge { target: "services.nginx.virtualHosts".into(), impact_type: "config_rewrite".into(), description: "VirtualHost proxy_pass targets may need updating".into(), severity: "warning".into() },
            Edge { target: "services.prometheus.exporters.nginx.port".into(), impact_type: "monitoring".into(), description: "Monitoring may lose connection".into(), severity: "info".into() },
        ]);

        // nginx enable
        edges.insert("services.nginx.enable".into(), vec![
            Edge { target: "networking.firewall.allowedTCPPorts".into(), impact_type: "firewall".into(), description: "Ports 80/443 need to be open".into(), severity: "warning".into() },
            Edge { target: "security.acme".into(), impact_type: "dependency".into(), description: "ACME certs may need nginx for HTTP challenge".into(), severity: "info".into() },
        ]);

        // PostgreSQL port
        edges.insert("services.postgresql.port".into(), vec![
            Edge { target: "networking.firewall.allowedTCPPorts".into(), impact_type: "firewall".into(), description: "Firewall must allow new PostgreSQL port".into(), severity: "breaking".into() },
            Edge { target: "services.*.database".into(), impact_type: "service_restart".into(), description: "All services connecting to PostgreSQL need reconfiguration".into(), severity: "breaking".into() },
        ]);

        // Firewall ports
        edges.insert("networking.firewall.allowedTCPPorts".into(), vec![
            Edge { target: "services.*.listen.port".into(), impact_type: "connectivity".into(), description: "Services on removed ports will become unreachable".into(), severity: "breaking".into() },
        ]);

        // SSH port
        edges.insert("services.openssh.ports".into(), vec![
            Edge { target: "networking.firewall.allowedTCPPorts".into(), impact_type: "firewall".into(), description: "New SSH port must be allowed through firewall".into(), severity: "breaking".into() },
            Edge { target: "services.fail2ban".into(), impact_type: "config_rewrite".into(), description: "fail2ban jail port config needs updating".into(), severity: "warning".into() },
        ]);

        // DNS
        edges.insert("networking.nameservers".into(), vec![
            Edge { target: "services.postgresql".into(), impact_type: "dependency".into(), description: "Database connections using DNS names may fail during transition".into(), severity: "info".into() },
            Edge { target: "services.dnsmasq".into(), impact_type: "conflict".into(), description: "May conflict with local DNS resolver".into(), severity: "warning".into() },
        ]);

        // User changes
        edges.insert("users.users".into(), vec![
            Edge { target: "services.*".into(), impact_type: "service_restart".into(), description: "Services running as modified users need restart".into(), severity: "warning".into() },
            Edge { target: "home-manager".into(), impact_type: "config_rewrite".into(), description: "Home-manager configs for this user may need update".into(), severity: "info".into() },
        ]);

        // Kernel params
        edges.insert("boot.kernelParams".into(), vec![
            Edge { target: "boot.loader".into(), impact_type: "reboot".into(), description: "Kernel parameter changes require reboot".into(), severity: "breaking".into() },
        ]);

        // NixOS channel/version
        edges.insert("system.stateVersion".into(), vec![
            Edge { target: "services.*".into(), impact_type: "breaking".into(), description: "State version changes may alter service defaults".into(), severity: "breaking".into() },
            Edge { target: "nixpkgs.config".into(), impact_type: "config_rewrite".into(), description: "Package availability may change".into(), severity: "warning".into() },
        ]);

        Self { edges }
    }

    /// Find all transitive impacts of changing an option.
    fn analyze(&self, option: &str, old_value: Option<&str>, new_value: &str) -> (Vec<Impact>, Vec<String>) {
        let mut impacts = Vec::new();
        let mut visited = HashSet::new();
        let mut chain = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back(option.to_string());
        visited.insert(option.to_string());

        while let Some(current) = queue.pop_front() {
            chain.push(current.clone());

            // Find direct edges (exact match or prefix match)
            let direct_edges = self.edges.get(&current);
            let wildcard_edges: Vec<&Edge> = self.edges
                .iter()
                .filter(|(k, _)| k.contains('*'))
                .flat_map(|(k, v)| {
                    let pattern = k.replace('*', "");
                    if current.starts_with(&pattern) || pattern.starts_with(
                        &current.split('.').take(2).collect::<Vec<_>>().join(".")
                    ) {
                        v.iter().collect::<Vec<_>>()
                    } else {
                        vec![]
                    }
                })
                .collect();

            let all_edges: Vec<&Edge> = direct_edges
                .map(|e| e.iter().collect())
                .unwrap_or_default()
                .into_iter()
                .chain(wildcard_edges)
                .collect();

            for edge in all_edges {
                if !visited.contains(&edge.target) {
                    visited.insert(edge.target.clone());
                    impacts.push(Impact {
                        target: edge.target.clone(),
                        impact_type: edge.impact_type.clone(),
                        severity: edge.severity.clone(),
                        description: edge.description.clone(),
                        remediation: Some(format!(
                            "Check {} and update to match new value",
                            edge.target
                        )),
                    });
                    queue.push_back(edge.target.clone());
                }
            }
        }

        // Special case analysis for port changes
        if option.contains("port") {
            if let (Some(old), Ok(new_port)) = (old_value, new_value.parse::<u16>()) {
                if let Ok(old_port) = old.parse::<u16>() {
                    if old_port != new_port {
                        impacts.push(Impact {
                            target: format!("iptables rule for port {old_port}"),
                            impact_type: "firewall".into(),
                            severity: "breaking".into(),
                            description: format!(
                                "Port change from {old_port} to {new_port} will drop existing connections"
                            ),
                            remediation: Some("Update firewall rules before changing service port".into()),
                        });
                    }
                }
            }
        }

        (impacts, chain)
    }
}

// ─── Impact Analyzer ─────────────────────────────────────────────────────

pub struct ImpactAnalyzer {
    graph: ImpactGraph,
}

impl ImpactAnalyzer {
    pub fn new() -> Self {
        Self {
            graph: ImpactGraph::new(),
        }
    }

    pub fn analyze(&self, request: &ImpactRequest) -> ImpactReport {
        let mut all_direct = Vec::new();
        let mut all_transitive = Vec::new();
        let mut all_chains = Vec::new();
        let mut required_changes = Vec::new();
        let mut warnings = Vec::new();

        for change in &request.changes {
            let (impacts, chain) = self.graph.analyze(
                &change.option,
                change.old_value.as_deref(),
                &change.new_value,
            );

            // First level = direct, rest = transitive
            if let Some(first) = impacts.first() {
                all_direct.push(first.clone());
                for impact in impacts.iter().skip(1) {
                    all_transitive.push(impact.clone());
                }
            }

            all_chains.extend(chain);

            // Generate required changes based on known patterns
            if change.option.contains("nginx") && change.option.contains("port") {
                required_changes.push(RequiredChange {
                    option: "networking.firewall.allowedTCPPorts".into(),
                    current_value: "[...existing...]".into(),
                    suggested_value: format!("[...existing... {}]", change.new_value),
                    reason: "New nginx port must be allowed through firewall".into(),
                });
            }

            if change.option.contains("openssh") && change.option.contains("port") {
                required_changes.push(RequiredChange {
                    option: "networking.firewall.allowedTCPPorts".into(),
                    current_value: "[22]".into(),
                    suggested_value: format!("[{}]", change.new_value),
                    reason: "New SSH port must be allowed through firewall".into(),
                });
                warnings.push(
                    "⚠️ Changing SSH port: keep old port open until you verify the new one works!".into()
                );
            }
        }

        // Compute overall risk
        let max_severity = all_direct.iter()
            .chain(all_transitive.iter())
            .map(|i| match i.severity.as_str() {
                "breaking" => 3,
                "warning" => 2,
                _ => 1,
            })
            .max()
            .unwrap_or(0);

        let risk_level = match max_severity {
            3 => "high",
            2 => "medium",
            _ => "low",
        }.to_string();

        let total_impacts = all_direct.len() + all_transitive.len();
        let summary = format!(
            "Analyzed {} change(s): {} direct impacts, {} transitive impacts, risk level: {}",
            request.changes.len(),
            all_direct.len(),
            all_transitive.len(),
            risk_level,
        );

        let mut recommendations = Vec::new();
        if risk_level == "high" {
            recommendations.push("Create a NixOS generation snapshot before applying".into());
            recommendations.push("Test with nixos-rebuild test before switching".into());
        }
        if !warnings.is_empty() {
            recommendations.push("Review warnings carefully before proceeding".into());
        }
        if required_changes.len() > 1 {
            recommendations.push("Apply all required changes atomically to avoid partial state".into());
        }

        ImpactReport {
            summary,
            risk_level,
            direct_impacts: all_direct,
            transitive_impacts: all_transitive,
            required_changes,
            warnings,
            recommendations,
            dependency_chain: all_chains,
        }
    }
}

// ─── Singleton ───────────────────────────────────────────────────────────

use std::sync::OnceLock;
pub static ANALYZER: OnceLock<ImpactAnalyzer> = OnceLock::new();

pub fn analyzer() -> &'static ImpactAnalyzer {
    ANALYZER.get_or_init(ImpactAnalyzer::new)
}

// ─── HTTP Handler ────────────────────────────────────────────────────────

/// POST /api/impact/analyze
pub async fn handle_analyze(
    Json(req): Json<ImpactRequest>,
) -> Json<ImpactReport> {
    let report = analyzer().analyze(&req);
    Json(report)
}
