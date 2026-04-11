#!/usr/bin/env python3
"""Record evo test sequence as a GIF with ANSI color support."""

import subprocess, os, re, sys, time
from PIL import Image, ImageDraw, ImageFont

# ANSI color map
ANSI_COLORS = {
    30: (0, 0, 0),        31: (205, 49, 49),     32: (13, 188, 121),
    33: (229, 229, 16),   34: (36, 114, 200),     35: (188, 63, 188),
    36: (17, 168, 205),   37: (229, 229, 229),    90: (102, 102, 102),
    91: (241, 76, 76),    92: (35, 209, 139),     93: (245, 245, 67),
    94: (59, 142, 234),   95: (214, 112, 214),    96: (41, 184, 219),
    97: (255, 255, 255),
}
BG_COLORS = {k+10: v for k, v in ANSI_COLORS.items() if k < 40}

COLS, ROWS = 100, 40
FONT_SIZE = 14
CHAR_W, CHAR_H = FONT_SIZE * 0.6, FONT_SIZE + 4
PAD = 12
BG = (30, 30, 30)
FG_DEFAULT = (204, 204, 204)

EVO = "/root/.openclaw/workspace/projects/evolution-os/evo/target/release/evo"
WORK = "/tmp/evo-test-workdir"

COMMANDS = [
    ("① 初始化 curl 包", f"{EVO} init --srpm /tmp/rpmbuild/SRPMS/curl-8.5.0-1.src.rpm curl", {"EVO_ROOT": WORK}, 2),
    ("② 查看状态", f"{EVO} status", {"EVO_ROOT": WORK}, 2),
    ("③ 创建 Patch", f"{EVO} patch create 'add-test-file' --desc '添加测试文件'", {"EVO_ROOT": WORK}, 2),
    ("④ 列出 Patch", f"{EVO} patch list", {"EVO_ROOT": WORK}, 1),
    ("⑤ 创建稳定标记", f"{EVO} tag --create v0.1-test --message '首次稳定版本'", {"EVO_ROOT": WORK}, 2),
    ("⑥ 列出标记", f"{EVO} tag --list", {"EVO_ROOT": WORK}, 1),
    ("⑦ 冻结系统", f"{EVO} freeze", {"EVO_ROOT": WORK}, 2),
    ("⑧ 解冻系统", f"{EVO} freeze --unfreeze", {"EVO_ROOT": WORK}, 2),
    ("⑨ 最终状态", f"{EVO} status", {"EVO_ROOT": WORK}, 2),
]


def parse_ansi(text):
    result = []
    i = 0
    fg, bg, bold = None, None, False
    while i < len(text):
        if text[i] == '\x1b' and i+1 < len(text) and text[i+1] == '[':
            j = i + 2
            while j < len(text) and text[j] not in 'mGKHJ':
                j += 1
            if j < len(text) and text[j] == 'm':
                codes = text[i+2:j].split(';')
                for c in codes:
                    try:
                        c = int(c)
                    except ValueError:
                        continue
                    if c == 0:
                        fg, bg, bold = None, None, False
                    elif c == 1:
                        bold = True
                    elif c in ANSI_COLORS:
                        fg = ANSI_COLORS[c]
                    elif c in BG_COLORS:
                        bg = BG_COLORS[c]
                i = j + 1
                continue
            i = j + 1
        elif text[i] == '\r':
            i += 1
        elif text[i] == '\n':
            result.append(('\n', fg, bg, bold))
            i += 1
        else:
            result.append((text[i], fg, bg, bold))
            i += 1
    return result


def render_text(lines, title=""):
    w = int(COLS * CHAR_W) + PAD * 2
    title_h = 36 if title else 0
    h = int(ROWS * CHAR_H) + PAD * 2 + title_h
    img = Image.new('RGB', (w, h), BG)
    draw = ImageDraw.Draw(img)
    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", FONT_SIZE)
        font_bold = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf", FONT_SIZE)
        font_title = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 18)
    except Exception:
        font = ImageFont.load_default()
        font_bold = font
        font_title = font
    if title:
        draw.rectangle([0, 0, w, title_h], fill=(42, 42, 42))
        draw.text((PAD, 8), f"\U0001f9ea evo \u5bb9\u5668\u6d4b\u8bd5  |  {title}", fill=(13, 188, 121), font=font_title)
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
            f = font_bold if bold else font
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
    os.makedirs(WORK, exist_ok=True)
    frames = []
    frame_durations = []

    for title, cmd, env, delay in COMMANDS:
        print(f"\u25b6 {title}: {cmd}")
        output = run_command(cmd, env)
        print(f"  Output: {len(output)} chars")

        all_text = f"$ {cmd}\n" + output
        parsed = parse_ansi(all_text)
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
