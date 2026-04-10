#!/usr/bin/env node
const fs = require('fs');

const WIDTH = 90;
const HEIGHT = 30;

// ANSI helpers
const B = '\x1b[1m';    // bold
const D = '\x1b[2m';    // dim
const R = '\x1b[0m';    // reset
const CY = '\x1b[36m';  // cyan
const GR = '\x1b[32m';  // green
const RD = '\x1b[31m';  // red
const YL = '\x1b[33m';  // yellow
const GY = '\x1b[90m';  // gray
const WH = '\x1b[37m';  // white

function padR(s, w) { return s + ' '.repeat(Math.max(0, w - s.replace(/\x1b\[[0-9;]*m/g, '').length)); }
function padL(s, w) { return ' '.repeat(Math.max(0, w - s.replace(/\x1b\[[0-9;]*m/g, '').length)) + s; }
function trunc(s, w) {
  let len = 0;
  let out = '';
  for (const m of s.matchAll(/\x1b\[[0-9;]*m|./g)) {
    if (m[0].startsWith('\x1b')) { out += m[0]; continue; }
    if (len >= w) break;
    out += m[0]; len++;
  }
  return out;
}

const sep = '─'.repeat(WIDTH - 4);

function frameContainers() {
  const leftW = Math.floor((WIDTH - 6) * 0.55);
  const rightW = WIDTH - 6 - leftW - 1;
  const lines = [];

  lines.push('');
  lines.push(`${CY}⬢${R} ${B}runb${R} — Lightweight OCI Container Runtime`);
  lines.push('');
  lines.push(` ${B}▸ Containers${R}    Layers    Overlay    System`);
  lines.push(` ${GY}${sep}${R}`);

  // Header row
  const lHdr = ` ${B}Containers${R} ${GY}(3)${R}`;
  const rHdr = ` ${B}nginx-app${R}`;
  lines.push(padR(lHdr, leftW + 2) + rHdr);

  lines.push('');
  // Row 1
  const l1 = ` ${CY}▸${R} nginx-app        ${GR}●${R} running`;
  const r1 = ` ${GY}State:${R} ${GR}running${R}`;
  lines.push(padR(l1, leftW + 2) + r1);
  // Row 2
  const l2 = `   redis-cache      ${GR}●${R} running`;
  const r2 = ` ${GY}PID:${R} 1234`;
  lines.push(padR(l2, leftW + 2) + r2);
  // Row 3
  const l3 = `   postgres-db      ${RD}✕${R} stopped`;
  const r3 = ` ${GY}Bundle:${R} /opt/bundles/nginx`;
  lines.push(padR(l3, leftW + 2) + r3);

  lines.push(padR('', leftW + 2) + ` ${GY}Rootfs:${R} /opt/bundles/nginx/rootfs`);
  lines.push(padR('', leftW + 2) + ` ${GY}Created:${R} 04/09 10:15`);
  lines.push(padR('', leftW + 2));
  lines.push(padR('', leftW + 2) + ` ${GY}Actions:${R}`);
  lines.push(padR('', leftW + 2) + `  s: Start  k: Stop  d: Delete  u: Upgrade`);

  lines.push('');
  lines.push('');
  lines.push('');
  lines.push('');
  lines.push(` ${YL}j/k${R} ${WH}Navigate${R}   ${YL}s${R} ${WH}Start${R}   ${YL}k${R} ${WH}Stop${R}   ${YL}d${R} ${WH}Delete${R}   ${YL}u${R} ${WH}Upgrade${R}   ${YL}r${R} ${WH}Refresh${R}`);
  lines.push(` ${GY}${sep}${R}`);
  lines.push(` ${YL}Tab${R} ${WH}Next${R}   ${YL}1-4${R} ${WH}Tab${R}   ${YL}Ctrl+Q${R} ${WH}Quit${R}${padL('runb-tui v0.1.0', 30)}`);

  return lines.join('\n');
}

function frameLayers() {
  const leftW = Math.floor((WIDTH - 6) * 0.55);
  const lines = [];

  lines.push('');
  lines.push(`${CY}⬢${R} ${B}runb${R} — Lightweight OCI Container Runtime`);
  lines.push('');
  lines.push(`   Containers  ${B}▸ Layers${R}    Overlay    System`);
  lines.push(` ${GY}${sep}${R}`);

  const lHdr = ` ${B}Layers${R} ${GY}(4)${R}`;
  const rHdr = ` ${B}Layer 1${R}`;
  lines.push(padR(lHdr, leftW + 2) + rHdr);

  lines.push(padR('', leftW + 2) + ` ${GY}Created:${R} 2026/04/09 10:15`);

  const l1 = ` ${CY}▸${R} layer-001  +23 -0 ~5   1.2KB`;
  const r1 = ` ${GY}Description:${R} base install`;
  lines.push(padR(l1, leftW + 2) + r1);

  const l2 = `   layer-002  +3  -1 ~8   0.8KB`;
  const r2 = ` ${GY}Files Added:${R} ${GR}+23${R}`;
  lines.push(padR(l2, leftW + 2) + r2);

  const l3 = `   layer-003  +12 -0 ~0   3.4KB`;
  const r3 = ` ${GY}Files Deleted:${R} ${RD}-0${R}`;
  lines.push(padR(l3, leftW + 2) + r3);

  const l4 = `   layer-004  +0  -0 ~2   0.1KB`;
  const r4 = ` ${GY}Files Changed:${R} ${YL}~5${R}`;
  lines.push(padR(l4, leftW + 2) + r4);

  lines.push(padR('', leftW + 2) + ` ${GY}Bytes Written:${R} 1.2KB`);

  lines.push('');
  lines.push('');
  lines.push('');
  lines.push('');
  lines.push(` ${YL}c${R} ${WH}Switch Container${R}   ${YL}i${R} ${WH}Init Layer${R}   ${YL}m${R} ${WH}Commit${R}   ${YL}b${R} ${WH}Benchmark${R}`);
  lines.push(` ${GY}${sep}${R}`);
  lines.push(` ${YL}Tab${R} ${WH}Next${R}   ${YL}1-4${R} ${WH}Tab${R}   ${YL}Ctrl+Q${R} ${WH}Quit${R}${padL('runb-tui v0.1.0', 30)}`);

  return lines.join('\n');
}

function frameOverlay() {
  const leftW = Math.floor((WIDTH - 6) * 0.55);
  const lines = [];

  lines.push('');
  lines.push(`${CY}⬢${R} ${B}runb${R} — Lightweight OCI Container Runtime`);
  lines.push('');
  lines.push(`   Containers    Layers  ${B}▸ Overlay${R}    System`);
  lines.push(` ${GY}${sep}${R}`);

  const lHdr = ` ${B}Overlay Mounts${R} ${GY}(2)${R}`;
  const rHdr = ` ${B}Overlay Detail${R}`;
  lines.push(padR(lHdr, leftW + 2) + rHdr);

  lines.push('');
  const l1 = ` ${CY}▸${R} /data/home  →  /home`;
  const r1 = ` ${GY}Host:${R} ${CY}/data/home${R}`;
  lines.push(padR(l1, leftW + 2) + r1);

  const l2 = `   /data/var   →  /var`;
  const r2 = ` ${GY}Container:${R} ${YL}/home${R}`;
  lines.push(padR(l2, leftW + 2) + r2);

  lines.push(padR('', leftW + 2));
  lines.push(padR('', leftW + 2) + ` ${B}runb.toml:${R}`);
  lines.push(padR('', leftW + 2) + ` ${GY}[overlay]${R}`);
  lines.push(padR('', leftW + 2) + ` ${GY}links = [${R}`);
  lines.push(padR('', leftW + 2) + ` ${GY}  { host = "/data/home", ...${R}`);
  lines.push(padR('', leftW + 2) + ` ${GY}  { host = "/data/var",  ...${R}`);
  lines.push(padR('', leftW + 2) + ` ${GY}]${R}`);

  lines.push('');
  lines.push('');
  lines.push('');
  lines.push(` ${YL}c${R} ${WH}Switch Container${R}   ${YL}p${R} ${WH}Prepare${R}   ${YL}t${R} ${WH}Teardown${R}   ${YL}v${R} ${WH}Verify${R}`);
  lines.push(` ${GY}${sep}${R}`);
  lines.push(` ${YL}Tab${R} ${WH}Next${R}   ${YL}1-4${R} ${WH}Tab${R}   ${YL}Ctrl+Q${R} ${WH}Quit${R}${padL('runb-tui v0.1.0', 30)}`);

  return lines.join('\n');
}

function frameSystem() {
  const lines = [];

  lines.push('');
  lines.push(`${CY}⬢${R} ${B}runb${R} — Lightweight OCI Container Runtime`);
  lines.push('');
  lines.push(`   Containers    Layers    Overlay  ${B}▸ System${R}`);
  lines.push(` ${GY}${sep}${R}`);

  lines.push(` ${B}System Info${R}`);
  lines.push(` ${GY}Version:${R} runb 0.1.0`);
  lines.push(` ${GY}Runtime Root:${R} /run/runb`);
  lines.push(` ${GY}Backend Options:${R} diff, git, tar, hardlink`);
  lines.push('');
  lines.push(` ${B}Architecture${R}`);
  lines.push(` ┌──────────────────────────────────────────────────────────┐`);
  lines.push(` │  runb (chroot-only OCI runtime)                          │`);
  lines.push(` │                                                          │`);
  lines.push(` │  create → start → stop → delete                         │`);
  lines.push(` │  overlay: prepare → teardown → verify → upgrade         │`);
  lines.push(` │  layers:  init → commit → list → rebase → bench         │`);
  lines.push(` │                                                          │`);
  lines.push(` │  Backends: diff │ git │ tar │ hardlink                   │`);
  lines.push(` └──────────────────────────────────────────────────────────┘`);

  lines.push('');
  lines.push('');
  lines.push('');
  lines.push(` ${YL}h${R} ${WH}Show Help${R}`);
  lines.push(` ${GY}${sep}${R}`);
  lines.push(` ${YL}Tab${R} ${WH}Next${R}   ${YL}1-4${R} ${WH}Tab${R}   ${YL}Ctrl+Q${R} ${WH}Quit${R}${padL('runb-tui v0.1.0', 30)}`);

  return lines.join('\n');
}

const frames = [frameContainers(), frameLayers(), frameOverlay(), frameSystem(), frameContainers()];
const delays = [0.5, 2.5, 2.5, 2.5, 2.5];

let header = JSON.stringify({
  version: 2,
  width: WIDTH,
  height: HEIGHT,
  timestamp: Math.floor(Date.now() / 1000),
  env: { TERM: 'xterm-256color', SHELL: '/bin/bash' },
});

let lines = [header];
let elapsed = 0;
for (let i = 0; i < frames.length; i++) {
  elapsed += delays[i];
  lines.push(JSON.stringify([elapsed, 'o', frames[i] + '\n']));
}

const outPath = '/root/.openclaw/workspace/projects/runb-tui/demo.cast';
fs.writeFileSync(outPath, lines.join('\n') + '\n');
console.log(`Written ${frames.length} frames, ${elapsed}s to ${outPath}`);
