# nix-evo Experimental Features

## Overview

This branch introduces 5 experimental features that extend nix-evo beyond its v0.1 diagnostic core. These features add real-time monitoring, audit compliance, self-healing automation, Nix flakes migration support, and MCP tooling for AI agent integration.

---

## Feature 1: WebSocket Live Dashboard

**File:** `evo/src/dashboard.rs` + `evo/static/dashboard.html`

Real-time system monitoring via WebSocket. The agent streams CPU, memory, disk, and service status metrics every N seconds to connected browser clients.

**How it works:**
- `GET /api/dashboard/ws?interval_secs=3` — WebSocket upgrade endpoint
- Collects metrics from `/proc/stat`, `/proc/meminfo`, `/proc/loadavg`, `df`, and `systemctl`
- CPU usage measured via two-sample diff of `/proc/stat` (100ms interval)
- Auto-reconnects on disconnect

**Dashboard features:**
- Circular gauges for CPU, memory, disk usage
- Live line charts for CPU and memory history (60-point sliding window)
- Load average display (1m / 5m / 15m)
- Critical services status table
- Zero external dependencies — pure Canvas rendering, no CDN

**Endpoint:** `http://localhost:7890/dashboard`

---

## Feature 2: Audit Trail

**File:** `evo/src/audit.rs`

JSONL-formatted audit log that records every API call with timestamp, action, parameters hash, client IP, result, and duration.

**Storage:** `~/.nix-evo/audit.log` (JSONL — one JSON object per line)

**Privacy:** Parameters are hashed (not stored raw) — only a 64-bit hash is recorded.

**Endpoints:**
- `GET /api/audit?action=config_apply&limit=50` — query logs with optional filters
- `GET /api/audit/stats` — aggregated statistics (action counts, error rate, avg duration)

**Use cases:**
- Security audit: who did what, when
- Debugging: correlate API calls with system state changes
- Compliance: append-only log with timestamps

---

## Feature 3: Self-Healing Agent

**File:** `evo/src/healer.rs`

Background tokio task that monitors critical services and automatically takes corrective action when they fail repeatedly.

**Default rules:**
| Service | Failures | Window | Action | Cooldown |
|---------|----------|--------|--------|----------|
| nginx.service | 3 | 5 min | restart | 10 min |
| sshd.service | 2 | 3 min | restart | 5 min |
| phpfpm.service | 3 | 5 min | restart | 10 min |

**How it works:**
1. Runs a check cycle every 30 seconds
2. For each monitored service, checks `systemctl is-active`
3. If unhealthy, records a failure event with timestamp
4. Counts failures within the configured time window
5. If threshold exceeded and not in cooldown → executes action (restart or rollback)
6. Clears failure history after healing action

**Endpoint:** `GET /api/healer/status` — returns running state, rules, service health, action history

---

## Feature 4: Nix Flakes Converter

**File:** `evo/src/flake.rs`

Analyzes `configuration.nix` and generates an equivalent `flake.nix`. Smart detection of dependencies, channels, and services.

**What it detects:**
- NixOS channel (from `nix-channel --list` or `NIX_PATH`)
- Hostname (from `networking.hostName`)
- Hardware configuration imports
- Service declarations
- `<nixpkgs>` / `<nixos>` legacy references (warns about migration)
- Overlays usage (warns about manual migration)
- Home-manager imports

**Endpoint:** `POST /api/flake/convert`

**Request body:**
```json
{
  "channel": "nixos-24.05",        // optional, auto-detected
  "hostname": "myserver",           // optional, auto-detected
  "config_content": "...",         // optional, reads from disk
  "extra_inputs": {                 // optional, additional flake inputs
    "home-manager": "github:nix-community/home-manager"
  }
}
```

**Response:** Generated `flake.nix` content, detected metadata, and migration warnings.

---

## Feature 5: MCP Server Experimental Tools

**File:** `mcp-server/src/experimental.ts`

Four new MCP tools that wrap the experimental agent API endpoints for AI agent use:

