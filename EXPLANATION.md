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

---

## Round 2: Tests, New Features, and Tooling

### New Features (3)

#### Feature 6: Config Diff Engine (`configdiff.rs`)
Deep configuration comparison with structured analysis:
- Parses NixOS configs into sections (services, packages, networking, security)
- Generates unified diff between two configurations
- Risk scoring (0-100) with categorized factors
- Detects service additions/removals, package changes, network/security modifications
- **Endpoint:** `POST /api/config/diff`

#### Feature 7: Service Dependency Graph (`depgraph.rs`)
BFS-based systemd dependency analyzer:
- Builds dependency graph from `systemctl show` data
- Detects circular dependencies via DFS
- Finds critical path (longest dependency chain)
- Calculates failure impact (what breaks if X goes down)
- **Endpoint:** `GET /api/deps?focus=sshd.service&depth=3`

#### Feature 8: Chaos Engineering (`chaos.rs`)
Controlled fault injection for resilience testing:
- Kill services, stop processes, saturate CPU, fill disk, drop packets
- Configurable intensity and duration
- Auto-recovery after experiment
- Experiment history tracking
- **Endpoints:** `GET /api/chaos/experiments`, `POST /api/chaos/start`, `POST /api/chaos/stop`

### Test Suites (5 files, ~50 tests)

| File | Type | Count | Coverage |
|------|------|-------|----------|
| `audit_tests.rs` | Rust unit | 13 | Audit log: serialization, hashing, filtering, action extraction |
| `flake_tests.rs` | Rust unit | 12 | Flake converter: hostname, services, legacy refs, generation |
| `integration_tests.rs` | Rust HTTP | 11 | All endpoints: routes, errors, CORS, JSON format |
| `experimental.test.ts` | TypeScript | ~30 | All MCP tools: mock API, multi-host, error handling |
| `integration_test.sh` | Shell E2E | ~20 | Full endpoint suite with JSON assertions |

### Tooling

- **`bench.py`** — Python benchmark: latency p50/p95/p99, RPS, concurrency stress test
- **`audit_middleware.rs`** — Automatic audit logging for every API call
- **`integration_test.sh`** — Runnable against live agent with colored output

### Updated MCP Tools

6 total experimental tools now:
- `dashboard_subscribe` — live metrics
- `audit_query` — audit logs
- `healer_status` — self-healer state
- `flake_convert` — flake.nix generation
- `config_diff` — deep config comparison (NEW)
- `service_deps` — dependency graph (NEW)

---

# Experimental V3 Features

## Feature 13: Chaos Monkey Engine

**File:** `evo/src/chaos.rs`

Built-in chaos engineering framework for testing system resilience. Predefined scenarios with configurable risk levels:
- **Service Kill & Recover**: Stop a service, observe if self-healer restarts it
- **Network Partition**: iptables packet drops between services
- **Disk Pressure**: Fill disk to test graceful degradation
- **CPU Stress**: Saturate cores, measure latency impact
- **Config Corruption**: Modify config, verify drift detection

Each experiment records pre/post observations and supports auto-recovery.

**Endpoints:**
- `GET /api/chaos/scenarios` — List available chaos scenarios
- `POST /api/chaos/run` — Execute an experiment
- `GET /api/chaos/status` — Check experiment status

---

## Feature 14: Config Drift Detector

**File:** `evo/src/drift.rs`

Scans the running system against the NixOS generation store to detect unauthorized changes:
- **File drift**: Modified files in /etc that differ from generation
- **Service drift**: Services in unexpected states
- **Package drift**: Missing binaries from generation
- **Health score**: 0-100 based on drift severity

**Endpoint:** `GET /api/drift/scan?paths=/etc/nginx&depth=3`

---

## Feature 15: AI Config Optimizer

**File:** `evo/src/optimizer.rs`

Analyzes the system and suggests optimizations with ready-to-use nix config snippets:
- **Unused services**: Detect running services that aren't needed (cups, avahi, bluetooth)
- **Security hardening**: SSH config, firewall, auto-upgrades
- **Storage**: Nix store cleanup, /tmp usage, generation pruning
- **Performance**: Swap analysis, kernel parameters

Each suggestion includes: category, impact level, effort level, nix snippet, and reference URL.

**Endpoint:** `GET /api/optimizer/analyze`

---

## Feature 16: Service Mesh Visualizer

**File:** `evo/src/mesh.rs`

Discovers the actual network topology of running services by parsing `ss` and `/proc/net`:
- Map all listening services with their addresses and PIDs
- Trace active TCP connections between services
- Discover unix socket connections
- Identify externally-exposed vs isolated services
- Resolve well-known ports to service names

**Endpoint:** `GET /api/mesh/topology`

---

## Complete API Endpoints (V1 + V2 + V3)

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
| `/api/chaos/scenarios` | GET | Chaos scenarios |
| `/api/chaos/run` | POST | Run chaos experiment |
| `/api/chaos/status` | GET | Chaos engine status |
| `/api/drift/scan` | GET | Config drift scan |
| `/api/optimizer/analyze` | GET | Config optimizer |
| `/api/mesh/topology` | GET | Service mesh map |
| `/api/config/diff` | POST | Config diff (v1) |
| `/api/deps` | GET | Dep graph (v1) |

