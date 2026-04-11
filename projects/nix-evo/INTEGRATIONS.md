# nix-evo Integration Points

## Current State

nix-evo v0.3 manages a single NixOS host via HTTP API + MCP.

## Future Integrations

### 1. Multi-Host Orchestration (v0.4)

Manage multiple NixOS hosts from a single MCP server:

```
MCP Server → Agent (web-server) 
          → Agent (db-server)
          → Agent (cache-server)
```

Features:
- Parallel config validation across hosts
- Rolling updates (one host at a time)
- Dependency ordering (db before web)

### 2. Docker Integration

nix-evo can manage Docker containers running on NixOS:

```nix
# Already supported via NixOS config
virtualisation.docker.enable = true;
```

New endpoints could:
- List running containers
- Show container logs
- Restart containers
- Update container images

### 3. systemd-nspawn Containers

NixOS containers via `containers.*` options:

```nix
containers.myapp = {
  config = { ... };  # Full NixOS config for the container
  autoStart = true;
};
```

nix-evo could:
- List containers and their status
- Show container configurations
- Manage container lifecycle

### 4. Kubernetes (nixos-generators)

Generate NixOS-based Kubernetes nodes:

```nix
# Generate k8s node image
nix build .#k8sNode
```

nix-evo could:
- Generate node images
- Manage node configurations
- Track cluster state

### 5. Monitoring Integration

- Expose Prometheus metrics from the agent
- Health check integration with external monitors
- Alert on generation changes (config drift detection)

### 6. CI/CD Integration

- GitHub Actions: validate config on PR
- GitLab CI: auto-deploy on merge
- Webhook triggers for config changes

## Priority

1. Multi-host (high impact, moderate effort)
2. Docker integration (high demand, low effort)
3. Monitoring (operational necessity)
4. CI/CD (nice to have)
