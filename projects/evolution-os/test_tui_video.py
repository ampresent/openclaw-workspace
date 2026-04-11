#!/usr/bin/env python3
"""Record evo TUI via tmux capture-pane → MP4 video with Unicode fallback."""

import subprocess, os, time, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gif_utils import (
    load_fonts, CHAR_W, CHAR_H, FONT_SIZE,
    ANSI_COLORS, CHAR_REPLACEMENTS, char_supported,
)
from PIL import Image, ImageDraw

COLS, ROWS = 100, 30
PAD = 12
BG = (18, 18, 24)
FG = (204, 204, 204)

# ANSI foreground color names → RGB
FG_MAP = {
    'black': (0,0,0), 'red': (205,49,49), 'green': (13,188,121),
    'yellow': (229,229,16), 'blue': (36,114,200), 'magenta': (188,63,188),
    'cyan': (17,168,205), 'white': (229,229,229),
    'bright black': (100,100,100), 'bright red': (241,76,76),
    'bright green': (35,209,139), 'bright yellow': (245,245,67),
    'bright blue': (59,142,234), 'bright magenta': (214,112,214),
    'bright cyan': (41,184,219), 'bright white': (255,255,255),
}

SESSION = "evo-tui-test"
EVO = "/root/.openclaw/workspace/projects/evolution-os/evo/target/release/evo"


def parse_tmux_capture(text):
    """Parse tmux capture-pane output with Unicode fallback."""
    mono, _, _, _ = load_fonts()
    lines = text.split('\n')
    result = []
    for line in lines:
        parsed = []
        fg, bg, bold = None, None, False
        i = 0
        while i < len(line):
            if line[i] == '\x1b' and i+1 < len(line) and line[i+1] == '[':
                j = i + 2
                while j < len(line) and line[j] != 'm':
                    j += 1
                if j < len(line):
                    codes = line[i+2:j].split(';')
                    for c in codes:
                        try:
                            c = int(c)
                        except ValueError:
                            continue
                        if c == 0:
                            fg, bg, bold = None, None, False
                        elif c == 1:
                            bold = True
                        elif c == 7:
                            fg, bg = bg, fg
                        elif c == 39:
                            fg = None
                        elif c == 49:
                            bg = None
                        elif 30 <= c <= 37:
                            names = ['black','red','green','yellow','blue','magenta','cyan','white']
                            fg = FG_MAP.get(names[c-30])
                        elif 90 <= c <= 97:
                            names = ['bright black','bright red','bright green','bright yellow',
                                     'bright blue','bright magenta','bright cyan','bright white']
                            fg = FG_MAP.get(names[c-90])
                        elif 40 <= c <= 47:
                            names = ['black','red','green','yellow','blue','magenta','cyan','white']
                            bg = FG_MAP.get(names[c-40])
                        elif 100 <= c <= 107:
                            names = ['bright black','bright red','bright green','bright yellow',
                                     'bright blue','bright magenta','bright cyan','bright white']
                            bg = FG_MAP.get(names[c-100])
                    i = j + 1
                    continue

            ch = line[i]
            # Apply Unicode fallback
            if ord(ch) >= 128 and not char_supported(mono, ch):
                if ch in CHAR_REPLACEMENTS:
                    repl_text, repl_color_code = CHAR_REPLACEMENTS[ch]
                    # Get replacement color
                    repl_fg = fg
                    if repl_color_code is not None:
                        repl_fg = ANSI_COLORS.get(repl_color_code)
                    for rc in repl_text:
                        parsed.append((rc, repl_fg, bg, bold))
                else:
                    parsed.append(('?', fg, bg, bold))
            else:
                parsed.append((ch, fg, bg, bold))
            i += 1
        result.append(parsed)
    return result