---

# Experimental V3 Features

## Feature 13: Nix Expression Interpreter (in-process)

**File:** `evo/src/nix_eval.rs`

A complete Nix expression parser and evaluator implemented entirely in Rust — no `nix eval` subprocess needed. This is a **subset** interpreter designed for syntax checking, config previews, and IDE integration.

**Supported Nix constructs:**
- Literals: integers, floats, booleans, null, strings
- Data structures: attrsets `{}`, lists `[]`
- Let bindings: `let x = 1; y = x + 2; in y`
- Conditionals: `if true then "yes" else "no"`
- Attribute access: `s.a.b`, `s.${expr}`, with `or` default
- Binary operators: `+`, `-`, `*`, `/`, `==`, `!=`, `&&`, `||`, `++` (concat), `//` (merge)
- With/inherit statements
- Lambda definitions and application
- Nested attribute paths: `services.nginx.enable = true`

**Architecture:**
- `Lexer` → tokenizes input with comment support (`// line`, `/* block */`)
- `Parser` → recursive descent parser producing AST (`NixExpr` enum)
- `Evaluator` → evaluates AST with scoped environment (`HashMap<String, NixValue>`)
- 7 unit tests covering: arithmetic, let-bindings, attrsets, if/else, attribute selection, list concatenation

**Endpoints:**
- `GET /api/nix/check?expression=...` — parse only, return AST
- `GET /api/nix/eval?expression=...` — parse + evaluate, return value

---

## Feature 14: Multi-Language Support Engine

**File:** `evo/src/i18n.rs`

Translates common NixOS error messages and `nixos-rebuild dry-build` output to user's language. Uses pattern matching against a built-in dictionary of error templates.

**Supported languages:**
| Language | Code | Error Patterns | Build Messages |
|----------|------|---------------|----------------|
| 简体中文 | zh-CN | 15 | 14 |
| 日本語 | ja-JP | 10 | 10 |
| Deutsch | de-DE | 9 | 9 |
| Français | fr-FR | 9 | 9 |

**Translated error categories:**
- Attribute not found (most common Nix error)
- Infinite recursion
- Syntax errors
- File not found
- Package collisions
- Hash mismatches
- Network errors
- Permission denied
- Out of memory
- Build failures

**Endpoints:**
- `POST /api/i18n/translate` — translate an error message or build output
- `GET /api/i18n/languages` — list supported languages with coverage stats

---

## Feature 15: Security Scanner

**Files:** `evo/src/security.rs` + `evo/static/security.html`

Comprehensive security audit of NixOS configuration. Scans for 8 categories of issues and produces a scored report.

**Scan categories:**
1. **Firewall** — enabled/disabled, port rules, default policy
2. **Open Ports** — detects dangerous ports (FTP, Telnet, Redis, MySQL, MongoDB, etc.), 0.0.0.0 bindings
3. **SSH** — root login, password auth, key-based auth
4. **Authentication** — empty/weak passwords, passwordless sudo
5. **Services** — Docker root mode, NFS exports, web server TLS, mail security
6. **Packages** — Nix store size, known vulnerable package patterns
7. **File Permissions** — world-writable files in /etc, config file modes
8. **Kernel** — MAC frameworks (AppArmor/SELinux), IP forwarding

**Scoring:** 100 minus penalties per finding (Critical: -20, High: -10, Medium: -5, Low: -2), clamped to 0-100.

**Endpoints:**
- `GET /api/security/scan?config_path=...` — full scan report
- `GET /api/security/score` — quick score only

**UI:** `/security` — dark-themed dashboard with circular score gauge, category filters, finding cards with line hints and recommendations.

---

## Feature 16: Interactive Config Builder (WebSocket)

**Files:** `evo/src/config_builder.rs` + `evo/static/builder.html`

WebSocket-based wizard that guides users through building a NixOS configuration step by step. Uses a state machine per connection.

**State machine:**
1. **Welcome** — connection established, send service list
2. **Select Services** — choose from 10 built-in services
3. **Configure Ports** — adjust port assignments
4. **Set Options** — configure service-specific options
5. **Review** — preview generated configuration.nix
6. **Done** — apply or start over

**Built-in services (10):**
| Service | Category | Default Ports |
|---------|----------|---------------|
| nginx | Web Server | 80, 443 |
| postgresql | Database | 5432 |
| redis | Cache | 6379 |
| openssh | Remote Access | 22 |
| grafana | Monitoring | 3000 |
| prometheus | Monitoring | 9090 |
| docker | Virtualization | — |
| caddy | Web Server | 80, 443 |
| fail2ban | Security | — |
| mysql | Database | 3306 |

Each service has configurable options (bool/string/select types) with defaults.

**Endpoint:** `WS /api/config-builder/ws`
**UI:** `/builder` — step-by-step wizard with service cards, port editing, option forms, syntax-highlighted preview.

