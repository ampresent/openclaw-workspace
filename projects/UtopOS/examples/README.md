# UtopOS Examples

Example configurations and usage patterns for common NixOS management tasks via UtopOS.

## Quick Reference

### Checking System Health

```
# Via curl
curl http://127.0.0.1:7890/api/snapshot | jq

# Via MCP (in Claude Code)
Use the system_snapshot tool
```

### Reading Current Config

```bash
curl http://127.0.0.1:7890/api/config
curl http://127.0.0.1:7890/api/config?path=/etc/nixos/hardware-configuration.nix
```

### Validating Before Applying

```bash
curl -X POST http://127.0.0.1:7890/api/config/validate \
  -H "Content-Type: application/json" \
  -d '{"config": "services.nginx.enable = true;"}'
```

### Viewing Service Logs

```bash
curl "http://127.0.0.1:7890/api/logs?unit=nginx.service&lines=100"
```

### Generation Management

```bash
# List generations
curl http://127.0.0.1:7890/api/generations

# Rollback
curl -X POST http://127.0.0.1:7890/api/rollback \
  -H "Content-Type: application/json" \
  -d '{"target": 41}'
```

## MCP Tool Examples (for AI Agents)

### Diagnose a Problem

1. `system_snapshot` — get overview
2. `service_logs` — check the failing service
3. `config_read` — review current config
4. `config_validate` — test proposed fix
5. `config_apply` — apply if safe

### Add a New Service

1. `config_validate` with the new config
2. Review risk assessment
3. `config_apply` with a descriptive message

### Rollback a Bad Change

1. `rollback_list` — find the target generation
2. `rollback_apply` — restore it

## Multi-Host Setup

See `hosts.toml.example` for multi-host configuration.

```toml
[hosts.default]
url = "http://127.0.0.1:7890"

[hosts.prod]
url = "http://127.0.0.1:7890"
ssh_tunnel = "admin@prod.example.com:7890"
token = "prod-token"
description = "Production web server"
```
