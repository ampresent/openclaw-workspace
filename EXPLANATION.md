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
