#!/usr/bin/env python3
"""Generate runb-tui demo GIF — superdesign theme."""

import os, re, struct
from PIL import Image, ImageDraw, ImageFont

# ── Config ──
FONT_SIZE = 14
COLS = 96
ROWS = 30
CHAR_W = FONT_SIZE * 0.601
CHAR_H = FONT_SIZE * 1.4
PAD_X = 24
PAD_Y = 20
BG = (24, 24, 27)        # zinc-900

# ANSI color map (dark terminal palette — superdesign inspired)
COLORS = {
    'reset':    (228, 228, 231),  # zinc-200
    'bold':     None,  # handled as brightness boost
    'dim':      (161, 161, 170),  # zinc-400
    'black':    (39, 39, 42),     # zinc-800
    'red':      (239, 68, 68),
    'green':    (34, 197, 94),
    'yellow':   (234, 179, 8),
    'blue':     (59, 130, 246),
    'magenta':  (168, 85, 247),
    'cyan':     (6, 182, 212),
    'white':    (228, 228, 231),
    'gray':     (113, 113, 122),  # zinc-500
    'bg_black': (24, 24, 27),
    'bg_cyan':  (6, 182, 212),
    'bg_red':   (239, 68, 68),
    'bg_green': (34, 197, 94),
    'bg_yellow':(234, 179, 8),
    'bg_blue':  (59, 130, 246),
    'bg_gray':  (113, 113, 122),
}

ANSI_RE = re.compile(r'\x1b\[([0-9;]*)m')

def try_font():
    candidates = [
        '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf',
        '/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf',
        '/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf',
        '/usr/share/fonts/TTF/DejaVuSansMono.ttf',
    ]
    for p in candidates:
        if os.path.exists(p):
            return ImageFont.truetype(p, FONT_SIZE)
    # fallback
    try:
        return ImageFont.truetype("DejaVuSansMono.ttf", FONT_SIZE)
    except:
        return ImageFont.load_default()

FONT = try_font()
FONT_BOLD = try_font()  # Pillow handles bold via brightness

def parse_ansi(text):
    """Yield (color, bg_color, bold, dim, text_segment) tuples."""
    pos = 0
    fg = COLORS['reset']
    bg = None
    bold = False
    dim = False

    for m in ANSI_RE.finditer(text):
        # yield text before this escape
        if m.start() > pos:
            yield (fg, bg, bold, dim, text[pos:m.start()])

        codes = m.group(1)
        if not codes:
            codes = '0'
        for code in codes.split(';'):
            code = int(code) if code else 0
            if code == 0:
                fg = COLORS['reset']; bg = None; bold = False; dim = False
            elif code == 1:
                bold = True; dim = False
            elif code == 2:
                dim = True; bold = False
            elif code == 22:
                bold = False; dim = False
            elif code == 30: fg = COLORS['black']
            elif code == 31: fg = COLORS['red']
            elif code == 32: fg = COLORS['green']
            elif code == 33: fg = COLORS['yellow']
            elif code == 34: fg = COLORS['blue']
            elif code == 35: fg = COLORS['magenta']
            elif code == 36: fg = COLORS['cyan']
            elif code == 37: fg = COLORS['white']
            elif code == 39: fg = COLORS['reset']
            elif code == 90: fg = COLORS['gray']
            elif code == 92: fg = COLORS['green']
            elif code == 93: fg = COLORS['yellow']
            elif code == 94: fg = COLORS['blue']
            elif code == 96: fg = COLORS['cyan']
            elif code == 97: fg = COLORS['white']
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


def render_text(draw, x, y, text):
    """Render a line of ANSI-colored text at (x, y)."""
    cx = x
    for fg, bg, bold, dim, seg in parse_ansi(text):
        if not seg:
            continue
        w = len(seg) * CHAR_W

        # Draw background rect
        if bg:
            draw.rectangle([cx, y, cx + w, y + CHAR_H - 2], fill=bg)

        # Adjust foreground brightness for bold/dim
        color = fg
        if bold:
            color = tuple(min(255, c + 40) for c in fg)
        elif dim:
            color = tuple(max(0, c - 50) for c in fg)

        draw.text((cx, y), seg, fill=color, font=FONT)
        cx += w

    return cx


