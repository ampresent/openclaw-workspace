import { execSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const RUNB_ROOT = '/run/runb';

export type ContainerState = 'created' | 'running' | 'stopped';

export interface ContainerMeta {
  id: string;
  bundle: string;
  rootfs: string;
  pid: number | null;
  state: ContainerState;
  created_at: number;
}

export interface OverlayEntry {
  host: string;
  container: string;
}

export interface LayerMeta {
  created_at: number;
  description: string;
  layer_number: number;
  stats: {
    files_changed: number;
    files_added: number;
    files_deleted: number;
    bytes_written: number;
  };
}

export interface RunbConfig {
  runbPath: string;
  rootDir: string;
}

export function getRunbPath(): string {
  // Try common locations
  const paths = [
    '/usr/local/bin/runb',
    '/usr/bin/runb',
    join(process.env.HOME || '~', '.local/bin/runb'),
  ];
  for (const p of paths) {
    if (existsSync(p)) return p;
  }
  // Try PATH
  try {
    return execSync('which runb', { encoding: 'utf8' }).trim();
  } catch {
    return 'runb'; // fallback
  }
}

export function listContainers(): ContainerMeta[] {
  if (!existsSync(RUNB_ROOT)) return [];

  const containers: ContainerMeta[] = [];
  try {
    const entries = readdirSync(RUNB_ROOT, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.isDirectory()) {
        const stateFile = join(RUNB_ROOT, entry.name, 'state.json');
        if (existsSync(stateFile)) {
          try {
            const meta: ContainerMeta = JSON.parse(readFileSync(stateFile, 'utf8'));
            containers.push(meta);
          } catch { /* skip corrupt */ }
        }
      }
    }
  } catch { /* /run/runb not readable */ }

  return containers.sort((a, b) => b.created_at - a.created_at);
}

export function getContainerState(id: string): ContainerMeta | null {
  const stateFile = join(RUNB_ROOT, id, 'state.json');
  if (!existsSync(stateFile)) return null;
  try {
    return JSON.parse(readFileSync(stateFile, 'utf8'));
  } catch {
    return null;
  }
}

export function getOverlayConfig(bundle: string): OverlayEntry[] {
  const configPath = join(bundle, 'runb.toml');
  if (!existsSync(configPath)) return [];
  try {
    const content = readFileSync(configPath, 'utf8');
    // Simple TOML parsing for overlay links
    const links: OverlayEntry[] = [];
    const linkRegex = /\{\s*host\s*=\s*"([^"]+)"\s*,\s*container\s*=\s*"([^"]+)"\s*\}/g;
    let match;
    while ((match = linkRegex.exec(content)) !== null) {
      links.push({ host: match[1], container: match[2] });
    }
    return links;
  } catch {
    return [];
  }
}

export function listLayers(containerId: string): LayerMeta[] {
  const meta = getContainerState(containerId);
  if (!meta) return [];

  const layersDir = join(meta.bundle, 'layers');
  if (!existsSync(layersDir)) return [];

  const layers: LayerMeta[] = [];
  try {
    const entries = readdirSync(layersDir, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.isDirectory() && entry.name.startsWith('layer-')) {
        const metaFile = join(layersDir, entry.name, 'meta.json');
        if (existsSync(metaFile)) {
          try {
            layers.push(JSON.parse(readFileSync(metaFile, 'utf8')));
          } catch { /* skip */ }
        }
      }
    }
  } catch { /* skip */ }

  return layers.sort((a, b) => a.layer_number - b.layer_number);
}

export function runRunb(args: string): string {
  const runbPath = getRunbPath();
  try {
    return execSync(`${runbPath} ${args}`, {
      encoding: 'utf8',
      timeout: 30000,
    }).trim();
  } catch (e: any) {
    return `ERROR: ${e.message || e}`;
  }
}

export function formatTime(timestamp: number): string {
  const d = new Date(timestamp * 1000);
  return d.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)}MB`;
}

export function stateColor(state: ContainerState): string {
  switch (state) {
    case 'running': return 'green';
    case 'created': return 'yellow';
    case 'stopped': return 'red';
  }
}

export function stateIcon(state: ContainerState): string {
  switch (state) {
    case 'running': return '●';
    case 'created': return '○';
    case 'stopped': return '✕';
  }
}
