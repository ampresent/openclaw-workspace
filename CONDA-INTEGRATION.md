# Conda/Micromamba Integration — nix-evo

## Status: V1 + V2 + V3 Complete ✅

## Overview

Full conda/micromamba environment management integrated into nix-evo-agent (Rust/Axum) and nix-evo MCP server (TypeScript). Provides 35 API endpoints and 32 MCP tools for comprehensive Python environment lifecycle management.

## Architecture

```
┌─────────────────┐     ┌──────────────────┐
│  MCP Server     │────▶│  nix-evo-agent   │
│  (TypeScript)   │     │  (Rust/Axum)     │
│  32 tools       │     │  35 endpoints    │
└─────────────────┘     └────────┬─────────┘
                                 │
                    ┌────────────┴────────────┐
                    ▼                         ▼
              ┌───────────┐           ┌───────────┐
              │ micromamba │           │   conda   │
              └───────────┘           └───────────┘
```

## V1 — Core Conda Management (8 endpoints, 6 MCP tools)

| Endpoint | Method | Description |
|---|---|---|
| `/api/conda/envs` | GET | List all conda environments |
| `/api/conda/packages` | GET | List packages in an environment |
| `/api/conda/create` | POST | Create new environment |
| `/api/conda/install` | POST | Install packages |
| `/api/conda/remove` | POST | Remove packages |
| `/api/conda/export` | GET | Export environment |
| `/api/conda/create-from-yml` | POST | Create from environment.yml |
| `/api/conda/envs/:name` | DELETE | Remove environment |

**MCP Tools:** `conda_list_envs`, `conda_env_info`, `conda_install`, `conda_export`, `conda_create`, `conda_remove`

**Rust modules:** `conda.rs`, `cmd/conda_handlers.rs`

## V2 — Advanced Features (14 endpoints, 7 MCP tools)

| Endpoint | Method | Description |
|---|---|---|
| `/api/conda/diag` | GET | Environment diagnostics |
| `/api/conda/drift` | GET | Drift detection |
| `/api/conda/lock` | POST | conda-lock integration |
| `/api/hybrid/snapshot` | GET | NixOS+conda hybrid view |
| `/api/python/envs` | GET | Unified Python env listing (8 managers) |
| `/api/env/sync` | POST | Environment sync/export |
| `/api/env/export-all` | POST | Export all formats |
| `/api/env/test` | POST | Run smoke tests |
| `/api/env/test/auto` | POST | Auto-detect test suite |
| `/api/resolve/package/:name` | GET | Cross-source package resolver |
| `/api/resolve/batch` | POST | Batch package resolve |
| `/api/cache/status` | GET | Build cache status |
| `/api/cache/clean` | POST | Cache cleanup |
| `/api/cache/mirror` | POST | Mirror setup |

**MCP Tools:** `python_envs`, `env_sync`, `env_test`, `resolve_package`, `cache_status`, `cache_clean`, `env_health`

**Rust modules:** `conda_diag.rs`, `hybrid.rs`, `conda_lock.rs`, `venv_bridge.rs`, `env_sync.rs`, `env_test.rs`, `resolver.rs`, `build_cache.rs`

## V3 — Advanced Conda Features (13 endpoints, 12 MCP tools) 🆕

### 🧬 Environment Fingerprinting (`env_fingerprint.rs`)
- SHA-256 hash of (sorted packages + versions + python version + channels)
- Deterministic: same env on different machines → same hash
- Fingerprint history tracking to `/var/lib/nix-evo/fingerprints/`
- Drift detection from last snapshot

| Endpoint | Method | Description |
|---|---|---|
| `/api/env/fingerprint` | GET | Compute fingerprint (?env=X&save=true) |
| `/api/env/fingerprint/compare` | POST | Compare two environments |
| `/api/env/fingerprint/history` | GET | Fingerprint history |
| `/api/env/fingerprint/drift` | GET | Drift since last snapshot |

**MCP Tools:** `env_fingerprint`, `env_fingerprint_compare`, `env_fingerprint_history`

### 🔀 Environment Migration Assistant (`env_migrate.rs`)
- conda ↔ micromamba migration (export + recreate)
- pip → conda (auto-detect conda-forge equivalents)
- requirements.txt → environment.yml (with pip-only detection)
- environment.yml → conda-lock.yml
- requirements.txt → conda-lock.yml (two-step)
- environment.yml → requirements.txt

| Endpoint | Method | Description |
|---|---|---|
| `/api/env/migrate` | POST | Execute migration |

