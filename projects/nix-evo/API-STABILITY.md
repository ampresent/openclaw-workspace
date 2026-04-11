# API Stability Guarantees

## Versioning Scheme

nix-evo uses URL-based API versioning: `/api/{version}/endpoint`

- **v1**: Stable (current default)
- **v2**: Beta (under development)
- Unversioned requests (`/api/endpoint`) route to the current stable version (v1)

## Stability Levels

### Stable (`/api/v1/`)

- **Guarantee**: Full backward compatibility within the major version
- **Breaking changes**: Only allowed in a new major version (v2, v3, etc.)
- **Deprecation notice**: 90 days minimum before removing a deprecated endpoint
- **New features**: Added as new endpoints, never changing existing behavior

### Beta (`/api/v2/`)

- **Guarantee**: API may change between releases
- **Breaking changes**: Possible, but will be documented in release notes
- **Migration path**: Upgrade guides provided when stable

## What Counts as a Breaking Change

- Removing an endpoint
- Removing or renaming a response field
- Changing the type of a response field
- Adding required request parameters
- Changing HTTP status codes for success/error cases

## What Does NOT Count as Breaking

- Adding new optional request parameters
- Adding new response fields
- Adding new endpoints
- Performance improvements
- Bug fixes that correct incorrect behavior

## Deprecation Process

1. **Announcement**: Deprecation notice in response headers + release notes
2. **Headers**: `Deprecation: true` and `Sunset: <date>` on deprecated endpoints
3. **Warning**: `X-API-Deprecation-Warning` header with migration guidance
4. **Timeline**: 90-day deprecation period before removal
5. **Removal**: Endpoint returns 410 Gone with migration instructions

## Response Headers

All responses include version information:

```
X-API-Version: v1              # Version used for this request
X-API-Current-Version: v1      # Current stable version
X-API-Latest-Version: v2       # Latest available version (may be beta)
```

Deprecated responses additionally include:

```
Deprecation: true
Sunset: Tue, 01 Jul 2026 00:00:00 GMT
X-API-Deprecation-Warning: v0 已弃用，请迁移到 v1
```

## Version Discovery

```bash
# List all versions and endpoints
curl http://localhost:3030/api/versions

# Check current version via headers
curl -I http://localhost:3030/api/snapshot
```

## Migration Guide (v0 → v1)

All endpoints are now under `/api/`. Authentication via Bearer token is available.

### Changed Endpoints

| Old (v0) | New (v1) | Notes |
|-----------|----------|-------|
| `/snapshot` | `/api/snapshot` | Same behavior |
| `/config` | `/api/config` | Same behavior |
| `/config/validate` | `/api/config/validate` | Added risk assessment |
| `/config/apply` | `/api/config/apply` | Added dry-run support |

### New Endpoints in v1

- `/api/config/generate` — AI config generation
- `/api/config/test` — test-before-switch
- `/api/backups` — backup management
- `/api/docker/*` — Docker integration
- `/api/cicd/*` — CI/CD webhooks
- `/api/observability/*` — logs, metrics, alerts
- `/api/dev/*` — developer mode

## Semantic Versioning

- **Patch** (v1.0.1): Bug fixes, no API changes
- **Minor** (v1.1.0): New endpoints, backward compatible
- **Major** (v2.0.0): Breaking changes, migration required