---

## Feature 17: Capacity Planning

**File:** `evo/src/capacity.rs`

Analyzes historical and current resource usage to predict when disk/memory will be exhausted and recommends allocation changes.

**What it analyzes:**
- **Disk:** Mount point usage (from `df`), Nix store size, `nix-collect-garbage --dry-run` savings estimate
- **Memory:** From `/proc/meminfo` — total, used, available, swap usage
- **CPU:** Core count (from `nproc`), load averages (from `/proc/loadavg`), per-core load

**Risk levels:** Low / Medium / High / Critical with configurable thresholds:
- Disk: 70% → Medium, 85% → High, 95% → Critical
- Memory: 70% → Medium, 85% → High, 95% → Critical
- CPU: 0.8 per-core → Medium, 1.5 → High, 2.0 → Critical

**Recommendation engine:** Generates actionable suggestions based on risk levels, with estimated savings.

**Endpoint:** `GET /api/capacity/forecast?include_recommendations=true`

---

## Feature 18: GitOps Bridge

**File:** `evo/src/gitops.rs`

Git-based configuration management with webhook support. Watches a git repo for NixOS config changes and can auto-pull, validate, and deploy.

**Workflow:**
1. Configure repo URL, branch, config path via `POST /api/gitops/configure`
2. Set up webhook pointing to `POST /api/gitops/webhook` (GitHub/Gitea format)
3. On push: auto-pull → validate (`nix-instantiate --parse`) → optionally deploy (`nixos-rebuild switch`)
4. Track current commit, pending commits, deploy history

**Deploy state machine:** Idle → Pulling → Validating → Deploying → Success/Failed

**Endpoints:**
- `POST /api/gitops/webhook` — receive push events
- `GET /api/gitops/status` — current state, commits, deploy history
- `POST /api/gitops/configure` — set repo and branch
- `POST /api/gitops/deploy` — manual trigger

---

## Feature 19: Plugin System

**File:** `evo/src/plugin.rs`

Dynamic plugin loading via shared libraries (.so/.dylib). Plugins are discovered by scanning `~/.nix-evo/plugins/`.

**Plugin C ABI (required exports):**
```c
const char* nix_evo_plugin_init();                          // return plugin name
const char* nix_evo_plugin_version();                       // return version string
const char* nix_evo_plugin_handle_request(                  // handle API request
    const char* method, const char* path, const char* body);
const char* nix_evo_plugin_health_check();                  // return "ok" or error
void nix_evo_plugin_cleanup();                              // free resources
```

**Features:**
- Auto-discovery on startup and on-demand via API
- Plugin manifest support (.json alongside .so for metadata)
- Health checking for all loaded plugins
- Request routing by plugin name
- Graceful error handling for failed plugins

**Endpoints:**
- `GET /api/plugins` — list all discovered plugins with status
- `GET /api/plugins/health` — health check all loaded plugins

---

## Feature 20: MCP Server V3 Tools

**File:** `mcp-server/src/experimental-v3.ts`

Eight new MCP tools wrapping all V3 features:

| Tool | Description | Agent Endpoint |
|------|-------------|----------------|
| `nix_eval_check` | Syntax check Nix expression | `GET /api/nix/check` |
| `nix_eval_run` | Evaluate Nix expression | `GET /api/nix/eval` |
| `i18n_translate` | Translate error messages | `GET /api/i18n/translate` |
| `security_scan` | Security audit | `GET /api/security/scan` |
| `config_builder_status` | Config builder info | WS /api/config-builder/ws |
| `capacity_forecast` | Resource forecasting | `GET /api/capacity/forecast` |
| `gitops_status` | GitOps state | `GET /api/gitops/status` |
| `plugins_list` | Plugin system status | `GET /api/plugins` |

All tools return human-readable formatted text + raw JSON via `structuredContent`.

---

## New API Endpoints Summary (V1 + V2 + V3)

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
| `/api/cluster/nodes` | POST/GET | Add/remove nodes |
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
| **`/api/nix/eval`** | **GET** | **Nix expression eval** |
| **`/api/nix/check`** | **GET** | **Nix syntax check** |
| **`/api/i18n/translate`** | **GET** | **Translate messages** |
| **`/api/i18n/languages`** | **GET** | **Supported languages** |
| **`/api/security/scan`** | **GET** | **Security scan** |
| **`/api/security/score`** | **GET** | **Security score** |
| **`/api/config-builder/ws`** | **WS** | **Config builder** |
| **`/api/capacity/forecast`** | **GET** | **Capacity planning** |
| **`/api/gitops/status`** | **GET** | **GitOps state** |
| **`/api/gitops/configure`** | **POST** | **GitOps config** |
| **`/api/gitops/deploy`** | **POST** | **GitOps deploy** |
| **`/api/gitops/webhook`** | **POST** | **Webhook receiver** |
| **`/api/plugins`** | **GET** | **Plugin list** |
| **`/api/plugins/health`** | **GET** | **Plugin health** |
| **`/security`** | **GET** | **Security UI** |
| **`/builder`** | **GET** | **Config builder UI** |

