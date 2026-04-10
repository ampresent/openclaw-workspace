#!/usr/bin/env python3
"""Generate runb + Docker demo GIF — showing full workflow."""

import os
from PIL import Image, ImageDraw, ImageFont

# ── Config ──
FONT_SIZE = 14
COLS = 96
ROWS = 32
CHAR_W = FONT_SIZE * 0.601
CHAR_H = FONT_SIZE * 1.4
PAD_X = 24
PAD_Y = 20
BG = (24, 24, 27)

COLORS = {
    'reset': (228, 228, 231),
    'dim': (161, 161, 170),
    'black': (39, 39, 42),
    'red': (239, 68, 68),
    'green': (34, 197, 94),
    'yellow': (234, 179, 8),
    'blue': (59, 130, 246),
    'cyan': (6, 182, 212),
    'white': (228, 228, 231),
    'gray': (113, 113, 122),
    'bg_cyan': (6, 182, 212),
    'bg_green': (34, 197, 94),
    'bg_red': (239, 68, 68),
    'bg_yellow': (234, 179, 8),
    'bg_blue': (59, 130, 246),
    'bg_gray': (113, 113, 122),
    'bg_black': (24, 24, 27),
}

import re
ANSI_RE = re.compile(r'\x1b\[([0-9;]*)m')

def try_font():
    candidates = [
        '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf',
        '/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf',
    ]
    for p in candidates:
        if os.path.exists(p):
            return ImageFont.truetype(p, FONT_SIZE)
    return ImageFont.load_default()

FONT = try_font()

def parse_ansi(text):
    pos = 0
    fg = COLORS['reset']; bg = None; bold = False; dim = False
    for m in ANSI_RE.finditer(text):
        if m.start() > pos:
            yield (fg, bg, bold, dim, text[pos:m.start()])
        for code in (m.group(1) or '0').split(';'):
            code = int(code) if code else 0
            if code == 0: fg = COLORS['reset']; bg = None; bold = False; dim = False
            elif code == 1: bold = True; dim = False
            elif code == 2: dim = True; bold = False
            elif code == 22: bold = False; dim = False
            elif 30 <= code <= 37: fg = [COLORS['black'],COLORS['red'],COLORS['green'],COLORS['yellow'],COLORS['blue'],COLORS['cyan'],COLORS['white'],COLORS['white']][code-30]
            elif code == 39: fg = COLORS['reset']
            elif 90 <= code <= 97: fg = [COLORS['gray'],COLORS['red'],COLORS['green'],COLORS['yellow'],COLORS['blue'],COLORS['cyan'],COLORS['cyan'],COLORS['white']][code-90]
            elif code == 40: bg = COLORS['bg_black']
            elif code == 41: bg = COLORS['bg_red']
            elif code == 42: bg = COLORS['bg_green']
            elif code == 43: bg = COLORS['bg_yellow']
            elif code == 44: bg = COLORS['bg_blue']
            elif code == 46: bg = COLORS['bg_cyan']
            elif code == 49: bg = None
        pos = m.end()
    if pos < len(text):
        yield (fg, bg, bold, dim, text[pos:])

def render_frame(terminal_text):
    lines = terminal_text.split('\n')
    while len(lines) < ROWS: lines.append('')
    img_w = int(COLS * CHAR_W + PAD_X * 2)
    img_h = int(ROWS * CHAR_H + PAD_Y * 2)
    img = Image.new('RGB', (img_w, img_h), BG)
    draw = ImageDraw.Draw(img)
    for i, line in enumerate(lines[:ROWS]):
        y = PAD_Y + i * CHAR_H
        cx = PAD_X
        for fg, bg, bold, dim, seg in parse_ansi(line):
            if not seg: continue
            w = len(seg) * CHAR_W
            if bg:
                draw.rectangle([cx, y, cx + w, y + CHAR_H - 2], fill=bg)
            color = fg
            if bold: color = tuple(min(255, c + 40) for c in fg)
            elif dim: color = tuple(max(0, c - 50) for c in fg)
            draw.text((cx, y), seg, fill=color, font=FONT)
            cx += w
    return img

# ── ANSI shortcuts ──
B = '\x1b[1m'; D = '\x1b[2m'; R = '\x1b[0m'
CY = '\x1b[36m'; GR = '\x1b[32m'; RD = '\x1b[31m'
YL = '\x1b[33m'; GY = '\x1b[90m'; WH = '\x1b[37m'; BL = '\x1b[94m'
BG_GY = '\x1b[40m'; BG_CY = '\x1b[46m'; BG_GR = '\x1b[42m'
BG_RD = '\x1b[41m'; BG_YL = '\x1b[43m'; BG_BL = '\x1b[44m'

W = COLS
SEP = '─' * (W - 4)
PROMPT = f'{GR}❯{R} '

