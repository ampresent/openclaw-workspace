# How to Contribute to nix-evo

## Quick Start

```bash
# Clone and enter
git clone https://github.com/ampresent/openclaw-workspace.git
cd openclaw-workspace/projects/nix-evo

# Build the Rust agent
cd evo
cargo build

# Install MCP server dependencies
cd ../mcp-server
npm install
```

## Project Structure

```
nix-evo/
├── evo/                         # Rust agent (HTTP API server)
│   ├── src/
│   │   ├── main.rs              # Entry point, router setup
│   │   ├── config.rs            # CLI config parsing
│   │   ├── error.rs             # Error types (AppError)
│   │   ├── auth.rs              # Bearer token auth middleware
│   │   ├── cmd/                 # API handlers (one file per endpoint)
│   │   │   ├── mod.rs           # Shared helpers (run_cmd, health)
│   │   │   ├── system_snapshot.rs
│   │   │   ├── config_validate.rs
│   │   │   └── ...
│   │   ├── ai_config.rs         # AI config generation
│   │   ├── backup.rs            # Backup/restore system
│   │   ├── docker.rs            # Docker integration
│   │   ├── cicd.rs              # CI/CD webhooks
│   │   ├── observability.rs     # Logs, metrics, alerts
│   │   └── dev.rs               # Dev mode / mocking
│   ├── nix/
│   │   ├── default.nix          # Nix build
│   │   └── modules/             # NixOS service modules
│   └── Cargo.toml
├── mcp-server/                  # TypeScript MCP server
│   ├── src/
│   │   ├── index.ts             # MCP tools + HTTP client
│   │   ├── ai-config.ts         # AI config prompt builder
│   │   └── ssh-tunnel.ts        # SSH tunnel management
│   ├── tests/
│   │   └── tools.test.ts        # Tool routing tests
│   └── package.json
├── examples/                    # Usage examples
├── CONTRIBUTING.md              # General contribution guide
├── DESIGN.md                    # Architecture decisions
├── SECURITY.md                  # Threat model
└── README.md
```

## Development Workflow

### 1. Set Up Dev Mode

nix-evo has a built-in dev mode that simulates NixOS commands without a real system:

```bash
# Start agent in dev mode
cargo run -- --dev

# Or toggle via API
curl -X POST http://localhost:3030/api/dev/mode \
  -H 'Content-Type: application/json' \
  -d '{"enabled": true, "mock_system": true, "mock_data_dir": "/tmp/nix-evo-dev"}'
```

Dev mode features:
- **Mock systemctl** — set service states via API
- **Mock nixos-rebuild** — simulate config applies, generates fake generations
- **Mock system snapshot** — returns realistic test data
- **Isolated state** — all mock data in `/tmp/nix-evo-dev/`

### 2. Adding a New API Endpoint

1. Create a handler file in `evo/src/cmd/`:

```rust
// evo/src/cmd/my_feature.rs
use axum::{extract::State, Json};
use std::sync::Arc;
use crate::AppState;
use crate::error::AppError;

#[derive(serde::Deserialize)]
pub struct MyRequest { /* ... */ }

#[derive(serde::Serialize)]
pub struct MyResponse { /* ... */ }

pub async fn handle(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MyRequest>,
) -> Result<Json<MyResponse>, AppError> {
    // Implementation
}
```

2. Register the module in `evo/src/cmd/mod.rs`:
```rust
pub mod my_feature;
```

3. Add the route in `evo/src/main.rs`:
```rust
.route("/my-feature", post(my_feature::handle))
```

4. Add MCP tool in `mcp-server/src/index.ts` (if user-facing)

### 3. Error Handling

Always use `AppError` variants, never return raw strings:

```rust
Err(AppError::Validation {
    field: "hostname".into(),
    message: "主机名不能为空".into(),
})?;

Err(AppError::CommandFailed {
    command: "nixos-rebuild".into(),
    message: format!("执行失败: {e}"),
})?;
```

### 4. Running Tests

```bash
# Rust unit tests
cd evo && cargo test

# MCP server tests
cd mcp-server && npm test

# All tests
make test  # (if Makefile exists)
```

### 5. Code Style

- **Rust**: Follow `rustfmt` defaults. Run `cargo fmt` before committing.
- **TypeScript**: Use ESLint config. Run `npm run lint`.
- **Chinese**: Error messages and user-facing strings in Chinese.
- **Naming**: API endpoints use kebab-case (`/config/validate`), Rust uses snake_case.

### 6. Commit Messages

```
nix-evo: Short description

Longer explanation if needed.

- Detail 1
- Detail 2
```

### 7. Testing Without NixOS

The dev mock system (`dev.rs`) lets you test the full API stack on any machine:

```bash
# 1. Enable dev mode
curl -X POST localhost:3030/api/dev/mode -d '{"enabled":true}'

# 2. Use mock endpoints
curl localhost:3030/api/dev/mock/snapshot  # Fake system snapshot

# 3. Simulate config apply
curl -X POST localhost:3030/api/dev/mock/generation \
  -d '{"description":"test nginx setup"}'

# 4. Reset
curl -X POST localhost:3030/api/dev/mock/reset
```

### 8. Adding NixOS Modules

Modules go in `evo/nix/modules/`. Each module should:

1. Use `services.nix-evo.<name>` namespace
2. Provide sensible defaults
3. Include `enable` option
4. Emit nix-evo metadata JSON to `/etc/nix-evo/services/`
5. Integrate with Nginx for reverse proxy when applicable

### 9. MCP Server Development

```bash
cd mcp-server
npm run build    # Compile TypeScript
npm run watch    # Watch mode
npm test         # Run tests

# Test with Claude Code
claude mcp add nix-evo-dev -- node dist/index.js --hosts-config ./test-hosts.toml
```

## Architecture Decisions

See [DESIGN.md](DESIGN.md) for detailed architecture docs. Key principles:

- **HTTP API first**: All functionality exposed as REST endpoints
- **MCP as thin client**: MCP server is a translation layer, not business logic
- **Shell-out to NixOS**: Use `nixos-rebuild`, `journalctl`, `systemctl` — don't reimplement
- **Security by default**: Auth middleware, path validation, command timeouts
- **Chinese i18n**: User-facing messages in Chinese

## Getting Help

- File issues on GitHub
- Check DESIGN.md and SECURITY.md for context
- Read existing handlers in `cmd/` for patterns
