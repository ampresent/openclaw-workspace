# Contributing to nix-evo

Thank you for your interest in contributing to nix-evo! This document provides guidelines for contributing.

## Project Structure

```
nix-evo/
├── evo/                    # Rust agent (HTTP API server)
│   ├── src/
│   │   ├── main.rs         # Axum HTTP server + routing
│   │   ├── config.rs       # CLI configuration (clap)
│   │   ├── auth.rs         # Bearer token auth middleware
│   │   ├── error.rs        # Typed error handling
│   │   ├── ai_config.rs    # AI-assisted config generation
│   │   ├── backup.rs       # Backup & disaster recovery
│   │   └── cmd/            # Request handlers (one per endpoint)
│   ├── nix/                # NixOS module & package definitions
│   └── flake.nix           # Nix flake for building
├── mcp-server/             # TypeScript MCP server
│   ├── src/
│   │   ├── index.ts        # MCP tool definitions & routing
│   │   ├── index.test.ts   # Unit tests
│   │   ├── ssh-tunnel.ts   # SSH tunnel management
│   │   └── ai-config.ts    # AI config generation (MCP side)
│   └── package.json
├── examples/               # Usage examples
├── DESIGN.md               # v0.1 design document
├── DESIGN-V0.2.md          # v0.2 design document
└── QUICKSTART.md           # Quick start guide
```

## Development Setup

### Prerequisites

- Rust 1.75+ (for the agent)
- Node.js 20+ (for MCP server)
- NixOS (for integration testing)

### Building

```bash
# Agent (must be built on NixOS)
cd evo
nix develop  # enter dev shell
cargo build

# MCP Server
cd mcp-server
npm install
npm run build
```

### Testing

```bash
# Rust unit tests
cd evo && cargo test

# TypeScript tests
cd mcp-server && npx tsx src/index.test.ts
cd mcp-server && npx tsx tests/tools.test.ts
```

## Making Changes

### Agent (Rust)

1. Each API endpoint lives in `evo/src/cmd/<name>.rs`
2. Register new modules in `evo/src/cmd/mod.rs`
3. Add routes in `evo/src/main.rs`
4. Use `AppError` for all error responses
5. Add unit tests in `#[cfg(test)] mod tests` blocks

### MCP Server (TypeScript)

1. Add new tools to the `TOOLS` array in `src/index.ts`
2. Add the handler case in the `CallToolRequestSchema` handler
3. Add a formatter function for human-readable output
4. Add tests to `tests/tools.test.ts`

### NixOS Module

1. Module options go in `evo/nix/module.nix`
2. Package derivation in `evo/nix/package.nix`
3. Test with: `nix build .#nixosConfigurations.test.config.system.build.toplevel`

## Code Style

### Rust
- Use `tracing` for logging (not `println!`)
- Use `tokio::fs` for async file I/O
- Error messages in Chinese (this is a Chinese-first project)
- Follow existing patterns in `cmd/` modules

### TypeScript
- ESM modules (`"type": "module"` in package.json)
- Async/await for all agent API calls
- Console output via `console.error` (stdout is for MCP protocol)

## Commit Messages

Format: `nix-evo: <scope> - <description>`

Examples:
- `nix-evo: Round 7 - Add request timing middleware`
- `nix-evo: Fix dry-build parsing for flake configs`
- `nix-evo: Add TLS support to agent`

## Pull Requests

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests
5. Submit PR with description of changes

## Adding New MCP Tools

1. Define the tool schema in `TOOLS` array
2. Implement the agent endpoint (if needed) in `evo/src/cmd/`
3. Add the routing case in `CallToolRequestSchema` handler
4. Add a human-readable formatter
5. Add tests
6. Update QUICKSTART.md if user-facing

## Security

- Never log API tokens
- Validate all user input (see `error.rs` for validation patterns)
- Use `auth.rs` middleware for new API endpoints
- Backup before destructive operations

## License

TBD — likely MIT or Apache 2.0