def frame_docker_build():
    return f"""{D}─────────────────────────────────────────────────────────────────────────────────────────────────{R}

  {B}Step 1: Docker Build{R}

  {GY}构建 runb + runb-tui 的联合镜像{R}

  {PROMPT}{GY}docker build{R} -t runb-all -f Dockerfile.runb-all .
  {GY}[+] Building 47.3s (14/14) FINISHED{R}
  {GY} => [builder-rust 1/3] FROM rust:1.85-bookworm{R}         {D}8.2s{R}
  {GY} => [builder-rust 2/3] WORKDIR /build/runb{R}             {D}0.0s{R}
  {GY} => [builder-rust 3/3] RUN cargo build --release{R}      {D}38.1s{R}
  {GR} => [builder-node 1/3] RUN npm ci{R}                     {D}6.4s{R}
  {GR} => [builder-node 2/3] RUN npx tsc{R}                    {D}3.2s{R}
  {GR} => [builder-node 3/3] RUN npm prune{R}                  {D}0.8s{R}
  {CY} => [runtime] COPY runb binary{R}                        {D}0.1s{R}
  {CY} => [runtime] COPY runb-tui dist/{R}                     {D}0.2s{R}
  {CY} => [runtime] Create demo bundles{R}                     {D}0.5s{R}
  {CY} => [runtime] Write entrypoint{R}                        {D}0.1s{R}
  {GR} => exporting to image{R}                                {D}1.2s{R}
  {GR} => => naming to docker.io/library/runb-all{R}

  {GY}镜像大小:{R} {WH}~180MB{R} {GY}(Rust static binary + Node.js runtime){R}
  {GY}runb 二进制:{R} {WH}1.3MB{R} {GY}(stripped + LTO){R}

{D}─────────────────────────────────────────────────────────────────────────────────────────────────{R}
  {BG_GY}{B} Tab {R} {WH}Next{R}   {BG_GY}{B} 1-5 {R} {WH}Frame{R}   {D}runb + Docker demo{R}
"""

def frame_docker_run():
    return f"""{D}─────────────────────────────────────────────────────────────────────────────────────────────────{R}

  {B}Step 2: Docker Run{R}

  {GY}启动容器（需要 --privileged 用于 chroot/mount）{R}

  {PROMPT}{GY}docker run{R} --privileged -it runb-all
  ╔══════════════════════════════════════════════╗
  ║  runb + runb-tui  (Docker)                   ║
  ║  Lightweight OCI Runtime with TUI Manager    ║
  ╚══════════════════════════════════════════════╝

    {GR}✓{R} Created: nginx
    {GR}✓{R} Created: redis

    启动 TUI...


  {GY}容器内结构:{R}
  ┌─────────────────────────────────────────────────────┐
  │  /usr/local/bin/runb       ← 1.3MB 静态二进制      │
  │  /usr/local/bin/runb-tui   ← TUI wrapper           │
  │  /opt/runb-tui/            ← Ink/React 应用         │
  │  /bundles/nginx/           ← OCI bundle + runb.toml │
  │  /bundles/redis/           ← OCI bundle             │
  │  /data/home/user.json      ← 持久化数据             │
  └─────────────────────────────────────────────────────┘

{D}─────────────────────────────────────────────────────────────────────────────────────────────────{R}
  {BG_GY}{B} Tab {R} {WH}Next{R}   {BG_GY}{B} 1-5 {R} {WH}Frame{R}   {D}runb + Docker demo{R}
"""

def frame_tui_containers():
    return f"""{D}─────────────────────────────────────────────────────────────────────────────────────────────────{R}

  {B}Step 3: runb-tui — Containers{R}

  {CY}⬢{R} {B}runb{R} {GY}─ Lightweight OCI Container Runtime{R}

   {BG_CY}{B} Containers {R}  Layers   Overlay   System

  {B}Containers{R} {GY}(2){R}
  {GR}● 2 created{R}  {GY}(2 total){R}

  {CY}▸{R} {WH}{B}nginx{R}             ○ created{R}
  {CY} {R}  redis              ○ created

       {GY}── nginx ──{R}
       {GY}State{R}    ○ created
       {GY}PID{R}     N/A
       {GY}Bundle{R}  {BL}/bundles/nginx{R}
       {GY}Rootfs{R}  {BL}/test-rootfs{R}
       {GY}Created{R} 04/09 12:08

       {GY}{B}Actions{R}
        {BG_GR}{B} s {R} Start  {BG_RD}{B} k {R} Stop  {BG_YL}{B} d {R} Delete  {BG_BL}{B} u {R} Upgrade

  {BG_GY}{B} j/k {R} {WH}Navigate{R}   {BG_GY}{B} s {R} {WH}Start{R}   {BG_GY}{B} k {R} {WH}Stop{R}   {BG_GY}{B} d {R} {WH}Delete{R}   {BG_GY}{B} u {R} {WH}Upgrade{R}   {BG_GY}{B} r {R} {WH}Refresh{R}
  {GY}{SEP}{R}
  {BG_GY}{B} Tab {R} {WH}Next{R}   {BG_GY}{B} 1-5 {R} {WH}Frame{R}   {D}runb + Docker demo{R}
"""

