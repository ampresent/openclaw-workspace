#!/usr/bin/env python3
"""
shell.py — 救援系统交互入口
自然语言对话 → 自动调度诊断/分析/修复
"""
import json
import os
import subprocess
import sys
import readline

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_DIR = os.path.join(SCRIPT_DIR, "..", "..")
REPAIR_SCRIPT = os.path.join(PROJECT_DIR, "src", "repair-engine", "repair.py")

HISTORY_FILE = os.path.expanduser("~/.rescue_history")
PROMPT = "🚑 rescue> "

# 内置命令
COMMANDS = {
    "help":    "显示帮助",
    "diag":    "运行诊断 (diag [模块|all])",
    "analyze": "分析诊断报告 (analyze <报告路径>)",
    "fix":     "执行修复 (fix [分析结果路径])",
    "scan":    "完整扫描+修复 (scan [--auto-yes] [--dry-run])",
    "status":  "检查模型服务状态",
    "target":  "设置/查看目标系统 (target [路径])",
    "exit":    "退出",
}

class RescueShell:
    def __init__(self, target="/mnt/rescue-target"):
        self.target = target
        self.history = []
        self.load_history()

    def load_history(self):
        try:
            readline.read_history_file(HISTORY_FILE)
        except FileNotFoundError:
            pass

    def save_history(self):
        try:
            readline.write_history_file(HISTORY_FILE)
        except Exception:
            pass

    def print_banner(self):
        print("""
 ╔══════════════════════════════════════════╗
 ║         🚑 Rescue System Shell           ║
 ║     系统故障诊断与修复救援工具            ║
 ╠══════════════════════════════════════════╣
 ║  输入 'help' 查看命令                    ║
 ║  输入自然语言描述问题，自动分析修复       ║
 ╚══════════════════════════════════════════╝
""")

    def print_help(self):
        print("\n可用命令:")
        for cmd, desc in COMMANDS.items():
            print(f"  {cmd:12s} — {desc}")
        print("\n也可以直接用自然语言描述问题，系统会自动诊断。")
        print()

    def check_model(self):
        """检查模型服务是否运行"""
        import urllib.request
        try:
            req = urllib.request.Request("http://127.0.0.1:8081/health")
            with urllib.request.urlopen(req, timeout=5) as resp:
                return resp.status == 200
        except Exception:
            return False

    def run_command(self, line):
        """解析并执行命令"""
        parts = line.strip().split()
        if not parts:
            return

        cmd = parts[0].lower()
        args = parts[1:]

        if cmd == "help":
            self.print_help()

        elif cmd == "status":
            if self.check_model():
                print("✅ 模型服务运行中 (http://127.0.0.1:8081)")
            else:
                print("❌ 模型服务未运行")
                print("   启动: bash src/model-server/start.sh")

        elif cmd == "target":
            if args:
                self.target = args[0]
                print(f"目标系统设置为: {self.target}")
            else:
                print(f"当前目标: {self.target}")
                if os.path.isdir(self.target):
                    print(f"  ✅ 目录存在")
                    if subprocess.run(["mountpoint", "-q", self.target], capture_output=True).returncode == 0:
                        print(f"  ✅ 已挂载")
                else:
                    print(f"  ⚠️  目录不存在，请先挂载目标系统")

        elif cmd == "diag":
            module = args[0] if args else "all"
            subprocess.run([os.path.join(PROJECT_DIR, "src", "diagnostics", "rescue-diag"),
                          module, self.target])

        elif cmd == "analyze":
            if not args:
                # 找最新的报告
                report_dir = "/tmp/rescue-diag-output"
                report_path = os.path.join(report_dir, "report.json")
                if not os.path.exists(report_path):
                    print("❌ 没有诊断报告，请先运行 diag")
                    return
            else:
                report_path = args[0]

            subprocess.run(["python3",
                          os.path.join(PROJECT_DIR, "src", "repair-engine", "analyzer.py"),
                          report_path])

        elif cmd == "fix":
            if not args:
                # 找最新的分析结果
                analysis_path = "/tmp/rescue-reports"
                files = sorted([f for f in os.listdir(analysis_path) if f.startswith("analysis_")],
                              reverse=True) if os.path.isdir(analysis_path) else []
                if not files:
                    print("❌ 没有分析结果，请先运行 analyze")
                    return
                analysis_path = os.path.join(analysis_path, files[0])
            else:
                analysis_path = args[0]

            extra_args = []
            if "--auto-yes" in args or "-y" in args:
                extra_args.append("--auto-yes")
            if "--dry-run" in args or "-n" in args:
                extra_args.append("--dry-run")

            subprocess.run(["python3",
                          os.path.join(PROJECT_DIR, "src", "repair-engine", "executor.py"),
                          analysis_path, "--target", self.target] + extra_args)

        elif cmd == "scan":
            if not self.check_model():
                print("❌ 模型服务未运行，无法完成完整扫描")
                print("   请先启动: bash src/model-server/start.sh")
                return

            extra_args = []
            if "--auto-yes" in args or "-y" in args:
                extra_args.append("--auto-yes")
            if "--dry-run" in args or "-n" in args:
                extra_args.append("--dry-run")

            subprocess.run(["python3", REPAIR_SCRIPT,
                          "--target", self.target] + extra_args)

        elif cmd in ("exit", "quit", "q"):
            print("👋 再见")
            self.save_history()
            sys.exit(0)

        else:
            # 自然语言 → 当作问题描述，触发完整扫描
            print(f"🤔 理解为你遇到了问题: \"{line}\"")
            print("   启动完整诊断分析流程...\n")
            subprocess.run(["python3", REPAIR_SCRIPT, "--target", self.target])

    def run(self):
        """主循环"""
        self.print_banner()
        print(f"目标系统: {self.target}")
        if self.check_model():
            print("模型服务: ✅ 运行中")
        else:
            print("模型服务: ❌ 未运行 (run: bash src/model-server/start.sh)")
        print()

        while True:
            try:
                line = input(PROMPT)
                if line.strip():
                    self.run_command(line)
            except KeyboardInterrupt:
                print("\n^C")
                continue
            except EOFError:
                print("\n退出")
                break

        self.save_history()


def main():
    target = "/mnt/rescue-target"
    if "--target" in sys.argv:
        idx = sys.argv.index("--target")
        if idx + 1 < len(sys.argv):
            target = sys.argv[idx + 1]

    shell = RescueShell(target=target)
    shell.run()


if __name__ == "__main__":
    main()
