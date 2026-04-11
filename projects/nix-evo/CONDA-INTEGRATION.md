# nix-evo conda/micromamba Integration

> Extending nix-evo from NixOS-only management to a unified platform covering both NixOS system configuration AND conda environments.

## Why Conda Integration?

NixOS excels at declarative system configuration — packages, services, networking. But the ML/AI/data-science ecosystem runs on conda. CUDA, cuDNN, MKL, optimized BLAS, and countless scientific packages are distributed through conda channels and are painful or impossible to get from nixpkgs.

**nix-evo conda integration bridges this gap**: manage your NixOS servers AND their conda environments through one AI-assisted interface.

### Target Users

- **ML/AI teams** running GPU servers with conda environments
- **Data science teams** with multiple project-specific conda envs
- **DevOps** managing servers where both system packages and scientific Python coexist
- **Anyone** who uses NixOS but needs conda for specific workloads

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    nix-evo-agent                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │  NixOS API   │  │  Conda API  │  │   Hybrid API    │ │
│  │  /api/*      │  │  /api/conda/*│  │  /api/hybrid/*  │ │
│  └──────┬───────┘  └──────┬──────┘  └───────┬─────────┘ │
│         │                 │                   │           │
│    nixos-rebuild     micromamba           both combined   │
└─────────┴─────────────────┴─────────────────┴─────────────┘
                              │
                    ┌─────────┴─────────┐
                    │   MCP Server      │
                    │  (stdio transport) │
                    └─────────┬─────────┘
                              │
                    ┌─────────┴─────────┐
                    │  Claude Code /    │
                    │  AI Agent         │
                    └───────────────────┘
```

### New Components

| Component | File | Purpose |
|-----------|------|---------|
| conda.rs | `evo/src/conda.rs` | micromamba CLI wrapper — list/create/install/remove/export |
| conda_diag.rs | `evo/src/conda_diag.rs` | Diagnostics, drift detection, outdated package scanning |
| hybrid.rs | `evo/src/hybrid.rs` | Unified NixOS+conda view, conflict detection |
| conda_lock.rs | `evo/src/conda_lock.rs` | conda-lock.yml parsing and generation |
| conda_handlers.rs | `evo/src/cmd/conda_handlers.rs` | HTTP API handlers for conda operations |
| conda_tools.ts | `mcp-server/src/conda_tools.ts` | 6 new MCP tools for conda management |
| conda-module.nix | `evo/nix/conda-module.nix` | NixOS module for declarative conda provisioning |
| conda-flake | `experiments/conda-flake/` | Experiment: Nix flakes wrapping conda |

## New API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/conda/envs` | GET | List all conda environments |
| `/api/conda/packages` | GET | List packages in an environment |
| `/api/conda/create` | POST | Create a new conda environment |
| `/api/conda/install` | POST | Install packages into an environment |
| `/api/conda/remove` | POST | Remove packages from an environment |
| `/api/conda/export` | GET | Export environment.yml |
| `/api/conda/create-from-yml` | POST | Create environment from environment.yml |
| `/api/conda/envs/:name` | DELETE | Remove an entire environment |
| `/api/conda/diag` | GET | Full diagnostics across all environments |
| `/api/conda/drift` | GET | Detect drift vs environment.yml |
| `/api/conda/lock` | POST | Generate conda-lock.yml |
| `/api/hybrid/snapshot` | GET | Unified NixOS+conda state view |

## New MCP Tools

| Tool | Description |
|------|-------------|
| `conda_list_envs` | List all conda environments with package counts |
| `conda_env_info` | Detailed info about one environment |
| `conda_install` | Install packages into an environment |
| `conda_export` | Export environment.yml (standard or explicit) |
| `conda_drift` | Compare installed state vs environment.yml |
| `conda_lock` | Generate conda-lock.yml for reproducibility |

## Architecture Decisions

### 1. micromamba over conda

**Decision**: Use micromamba as the preferred backend.

**Rationale**:
- 10-50x faster than conda for environment solving
- Single binary, no base environment needed
- Fully compatible with conda packages and channels
- Available in nixpkgs

**Fallback**: If micromamba isn't found, fall back to conda.

### 2. Hybrid approach (not pure Nix)

**Decision**: Don't try to replace conda with Nix.

**Rationale**:
- CUDA/cuDNN/MKL are only reliably available via conda
- Many scientific packages have conda-only builds
- conda environments are the de facto standard for ML/DS teams
- Forcing pure Nix on these teams would be counterproductive

### 3. conda-lock for reproducibility

**Decision**: Use conda-lock.yml as the reproducibility mechanism.

**Rationale**:
- conda-lock resolves all platforms and pins exact versions
- The lockfile can be committed to git
- Works across different conda/micromamba versions
- Better than environment.yml alone (which resolves differently over time)

### 4. Drift detection as a first-class feature

**Decision**: Always compare actual installed state vs declared state.

**Rationale**:
- Conda environments drift over time (pip install, manual conda install)
- Teams need to know when environments have diverged from their declaration
- AI agents can use this to suggest fixes or updates

### 5. NixOS module for declarative provisioning

**Decision**: Provide a NixOS module that manages conda environments like services.

**Rationale**:
- NixOS users expect everything to be declarative
- Auto-provisioning on system rebuild is powerful
- Combines Nix's infrastructure management with conda's package ecosystem

## Comparison: Pure Nix vs Nix+Conda Hybrid

| Aspect | Pure Nix | Nix+Conda Hybrid |
|--------|----------|------------------|
| **Reproducibility** | Full (nix hash) | conda-lock.yml |
| **Build speed** | Cache hits = fast; misses = slow | micromamba is fast |
| **ML/CUDA support** | Poor (complex, fragile) | Excellent (conda channels) |
| **Scientific packages** | Incomplete | Complete |
| **Learning curve** | High (Nix language) | Medium (environment.yml) |
| **Team adoption** | Low (needs Nix knowledge) | High (standard conda) |
| **Binary caching** | Excellent (/nix/store) | Channel-based |
| **System packages** | Excellent | Not conda's domain |
| **Service management** | Excellent (systemd) | Not conda's domain |

### Recommendation

**Use the hybrid approach when**:
- Your workload includes ML/AI/data science
- You need CUDA or other GPU packages
- Your team already knows conda
- You want NixOS for system management but conda for Python packages

**Use pure Nix when**:
- Your workload is purely web services / CLI tools
- All your packages are available in nixpkgs
- You need full hermetic builds
- You don't have a conda-dependent team

## Example: AI Training Server

```nix
# configuration.nix — NixOS manages the system
{ config, pkgs, ... }: {
  imports = [ ./nix/conda-module.nix ];

  # NixOS manages: nginx, firewall, SSH, GPU drivers
  services.nginx.enable = true;
  networking.firewall.allowedTCPPorts = [ 80 443 ];
  hardware.opengl.enable = true;

  # nix-evo manages: conda environments
  services.nix-evo-conda = {
    enable = true;
    backend = "micromamba";
    environments = {
      training = {
        python = "3.11";
        channels = [ "conda-forge" "nvidia" ];
        packages = [ "pytorch" "torchvision" "cudatoolkit=12.1" ];
      };
      inference = {
        python = "3.11";
        channels = [ "conda-forge" ];
        packages = [ "onnxruntime-gpu" "numpy" "fastapi" ];
      };
    };
  };
}
```

Then via AI agent:
```
User: "Check if the training environment has any conflicts"
AI: [calls conda_list_envs, then conda_drift]
    "Training env looks good. Inference env has 3 outdated packages
     and drifted from its environment.yml — 5 extra packages installed."
```

## Future Work

- [ ] conda-lock integration with offline channel mirroring
- [ ] Automatic environment snapshots (like NixOS generations, but for conda)
- [ ] Security vulnerability scanning via conda-audit
- [ ] Multi-server conda environment synchronization
- [ ] Web dashboard showing NixOS + conda state side-by-side
- [ ] Integration with devenv.sh for dev shell management
- [ ] Support for pixi (another fast conda frontend)

## Status

| Feature | Status | Notes |
|---------|--------|-------|
| micromamba wrapper | ✅ Complete | Auto-detects backend |
| Environment diagnostics | ✅ Complete | Conflict + drift detection |
| MCP tools | ✅ Complete | 6 tools with Chinese formatting |
| Hybrid management | ✅ Complete | 4 alignment strategies |
| conda-lock | ✅ Complete | Parse + generate |
| NixOS module | ✅ Complete | Declarative provisioning |
| Flake experiment | ✅ Complete | Nix pins tool, conda pins packages |
