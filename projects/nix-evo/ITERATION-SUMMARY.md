# nix-evo Iteration Summary

> 2026-04-12 — Subagent iteration run

## Overview

Completed 5 rounds of improvements to the nix-evo project (NixOS management tool for AI agents).

## Commits (6 total)

1. `0122d54` — **Round 1: Agent robustness**
2. `75ff67a` — **Round 2: MCP server completeness**
3. `8326023` — **Round 3: Testing, quality, NixOS module updates**
4. `db0a6d7` — **Round 4: Documentation updates**
5. `f88785d` — **Round 5: v0.2 features (SSH tunnel, design docs)**
6. `4498885` — **Update STATUS.md and TODO.md**

## Round 1: Agent Robustness (Rust)

### Error Types (`error.rs`)
- Added `AppError` enum with 6 typed variants: `CommandFailed`, `IoError`, `Validation`, `NotFound`, `Unauthorized`, `Internal`
- Implements `IntoResponse` for structured JSON error responses with HTTP status codes
- All error messages in Chinese for end users

### Auth Middleware (`auth.rs`)
- Bearer token authentication via `--api-token` CLI flag or `NIX_EVO_TOKEN` env var
- Only protects `/api/*` endpoints, `/health` stays public
- When no token configured, all requests allowed through (local dev scenario)

### Dry-Build Parsing Improvements (`config_validate.rs`)
- Multi-strategy dry-build invocation: flake → no-flake → basic → impure
- Robust package name extraction from `/nix/store/hash-pkg-version` paths
- Handles `will be built`, `will be fetched`, arrow notation, and section markers

### Input Validation
- Path traversal protection in `config_read` (must be under `/etc/nixos/`)
- Unit name validation in `service_logs` (reject control characters, semicolons)
- Empty field checks in `package_info`, `config_validate`, `config_apply`

### Other
- Generation description reading from `nix-evo-description` files
- Fixed memory parsing for different `free -h` output versions
- Router restructured with `/api` nesting

## Round 2: MCP Server Completeness (TypeScript)

### hosts.toml Parsing
- Minimal inline TOML parser for `[hosts.name]` sections
- Supports `url`, `token`, `ssh_tunnel`, `description` fields
- Fallback to env vars when no hosts.toml found
- Config path: `~/.config/nix-evo/hosts.toml`

### Host Selection Logic
- Priority: explicit `host` param → `default` → single-host auto
- Error message lists available hosts when selection is ambiguous

### Human-Readable Formatting
- `system_snapshot`: Emoji summary with failed services, disk warnings, memory
- `config_validate`: Risk assessment card with package changes
- `generation_diff`/`rollback_list`: Timeline format
- All tools: formatted summary + raw JSON block

## Round 3: Testing & Quality

### Unit Tests
- `config_validate.rs`: 5 tests for risk assessment (safe/moderate/dangerous scenarios)
- `config_validate.rs`: 2 tests for dry-build package parsing
- `mcp-server/src/index.test.ts`: 4 tests for hosts.toml parsing

### NixOS Module Updates
- `tokenFile` option for secure token injection
- Added `/nix/var/nix/profiles` to ReadWritePaths for generation descriptions
- Added `hostname` and `utillinux` to PATH dependencies

## Round 4: Documentation

### QUICKSTART.md
- 5-step setup guide: install agent, install MCP server, configure hosts, connect Claude Code, use
- Multi-host configuration examples
- SSH tunnel setup instructions
- Troubleshooting section

### DESIGN.md Updates (Section 八)
- Error type table with HTTP status codes
- Auth mechanism description
- Dry-build parsing strategy documentation
- Risk assessment scoring table
- hosts.toml format spec
- Generation description storage design
- MCP output format documentation

## Round 5: v0.2 Features

### SSH Tunnel Auto-Setup (`ssh-tunnel.ts`)
- Parse `ssh_tunnel` config (user@host:port format)
- Auto-establish SSH tunnels with available port discovery
- 10-second connection timeout
- Port readiness waiting with retry
- Cleanup on SIGTERM/SIGINT
- Integrated into MCP server's request flow

### DESIGN-V0.2.md
- **SSH tunnel**: Full design with implementation notes
- **test-before-switch**: `nixos-rebuild test` → wait → `switch` workflow, `config_test` endpoint design
- **Secrets management**: agenix/sops-nix integration design with security constraints

## What's Next

### v0.1 Remaining
- Generate `Cargo.lock` (needs NixOS machine)
- Integration tests on real NixOS system

