#!/usr/bin/env python3
"""Test evo with gcc -fno-lto patch, record as MP4 video."""

import subprocess, os, time
from PIL import Image, ImageDraw, ImageFont

FONT_SIZE = 14
CHAR_W, CHAR_H = int(FONT_SIZE * 0.6), FONT_SIZE + 4
COLS, ROWS = 100, 42
PAD = 16
BG_COLOR = (24, 24, 28)
FG_COLOR = (204, 204, 204)
TITLE_BG = (35, 35, 42)
GREEN = (35, 209, 139)
RED = (241, 76, 76)
DIM = (100, 100, 100)
YELLOW = (229, 229, 16)
CYAN = (41, 184, 219)

EVO = "/root/.openclaw/workspace/projects/evolution-os/evo/target/release/evo"
WORK = "/tmp/evo-gcc-test"
FPS = 2

FONT_PATH = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
FONT_BOLD_PATH = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"
FONT_TITLE_PATH = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"


def get_fonts():
    try:
        return (
            ImageFont.truetype(FONT_PATH, FONT_SIZE),
            ImageFont.truetype(FONT_BOLD_PATH, FONT_SIZE),
            ImageFont.truetype(FONT_TITLE_PATH, 18),
        )
    except Exception:
        f = ImageFont.load_default()
        return f, f, f


def render_frame(title, prompt, output_lines, scroll=0):
    w = int(COLS * CHAR_W) + PAD * 2
    title_h = 40
    h = int(ROWS * CHAR_H) + PAD * 2 + title_h
    img = Image.new('RGB', (w, h), BG_COLOR)
    draw = ImageDraw.Draw(img)
    font, font_bold, font_title = get_fonts()

    draw.rectangle([0, 0, w, title_h], fill=TITLE_BG)
    draw.text((PAD, 10), f"\U0001f9ea  evo \u00d7 gcc  |  {title}", fill=GREEN, font=font_title)
    draw.line([(0, title_h), (w, title_h)], fill=(60, 60, 60), width=1)

    y_start = PAD + title_h
    draw.text((PAD, y_start), f"$ {prompt}", fill=GREEN, font=font_bold)

    y = y_start + CHAR_H + 4
    visible = output_lines[scroll:scroll + ROWS - 3]
    for line in visible:
        if y > h - PAD:
            break
        if '\u2713' in line or '\u2714' in line or line.strip().startswith('\u2713'):
            color = GREEN
        elif '\u2717' in line or 'FAIL' in line:
            color = RED
        elif line.strip().startswith('\u2192') or line.strip().startswith('  \u2192'):
            color = DIM
        elif line.strip().startswith('Patch') or line.strip().startswith('diff'):
            color = CYAN
        elif line.strip().startswith('+'):
            color = GREEN
        elif line.strip().startswith('-'):
            color = RED
        else:
            color = FG_COLOR
        draw.text((PAD, y), line[:COLS], fill=color, font=font)
        y += CHAR_H
    return img


def run_cmd(cmd, env_extra=None):
    env = os.environ.copy()
    env['EVO_ROOT'] = WORK
    env.update(env_extra or {})
    r = subprocess.run(cmd, shell=True, capture_output=True, text=True, env=env, timeout=30)
    return (r.stdout + r.stderr).strip(), r.returncode


def wrap_lines(text, width=COLS):
    lines = []
    for line in text.split('\n'):
        if len(line) <= width:
            lines.append(line)
        else:
            for i in range(0, len(line), width):
                lines.append(line[i:i+width])
    return lines


