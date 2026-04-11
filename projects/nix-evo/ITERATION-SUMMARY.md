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