### v0.2 Implementation
- `config_test` endpoint (test-before-switch)
- Secrets management CRUD endpoints
- TUI dashboard

### v0.3
- Multi-host orchestration
- Webhook triggers
- Config templates/shared modules

## Additional Rounds (Subagent 2 — Rounds 6-12)

### Round 6: MCP Tool Routing Tests
- Created `mcp-server/tests/tools.test.ts` with 30+ test cases
- Tests: host resolution (5), request construction (5), tool validation (5), response formatting (4), tool routing matrix (6), error handling (3)
- Validates all 9 original tools have correct method/path mappings

### Round 7: Performance & Observability
- Enhanced health endpoint: includes NixOS detection, uptime, version
- Command timeout support: `run_cmd_with_timeout` with configurable 120s default
- Request tracking infrastructure in cmd/mod.rs

### Round 8: AI Configuration Generation
- `evo/src/ai_config.rs`: Template-based NixOS config from natural language
- 9 built-in patterns: nginx, docker, ssh, firewall, postgresql, redis, node, python, borgbackup
- Each pattern includes config snippet, explanation, packages, services, risk level
- Keyword scoring for best-match selection
- `mcp-server/src/ai-config.ts`: MCP-side pattern matching and LLM prompt builder
- New MCP tool: `config_generate`

### Round 9: Backup & Disaster Recovery
- `evo/src/backup.rs`: Full backup system
- Snapshot /etc/nixos before applies
- Backup rotation: 20 auto + 50 manual max
- Restore with safety backup (creates backup before restore)
- Dry-run preview mode
- New endpoints: GET /api/backups, POST /api/backup/create, POST /api/backup/restore, POST /api/backup/rotate
- New MCP tools: backup_list, backup_create, backup_restore

### Round 10: Community & Ecosystem
- `CONTRIBUTING.md`: Development setup, code style, PR guidelines
- `evo/nix/README.md`: NixOS module installation guide with all options
- `NIXPKGS-PR-TEMPLATE.md`: Draft nixpkgs submission template
- `examples/nginx-setup.sh`, `examples/docker-setup.sh`: Usage examples

### Round 11: Security & Test-Before-Switch
- `evo/src/cmd/config_test.rs`: Test-before-switch endpoint
  - Runs `nixos-rebuild test` (reversible by reboot)
  - Optional auto-switch after configurable delay (default 5 min)
  - Cancel endpoint for stopping auto-switch
  - Records [TEST] prefix in generation descriptions
- `evo/src/tls.rs`: TLS configuration structure (design + stub)
- `SECURITY.md`: Comprehensive threat model and hardening roadmap

### Round 12: Integration Roadmap
- `INTEGRATIONS.md`: Multi-host orchestration, Docker, systemd-nspawn, Kubernetes, monitoring, CI/CD
- `examples/docker-compose-monitoring.yml`: Monitoring stack example

## New Files Created (Rounds 6-12)

| File | Description |
|------|-------------|
| mcp-server/tests/tools.test.ts | MCP tool routing tests (30+ tests) |
| evo/src/ai_config.rs | AI config generation (9 patterns) |
| evo/src/backup.rs | Backup & disaster recovery |
| evo/src/cmd/config_test.rs | Test-before-switch endpoint |
| evo/src/tls.rs | TLS config structure |
| mcp-server/src/ai-config.ts | MCP-side AI config |
| CONTRIBUTING.md | Developer guide |
| SECURITY.md | Threat model & hardening |
| INTEGRATIONS.md | Integration roadmap |
| evo/nix/README.md | NixOS module guide |
| NIXPKGS-PR-TEMPLATE.md | nixpkgs submission template |
| examples/nginx-setup.sh | Nginx setup example |
| examples/docker-setup.sh | Docker setup example |
| examples/docker-compose-monitoring.yml | Monitoring stack example |

## Updated Files

| File | Changes |
|------|---------|
| evo/src/main.rs | Added ai_config, backup, config_test modules; new routes |
| evo/src/cmd/mod.rs | Added config_test, ai_config modules; health handler; timeout support |
| mcp-server/src/index.ts | Added config_generate, backup_list, backup_create, backup_restore tools + formatters |
| LOG.md | Updated with rounds 6-12 |
| STATUS.md | Updated to v0.3 with full endpoint/tool inventory |

## What's Next (v0.4+)

- TLS implementation (rustls integration)
- Multi-host orchestration
- Real LLM integration for config generation
- Docker/systemd-nspawn container management
- Prometheus metrics export
- CI/CD webhook triggers
- Rate limiting middleware