def main():
    frames = []

    def add_step(title, cmd, output, hold=3):
        out_lines = wrap_lines(output)
        frames.append(render_frame(title, cmd, []))
        frames.append(render_frame(title, cmd, []))
        frames.append(render_frame(title, cmd, out_lines))
        for _ in range(hold * FPS):
            frames.append(render_frame(title, cmd, out_lines))

    out, _ = run_cmd(f"{EVO} status")
    add_step("\u2460 \u521d\u59cb\u72b6\u6001", "evo status", out, hold=3)

    out, _ = run_cmd("head -20 src/gcc/configure | cat")
    add_step("\u2461 \u67e5\u770b gcc configure", "head -20 src/gcc/configure", out, hold=2)

    out, _ = run_cmd("grep -n 'LTO' src/gcc/Makefile.in | head -10")
    add_step("\u2462 \u67e5\u627e LTO \u76f8\u5173\u914d\u7f6e", "grep -n 'LTO' gcc/Makefile.in | head", out, hold=3)

    gcc_c_path = os.path.join(WORK, "src/gcc/gcc.c")
    if os.path.exists(gcc_c_path):
        with open(gcc_c_path, 'r') as f:
            content = f.read()
        marker = '#include "config.h"'
        if marker in content:
            injection = (
                '#include "config.h"\n'
                '\n'
                '/* evo patch: default -fno-lto \u2014 disable LTO by default.\n'
                ' * Prevents excessive memory usage during builds.\n'
                ' * Users can still enable with -flto explicitly.\n'
                ' */\n'
                '#ifndef EVO_NO_LTO_DEFAULT\n'
                '#define EVO_NO_LTO_DEFAULT 1\n'
                '#endif\n'
            )
            content = content.replace(marker, injection, 1)
            with open(gcc_c_path, 'w') as f:
                f.write(content)

    makefile_path = os.path.join(WORK, "src/gcc/Makefile.in")
    if os.path.exists(makefile_path):
        with open(makefile_path, 'r') as f:
            content = f.read()
        old_cflags = "CFLAGS = -g"
        if old_cflags in content:
            new_cflags = "CFLAGS = -g -fno-lto  # evo: disable LTO by default"
            content = content.replace(old_cflags, new_cflags, 1)
            with open(makefile_path, 'w') as f:
                f.write(content)

    out, _ = run_cmd("git diff --stat HEAD")
    add_step("\u2463 \u4fee\u6539 gcc \u6e90\u7801\uff08\u6ce8\u5165 -fno-lto\uff09", "manual edit \u2192 gcc.c + Makefile.in", out, hold=3)

    out, _ = run_cmd("git diff HEAD | head -60")
    add_step("\u2464 \u67e5\u770b\u53d8\u66f4\u8be6\u60c5", "git diff HEAD | head -60", out, hold=4)

    out, _ = run_cmd(f"{EVO} patch create gcc --message 'default-no-lto'")
    add_step("\u2465 \u521b\u5efa Patch", "evo patch create gcc -m 'default-no-lto'", out, hold=3)

    out, _ = run_cmd(f"{EVO} patch list gcc")
    add_step("\u2466 \u5217\u51fa Patch \u6808", "evo patch list gcc", out, hold=2)

    out, _ = run_cmd(f"{EVO} patch show gcc 1")
    show_lines = wrap_lines(out)
    if len(show_lines) > 30:
        show_lines = show_lines[:30] + ["...", "(truncated)"]
    add_step("\u2467 \u67e5\u770b Patch \u5185\u5bb9", "evo patch show gcc 1", '\n'.join(show_lines), hold=4)

    run_cmd("git checkout HEAD~1 -- . && git clean -fd")
    out, _ = run_cmd(f"{EVO} patch apply gcc")
    add_step("\u2468 \u4ece Patch \u91cd\u65b0\u5e94\u7528", "evo patch apply gcc", out, hold=3)

    out, _ = run_cmd("grep -n 'fno-lto' src/gcc/Makefile.in")
    add_step("\u2469 \u9a8c\u8bc1 -fno-lto \u5df2\u6ce8\u5165", "grep -n 'fno-lto' gcc/Makefile.in", out, hold=3)

    out, _ = run_cmd(f"{EVO} tag --create v0.1-no-lto --message 'gcc\u9ed8\u8ba4\u7981\u7528LTO'")
    add_step("\u246a \u521b\u5efa\u7a33\u5b9a\u6807\u8bb0", "evo tag --create v0.1-no-lto", out, hold=2)

    out, _ = run_cmd(f"{EVO} status")
    add_step("\u246b \u6700\u7ec8\u72b6\u6001", "evo status", out, hold=4)

    print(f"\nTotal frames: {len(frames)}")

    frame_dir = "/tmp/evo-frames"
    os.makedirs(frame_dir, exist_ok=True)
    for i, frame in enumerate(frames):
        frame.save(f"{frame_dir}/frame_{i:04d}.png")

    mp4_path = "/tmp/evo-gcc-test.mp4"
    cmd = (
        f"ffmpeg -y -framerate {FPS} -i {frame_dir}/frame_%04d.png "
        f"-vf 'scale=trunc(iw/2)*2:trunc(ih/2)*2' "
        f"-c:v libx264 -pix_fmt yuv420p -preset fast "
        f"{mp4_path}"
    )
    r = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    if r.returncode == 0:
        size_mb = os.path.getsize(mp4_path) / (1024 * 1024)
        print(f"\n\u2705 Video saved: {mp4_path} ({size_mb:.1f} MB)")
    else:
        print(f"ffmpeg error: {r.stderr}")
        gif_path = "/tmp/evo-gcc-test.gif"
        frames[0].save(gif_path, save_all=True, append_images=frames[1:],
                       duration=[1000//FPS]*len(frames), loop=0, optimize=True)
        print(f"Fallback GIF: {gif_path}")

    for f in os.listdir(frame_dir):
        os.remove(os.path.join(frame_dir, f))
    os.rmdir(frame_dir)


if __name__ == '__main__':
    main()