| Tool | Description | Agent Endpoint |
|------|-------------|----------------|
| `dashboard_subscribe` | Fetch real-time system snapshot | `GET /api/snapshot` |
| `audit_query` | Query audit logs with filters | `GET /api/audit` |
| `healer_status` | Check self-healer state | `GET /api/healer/status` |
| `flake_convert` | Convert configuration.nix to flake | `POST /api/flake/convert` |

Each tool returns both a human-readable formatted summary and raw JSON, matching the existing MCP server pattern.

---

## New API Endpoints Summary

| Endpoint | Method | Feature |
|----------|--------|---------|
| `/api/dashboard/ws` | WebSocket | Live Dashboard |
| `/dashboard` | GET | Dashboard HTML page |
| `/api/audit` | GET | Audit log query |
| `/api/audit/stats` | GET | Audit statistics |
| `/api/healer/status` | GET | Healer status |
| `/api/flake/convert` | POST | Flake conversion |

---

## Architecture Notes

- All features follow the existing axum + tokio patterns established in v0.1
- Audit uses file-based JSONL storage (no database dependency)
- Healer uses `tokio::sync::RwLock` for thread-safe state
- Dashboard reads from `/proc` filesystem (Linux-specific, matches NixOS target)
- Flake converter uses subprocess calls matching the existing `run_cmd` helper
- Static dashboard HTML is embedded via `include_str!()` — no file serving infrastructure needed

---

# Experimental V2 Features

## Feature 6: Multi-Cluster Orchestrator

**File:** `evo/src/cluster.rs`

Manage multiple NixOS servers from a single agent. Supports three deployment strategies:
- **fan-out**: Send command to all nodes in parallel
- **fan-in**: Same as fan-out, collects and aggregates all results  
- **rolling**: Deploy one-by-one, stop on first failure

Each node can be configured with either SSH tunnel or agent API URL. Health checks measure reachability and latency.

**Endpoints:**
- `POST /api/cluster/deploy` — Execute command across cluster
- `GET /api/cluster/status` — Node reachability + last deploy state
- `POST /api/cluster/nodes` — Add a node to the cluster
- `DELETE /api/cluster/nodes?name=xxx` — Remove a node

**Deploy request body:**
```json
{
  "command": "nixos-rebuild switch",
  "strategy": "rolling",
  "nodes": ["web1", "web2"],
  "stop_on_failure": true,
  "timeout_secs": 300
}
```

---

## Feature 7: Nix Flake Marketplace Browser

**File:** `evo/src/marketplace.rs`

Search nixpkgs via the search.nixos.org API. Returns package name, version, description, license, and homepage. "Info" endpoint generates ready-to-use nix config snippets.

**Endpoints:**
- `GET /api/marketplace/search?q=nginx&channel=unstable&limit=10` — Search packages
- `GET /api/marketplace/info?package=nginx&channel=unstable` — Package details + nix snippet

---

## Feature 8: Config Dependency Graph

**Files:** `evo/src/deps.rs` + `evo/static/deps.html`

Parse `configuration.nix` to extract enabled services, then build a dependency DAG from a built-in map of 30+ well-known NixOS services. Supports 5 node kinds (service, library, runtime, tool) and 4 edge types (requires, uses, imports, wants).

Exports as JSON or Graphviz DOT format. Interactive SVG visualization at `/deps` with force-directed and tree layouts.

**Endpoints:**
- `GET /api/deps/graph?format=json&config_path=/etc/nixos/configuration.nix&depth=5`
- `POST /api/deps/graph/analyze` — Analyze inline config content

---

## Feature 9: NixOS Generation Timeline

**Files:** `evo/src/timeline.rs` + `evo/static/timeline.html`

Visual timeline of all NixOS generations. Each generation shows date, NixOS version, kernel version, risk level, and description. "Compare any two" mode runs `nix store diff-closures` to show added/removed packages.

Interactive timeline UI at `/timeline` with click-to-select comparison.