def render_frame(terminal_text, cursor_row=None):
    """Render a full terminal frame to an Image."""
    lines = terminal_text.split('\n')
    # Pad to ROWS
    while len(lines) < ROWS:
        lines.append('')

    img_w = int(COLS * CHAR_W + PAD_X * 2)
    img_h = int(ROWS * CHAR_H + PAD_Y * 2)
    img = Image.new('RGB', (img_w, img_h), BG)
    draw = ImageDraw.Draw(img)

    for i, line in enumerate(lines[:ROWS]):
        y = PAD_Y + i * CHAR_H
        render_text(draw, PAD_X, y, line)

    return img


# ═══════════════════════════════════════════════════════
# Frame definitions — superdesign theme
# ═══════════════════════════════════════════════════════

B = '\x1b[1m'   # bold
D = '\x1b[2m'   # dim
R = '\x1b[0m'   # reset
CY = '\x1b[36m' # cyan
GR = '\x1b[32m' # green
RD = '\x1b[31m' # red
YL = '\x1b[33m' # yellow
GY = '\x1b[90m' # gray
WH = '\x1b[37m' # white
BL = '\x1b[94m' # bright blue

# Background colors for badges
BG_GY = '\x1b[40m'   # dark bg
BG_CY = '\x1b[46m'   # cyan bg
BG_GR = '\x1b[42m'   # green bg
BG_RD = '\x1b[41m'   # red bg
BG_YL = '\x1b[43m'   # yellow bg
BG_BL = '\x1b[44m'   # blue bg

W = COLS
SEP = '─' * (W - 4)

def frame_containers():
    lw = int((W - 6) * 0.45)
    lines = []
    lines.append('')
    lines.append(f'{CY}⬢{R} {B}runb{R} {GY}─ Lightweight OCI Container Runtime{R}')
    lines.append('')
    lines.append(f' {BG_CY}{B} Containers {R}  Layers   Overlay   System')
    lines.append('')
    lines.append(f' {B}Containers{R} {GY}(3){R}')

    # Summary badges
    lines.append(f' {GR}● 2 running{R}  {GY}  {RD}○ 1 stopped{R}  {GY}(3 total){R}')
    lines.append('')

    lines.append(f' {CY}▸{R} {WH}{B}nginx-app{R}         {GR}● running{R}')
    lines.append(f' {CY} {R}  redis-cache       {GR}● running{R}')
    lines.append(f' {CY} {R}  postgres-db       {RD}✕ stopped{R}')
    lines.append('')
    lines.append('')
    lines.append('')
    lines.append('')

    # Right pane detail
    lw_s = ' ' * (lw + 2)
    # We'll just show it below since this is a single flat render
    lines[5]  += f'     {GY}── nginx-app ──{R}'
    lines[7]  += f'       {GY}State{R}    {GR}● running{R}'
    lines[8]  += f'       {GY}PID{R}     1234'
    lines[9]  += f'       {GY}Bundle{R}  {BL}/opt/bundles/nginx{R}'
    lines[10] += f'       {GY}Rootfs{R}  {BL}/opt/bundles/nginx/rootfs{R}'
    lines[11] += f'       {GY}Created{R} 04/09 10:15'
    lines[13] += f'       {GY}{B}Actions{R}'
    lines[14] += f'        {BG_GR}{B} s {R} Start  {BG_RD}{B} k {R} Stop  {BG_YL}{B} d {R} Delete  {BG_BL}{B} u {R} Upgrade'

    lines.append(f' {BG_GY}{B} j/k {R} {WH}Navigate{R}   {BG_GY}{B} s {R} {WH}Start{R}   {BG_GY}{B} k {R} {WH}Stop{R}   {BG_GY}{B} d {R} {WH}Delete{R}   {BG_GY}{B} u {R} {WH}Upgrade{R}   {BG_GY}{B} r {R} {WH}Refresh{R}')
    lines.append(f' {GY}{SEP}{R}')
    lines.append(f' {BG_GY}{B} Tab {R} {WH}Next{R}   {BG_GY}{B} 1-4 {R} {WH}Tab{R}   {BG_GY}{B} ^Q {R} {WH}Quit{R}                         {D}runb-tui v0.1.0{R}')

    return '\n'.join(lines)


