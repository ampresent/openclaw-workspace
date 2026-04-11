# nix-evo Iteration Summary

> 2026-04-12 — Subagent iteration run (Rounds 1-19)

## Overview

Completed 19 rounds of improvements to the nix-evo project (NixOS management tool for AI agents).
Grew from 5 endpoints to 39 endpoints, 13 MCP tools, 3 NixOS modules.

## Commits

| # | Commit | Description |
|---|--------|-------------|
| 1 | `0122d54` | Agent robustness (error types, auth, validation) |
| 2 | `75ff67a` | MCP server completeness (hosts.toml, formatting) |
| 3 | `8326023` | Testing, quality, NixOS module updates |
| 4 | `db0a6d7` | Documentation (QUICKSTART, DESIGN updates) |
| 5 | `f88785d` | v0.2 features (SSH tunnel, design docs) |
| 6 | `4498885` | Status updates |
| 7 | (rounds 6-12) | Tests, AI config, backup, community, security, integrations |
| 8 | `cc63171` | **Round 13**: Docker & Container Integration |
| 9 | `2b77678` | **Round 14**: CI/CD Pipeline Design |
| 10 | `ac63cbd` | **Round 15**: Observability Stack |
| 11 | `9dbb067` | **Round 16**: NixOS Module Ecosystem |
| 12 | `dc9e6af` | **Round 17**: Developer Experience |
| 13 | `1cc7de8` | **Round 18**: API Versioning & Stability |
| 14 | `681cb06` | **Round 19**: Smart Advisor (Rollback + Capacity) |

## Rounds 1-5 (Subagent 1)

See original section above for details.

## Rounds 6-12 (Subagent 2)

See original section above for details.

## Round 13: Docker & Container Integration

### `evo/src/docker.rs` (472 lines)
- **Container discovery**: `docker ps -a` with structured parsing
- **Compose stack listing**: `docker compose ls` with fallback to v1
- **13 NixOS alternatives**: nginx, postgres, redis, grafana, prometheus, caddy, mysql, mongodb, nextcloud, jellyfin, gitea, vaultwarden, minio
- **Image matching**: registry-aware (`ghcr.io/nginx/nginx` → matches `nginx`)
- **Migration difficulty ratings**: easy / moderate / hard per service

### Endpoints
- `GET /api/docker/status` — Full Docker environment: containers, compose stacks, NixOS suggestions
- `POST /api/docker/compose-validate` — Validate compose file, suggest NixOS alternatives for each service

## Round 14: CI/CD Pipeline Design

### `evo/src/cicd.rs` (479 lines)
- **Git webhook receiver**: Handles push, pull_request, merge_request events
- **Auto-detection**: Checks if commits include `.nix` or `nixos` changes
- **CI validation pipeline**: dry-build → VM test check → config diff
- **Preview deployments**: Config validation before apply
- **Deployment tracking**: Records persisted to `/var/lib/nix-evo/deployments/`

### Endpoints
- `POST /api/cicd/webhook` — Receive Git webhook events
- `POST /api/cicd/preview-deploy` — Trigger preview deployment
- `GET /api/cicd/deployments` — List recent deployments (last 50)
- `GET /api/cicd/deployments/:id` — Deployment details

## Round 15: Observability Stack

### `evo/src/observability.rs` (654 lines)
- **Structured logging**: journald → JSON with priority, unit, PID, hostname filtering
- **Prometheus metrics**: Text exposition format with CPU, memory, disk, generation, failed services
- **Alert system**: 4 default rules (disk>90%, memory>90%, service failures, CPU>95%)
- **Alert evaluation**: Threshold-based with operators (gt, lt, eq, ne, gte, lte)
- **Integration config**: Grafana/Loki/Prometheus connection settings

### Endpoints
- `POST /api/observability/logs` — Query journald with filters (unit, priority, time, search, limit)
- `GET /api/observability/metrics` — Prometheus-compatible metrics
- `GET /api/observability/alerts` — List alert rules and active alerts
- `POST /api/observability/alerts/check` — Evaluate alert conditions
- `POST /api/observability/alerts/rules` — Add/update alert rules
- `GET /api/observability/config` — Integration configs