**MCP Tool:** `env_migrate`

### 🏥 Environment Repair Engine (`env_repair.rs`)
- Shared library check (missing .so, broken symlinks)
- Metadata integrity (corrupt conda-meta/*.json)
- Version conflict detection
- Orphaned dist-info detection
- pip check integration
- Auto-fix mode

| Endpoint | Method | Description |
|---|---|---|
| `/api/env/repair` | POST | Diagnose & repair |

**MCP Tool:** `env_repair`

### 📈 Package Risk Assessment (`pkg_risk.rs`)
- conda-forge search for version/channel
- PyPI query for version/license/maintainers
- Risk scoring: unmaintained, single-maintainer, no license, low downloads
- Risk levels: low/medium/high/critical

| Endpoint | Method | Description |
|---|---|---|
| `/api/pkg/risk/:name` | GET | Single package risk |
| `/api/pkg/risk/batch` | POST | Batch assessment |

**MCP Tools:** `pkg_risk`, `pkg_risk_batch`

### 🎯 Environment Templates (`env_templates.rs`)
- Pre-built templates: ML-GPU, Data Science, Web Dev, Bioinformatics, Deep Learning, Jupyter
- Pinned versions for reproducibility
- Optional/required package separation
- Custom python version and extra packages

| Endpoint | Method | Description |
|---|---|---|
| `/api/env/templates` | GET | List all templates |
| `/api/env/templates/:name` | GET | Template detail |
| `/api/env/provision` | POST | Provision from template |

**MCP Tools:** `env_templates`, `env_provision`

### 🌐 Remote Environment Sync (`env_remote.rs`)
- Push: export local → send to remote API → recreate
- Pull: fetch from remote API → create locally
- Supports SSH tunnel via nix-evo-agent host config
- Fallback to manual commands if remote unreachable

| Endpoint | Method | Description |
|---|---|---|
| `/api/env/push` | POST | Push to remote |
| `/api/env/pull` | POST | Pull from remote |

**MCP Tools:** `env_push`, `env_pull`

## Total API Endpoints: 35

| Version | Endpoints | MCP Tools | Rust Files |
|---|---|---|---|
| V1 | 8 | 6 | conda.rs, cmd/conda_handlers.rs |
| V2 | 14 | 7 | conda_diag.rs, hybrid.rs, conda_lock.rs, venv_bridge.rs, env_sync.rs, env_test.rs, resolver.rs, build_cache.rs |
| V3 | 13 | 12 | env_fingerprint.rs, env_migrate.rs, env_repair.rs, pkg_risk.rs, env_templates.rs, env_remote.rs, conda_tools_v3.ts |
| **Total** | **35** | **32** | **16 Rust + 3 TS** |

## Dependencies Added (Cargo.toml)
- `sha2 = "0.10"` — SHA-256 hashing for fingerprints
- `hex = "0.4"` — Hex encoding
- `chrono = { version = "0.4", features = ["serde"] }` — Timestamps
- `reqwest = { version = "0.12", features = ["json"] }` — Remote HTTP calls

## Key Design Decisions

1. **Backend auto-detection**: `detect_backend()` tries micromamba first, then conda
2. **Graceful degradation**: All operations return structured results even on failure
3. **BTreeMap for hashing**: Deterministic fingerprint regardless of package enumeration order
4. **Dry-run support**: Migration and template provisioning support `dry_run: true`
5. **Pip fallback**: Pip-only packages are kept in the pip section of environment.yml during migration

## V4 — Advanced & Wild Conda Features (9 endpoints, 9 MCP tools, 6 Rust modules) 🆕

### 🧪 Environment Branching & Cloning (`env_branch.rs`)
- Branch an environment like git: clone for testing without affecting original
- Diff two branches: packages only in A/B, version differences, channel differences, similarity %
- Merge branches with strategies: prefer-source, prefer-target, union
- Uses `conda env create --clone` for true cloning

| Endpoint | Method | Description |
|---|---|---|
| `/api/env/branch` | POST | Clone/branch an environment |
| `/api/env/diff` | GET | Compare two environment branches |
| `/api/env/merge` | POST | Merge two environment branches |

**MCP Tools:** `env_branch`, `env_diff`, `env_merge`

### 🔐 Conda Supply Chain Security (`conda_sbom.rs`)
- Generate SBOM (Software Bill of Materials) for any conda env
- SPDX and CycloneDX format support
- Detect packages from untrusted/unknown channels
- Verify package metadata integrity (build string, platform, channel)
- Trusted channel list: conda-forge, defaults, bioconda, pytorch, nvidia, intel, etc.

| Endpoint | Method | Description |
|---|---|---|
| `/api/conda/sbom` | GET | Generate SBOM (?env=X&format=cyclonedx) |
| `/api/conda/verify` | POST | Verify package integrity |

**MCP Tools:** `conda_sbom`, `conda_verify`

### 📦 Conda-as-Nix Flakes Generator (`conda_to_nix.rs`)
- Convert a conda environment or environment.yml to a Nix flake
- 50+ well-known conda → nixpkgs package mappings (numpy, pandas, pytorch, etc.)
- Heuristic fallback: unmapped packages get `python3Packages.<name>` guess
- Generates working flake.nix with devShell, inputs, and shellHook

| Endpoint | Method | Description |
|---|---|---|
| `/api/conda/to-nix` | POST | Convert conda env to Nix flake |

**MCP Tool:** `conda_to_nix`

### 🏃 Conda Runtime Optimizer (`conda_optimize.rs`)
- Full environment optimization analysis with health score (0-100)
- Detect: mixed channels, duplicate packages, oversized envs, unused dev tools
- Suggestions: mamba solver, conda-pack for deployment, fresh minimal env
- Channel distribution analysis
- Optional disk size check via `du`

| Endpoint | Method | Description |
|---|---|---|
| `/api/conda/optimize` | GET | Analyze environment for optimization |

**MCP Tool:** `conda_optimize`

### 🌐 Multi-Architecture Conda Support (`conda_multiarch.rs`)
- Check package availability across linux-64, linux-aarch64, osx-64, osx-arm64
- Auto-detect current architecture via `uname -m`
- Uses `conda search --platform` for real availability checks
- Known x86-only and aarch64-only package lists
- Migration feasibility score and blocker identification

| Endpoint | Method | Description |
|---|---|---|
| `/api/conda/multiarch/:env` | GET | Multi-arch migration analysis |

**MCP Tool:** `conda_multiarch`

### 📊 Conda Ecosystem Analytics (`conda_analytics.rs` + `conda-analytics.html`)
- Package importance ranking by reverse dependency count
- Dependency impact analysis: "what breaks if I remove numpy?" (BFS transitive deps)
- Channel health assessment with trust levels
- Dependency graph stats: nodes, edges, max depth, orphan packages, hub packages
- Risk indicators: bloat detection, untrusted channel warnings
- Interactive dark-theme dashboard at `/dashboard/conda-analytics`

| Endpoint | Method | Description |
|---|---|---|
| `/api/conda/analytics` | GET | Full ecosystem analytics |

**MCP Tool:** `conda_analytics`

## Total API Endpoints: 44

| Version | Endpoints | MCP Tools | Rust Files |
|---|---|---|---|
| V1 | 8 | 6 | conda.rs, cmd/conda_handlers.rs |
| V2 | 14 | 7 | conda_diag.rs, hybrid.rs, conda_lock.rs, venv_bridge.rs, env_sync.rs, env_test.rs, resolver.rs, build_cache.rs |
| V3 | 13 | 12 | env_fingerprint.rs, env_migrate.rs, env_repair.rs, pkg_risk.rs, env_templates.rs, env_remote.rs |
| V4 | 9 | 9 | env_branch.rs, conda_sbom.rs, conda_to_nix.rs, conda_optimize.rs, conda_multiarch.rs, conda_analytics.rs |
| **Total** | **44** | **41** | **22 Rust + 4 TS** |

## Dependencies (Cargo.toml)
- `sha2 = "0.10"` — SHA-256 hashing for fingerprints
- `hex = "0.4"` — Hex encoding
- `chrono = { version = "0.4", features = ["serde"] }` — Timestamps
- `reqwest = { version = "0.12", features = ["json"] }` — Remote HTTP calls

## Key Design Decisions (V4)

1. **Git-like branching**: `conda env create --clone` provides true environment isolation for testing
2. **SPDX + CycloneDX**: Dual format SBOM support for industry-standard supply chain security
3. **Heuristic Nix mapping**: Unknown conda packages get `python3Packages.<name>` guess with exact_match=false flag
4. **Health scoring**: Weighted score penalizes mixed channels, duplicates, and bloat
5. **BFS impact analysis**: Transitive dependency walk reveals true blast radius of package removal
6. **Architecture detection**: `uname -m` → conda platform mapping for cross-arch planning