---

## Architecture Notes (V3)

- Nix interpreter is pure Rust — no subprocess calls, runs in <1ms for simple expressions
- i18n uses pattern matching against error templates — extensible by adding more patterns
- Security scanner reads `/proc`, runs `ss` and `nix-store` — Linux-specific, matches NixOS target
- Config builder uses `tokio::sync::broadcast` for multi-client WebSocket coordination
- Capacity planner reads `/proc/meminfo`, `/proc/loadavg`, `df` — zero external dependencies
- GitOps uses `OnceLock` for global state, file-based git operations
- Plugin system defines C ABI for interop — supports .so/.dylib on Linux/macOS
- All V3 features follow the existing axum + tokio patterns from V1/V2
- MCP V3 tools include both human-readable formatting and `structuredContent` for AI agent consumption

---

# Experimental V4 Features

## Feature 21: AI-Powered Nix Doctor

**Files:** `evo/src/doctor.rs` + `evo/static/doctor.html`

An intelligent diagnostic engine that matches NixOS error messages against a built-in knowledge base of 12 known issue patterns with solutions.

**Knowledge base covers:**
| Category | Patterns | Examples |
|----------|----------|---------|
| Evaluation | Missing attribute, infinite recursion | "attribute X missing", "infinite recursion encountered" |
| Build | Hash mismatch, sandbox failure | "hash mismatch", "cannot build in sandbox" |
| Packages | Package collisions | "collision between" |
| System | Disk full, permission denied, broken symlink, daemon down, store corruption | "No space left on device", "Permission denied" |
| Services | Service failures | "unit entered failed state" |
| Configuration | Channel conflicts | "conflicting channel", "<nixpkgs>" |

**Matching algorithm:**
- Supports glob patterns with `.*` wildcards (e.g., `attribute .* missing`)
- Confidence scoring based on number of matched patterns / total patterns
- Results sorted by confidence descending
- Context parameter allows combining multiple error sources

**Endpoints:**
- `POST /api/doctor/diagnose` — paste error, get ranked diagnosis with fix commands
- `GET /api/doctor/knowledge` — list all knowledge base entries

**UI:** `/doctor` — paste error area, example errors, severity badges, copy-to-clipboard commands, documentation links.

---

## Feature 22: Service Orchestration Composer

**Files:** `evo/src/composer.rs` + `evo/static/composer.html`

Define complex multi-service NixOS deployments as "compositions" — like docker-compose but for NixOS services, with dependency-aware startup ordering.