## Round 16: NixOS Module Ecosystem

### `evo/nix/modules/nextcloud.nix`
- PostgreSQL + Redis + Nginx with auto-SSL
- Collabora Online integration option
- Periodic backup (pg_dump + tar)
- Security headers, max upload config
- nix-evo metadata for service discovery

### `evo/nix/modules/jellyfin.nix`
- Hardware transcoding (VA-API/VDPAU) support
- Nginx reverse proxy with WebSocket
- Media directory management
- Periodic library scan timer

### `evo/nix/modules/monitoring-stack.nix`
- Prometheus with 5 alert rules (disk, memory, service, CPU, generation change)
- Node exporter with systemd/processes collectors
- Loki + Promtail for log aggregation
- Grafana with auto-provisioned datasources
- Optional Alertmanager with webhook routing

## Round 17: Developer Experience

### `evo/src/dev.rs` (270 lines)
- **Dev mode toggle**: Switch between real and mock system
- **Mock services**: Set service states via API
- **Mock generations**: Simulate config applies
- **Mock snapshot**: Realistic test data without NixOS
- **Isolated state**: All data in `/tmp/nix-evo-dev/`

### `HOW-TO-CONTRIBUTE.md`
- Project structure walkthrough
- Step-by-step: adding endpoints, error handling, testing
- Dev mode usage guide
- MCP server development
- Code style guidelines

## Round 18: API Versioning & Stability

### `evo/src/api_version.rs`
- Version registry: v1 (stable) + v2 (beta)
- URL extraction: `/api/v1/...`, `/api/v2/...`, or unversioned
- Deprecation headers: `Deprecation`, `Sunset`, `X-API-Deprecation-Warning`
- Version discovery endpoint

### `API-STABILITY.md`
- Breaking change definitions
- 90-day deprecation policy
- Response header conventions
- Migration guide (v0 → v1)
- Semantic versioning commitment

## Round 19: Smart Advisor (Cherry-picked from feature/experimental)

### `evo/src/advisor.rs` (470 lines)
- **Rollback advisor**: Scores generations by recency, stability, service coverage
  - Detects current issues (failed services, disk, critical services)
  - Returns ranked candidates with confidence scores
- **Capacity planner**: Disk/memory/CPU analysis with risk levels
  - Nix store size + GC savings estimate
  - Actionable recommendations

### Endpoints
- `POST /api/advisor/rollback` — Smart rollback recommendation
- `GET /api/advisor/capacity` — System capacity analysis

## New Files Created (Rounds 13-19)

| File | Lines | Description |
|------|-------|-------------|
| evo/src/docker.rs | 472 | Docker container + compose integration |
| evo/src/cicd.rs | 479 | CI/CD webhooks + preview deployments |
| evo/src/observability.rs | 654 | Logs, metrics, alerting |
| evo/src/dev.rs | 270 | Dev mode with mock system |
| evo/src/api_version.rs | 220 | API version management |
| evo/src/advisor.rs | 470 | Rollback advisor + capacity planner |
| evo/nix/modules/nextcloud.nix | 220 | Nextcloud NixOS module |
| evo/nix/modules/jellyfin.nix | 185 | Jellyfin NixOS module |
| evo/nix/modules/monitoring-stack.nix | 340 | Monitoring stack NixOS module |
| HOW-TO-CONTRIBUTE.md | 175 | Developer guide |
| API-STABILITY.md | 100 | API stability guarantees |

## Final Stats

- **API Endpoints**: 39 (from 5 in v0.1)
- **MCP Tools**: 13
- **NixOS Modules**: 3
- **Rust source files**: 16 (main + 15 modules)
- **Alert rules**: 4 default + custom support
- **NixOS alternatives**: 13 known Docker→NixOS mappings