def frame_tui_running():
    return f"""{D}─────────────────────────────────────────────────────────────────────────────────────────────────{R}

  {B}Step 4: runb-tui — 运行中{R}

  {CY}⬢{R} {B}runb{R} {GY}─ Lightweight OCI Container Runtime{R}

   {BG_CY}{B} Containers {R}  Layers   Overlay   System

  {B}Containers{R} {GY}(2){R}
  {GR}● 1 running{R}  {GY}  {RD}○ 1 stopped{R}  {GY}(2 total){R}

  {CY}▸{R} {WH}{B}nginx{R}             {GR}● running{R}
  {CY} {R}  redis              ○ created

       {GY}── nginx ──{R}
       {GY}State{R}    {GR}● running{R}
       {GY}PID{R}     {WH}42{R}
       {GY}Bundle{R}  {BL}/bundles/nginx{R}
       {GY}Rootfs{R}  {BL}/test-rootfs{R}
       {GY}Created{R} 04/09 12:08

       {GY}{B}Actions{R}
        {BG_GR}{B} s {R} Start  {BG_RD}{B} k {R} Stop  {BG_YL}{B} d {R} Delete  {BG_BL}{B} u {R} Upgrade

  {BG_GY}{B} j/k {R} {WH}Navigate{R}   {BG_GY}{B} s {R} {WH}Start{R}   {BG_GY}{B} k {R} {WH}Stop{R}   {BG_GY}{B} d {R} {WH}Delete{R}   {BG_GY}{B} u {R} {WH}Upgrade{R}   {BG_GY}{B} r {R} {WH}Refresh{R}
  {GY}{SEP}{R}
  {BG_GY}{B} Tab {R} {WH}Next{R}   {BG_GY}{B} 1-5 {R} {WH}Frame{R}   {D}runb + Docker demo{R}
"""

def frame_tui_overlay():
    return f"""{D}─────────────────────────────────────────────────────────────────────────────────────────────────{R}

  {B}Step 5: runb-tui — Overlay 热升级{R}

  {CY}⬢{R} {B}runb{R} {GY}─ Lightweight OCI Container Runtime{R}

   Containers   Layers   {BG_CY}{B} Overlay {R}  System

  {B}Overlay Mounts{R} {GY}(2){R}

  {CY}▸{R} {BL}/data/home{R}  →  {YL}/home{R}
  {CY} {R}  {BL}/data/var{R}   →  {YL}/var{R}

       {GY}── Overlay Detail ──{R}
       {GY}Host{R}       {BL}/data/home{R}
       {GY}Container{R}  {YL}/home{R}

       {GY}{B}Config (runb.toml){R}
       {GY}[overlay]{R}
       {GY}links = [{R}
         {GY}{{ host = "{BL}/data/home{R}{GY}", container = "{YL}/home{R}{GY}" }}{R}
         {GY}{{ host = "{BL}/data/var{R}{GY}",  container = "{YL}/var{R}{GY}"  }}{R}
       {GY}]{R}

  {WH}热升级流程:{R}
  {GY}stop → teardown → delete → create → prepare → start{R}
  {GR}宿主数据 /data/* 通过 bind mount 保留！{R}

  {BG_GY}{B} c {R} {WH}Switch{R}   {BG_GY}{B} p {R} {WH}Prepare{R}   {BG_GY}{B} t {R} {WH}Teardown{R}   {BG_GY}{B} v {R} {WH}Verify{R}
  {GY}{SEP}{R}
  {BG_GY}{B} Tab {R} {WH}Next{R}   {BG_GY}{B} 1-5 {R} {WH}Frame{R}   {D}runb + Docker demo{R}
"""

OUT_DIR = '/root/.openclaw/workspace/projects/runb-tui'

FRAMES = [
    ('demo_docker_01.png', frame_docker_build(), 3.0),
    ('demo_docker_02.png', frame_docker_run(), 3.0),
    ('demo_docker_03.png', frame_tui_containers(), 3.0),
    ('demo_docker_04.png', frame_tui_running(), 3.0),
    ('demo_docker_05.png', frame_tui_overlay(), 3.0),
]

frame_paths = []
durations = []
for name, text, dur in FRAMES:
    img = render_frame(text)
    path = os.path.join(OUT_DIR, name)
    img.save(path)
    frame_paths.append(path)
    durations.append(dur)
    print(f'  ✓ {name} ({img.size[0]}x{img.size[1]})')

# Build GIF with ffmpeg
gif_path = os.path.join(OUT_DIR, 'demo-docker.gif')
concat_path = os.path.join(OUT_DIR, '_concat_docker.txt')
with open(concat_path, 'w') as f:
    for p, d in zip(frame_paths, durations):
        f.write(f"file '{p}'\n")
        f.write(f"duration {d}\n")
    f.write(f"file '{frame_paths[-1]}'\n")

os.system(
    f'ffmpeg -y -f concat -safe 0 -i {concat_path} '
    f'-vf "split[s0][s1];[s0]palettegen=stats_mode=diff[p];[s1][p]paletteuse=dither=bayer:bayer_scale=3" '
    f'-loop 0 {gif_path} 2>/dev/null'
)

for p in frame_paths:
    os.remove(p)
os.remove(concat_path)

sz = os.path.getsize(gif_path)
print(f'\n✅ Demo GIF: {gif_path}')
print(f'   Size: {sz/1024:.0f} KB')
