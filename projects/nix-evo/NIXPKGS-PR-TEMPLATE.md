# nixpkgs Submission Template (Draft)

This is a draft template for submitting nix-evo to nixpkgs. Not yet ready for submission.

## Package: nix-evo-agent

### Meta
- **Name**: nix-evo-agent
- **Version**: 0.2.0
- **License**: MIT (TBD)
- **Platforms**: linux (NixOS only — requires nixos-rebuild)
- **Description**: NixOS diagnostic and management agent for AI agents

### Dependencies
- Rust (build)
- nixos-rebuild (runtime)
- systemd (runtime)

### Module
- `services.nix-evo-agent` NixOS module included

### PR Checklist
- [ ] `nix-build` succeeds
- [ ] `nixos-rebuild build` succeeds with module
- [ ] Tests pass (`cargo test`)
- [ ] License is set
- [ ] Maintainer is set
- [ ] Description is accurate

### PR Description Template

```markdown
## Description

nix-evo-agent is a lightweight HTTP API server that exposes NixOS system
diagnostics and safe configuration management for AI agents.

It provides:
- System health snapshots (services, disk, memory)
- Safe config validation via dry-build
- Generation management and rollback
- Risk assessment for configuration changes

## Motivation

AI agents like Claude Code can manage NixOS servers through this API,
making system administration accessible to non-experts through natural
language interaction.

## Things done

- [ ] Built on platform(s): x86_64-linux
- [ ] Tested basic functionality
- [ ] Module tested with `nixos-rebuild build`

## Checklist

- [ ] Code follows nixpkgs conventions
- [ ] Package builds with `nix-build`
- [ ] Module works with `nixos-rebuild`
- [ ] Tests pass
```

## Notes for submitters

1. The agent requires NixOS-specific tools (`nixos-rebuild`), so it's linux-only
2. The MCP server (TypeScript) could be packaged separately as a Node.js package
3. Consider adding `nix-evo` as the top-level package name, with the module under `services.nix-evo-agent`
