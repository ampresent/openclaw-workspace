# nix-evo NixOS Module

## Installation

### As a flake input

Add to your `flake.nix`:

```nix
{
  inputs.nix-evo.url = "github:your-org/nix-evo";

  outputs = { self, nixpkgs, nix-evo, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        nix-evo.nixosModules.default
        {
          services.nix-evo-agent = {
            enable = true;
            port = 7890;
            # tokenFile = "/run/secrets/nix-evo-token";  # optional
          };
        }
      ];
    };
  };
}
```

### Manual installation

Copy `module.nix` to your NixOS config directory and import it:

```nix
# configuration.nix
{
  imports = [ ./module.nix ];

  services.nix-evo-agent = {
    enable = true;
    port = 7890;
  };
}
```

## Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable` | bool | `false` | Enable the nix-evo-agent service |
| `port` | int | `7890` | HTTP API port |
| `host` | string | `"127.0.0.1"` | Bind address |
| `tokenFile` | string | `null` | Path to file containing API token |
| `nixosDir` | string | `"/etc/nixos"` | NixOS config directory |
| `maxLogLines` | int | `200` | Maximum log lines to return |

## Security

- By default, the agent only listens on `127.0.0.1`
- Use `tokenFile` to enable API authentication
- For remote access, use SSH tunnels (configured on the MCP client side)
- The agent runs as a dedicated `nix-evo` user with minimal permissions

## Service Management

```bash
# Status
systemctl status nix-evo-agent

# Logs
journalctl -u nix-evo-agent -f

# Restart
systemctl restart nix-evo-agent

# Test the API
curl http://127.0.0.1:7890/health
```