def frame_layers():
    lines = []
    lines.append('')
    lines.append(f'{CY}⬢{R} {B}runb{R} {GY}─ Lightweight OCI Container Runtime{R}')
    lines.append('')
    lines.append(f'  Containers  {BG_CY}{B} Layers {R}  Overlay   System')
    lines.append('')
    lines.append(f' {B}Layers{R} {GY}(4){R}')
    lines.append('')
    lines.append(f' {CY}▸{R} layer-001  +23 -0 ~5   1.2KB')
    lines.append(f' {CY} {R} layer-002  +3  -1 ~8   0.8KB')
    lines.append(f' {CY} {R} layer-003  +12 -0 ~0   3.4KB')
    lines.append(f' {CY} {R} layer-004  +0  -0 ~2   0.1KB')
    lines.append('')
    lines.append('')
    lines.append('')
    lines.append('')

    # Right pane
    lines[5]  += f'       {GY}── Layer 1 ──{R}'
    lines[7]  += f'         {GY}Created{R}     2026/04/09 10:15'
    lines[8]  += f'         {GY}Commit{R}      base install'
    lines[10] += f'         {GR}+23 added{R}   {RD}-0 deleted{R}   {YL}~5 changed{R}'
    lines[11] += f'         {GY}Written:{R} 1.2KB  │ 28 total operations'

    lines.append(f' {BG_GY}{B} c {R} {WH}Switch{R}   {BG_GY}{B} i {R} {WH}Init{R}   {BG_GY}{B} m {R} {WH}Commit{R}   {BG_GY}{B} b {R} {WH}Bench{R}')
    lines.append(f' {GY}{SEP}{R}')
    lines.append(f' {BG_GY}{B} Tab {R} {WH}Next{R}   {BG_GY}{B} 1-4 {R} {WH}Tab{R}   {BG_GY}{B} ^Q {R} {WH}Quit{R}                         {D}runb-tui v0.1.0{R}')

    return '\n'.join(lines)


def frame_overlay():
    lines = []
    lines.append('')
    lines.append(f'{CY}⬢{R} {B}runb{R} {GY}─ Lightweight OCI Container Runtime{R}')
    lines.append('')
    lines.append(f'  Containers   Layers   {BG_CY}{B} Overlay {R}  System')
    lines.append('')
    lines.append(f' {B}Overlay Mounts{R} {GY}(2){R}')
    lines.append('')
    lines.append(f' {CY}▸{R} {BL}/data/home{R}  →  {YL}/home{R}')
    lines.append(f' {CY} {R}  {BL}/data/var{R}   →  {YL}/var{R}')
    # Pad enough lines for right pane overlay
    for _ in range(9):
        lines.append('')

    # Right pane
    lines[5]  += f'       {GY}── Overlay Detail ──{R}'
    lines[7]  += f'         {GY}Host{R}       {BL}/data/home{R}'
    lines[8]  += f'         {GY}Container{R}  {YL}/home{R}'
    lines[10] += f'         {GY}{B}Config (runb.toml){R}'
    lines[11] += f'         {GY}[overlay]{R}'
    lines[12] += f'         {GY}links = [{R}'
    lines[13] += f'           {GY}{{ host = "{BL}/data/home{R}{GY}", container = "{YL}/home{R}{GY}" }}{R}'
    lines[14] += f'           {GY}{{ host = "{BL}/data/var{R}{GY}",  container = "{YL}/var{R}{GY}"  }}{R}'
    lines[15] += f'         {GY}]{R}'

    lines.append(f' {BG_GY}{B} c {R} {WH}Switch{R}   {BG_GY}{B} p {R} {WH}Prepare{R}   {BG_GY}{B} t {R} {WH}Teardown{R}   {BG_GY}{B} v {R} {WH}Verify{R}')
    lines.append(f' {GY}{SEP}{R}')
    lines.append(f' {BG_GY}{B} Tab {R} {WH}Next{R}   {BG_GY}{B} 1-4 {R} {WH}Tab{R}   {BG_GY}{B} ^Q {R} {WH}Quit{R}                         {D}runb-tui v0.1.0{R}')

    return '\n'.join(lines)