**Core concepts:**
- **Composition**: A named set of services with version and metadata
- **ServiceDef**: name, package, enable flag, dependencies, health check, restart policy, scaling hints, env vars, ports
- **StartupPlan**: Topologically sorted layers (Kahn's algorithm) — services in the same layer start in parallel

**Features:**
- Topological sort with cycle detection
- Layer-based parallel startup planning
- Auto-generates NixOS config snippets (maps service names to `services.*.enable`)
- Validation: circular dependencies, missing dependencies, duplicate names, missing health checks
- Systemd dependency injection (`after`, `requires`)

**Built-in service mapping:** nginx → `services.nginx`, postgresql → `services.postgresql`, redis → `services.redis`, etc.

**Endpoints:**
- `POST /api/compose` — plan/deploy/validate a composition
- `GET /api/compose/status` — runtime status of all compositions

**UI:** `/composer` — service catalog with toggle selection, 4 templates (Web Stack, Monitoring, Database Cluster, Full Stack), tabbed view for Startup Plan / Dependencies / NixOS Config / Warnings.

---

## Feature 23: Predictive Failure Detection

**File:** `evo/src/predict.rs`

Analyzes system metrics trends and predicts failures BEFORE they happen. Uses a simplified linear projection model.

**What it detects:**
- **Disk exhaustion**: Projects hours-to-full based on current usage (higher usage = faster assumed growth rate)
- **Memory pressure**: Alerts at >85% with OOM risk estimation
- **CPU overload**: Compares 1-minute load average against core count
- **Failed services**: Detects services in failed state

**Risk scoring:** 0-100 composite based on alert severity weights (critical: 25pts, warning: 15pts, info: 5pts) plus memory/disk thresholds.

**Alert structure:** Each alert includes severity, category, title, description, estimated time, metric trend (current value, rate, direction, projected hours), and recommended actions.

**Endpoint:** `GET /api/predict/alerts`

---

## Feature 24: NixOS Config Streaming

**File:** `evo/src/stream.rs`

WebSocket that streams real-time config file changes detected by a background file watcher.

**File watcher:**
- Polls `/etc/nixos/` and `/etc/nix/` every 5 seconds
- Watches `.nix` and `.conf` files
- Detects: created, modified, deleted, discovered (first seen) events
- Records file size and modification time

**Git integration:**
- Auto-fetches git commit info for changed files (`git log -1 --format=...`)
- Diff preview: first 500 chars of `git diff HEAD` against the file

**Broadcast architecture:**
- Uses `tokio::sync::broadcast` (1024 capacity)
- Supports multiple concurrent WebSocket clients
- Welcome message on connect with configuration info

**Endpoint:** `WS /api/stream/config`

---

## Feature 25: Cross-Distro Compatibility Layer

**File:** `evo/src/compat.rs`

Translates NixOS config concepts to other Linux distributions.

**Supported distros:** Ubuntu, Debian, Fedora, Arch, Alpine

**Translation engine:**
- **Package mapping**: NixOS package names → distro-specific names (e.g., `postgresql_16` → `postgresql-16` on Debian, `postgresql-server` on Fedora)
- **Systemd unit generation**: Creates `.service` files from NixOS service definitions with `After=`, `Type=forking`, `Restart=on-failure`
- **Install script generation**: Produces shell scripts with correct package manager (`apt-get`, `dnf`, `pacman`, `apk`) and `systemctl enable/start` commands
- **Service detection**: Parses `services.*.enable = true` patterns from NixOS config text

**9 mapped services:** nginx, postgresql, redis, openssh, mysql, caddy, docker, prometheus, grafana

**Endpoint:** `POST /api/compat/translate`

---

## Feature 26: System Health Score

**Files:** `evo/src/health_score.rs` + `evo/static/health.html`

Composite health score (0-100) for the entire system with letter grading.

**6 weighted factors:**

| Factor | Weight | Data Source | Thresholds |
|--------|--------|-------------|------------|
| Services | 25% | `systemctl --failed` | 0 failed = 100, 1-2 = warning, 3+ = critical |
| Disk Space | 20% | `df /` | <60% = 100, 60-80 = good, 80-90 = warning, >90 = critical |
| Memory | 15% | `free -b` | <70% = 100, 70-85 = warning, >85 = critical |
| Security | 15% | firewall, SSH, updates | Checks root login, password auth, unattended-upgrades |
| Config Quality | 10% | file checks, parse | Parse errors, generation count, flake presence |
| Update Freshness | 15% | `/run/current-system` age | <30d = 100, 30-90d = warning, >90d = critical |

**Scoring:** Weighted average → letter grade (A: 90+, B: 80+, C: 70+, D: 60+, F: <60)

**Trend history:** In-memory ring buffer of last 100 score points for time-series visualization.

**Endpoint:** `GET /api/health/score`

**UI:** `/health` — SVG gauge with animated ring, factor cards with colored bars, summary pills, SVG trend chart with area fill.

---

## Feature 27: MCP Server V4 Tools

**File:** `mcp-server/src/experimental-v4.ts`

Six new MCP tools wrapping all V4 features:

| Tool | Description | Agent Endpoint |
|------|-------------|----------------|
| `nix_doctor` | Diagnose NixOS errors from knowledge base | `POST /api/doctor/diagnose` |
| `compose_services` | Define multi-service compositions | `POST /api/compose` |
| `predict_alerts` | Predictive failure detection | `GET /api/predict/alerts` |
| `compat_translate` | NixOS → other distro translation | `POST /api/compat/translate` |
| `health_score` | Composite system health score | `GET /api/health/score` |
| `stream_config_status` | Config streaming WebSocket info | WS /api/stream/config |

---

## New API Endpoints Summary (V4)

| Endpoint | Method | Feature |
|----------|--------|---------|
| `/api/doctor/diagnose` | POST | Error diagnosis |
| `/api/doctor/knowledge` | GET | Knowledge base list |
| `/api/compose` | POST | Service composition |
| `/api/compose/status` | GET | Composition status |
| `/api/predict/alerts` | GET | Predictive alerts |
| `/api/stream/config` | WS | Config change stream |
| `/api/compat/translate` | POST | Cross-distro translation |
| `/api/health/score` | GET | Health score |
| `/doctor` | GET | Doctor UI |
| `/composer` | GET | Composer UI |
| `/health` | GET | Health score UI |

---

## Complete Feature Count

| Version | Features | Endpoints | MCP Tools | HTML UIs |
|---------|----------|-----------|-----------|----------|
| V0.1 | 8 core | 8 | 0 | 0 |
| V1 | 5 | 6 | 4 | 1 |
| V2 | 6 | 12 | 7 | 3 |
| V3 | 7 | 14 | 8 | 2 |
| V4 | 7 | 11 | 6 | 3 |
| **Total** | **33** | **~51** | **25** | **9** |

## Architecture Notes (V4)

- Doctor uses `OnceLock<Vec<DiagnosisEntry>>` for immutable knowledge base — zero allocations after init
- Composer implements Kahn's algorithm for topological sort with cycle detection
- Predict uses simplified linear projection (sufficient for "will be full in X hours" estimation)
- Stream uses `tokio::sync::broadcast` with 1024 capacity for multi-client WebSocket fan-out
- Compat generates shell scripts and systemd units from structured data — no template engine needed
- Health score uses `RwLock<Vec<ScoreHistory>>` for thread-safe trend recording
- All V4 modules follow existing axum + tokio + serde patterns from V1-V3
- No new external dependencies added — all V4 features use existing Cargo.toml deps

---

## V5 Features — The Frontier (6 features, 20 endpoints, 12 MCP tools, 1 HTML UI)

### Feature 1: 🧬 Nix Config DNA — Genetic Optimization

**File:** `evo/src/dna.rs`

Treats NixOS configuration as DNA — individual config options are "genes" that can be mutated, crossed over, and selected through genetic algorithms.

**How it works:**
- Each config option is a `Gene` with name, value (Bool/Int/Float/String/List), category, and mutability flag
- A `Genome` is a complete configuration — a collection of genes with a fitness score
- `FitnessScore` evaluates four objectives: build speed, disk size, security, boot time
- `DnaEngine` runs genetic operations: mutation (random value changes), crossover (combine two parents), elite selection (keep top N)
- Population seeded from a base config, then evolved for N generations

**Endpoints:**
- `POST /api/dna/evolve` — Run evolution with optional seed genes and config
- `GET /api/dna/population` — View current population and fitness scores

**MCP Tools:** `dna_evolve`, `dna_population`

---

### Feature 2: 🎭 NixOS Config Theater — Replay & Undo

**File:** `evo/src/theater.rs`

Every config change is a "scene" in a play. Replay the entire history, undo any single scene, or branch into alternative timelines.

**How it works:**
- Each config change creates a `Scene` with diff, author, timestamp, and act number
- `replay()` steps through scenes chronologically, computing cumulative diffs
- `undo_scene()` removes a scene and generates its inverse diff — surgical undo, not just rollback
- `branch()` forks at any scene to create "what if?" alternative timelines
- Branches can have their own scenes and be compared for divergence

**Endpoints:**
- `POST /api/theater/record` — Record a config change as a scene
- `GET /api/theater/replay` — Replay scenes (with optional range)
- `POST /api/theater/undo` — Undo a single scene by ID
- `POST /api/theater/branch` — Create alternative timeline
- `GET /api/theater/branches` — List all branches

**MCP Tools:** `theater_record`, `theater_replay`, `theater_undo`, `theater_branch`

---

### Feature 3: 🔗 Nix Config Blockchain

**File:** `evo/src/chain.rs`

Hash-chained config change history for tamper-evident audit trails. Not real blockchain — just SHA-256 hash chaining for integrity verification.

**How it works:**
- Each config change is a `Block` containing action, description, and SHA-256 hash of previous block
- Genesis block bootstraps the chain
- `verify()` walks the entire chain, checking hash integrity and link continuity
- Blocks include metadata: author, generation, diff summary, config snapshot hash

**Endpoints:**
- `GET /api/chain/verify` — Verify entire chain integrity
- `GET /api/chain/history` — Get chain history with optional filters
- `POST /api/chain/add` — Add a new block

**MCP Tools:** `chain_verify`, `chain_history`

---

### Feature 4: 🌊 Real-Time Collaborative Config Editing

**File:** `evo/src/collab.rs`

WebSocket-based collaborative editing with Operational Transformation — like Google Docs for NixOS config files.

**How it works:**
- `OTEngine` implements Operational Transformation for Insert, Delete, and Cursor operations
- Transform functions ensure convergence: two concurrent edits applied in either order produce the same result
- `CollabSession` manages document state, revision tracking, peer cursors, and broadcast
- Each peer gets a unique color for cursor visualization
- WebSocket protocol: init → sync → operation/cursor messages

**Endpoints:**
- `WS /api/collab/ws` — Collaborative editing WebSocket

**MCP Tools:** `collab_info`

---

### Feature 5: 🎯 Config Benchmarking Suite

**File:** `evo/src/bench.rs`

Measure the impact of config changes with statistical rigor — boot time, build time, disk size, security score, memory usage.

**How it works:**
- Runs real system commands: `systemd-analyze time`, `nixos-rebuild dry-build`, `du -sm /nix/store`, `iptables -L`, `free -m`
- Collects metrics with stddev, min/max, and 95% confidence intervals
- `compare()` computes delta % between two runs, determines direction (better/worse), and flags statistical significance
- Overall grading: A+ through F based on combined security + performance score

**Endpoints:**
- `POST /api/bench/run` — Run benchmarks
- `GET /api/bench/results` — Get all benchmark results
- `GET /api/bench/compare` — Compare two runs

**MCP Tools:** `bench_run`, `bench_compare`

---

### Feature 6: 🗺️ NixOS Topology Map

**Files:** `evo/src/topology.rs` + `evo/static/topology.html`

Auto-discover all services and visualize them as an interactive network topology map.

**How it works:**
- Discovers services via `systemctl list-units`, ports via `ss -tlnp`, connections via `ss -tn`
- Classifies services: Database, ReverseProxy, Cache, Queue, Storage, Network
- Infers dependencies from port connections
- SVG-based visualization with physics simulation (repulsion + attraction forces)
- Drag-and-drop nodes, hover tooltips, color-coded types, real-time stats

**Endpoints:**
- `GET /api/topology` — Full topology (nodes + edges)
- `GET /api/topology/services` — Services only
- `GET /api/topology/connections` — Connections only

**UI:** `http://localhost:7890/topology`

**MCP Tools:** `topology_map`

---

## Version Summary (Updated)

| Version | Features | Endpoints | MCP Tools | HTML UIs |
|---------|----------|-----------|-----------|----------|
| V0.1 | 8 core | 8 | 0 | 0 |
| V1 | 5 | 6 | 4 | 1 |
| V2 | 6 | 12 | 7 | 3 |
| V3 | 7 | 14 | 8 | 2 |
| V4 | 7 | 11 | 6 | 3 |
| **V5** | **6** | **20** | **12** | **1** |
| **Total** | **39** | **~71** | **37** | **10** |

## Architecture Notes (V5)

- DNA engine uses `LazyLock<Arc<DnaEngine>>` for zero-cost global state
- Theater implements surgical undo via inverse diff generation — not full rollback
- Chain uses SHA-256 (sha2 crate) with configurable difficulty for integrity proofs
- Collab OT engine handles all 6 transform cases: insert×insert, insert×delete, delete×delete
- Bench uses real system commands — results reflect actual system state
- Topology SVG renderer implements force-directed layout with damping
- All V5 modules use existing axum + tokio + serde + chrono deps — no new Cargo dependencies
- MCP V5 tools provide 12 new AI agent capabilities across all 6 V5 features

---

# V6 Features — The Strangest Ideas That Might Actually Work (6 features, 19 endpoints, 10 MCP tools, 1 HTML UI)

### Feature 1: 🕰️ Time-Travel Debugging

**File:** `evo/src/timetravel.rs`

Record system state snapshots at intervals and travel back in time. "Show me what the system looked like 3 hours ago."

**How it works:**
- Captures full system state: services (via systemctl), disk (via df), memory (via free), network (via ip -j addr + ss), packages (via nix-store), config hash
- Each snapshot gets a unique ID and timestamp
- `diff()` compares any two snapshots: service status changes, package additions/removals, disk growth, memory delta, port changes, config file modifications
- `replay()` produces a time-series of frames for a date range — see the system's health trajectory

**Endpoints:**
- `POST /api/timetravel/snapshot` — Capture a snapshot (with optional label)
- `GET /api/timetravel/snapshots` — List all snapshots
- `GET /api/timetravel/diff?from=ID&to=ID` — Compare two snapshots
- `GET /api/timetravel/replay?from=EPOCH&to=EPOCH&limit=100` — Replay sequence

**MCP Tools:** `timetravel_snapshot`, `timetravel_diff`, `timetravel_replay`

---

### Feature 2: 🎲 Chaos Engineering for NixOS

**File:** `evo/src/chaos.rs` (extended) + `evo/static/chaos.html`

Controlled fault injection to test system resilience. Break things on purpose, measure recovery.

**Predefined scenarios:**
| Scenario | Action | Risk |
|----------|--------|------|
| Service Kill & Recover | `systemctl stop <svc>` | Medium |
| Network Partition | `iptables DROP` rules | High |
| Disk Pressure | `dd` fill to 95% | High |
| CPU Stress | `stress --cpu N` | Low |
| Config Corruption | Modify config, test drift | Medium |

**New in V6:**
- `POST /api/chaos/start` — Start from predefined scenario with optional overrides
- `GET /api/chaos/report` — Resilience score (0-100), experiment history, avg recovery time

**UI:** `/chaos` — Resilience score ring, clickable scenario cards, live experiment log, history table.

**MCP Tools:** `chaos_start`, `chaos_report`

---

### Feature 3: 🧩 Nix Config Pattern Library

**File:** `evo/src/patterns.rs`

Curated collection of 10 common NixOS patterns with explanations, Nix code, and search.

**Patterns included:**
| Pattern | Category | Difficulty | Security |
|---------|----------|------------|----------|
| Nginx Reverse Proxy | WebServer | Beginner | Standard |
| PostgreSQL Database | Database | Beginner | Standard |
| Basic Firewall | Security | Beginner | Standard |
| Docker/Podman Containers | Containers | Intermediate | Standard |
| Prometheus + Grafana | Monitoring | Intermediate | Standard |
| Hardened SSH | Security | Advanced | Hardened |
| WireGuard VPN | Networking | Advanced | Hardened |
| ZFS Storage Pool | Storage | Expert | Standard |
| Dev Shell + Direnv | Development | Beginner | Minimal |
| ACME SSL Certificates | Security | Beginner | Standard |

Each pattern includes: plain-English explanation, ready-to-use Nix config, use cases, tags, and dependency references.

**Endpoints:**
- `GET /api/patterns?q=...&category=...&difficulty=...&security=...` — Search patterns
- `GET /api/patterns/:id` — Full pattern with Nix code

**MCP Tools:** `patterns_search`, `patterns_detail`

---

### Feature 4: 🔮 Config Impact Analyzer

**File:** `evo/src/impact.rs`

Before applying a config change, analyze what WILL break. Transitive dependency analysis with BFS traversal.

**How it works:**
- Built-in dependency graph maps NixOS option relationships (nginx→firewall, SSH→fail2ban, PostgreSQL→dependents, etc.)
- BFS traversal finds transitive effects — changing nginx port affects firewall, monitoring, and potentially DNS
- Special case analysis for port changes (detects stale firewall rules)
- Generates required cascading changes and risk assessment

**Risk levels:** Low (option changes only), Medium (warnings), High (breaking changes detected)

**Endpoint:** `POST /api/impact/analyze`

**MCP Tool:** `impact_analyze`

---

### Feature 5: 🌍 Distributed Config Sync

**File:** `evo/src/dist_sync.rs`

Sync configs across a fleet of NixOS servers with CRDT-inspired conflict resolution.

**How it works:**
- Version vector causal ordering (node_id → sequence number)
- Operation log: Set, Delete, Append operations with metadata
- Last-write-wins conflict detection with conflict reporting
- Fleet status: node count, in-sync detection, config hash comparison

**Endpoints:**
- `POST /api/sync/init` — Register a node in the sync group
- `POST /api/sync/push` — Push config operations from a node
- `GET /api/sync/status` — Fleet status and sync state
- `GET /api/sync/config` — Merged config across all nodes

**MCP Tool:** `sync_status`

---

### Feature 6: 📱 Mobile-First API Responses

**File:** `evo/src/mobile.rs`

Ultra-compact JSON for mobile clients. Single-character field names minimize bandwidth.

**Compact format example:**
```json
{"h":"myserver","s":"o","u":86400,"m":45.2,"d":62.1,"l":0.85,"f":0,"fs":[],"ts":1712900000,"v":1}
```
(s=status: o=ok, w=warning, c=critical; m=memory%, d=disk%, l=load)

**Features:**
- Push notification alert store with acknowledge/subscribe
- Offline-first sync protocol with change tracking tokens
- Alert levels: info, warning, critical

**Endpoints:**
- `GET /api/mobile/status` — Ultra-compact system status
- `GET /api/mobile/alerts?unack=true` — Alert list
- `POST /api/mobile/alerts/ack` — Acknowledge an alert
- `POST /api/mobile/subscribe` — Subscribe to push notifications
- `GET /api/mobile/sync?token=...` — Offline sync with change delta

**MCP Tool:** `mobile_status`

---

### Feature 7: MCP Server V6 Tools

**File:** `mcp-server/src/experimental-v6.ts`

10 new MCP tools wrapping all V6 features:

| Tool | Description | Agent Endpoint |
|------|-------------|----------------|
| `timetravel_snapshot` | Capture system state | `POST /api/timetravel/snapshot` |
| `timetravel_diff` | Compare two snapshots | `GET /api/timetravel/diff` |
| `timetravel_replay` | Replay snapshot sequence | `GET /api/timetravel/replay` |
| `chaos_start` | Start chaos experiment | `POST /api/chaos/start` |
| `chaos_report` | Resilience score & history | `GET /api/chaos/report` |
| `patterns_search` | Search NixOS patterns | `GET /api/patterns` |
| `patterns_detail` | Pattern with Nix code | `GET /api/patterns/:id` |
| `impact_analyze` | Predict change impact | `POST /api/impact/analyze` |
| `sync_status` | Distributed sync status | `GET /api/sync/status` |
| `mobile_status` | Compact mobile status | `GET /api/mobile/status` |

---

## Version Summary (Updated)

| Version | Features | Endpoints | MCP Tools | HTML UIs |
|---------|----------|-----------|-----------|----------|
| V0.1 | 8 core | 8 | 0 | 0 |
| V1 | 5 | 6 | 4 | 1 |
| V2 | 6 | 12 | 7 | 3 |
| V3 | 7 | 14 | 8 | 2 |
| V4 | 7 | 11 | 6 | 3 |
| V5 | 6 | 20 | 12 | 1 |
| **V6** | **6** | **19** | **10** | **1** |
| **Total** | **45** | **~90** | **47** | **11** |

## Architecture Notes (V6)

- Timetravel uses `OnceLock<TimeTravelEngine>` with `RwLock<Vec<Snapshot>>` for thread-safe snapshot storage (max 1000, FIFO eviction)
- Chaos extends existing `ChaosEngine` with scenario-based start and aggregate reporting
- Patterns are compile-time built-in (no database) — 10 patterns with full Nix code, searchable by query/category/difficulty/security
- Impact analyzer uses BFS over a hand-coded dependency graph — covers nginx, PostgreSQL, SSH, firewall, DNS, kernel, users, state version
- Dist sync uses version vectors for causal ordering — last-write-wins with conflict reporting
- Mobile API uses single-char JSON keys to minimize bandwidth (typical response: ~150 bytes)
- All V6 modules use existing Cargo.toml deps — no new dependencies
- MCP V6 tools provide human-readable formatted text + structuredContent for all 10 tools
