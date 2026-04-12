# UtopOS Security Design

## Threat Model

### Attack Surface
1. **HTTP API**: The agent listens on a TCP port
2. **Command Execution**: Agent runs nixos-rebuild, cp, rsync, etc.
3. **File Access**: Agent reads/writes /etc/nixos and /nix/var/nix/profiles
4. **MCP Server**: Runs on user's machine, talks to agent over network

### Trust Boundaries
- **Localhost only**: By default, agent binds to 127.0.0.1
- **SSH tunnel**: Remote access is via SSH tunnels (MCP side responsibility)
- **Token auth**: Optional Bearer token for API protection

### Assumptions
- The NixOS machine running the agent is trusted
- The network between MCP server and agent is trusted (localhost or SSH tunnel)
- The AI model using MCP tools acts in good faith (but may make mistakes)

## Security Controls

### 1. Network Binding (Critical)
- Default: `127.0.0.1:7890` — localhost only
- Never bind to `0.0.0.0` without TLS
- SSH tunnel for remote access (not exposed ports)

### 2. Authentication
- Optional Bearer token via `--api-token` or `NIX_EVO_TOKEN`
- Token checked in middleware, not per-endpoint
- `/health` endpoint is public (no sensitive info)

### 3. Input Validation
- Path traversal protection in config_read (must be under /etc/nixos/)
- Service name validation (reject control chars, semicolons)
- Config content not empty checks
- Command timeout (120s default)

### 4. Command Safety
- Commands run as the agent's user (not root directly)
- nixos-rebuild requires appropriate permissions (set via NixOS module)
- No shell injection: all commands use tokio::process::Command with args array
- Commands are hardcoded (no user-supplied command names)

### 5. File Safety
- Backup before every write (configuration.nix.bak)
- Generation descriptions are append-only metadata
- No access to sensitive files outside /etc/nixos and /nix/var/nix/profiles

### 6. Backup Security
- Backups stored in /var/lib/UtopOS/backups/ (agent-owned directory)
- Backup rotation prevents disk exhaustion
- Restore creates a safety backup before overwriting

## Future Hardening (v0.3+)

### TLS Support
- Optional rustls integration for HTTPS
- Self-signed cert generation for development
- Certificate file support for production

### mTLS (Mutual TLS)
- Client certificate verification
- Per-host identity verification

### Capability Dropping
- Run agent with minimal capabilities (CAP_DAC_OVERRIDE only)
- Use systemd's CapabilityBoundingSet

### Rate Limiting
- Per-token rate limits
- Expensive operations (apply, test) have separate limits
- Tower middleware for request throttling

### Audit Logging
- Log all mutations (apply, test, rollback, backup restore)
- Include token identity (hash, not value) in logs
- Structured logging for SIEM integration

### Secrets Integration
- agenix/sops-nix support (see DESIGN-V0.2.md)
- Secrets never logged or returned in API responses