def render_frame(parsed_lines, title, step_label):
    mono, mono_bold, font_title, _ = load_fonts()
    w = COLS * CHAR_W + PAD * 2
    title_h = 38
    h = ROWS * CHAR_H + PAD * 2 + title_h
    img = Image.new('RGB', (w, h), BG)
    draw = ImageDraw.Draw(img)

    # Title
    draw.rectangle([0, 0, w, title_h], fill=(30, 30, 38))
    draw.text((PAD, 10), f"\u25cf  evo status --live  |  {title}", fill=(41,184,219), font=font_title)
    draw.line([(0, title_h), (w, title_h)], fill=(50,50,60), width=1)

    # Step label (bottom-right)
    draw.text((w - PAD - len(step_label) * CHAR_W, h - PAD - CHAR_H),
              step_label, fill=(100,100,100), font=mono)

    # Render lines
    for r, line_chars in enumerate(parsed_lines):
        if r >= ROWS:
            break
        y = title_h + PAD + r * CHAR_H
        x = PAD
        for ch, fg, bg, bold in line_chars[:COLS]:
            if bg:
                draw.rectangle([x, y, x+CHAR_W, y+CHAR_H], fill=bg)
            if ch != ' ':
                f = mono_bold if bold else mono
                draw.text((x, y+1), ch, fill=fg or FG, font=f)
            x += CHAR_W
    return img


def tmux_cmd(*args):
    r = subprocess.run(['tmux'] + list(args), capture_output=True, text=True)
    return r.stdout.strip(), r.returncode


def main():
    # Kill existing session
    tmux_cmd('kill-session', '-t', SESSION)

    # Create detached tmux session with correct size
    env = os.environ.copy()
    env['EVO_ROOT'] = '/tmp/evo-gcc-test'
    tmux_cmd('new-session', '-d', '-s', SESSION,
             '-x', str(COLS), '-y', str(ROWS),
             f'export EVO_ROOT=/tmp/evo-gcc-test && {EVO} status --live')

    frames = []
    actions = [
        (1.0, "\u25cf TUI \u52a0\u8f7d\u4e2d"),
        (1.5, "\u2460 \u521d\u59cb\u770b\u677f"),
        (0.3, "\u2461 \u6309 \u2193"),
        (0.8, "\u2462 \u9009\u62e9 gcc"),
        (0.3, "\u2463 \u6309 Enter"),
        (1.2, "\u2464 \u5c55\u5f00 Patch \u8be6\u60c5"),
        (0.3, "\u2465 \u6309 r"),
        (1.0, "\u2466 \u5237\u65b0\u72b6\u6001"),
        (0.3, "\u2467 \u6309 f"),
        (1.2, "\u2468 \u51bb\u7ed3\u7cfb\u7edf"),
        (0.3, "\u2469 \u518d\u6309 f"),
        (1.0, "\u246a \u89e3\u51bb"),
        (0.5, "\u246b \u9000\u51fa"),
    ]

    key_seq = [
        None, None,
        'j', None,
        'Enter', None,
        'r', None,
        'f', None,
        'f', None,
        'q',
    ]

    for i, (delay, label) in enumerate(actions):
        key = key_seq[i] if i < len(key_seq) else None
        if key:
            if key == 'Enter':
                tmux_cmd('send-keys', '-t', SESSION, 'Enter')
            else:
                tmux_cmd('send-keys', '-t', SESSION, key)
            time.sleep(0.15)

        # Capture pane
        out, _ = tmux_cmd('capture-pane', '-t', SESSION, '-p', '-S', '0', '-E', str(ROWS-1))
        parsed = parse_tmux_capture(out)
        frames.append(render_frame(parsed, label, f"frame {len(frames)+1}/{len(actions)}"))
        time.sleep(delay)

    # Quit
    tmux_cmd('send-keys', '-t', SESSION, 'q')
    time.sleep(0.5)
    tmux_cmd('kill-session', '-t', SESSION)

    print(f"Captured {len(frames)} frames")

    # Export MP4
    frame_dir = "/tmp/evo-tui-frames"
    os.makedirs(frame_dir, exist_ok=True)
    for i, frame in enumerate(frames):
        frame.save(f"{frame_dir}/frame_{i:04d}.png")

    mp4_path = "/tmp/evo-tui.mp4"
    r = subprocess.run(
        f"ffmpeg -y -framerate 2 -i {frame_dir}/frame_%04d.png "
        f"-vf 'scale=trunc(iw/2)*2:trunc(ih/2)*2' "
        f"-c:v libx264 -pix_fmt yuv420p -preset fast {mp4_path}",
        shell=True, capture_output=True, text=True
    )
    if r.returncode == 0:
        size_kb = os.path.getsize(mp4_path) / 1024
        print(f"\n\u2705 Video: {mp4_path} ({size_kb:.0f} KB)")
    else:
        print(f"Error: {r.stderr[:200]}")

    # Cleanup
    for f in os.listdir(frame_dir):
        os.remove(os.path.join(frame_dir, f))
    os.rmdir(frame_dir)


if __name__ == '__main__':
    main()
