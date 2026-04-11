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
