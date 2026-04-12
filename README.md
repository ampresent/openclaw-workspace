# nix-patchwatch

A LazyGit-style TUI for NixOS system management.

## Features

- **Patches** — View all applied local patches, overlays, and configuration diffs
- **Upstream Diff** — See what's changed relative to upstream NixOS channels
- **Merge Requests** — Submit patches upstream with a guided wizard (press `M`)
- **Auto-refresh** — System state updates every 30 seconds

## Usage

```bash
npm start
```

### Keybindings

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch between tabs |
| `↑`/`↓` or `j`/`k` | Navigate lists |
| `Enter` | Select / view details |
| `M` | Open MR submission wizard |
| `R` | Force refresh |
| `Q` / `Ctrl+C` | Quit |

## Tech Stack

- [Ink](https://github.com/vadimdemedes/ink) — React for CLIs
- [React](https://react.dev) — UI framework
- [execa](https://github.com/sindresorhus/execa) — Shell command execution
- [diff](https://github.com/kpdecker/jsdiff) — Diff computation

## Development

```bash
npm run dev  # Auto-reload on changes
```