**Endpoints:**
- `GET /api/timeline?limit=50` — List generations
- `GET /api/timeline/compare?from=42&to=45` — Compare two generations

---

## Feature 10: Smart Rollback Advisor

**File:** `evo/src/advisor.rs`

When something breaks, analyzes recent generations with a multi-factor scoring system:
- **Service health** (45%): Does this generation have the critical services configured?
- **Uptime** (25%): How long did this generation survive before being replaced?
- **Recency** (20%): How recent is this generation?
- **Age stability** (10%): Is the generation old enough to be considered stable?

Returns a ranked list of candidates with confidence scores and reasons. Not just "go to previous" — picks the objectively best target.

**Endpoint:** `POST /api/advisor/recommend`

---

## Feature 11: Prometheus Metrics Exporter

**File:** `evo/src/metrics.rs`

Exposes a `/metrics` endpoint in Prometheus text exposition format. Tracks:
- API request counts (total and by path)
- API error counts
- Response time histogram (10ms to 5s buckets)
- NixOS generation count
- Self-healer actions (total and by service)
- Active WebSocket connections
- Cluster node count and deploy metrics
- Audit log entry count

Ready for direct scrape by Prometheus/Grafana. Endpoint is outside API token auth.

**Endpoint:** `GET /metrics`

---

## Feature 12: MCP Server V2 Tools

**File:** `mcp-server/src/experimental-v2.ts`

Seven new MCP tools wrapping all V2 features:

| Tool | Description | Agent Endpoint |
|------|-------------|----------------|
| `cluster_deploy` | Deploy across cluster | `POST /api/cluster/deploy` |
| `cluster_status` | Check cluster health | `GET /api/cluster/status` |
| `marketplace_search` | Search nixpkgs | `GET /api/marketplace/search` |
| `deps_graph` | Config dependency graph | `GET /api/deps/graph` |
| `timeline_view` | Generation timeline + compare | `GET /api/timeline` |
| `advisor_recommend` | Smart rollback suggestion | `POST /api/advisor/recommend` |
| `metrics_export` | Prometheus metrics | `GET /metrics` |

---

## New API Endpoints Summary (V1 + V2)

| Endpoint | Method | Feature |
|----------|--------|---------|
| `/api/dashboard/ws` | WebSocket | Live Dashboard |
| `/dashboard` | GET | Dashboard HTML |
| `/api/audit` | GET | Audit log query |
| `/api/audit/stats` | GET | Audit statistics |
| `/api/healer/status` | GET | Healer status |
| `/api/flake/convert` | POST | Flake conversion |
| `/api/cluster/deploy` | POST | Cluster deploy |
| `/api/cluster/status` | GET | Cluster status |
| `/api/cluster/nodes` | POST/DELETE | Add/remove nodes |
| `/api/marketplace/search` | GET | Package search |
| `/api/marketplace/info` | GET | Package details |
| `/api/deps/graph` | GET | Dependency graph |
| `/api/deps/graph/analyze` | POST | Inline config analysis |
| `/api/timeline` | GET | Generation timeline |
| `/api/timeline/compare` | GET | Compare generations |
| `/api/advisor/recommend` | POST | Smart rollback |
| `/api/advisor/status` | GET | Advisor quick status |
| `/metrics` | GET | Prometheus metrics |
| `/deps` | GET | Deps graph UI |
| `/timeline` | GET | Timeline UI |

---

## Architecture Notes

- All features follow the existing axum + tokio patterns
- Cluster uses `OnceLock` for global singleton pattern
- Marketplace uses reqwest HTTP client with 15s timeout
- Deps graph has a built-in dependency map for 30+ NixOS services
- Timeline falls back to directory parsing if `nixos-rebuild` is unavailable
- Advisor uses weighted composite scoring with configurable critical services
- Metrics uses `AtomicU64` + `RwLock` for thread-safe lock-free counting
- `/metrics` endpoint is deliberately outside auth (standard for Prometheus)
- MCP tools include human-readable formatting + raw JSON for all responses
