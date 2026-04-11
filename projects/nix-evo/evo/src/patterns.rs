use axum::{extract::{Path, Query}, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

// ─── Pattern Definition ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub name: String,
    pub category: PatternCategory,
    pub difficulty: Difficulty,
    pub security_rating: SecurityRating,
    pub use_cases: Vec<String>,
    pub description: String,
    pub explanation: String,
    pub nix_code: String,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>, // other pattern IDs this depends on
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PatternCategory {
    WebServer,
    Database,
    Networking,
    Security,
    Monitoring,
    Containers,
    Storage,
    Boot,
    Desktop,
    Development,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum SecurityRating {
    Minimal,
    Standard,
    Hardened,
    Paranoid,
}

// ─── Pattern Library ─────────────────────────────────────────────────────

pub struct PatternLibrary {
    patterns: Vec<Pattern>,
}

impl PatternLibrary {
    pub fn new() -> Self {
        Self {
            patterns: Self::builtin_patterns(),
        }
    }

    /// Search patterns by query, category, difficulty, security rating.
    pub fn search(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        difficulty: Option<&str>,
        security: Option<&str>,
    ) -> Vec<&Pattern> {
        self.patterns
            .iter()
            .filter(|p| {
                // Text search
                if let Some(q) = query {
                    let q_lower = q.to_lowercase();
                    let matches_name = p.name.to_lowercase().contains(&q_lower);
                    let matches_desc = p.description.to_lowercase().contains(&q_lower);
                    let matches_tags = p.tags.iter().any(|t| t.to_lowercase().contains(&q_lower));
                    let matches_use = p.use_cases.iter().any(|u| u.to_lowercase().contains(&q_lower));
                    if !matches_name && !matches_desc && !matches_tags && !matches_use {
                        return false;
                    }
                }
                // Category filter
                if let Some(cat) = category {
                    let cat_str = format!("{:?}", p.category).to_lowercase();
                    if cat_str != cat.to_lowercase() {
                        return false;
                    }
                }
                // Difficulty filter
                if let Some(diff) = difficulty {
                    let diff_str = format!("{:?}", p.difficulty).to_lowercase();
                    if diff_str != diff.to_lowercase() {
                        return false;
                    }
                }
                // Security filter
                if let Some(sec) = security {
                    let sec_str = format!("{:?}", p.security_rating).to_lowercase();
                    if sec_str != sec.to_lowercase() {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    /// Get a pattern by ID.
    pub fn get(&self, id: &str) -> Option<&Pattern> {
        self.patterns.iter().find(|p| p.id == id)
    }

    /// List all categories with counts.
    pub fn categories(&self) -> Vec<(String, usize)> {
        let mut cats: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        for p in &self.patterns {
            *cats.entry(format!("{:?}", p.category)).or_insert(0) += 1;
        }
        cats.into_iter().collect()
    }

    fn builtin_patterns() -> Vec<Pattern> {
        vec![
            Pattern {
                id: "nginx-reverse-proxy".into(),
                name: "Nginx Reverse Proxy".into(),
                category: PatternCategory::WebServer,
                difficulty: Difficulty::Beginner,
                security_rating: SecurityRating::Standard,
                use_cases: vec![
                    "reverse proxy".into(), "web server".into(), "SSL termination".into(),
                    "load balancing".into(), "frontend proxy".into(),
                ],
                description: "Set up Nginx as a reverse proxy with SSL termination".into(),
                explanation: r#"A reverse proxy sits in front of your backend services, handling incoming HTTP(S) requests and forwarding them. Benefits include:
- SSL/TLS termination at the edge
- Single entry point for multiple services
- Load balancing across backends
- Static file serving
- Request buffering and rate limiting"#.into(),
                nix_code: r#"{
  services.nginx = {
    enable = true;
    recommendedProxySettings = true;
    recommendedTlsSettings = true;
    recommendedOptimisation = true;
    recommendedGzipSettings = true;

    virtualHosts."example.com" = {
      enableACME = true;
      forceSSL = true;
      locations."/" = {
        proxyPass = "http://127.0.0.1:3000";
        proxyWebsockets = true;
      };
    };
  };

  security.acme.acceptTerms = true;
  security.acme.defaults.email = "admin@example.com";
}"#.into(),
                tags: vec!["nginx".into(), "proxy".into(), "ssl".into(), "https".into()],
                dependencies: vec![],
            },
            Pattern {
                id: "postgresql-server".into(),
                name: "PostgreSQL Database Server".into(),
                category: PatternCategory::Database,
                difficulty: Difficulty::Beginner,
                security_rating: SecurityRating::Standard,
                use_cases: vec![
                    "database".into(), "postgresql".into(), "sql".into(), "data storage".into(),
                ],
                description: "Set up PostgreSQL with proper authentication and performance tuning".into(),
                explanation: r#"PostgreSQL is a powerful, open-source relational database. This pattern sets up:
- PostgreSQL with systemd management
- Authentication via peer/md5
- Performance tuning for typical workloads
- Automatic backups via pg_dump"#.into(),
                nix_code: r#"{
  services.postgresql = {
    enable = true;
    package = pkgs.postgresql_16;
    ensureDatabases = [ "myapp" ];
    ensureUsers = [
      { name = "myapp";
        ensureDBOwnership = true;
      }
    ];
    authentication = ''
      # TYPE  DATABASE  USER      ADDRESS       METHOD
      local   all       all                     peer
      host    all       all       127.0.0.1/32  scram-sha-256
    '';
    settings = {
      shared_buffers = "256MB";
      work_mem = "16MB";
      maintenance_work_mem = "128MB";
      effective_cache_size = "1GB";
      max_connections = 100;
    };
  };
}"#.into(),
                tags: vec!["postgresql".into(), "database".into(), "sql".into()],
                dependencies: vec![],
            },
            Pattern {
                id: "firewall-basic".into(),
                name: "Basic Firewall Configuration".into(),
                category: PatternCategory::Security,
                difficulty: Difficulty::Beginner,
                security_rating: SecurityRating::Standard,
                use_cases: vec![
                    "firewall".into(), "network security".into(), "iptables".into(),
                    "port filtering".into(),
                ],
                description: "Allow only essential ports, block everything else".into(),
                explanation: r#"A firewall is your first line of defense. This pattern:
- Allows SSH (22), HTTP (80), HTTPS (443)
- Allows established/related connections
- Drops everything else
- Logs dropped packets for monitoring"#.into(),
                nix_code: r#"{
  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 22 80 443 ];
    allowedUDPPorts = [ ];
    allowPing = true;
    logReversePathDrops = true;
    extraCommands = ''
      # Rate-limit new connections on port 22 (SSH)
      iptables -A nixos-fw -p tcp --dport 22 \
        -m state --state NEW -m recent --set
      iptables -A nixos-fw -p tcp --dport 22 \
        -m state --state NEW -m recent --update \
        --seconds 60 --hitcount 6 -j nixos-fw-refuse
    '';
  };
}"#.into(),
                tags: vec!["firewall".into(), "security".into(), "iptables".into(), "ssh".into()],
                dependencies: vec![],
            },
            Pattern {
                id: "docker-containers".into(),
                name: "Docker/Podman Container Runtime".into(),
                category: PatternCategory::Containers,
                difficulty: Difficulty::Intermediate,
                security_rating: SecurityRating::Standard,
                use_cases: vec![
                    "containers".into(), "docker".into(), "podman".into(),
                    "microservices".into(), "isolation".into(),
                ],
                description: "Run containers with rootless Podman or Docker".into(),
                explanation: r#"Containers provide isolated environments for applications. NixOS supports both Docker and Podman:
- Docker: traditional, wide ecosystem
- Podman: rootless by default, daemonless, Docker-compatible CLI
This pattern configures Podman with rootless containers for better security.".into(),
                nix_code: r#"{
  virtualisation = {
    podman = {
      enable = true;
      dockerCompat = true;
      defaultNetwork.settings.dns_enabled = true;
    };
    oci-containers = {
      backend = "podman";
      containers = {
        myapp = {
          image = "myapp:latest";
          ports = [ "3000:3000" ];
          environment = {
            NODE_ENV = "production";
          };
          volumes = [
            "/var/lib/myapp/data:/data"
          ];
          autoStart = true;
        };
      };
    };
  };
}"#.into(),
                tags: vec!["docker".into(), "podman".into(), "containers".into(), "oci".into()],
                dependencies: vec![],
            },
            Pattern {
                id: "prometheus-monitoring".into(),
                name: "Prometheus + Grafana Monitoring".into(),
                category: PatternCategory::Monitoring,
                difficulty: Difficulty::Intermediate,
                security_rating: SecurityRating::Standard,
                use_cases: vec![
                    "monitoring".into(), "metrics".into(), "grafana".into(),
                    "prometheus".into(), "alerting".into(),
                ],
                description: "Full monitoring stack with Prometheus, Grafana, and node_exporter".into(),
                explanation: r#"Monitor your system with the industry-standard stack:
- Prometheus: time-series metrics collection
- Grafana: dashboards and visualization
- node_exporter: system metrics (CPU, memory, disk, network)
- Alertmanager: alert routing and notifications"#.into(),
                nix_code: r#"{
  services.prometheus = {
    enable = true;
    port = 9090;
    scrapeConfigs = [
      {
        job_name = "node";
        static_configs = [{
          targets = [ "127.0.0.1:9100" ];
        }];
      }
    ];
  };

  services.prometheus.exporters.node = {
    enable = true;
    port = 9100;
    enabledCollectors = [ "systemd" "filesystem" ];
  };

  services.grafana = {
    enable = true;
    settings = {
      server = {
        http_port = 3000;
        domain = "grafana.example.com";
      };
    };
  };

  networking.firewall.allowedTCPPorts = [ 9090 9100 3000 ];
}"#.into(),
                tags: vec!["monitoring".into(), "prometheus".into(), "grafana".into(), "metrics".into()],
                dependencies: vec![],
            },
            Pattern {
                id: "hardened-ssh".into(),
                name: "Hardened SSH Configuration".into(),
                category: PatternCategory::Security,
                difficulty: Difficulty::Advanced,
                security_rating: SecurityRating::Hardened,
                use_cases: vec![
                    "ssh".into(), "remote access".into(), "hardening".into(),
                    "key-based auth".into(),
                ],
                description: "Lock down SSH with key-only auth, fail2ban, and restricted access".into(),
                explanation: r#"SSH is often the primary remote access vector. Harden it by:
- Disabling password authentication
- Requiring Ed25519 keys
- Rate-limiting with fail2ban
- Restricting to specific users
- Using non-standard port (optional)"#.into(),
                nix_code: r#"{
  services.openssh = {
    enable = true;
    settings = {
      PasswordAuthentication = false;
      KbdInteractiveAuthentication = false;
      PermitRootLogin = "prohibit-password";
      X11Forwarding = false;
      MaxAuthTries = 3;
      AllowUsers = [ "admin" ];
    };
    extraConfig = ''
      ClientAliveInterval 300
      ClientAliveCountMax 2
      LoginGraceTime 30
    '';
  };

  services.fail2ban = {
    enable = true;
    jails.sshd.settings = {
      enabled = true;
      port = "ssh";
      filter = "sshd";
      maxretry = 5;
      findtime = 600;
      bantime = 3600;
    };
  };

  # Only allow SSH through firewall
  networking.firewall.allowedTCPPorts = [ 22 ];
}"#.into(),
                tags: vec!["ssh".into(), "security".into(), "hardening".into(), "fail2ban".into()],
                dependencies: vec!["firewall-basic".into()],
            },
            Pattern {
                id: "wireguard-vpn".into(),
                name: "WireGuard VPN".into(),
                category: PatternCategory::Networking,
                difficulty: Difficulty::Advanced,
                security_rating: SecurityRating::Hardened,
                use_cases: vec![
                    "vpn".into(), "wireguard".into(), "site-to-site".into(),
                    "remote access".into(), "private network".into(),
                ],
                description: "Set up a WireGuard VPN for secure site-to-site or remote access".into(),
                explanation: r#"WireGuard is a modern, fast VPN protocol built into the Linux kernel:
- Minimal attack surface (~4000 lines of code)
- High performance (faster than OpenVPN)
- Simple configuration
- Perfect for site-to-site or road warrior setups"#.into(),
                nix_code: r#"{
  networking.wireguard.interfaces = {
    wg0 = {
      ips = [ "10.100.0.1/24" ];
      listenPort = 51820;
      privateKeyFile = "/etc/wireguard/private.key";
      peers = [
        {
          # Remote peer
          publicKey = "REPLACE_WITH_PEER_PUBLIC_KEY";
          allowedIPs = [ "10.100.0.2/32" ];
        }
      ];
    };
  };

  networking.firewall.allowedUDPPorts = [ 51820 ];

  # Generate keys: wg genkey | tee private.key | wg pubkey > public.key
}"#.into(),
                tags: vec!["wireguard".into(), "vpn".into(), "networking".into(), "tunnel".into()],
                dependencies: vec!["firewall-basic".into()],
            },
            Pattern {
                id: "zfs-storage".into(),
                name: "ZFS Storage Pool".into(),
                category: PatternCategory::Storage,
                difficulty: Difficulty::Expert,
                security_rating: SecurityRating::Standard,
                use_cases: vec![
                    "zfs".into(), "storage".into(), "raid".into(), "snapshots".into(),
                    "data integrity".into(),
                ],
                description: "Set up ZFS with snapshots, scrubbing, and auto-snapshots".into(),
                explanation: r#"ZFS combines filesystem and volume manager with enterprise features:
- Data integrity via checksums
- Snapshots and clones
- Compression (lz4, zstd)
- RAID-Z for redundancy
- Send/receive for backups"#.into(),
                nix_code: r#"{
  boot.supportedFilesystems = [ "zfs" ];
  boot.zfs.forceImportRoot = false;

  networking.hostId = "abcd1234"; # head -c8 /etc/machine-id

  services.zfs = {
    autoScrub.enable = true;
    autoSnapshot = {
      enable = true;
      frequent = 4;
      hourly = 24;
      daily = 7;
      weekly = 4;
      monthly = 12;
    };
    trim.enable = true;
  };

  # Example: create pool first:
  # zpool create -o ashift=12 tank mirror /dev/sda /dev/sdb
  # zfs set compression=lz4 tank
  # zfs set atime=off tank
}"#.into(),
                tags: vec!["zfs".into(), "storage".into(), "snapshots".into(), "raid".into()],
                dependencies: vec![],
            },
            Pattern {
                id: "dev-shell".into(),
                name: "Development Shell with Direnv".into(),
                category: PatternCategory::Development,
                difficulty: Difficulty::Beginner,
                security_rating: SecurityRating::Minimal,
                use_cases: vec![
                    "development".into(), "direnv".into(), "nix shell".into(),
                    "project environments".into(),
                ],
                description: "Per-project development environments with direnv + nix flakes".into(),
                explanation: r#"direnv automatically loads project-specific environments when you cd into a directory. Combined with Nix flakes:
- Each project gets its own set of tools
- No global pollution
- Reproducible across machines
- Automatic shell activation"#.into(),
                nix_code: r#"{
  # Enable direnv system-wide
  programs.direnv = {
    enable = true;
    nix-direnv.enable = true;
  };

  # Users need nix flakes enabled
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  # Example flake.nix for a project:
  # {
  #   inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  #   outputs = { nixpkgs, ... }: {
  #     devShells.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.mkShell {
  #       packages = with pkgs; [ nodejs_20 python3 go rustc cargo ];
  #     };
  #   };
  # }
}"#.into(),
                tags: vec!["development".into(), "direnv".into(), "flakes".into(), "shell".into()],
                dependencies: vec![],
            },
            Pattern {
                id: "acme-certs".into(),
                name: "Automatic SSL Certificates (ACME/Let's Encrypt)".into(),
                category: PatternCategory::Security,
                difficulty: Difficulty::Beginner,
                security_rating: SecurityRating::Standard,
                use_cases: vec![
                    "ssl".into(), "tls".into(), "letsencrypt".into(), "acme".into(),
                    "https".into(), "certificates".into(),
                ],
                description: "Automatic SSL certificate provisioning and renewal".into(),
                explanation: r#"Let's Encrypt provides free, automated SSL certificates. NixOS has built-in ACME support:
- Automatic certificate issuance
- Automatic renewal before expiry
- Integration with Nginx/Apache
- DNS or HTTP challenge support"#.into(),
                nix_code: r#"{
  security.acme = {
    acceptTerms = true;
    defaults = {
      email = "admin@example.com";
      server = "https://acme-v02.api.letsencrypt.org/directory";
    };
    certs."example.com" = {
      dnsProvider = "cloudflare"; # or use webroot for HTTP challenge
      credentialsFile = "/etc/nixos/acme-credentials";
      group = "nginx";
    };
  };

  # Nginx picks up certs automatically with enableACME
  services.nginx.virtualHosts."example.com" = {
    enableACME = true;
    forceSSL = true;
    locations."/" = {
      proxyPass = "http://127.0.0.1:8080";
    };
  };
}"#.into(),
                tags: vec!["ssl".into(), "tls".into(), "letsencrypt".into(), "acme".into(), "certificates".into()],
                dependencies: vec![],
            },
        ]
    }
}

