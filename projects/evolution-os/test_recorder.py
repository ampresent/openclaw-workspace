#!/usr/bin/env python3
"""Record evo test sequence as a GIF with ANSI color support + Unicode fallback."""

import subprocess, os, time, sys

# Add script dir to path for gif_utils
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gif_utils import (
    load_fonts, parse_ansi_with_fallback,
    CHAR_W, CHAR_H, FONT_SIZE, ANSI_COLORS, BG_COLORS,
)
from PIL import Image, ImageDraw

COLS, ROWS = 100, 40
PAD = 12
BG = (30, 30, 30)
FG_DEFAULT = (204, 204, 204)

EVO = "/root/.openclaw/workspace/projects/evolution-os/evo/target/release/evo"
WORK = "/tmp/evo-test-workdir"

COMMANDS = [
    ("[1] 初始化 curl 包", f"{EVO} init --srpm /tmp/rpmbuild/SRPMS/curl-8.5.0-1.src.rpm curl", {"EVO_ROOT": WORK}, 2),
    ("[2] 查看状态", f"{EVO} status", {"EVO_ROOT": WORK}, 2),
    ("[3] 创建 Patch", f"{EVO} patch create 'add-test-file' --desc '添加测试文件'", {"EVO_ROOT": WORK}, 2),
    ("[4] 列出 Patch", f"{EVO} patch list", {"EVO_ROOT": WORK}, 1),
    ("[5] 创建稳定标记", f"{EVO} tag --create v0.1-test --message '首次稳定版本'", {"EVO_ROOT": WORK}, 2),
    ("[6] 列出标记", f"{EVO} tag --list", {"EVO_ROOT": WORK}, 1),
    ("[7] 冻结系统", f"{EVO} freeze", {"EVO_ROOT": WORK}, 2),
    ("[8] 解冻系统", f"{EVO} freeze --unfreeze", {"EVO_ROOT": WORK}, 2),
    ("[9] 最终状态", f"{EVO} status", {"EVO_ROOT": WORK}, 2),
]


def render_text(lines, title=""):
    mono, mono_bold, font_title, _ = load_fonts()
    w = int(COLS * CHAR_W) + PAD * 2
    title_h = 36 if title else 0
    h = int(ROWS * CHAR_H) + PAD * 2 + title_h
    img = Image.new('RGB', (w, h), BG)
    draw = ImageDraw.Draw(img)
    if title:
        draw.rectangle([0, 0, w, title_h], fill=(42, 42, 42))
        # Use safe title (already replaced in COMMANDS above)
        draw.text((PAD, 8), f"\u25cf evo \u5bb9\u5668\u6d4b\u8bd5  |  {title}", fill=(13, 188, 121), font=font_title)
        draw.line([(0, title_h), (w, title_h)], fill=(60, 60, 60), width=1)
    for line_idx, line_chars in enumerate(lines):
        if line_idx >= ROWS:
            break
        y = PAD + line_idx * CHAR_H + title_h
        x = PAD
        for char, fg, bg, bold in line_chars:
            if char == '\n':
                break
            if bg:
                draw.rectangle([x, y, x + CHAR_W, y + CHAR_H], fill=bg)
            f = mono_bold if bold else mono
            color = fg or FG_DEFAULT
            draw.text((x, y + 1), char, fill=color, font=f)
            x += CHAR_W
    return img


def run_command(cmd, env_extra):
    env = os.environ.copy()
    env.update(env_extra)
    env['TERM'] = 'xterm-256color'
    env['FORCE_COLOR'] = '1'
    result = subprocess.run(
        cmd, shell=True, capture_output=True, text=True, env=env,
        timeout=60, cwd='/root/.openclaw/workspace/projects/evolution-os'
    )
    return result.stdout + result.stderr


def main():
    mono, _, _, _ = load_fonts()
    os.makedirs(WORK, exist_ok=True)
    frames = []
    frame_durations = []

    for title, cmd, env, delay in COMMANDS:
        print(f"\u25b6 {title}: {cmd}")
        output = run_command(cmd, env)
        print(f"  Output: {len(output)} chars")

        all_text = f"$ {cmd}\n" + output
        # Use fallback-aware parser
        parsed = parse_ansi_with_fallback(all_text, mono)
        lines = []
        current_line = []
        for item in parsed:
            if item[0] == '\n':
                lines.append(current_line)
                current_line = []
            else:
                current_line.append(item)
        if current_line:
            lines.append(current_line)

        frames.append(render_text(lines, title))
        frame_durations.append(delay * 1000)
        time.sleep(0.3)

    gif_path = "/tmp/evo-test.gif"
    if frames:
        frames[0].save(
            gif_path, save_all=True, append_images=frames[1:],
            duration=frame_durations, loop=0, optimize=True,
        )
        size_kb = os.path.getsize(gif_path) / 1024
        print(f"\n\u2705 GIF saved: {gif_path} ({size_kb:.0f} KB)")
    else:
        print("\u274c No frames captured")


if __name__ == '__main__':
    main()
