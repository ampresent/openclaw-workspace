#!/usr/bin/env python3
"""
gen_gif.py — 生成救援系统执行过程的终端 GIF 录屏
"""
from PIL import Image, ImageDraw, ImageFont
import os

FONT_PATH = "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf"
FONT_BOLD = "/usr/share/fonts/truetype/noto/NotoSansMono-Bold.ttf"
OUTPUT = os.path.join(os.path.dirname(__file__), "rescue-demo.gif")

# 终端参数
COLS, ROWS = 88, 32
FONT_SIZE = 14
PAD_X, PAD_Y = 16, 12
CHAR_W = 9.6
CHAR_H = FONT_SIZE + 4
IMG_W = int(COLS * CHAR_W + PAD_X * 2)
IMG_H = int(ROWS * CHAR_H + PAD_Y * 2)

# 颜色 (Solarized Dark)
BG = (0, 43, 54)
FG = (131, 148, 150)
GREEN = (133, 153, 0)
RED = (220, 50, 47)
YELLOW = (181, 137, 0)
CYAN = (42, 161, 152)
WHITE = (238, 232, 213)
BOLD_FG = (253, 246, 227)
DIM = (88, 110, 117)

font = ImageFont.truetype(FONT_PATH, FONT_SIZE)
font_bold = ImageFont.truetype(FONT_BOLD, FONT_SIZE)

def colored_line(text, color=FG):
    return (text, color)

def make_frame(lines, cursor_pos=None):
    img = Image.new("RGB", (IMG_W, IMG_H), BG)
    draw = ImageDraw.Draw(img)
    for i, item in enumerate(lines):
        if isinstance(item, tuple):
            text, color = item
        else:
            text, color = item, FG
        y = PAD_Y + i * CHAR_H
        draw.text((PAD_X, y), text[:COLS], fill=color, font=font)
    if cursor_pos is not None:
        y = PAD_Y + cursor_pos * CHAR_H
        draw.rectangle([PAD_X, y, PAD_X + 8, y + FONT_SIZE + 2], fill=FG)
    return img

# 定义动画场景
PROMPT = "🚑 rescue> "