// ─── Singleton ───────────────────────────────────────────────────────────

use std::sync::OnceLock;
pub static LIBRARY: OnceLock<PatternLibrary> = OnceLock::new();

pub fn library() -> &'static PatternLibrary {
    LIBRARY.get_or_init(PatternLibrary::new)
}

// ─── HTTP Handlers ───────────────────────────────────────────────────────

/// GET /api/patterns
#[derive(Debug, Deserialize)]
pub struct PatternQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub difficulty: Option<String>,
    pub security: Option<String>,
}

pub async fn handle_list(Query(q): Query<PatternQuery>) -> Json<serde_json::Value> {
    let lib = library();
    let results = lib.search(
        q.q.as_deref(),
        q.category.as_deref(),
        q.difficulty.as_deref(),
        q.security.as_deref(),
    );
    let categories = lib.categories();

    // Return lightweight summaries (without full nix_code)
    let summaries: Vec<serde_json::Value> = results.iter().map(|p| {
        serde_json::json!({
            "id": p.id,
            "name": p.name,
            "category": format!("{:?}", p.category),
            "difficulty": format!("{:?}", p.difficulty),
            "security_rating": format!("{:?}", p.security_rating),
            "description": p.description,
            "use_cases": p.use_cases,
            "tags": p.tags,
            "dependencies": p.dependencies,
        })
    }).collect();

    Json(serde_json::json!({
        "total": summaries.len(),
        "categories": categories.iter().map(|(c, n)| serde_json::json!({ "name": c, "count": n })).collect::<Vec<_>>(),
        "patterns": summaries,
    }))
}

/// GET /api/patterns/:id
pub async fn handle_get(Path(id): Path<String>) -> Result<Json<Pattern>, AppError> {
    let lib = library();
    lib.get(&id)
        .cloned()
        .map(Json)
        .ok_or_else(|| AppError::NotFound {
            resource: format!("pattern: {id}"),
        })
}