def frame_system():
    lines = []
    lines.append('')
    lines.append(f'{CY}⬢{R} {B}runb{R} {GY}─ Lightweight OCI Container Runtime{R}')
    lines.append('')
    lines.append(f'  Containers   Layers   Overlay   {BG_CY}{B} System {R}')
    lines.append('')
    lines.append(f' {GY}── System Info ──{R}')
    lines.append('')
    lines.append(f'   {GY}Version{R}      runb 0.1.0')
    lines.append(f'   {GY}Runtime Root{R} /run/runb')
    lines.append(f'   {GY}Backends{R}     diff · git · tar · hardlink')
    lines.append('')
    lines.append(f' {GY}{B}Architecture{R}')
    lines.append('')
    lines.append(f'  {CY}┌──────────────────────────────────────────────────────┐{R}')
    lines.append(f'  {CY}│{R}  {WH}{B}runb{R}  {GY}— chroot-only OCI runtime{R}                {CY}│{R}')
    lines.append(f'  {CY}│{R}                                                      {CY}│{R}')
    lines.append(f'  {CY}│{R}  {WH}create → start → stop → delete{R}                   {CY}│{R}')
    lines.append(f'  {CY}│{R}  {WH}overlay: prepare → teardown → verify → upgrade{R}   {CY}│{R}')
    lines.append(f'  {CY}│{R}  {WH}layers: init → commit → list → rebase → bench{R}    {CY}│{R}')
    lines.append(f'  {CY}│{R}                                                      {CY}│{R}')
    lines.append(f'  {CY}│{R}  {GY}Backends: diff │ git │ tar │ hardlink{R}             {CY}│{R}')
    lines.append(f'  {CY}└──────────────────────────────────────────────────────┘{R}')
    lines.append('')
    lines.append('')
    lines.append(f' {BG_GY}{B} h {R} {WH}Help{R}')
    lines.append(f' {GY}{SEP}{R}')
    lines.append(f' {BG_GY}{B} Tab {R} {WH}Next{R}   {BG_GY}{B} 1-4 {R} {WH}Tab{R}   {BG_GY}{B} ^Q {R} {WH}Quit{R}                         {D}runb-tui v0.1.0{R}')

    return '\n'.join(lines)


# ═══════════════════════════════════════════════════════
# Generate frames
# ═══════════════════════════════════════════════════════

OUT_DIR = '/root/.openclaw/workspace/projects/runb-tui'
FRAMES = [
    ('frame_01_containers.png', frame_containers()),
    ('frame_02_layers.png',     frame_layers()),
    ('frame_03_overlay.png',    frame_overlay()),
    ('frame_04_system.png',     frame_system()),
]

frame_paths = []
for name, text in FRAMES:
    img = render_frame(text)
    path = os.path.join(OUT_DIR, name)
    img.save(path)
    frame_paths.append(path)
    print(f'  ✓ {name} ({img.size[0]}x{img.size[1]})')

# ── Assemble GIF with ffmpeg ──
# Each frame shown for 2.5s, except last (back to containers) for 1.5s
gif_path = os.path.join(OUT_DIR, 'demo.gif')

# Write ffmpeg concat file
concat_path = os.path.join(OUT_DIR, '_concat.txt')
with open(concat_path, 'w') as f:
    for p in frame_paths:
        f.write(f"file '{p}'\n")
        f.write(f"duration 2.5\n")
    # last frame repeats to hold it
    f.write(f"file '{frame_paths[-1]}'\n")

os.system(
    f'ffmpeg -y -f concat -safe 0 -i {concat_path} '
    f'-vf "split[s0][s1];[s0]palettegen=stats_mode=diff[p];[s1][p]paletteuse=dither=bayer:bayer_scale=3" '
    f'-loop 0 {gif_path} 2>/dev/null'
)

# Cleanup temp frames
for p in frame_paths:
    os.remove(p)
os.remove(concat_path)

print(f'\n✅ Demo GIF: {gif_path}')
sz = os.path.getsize(gif_path)
print(f'   Size: {sz/1024:.0f} KB')