scenes = [
    # Scene 0: Banner
    {
        "lines": [
            colored_line(""), 
            colored_line(" ╔══════════════════════════════════════════╗", CYAN),
            colored_line(" ║         🚑 Rescue System Shell           ║", CYAN),
            colored_line(" ║     系统故障诊断与修复救援工具            ║", CYAN),
            colored_line(" ╠══════════════════════════════════════════╣", CYAN),
            colored_line(" ║  输入 'help' 查看命令                    ║", CYAN),
            colored_line(" ║  输入自然语言描述问题，自动分析修复       ║", CYAN),
            colored_line(" ╚══════════════════════════════════════════╝", CYAN),
            colored_line(""), 
            colored_line("目标系统: /mnt/rescue-target", DIM),
            colored_line("模型服务: ✅ 运行中 (Qwen2.5-7B)", GREEN),
            colored_line(""), 
            colored_line(PROMPT + "scan", WHITE),
        ],
        "hold": 40,
    },
    # Scene 1: Running diagnostics
    {
        "lines": [
            colored_line(""), 
            colored_line(" ╔══════════════════════════════════════════╗", CYAN),
            colored_line(" ║         🚑 Rescue System Shell           ║", CYAN),
            colored_line(" ╚══════════════════════════════════════════╝", CYAN),
            colored_line(""), 
            colored_line("═══ 全系统诊断 ═══", CYAN),
            colored_line("目标: /mnt/rescue-target", DIM),
            colored_line("输出: /tmp/rescue-diag-output", DIM),
            colored_line(""), 
            colored_line("▶ 运行诊断: disk", CYAN),
            colored_line("", FG),
            colored_line("", FG),
            colored_line(PROMPT, WHITE),
        ],
        "hold": 20,
    },
    # Scene 2: More diagnostics
    {
        "lines": [
            colored_line("═══ 全系统诊断 ═══", CYAN),
            colored_line("目标: /mnt/rescue-target", DIM),
            colored_line(""), 
            colored_line("▶ 运行诊断: disk", CYAN),
            colored_line("✓ disk 完成 (32ms)", GREEN),
            colored_line("▶ 运行诊断: boot", CYAN),
            colored_line("✓ boot 完成 (28ms)", GREEN),
            colored_line("▶ 运行诊断: services", CYAN),
            colored_line("✓ services 完成 (58ms)", GREEN),
            colored_line("▶ 运行诊断: memory", CYAN),
            colored_line("", FG),
            colored_line(PROMPT, WHITE),
        ],
        "hold": 15,
    },
    # Scene 3: All diagnostics done
    {
        "lines": [
            colored_line("✓ disk 完成 (32ms)", GREEN),
            colored_line("✓ boot 完成 (28ms)", GREEN),
            colored_line("✓ services 完成 (58ms)", GREEN),
            colored_line("✓ memory 完成 (26ms)", GREEN),
            colored_line("▶ 运行诊断: network", CYAN),
            colored_line("✓ network 完成 (268ms)", GREEN),
            colored_line("▶ 运行诊断: packages", CYAN),
            colored_line("✓ packages 完成 (38ms)", GREEN),
            colored_line("▶ 运行诊断: kernel", CYAN),
            colored_line("✓ kernel 完成 (25ms)", GREEN),
            colored_line(""), 
            colored_line("═══ 综合诊断报告 ═══", CYAN),
            colored_line("报告已生成: /tmp/rescue-diag-output/report.json", GREEN),
        ],
        "hold": 30,
    },
    # Scene 4: Model analysis
    {
        "lines": [
            colored_line("═══ 综合诊断报告 ═══", CYAN),
            colored_line("报告已生成: /tmp/rescue-diag-output/report.json", GREEN),
            colored_line(""), 
            colored_line("🧠 正在分析诊断报告...", YELLOW),
            colored_line("", FG),
            colored_line("  加载知识库: sysadmin-toolbox 参考...", DIM),
            colored_line("  调用模型: Qwen2.5-7B-Instruct", DIM),
            colored_line("  上下文: 诊断报告(6KB) + 知识参考(3KB)", DIM),
            colored_line("", FG),
            colored_line("", FG),
            colored_line(PROMPT, WHITE),
        ],
        "hold": 30,
    },
    # Scene 5: Analysis results
    {
        "lines": [
            colored_line("🧠 分析完成", GREEN),
            colored_line("", FG),
            colored_line("═══ 诊断结果 ═══", BOLD_FG),
            colored_line("", FG),
            colored_line("🔴 总体严重度: medium", YELLOW),
            colored_line("", FG),
            colored_line("  [1] 🟡 multipath UUID 溢出 (boot)", YELLOW),
            colored_line("      NVMe 设备 UUID 过长，device-mapper 路径管理受影响", FG),
            colored_line("      修复: echo 'blacklist { devnode \"nvme*\" }' >> /etc/multipath.conf", FG),
            colored_line("", FG),
            colored_line("  [2] 🟡 sshd 未运行 (services)", YELLOW),
            colored_line("      远程救援不可用，需手动启动", FG),
            colored_line("      修复: systemctl enable --now sshd", FG),
        ],
        "hold": 35,
    },
    # Scene 6: More findings + fix prompt
    {
        "lines": [
            colored_line("  [2] 🟡 sshd 未运行 (services)", YELLOW),
            colored_line("      修复: systemctl enable --now sshd", FG),
            colored_line("", FG),
            colored_line("  [3] 🟢 无 swap 配置 (memory)", GREEN),
            colored_line("      OOM 时无安全网，建议配置 2GB swap", FG),
            colored_line("      修复: fallocate -l 2G /swapfile && mkswap /swapfile && swapon /swapfile", FG),
            colored_line("", FG),
            colored_line("═══ 修复执行 ═══", CYAN),
            colored_line("发现问题: 3 | 可自动修复: 2", WHITE),
            colored_line("", FG),
            colored_line("处理 [1/3] multipath UUID 溢出...", CYAN),
            colored_line("执行修复? [y]执行 [s]跳过 [q]退出: y", GREEN),
        ],
        "hold": 30,
    },
    # Scene 7: Execution
    {
        "lines": [
            colored_line("执行修复? [y]执行 [s]跳过 [q]退出: y", GREEN),
            colored_line("  ▶ 执行修复...", CYAN),
            colored_line("  ✅ 修复成功", GREEN),
            colored_line("", FG),
            colored_line("处理 [2/3] sshd 未运行...", CYAN),
            colored_line("执行修复? [y]执行 [s]跳过 [q]退出: y", GREEN),
            colored_line("  ▶ 执行修复...", CYAN),
            colored_line("  ✅ 修复成功", GREEN),
            colored_line("", FG),
            colored_line("════════════════════════════════════════", DIM),
            colored_line("📊 修复结果汇总", BOLD_FG),
            colored_line("  ✅ 执行成功: 2    ⏭️ 跳过: 0    ❌ 失败: 0", WHITE),
        ],
        "hold": 40,
    },
    # Scene 8: Final
    {
        "lines": [
            colored_line("📊 修复结果汇总", BOLD_FG),
            colored_line("  ✅ 执行成功: 2    ⏭️ 跳过: 0    ❌ 失败: 0", WHITE),
            colored_line("", FG),
            colored_line("📄 修复报告: /tmp/rescue-reports/repair_20260411_232100.json", DIM),
            colored_line("", FG),
            colored_line("═══ 救援完成 ═══", CYAN),
            colored_line("", FG),
            colored_line("  ✅ multipath 配置已修复", GREEN),
            colored_line("  ✅ sshd 已启动并设为开机自启", GREEN),
            colored_line("  💡 建议手动配置 swap (2GB)", YELLOW),
            colored_line("", FG),
            colored_line(PROMPT + "_", WHITE),
        ],
        "hold": 60,
    },
]

# 生成帧
frames = []
for scene in scenes:
    img = make_frame(scene["lines"])
    for _ in range(scene.get("hold", 20)):
        frames.append(img.copy())

# 导出 GIF
frames[0].save(
    OUTPUT,
    save_all=True,
    append_images=frames[1:],
    duration=80,  # ms per frame
    loop=0,
    optimize=True,
)
print(f"GIF saved: {OUTPUT} ({len(frames)} frames, {os.path.getsize(OUTPUT)//1024}KB)")
