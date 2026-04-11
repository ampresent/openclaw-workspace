#!/usr/bin/env python3
"""
executor.py — 执行修复方案
"""
import json
import sys
import os
import subprocess
import shutil
from datetime import datetime


class RepairExecutor:
    """修复方案执行器"""

    def __init__(self, auto_yes=False, dry_run=False, target="/mnt/rescue-target"):
        self.auto_yes = auto_yes
        self.dry_run = dry_run
        self.target = target
        self.log = []
        self.executed = []

    def confirm(self, finding: dict) -> bool:
        """用户确认"""
        severity = finding.get("severity", "unknown")
        issue = finding.get("issue", "未知问题")
        commands = finding.get("fix", {}).get("commands", [])
        risks = finding.get("fix", {}).get("risks", [])
        rollback = finding.get("fix", {}).get("rollback_commands", [])

        severity_emoji = {"critical": "🔴", "high": "🟠", "medium": "🟡", "low": "🟢"}.get(severity, "⚪")

        print(f"\n{'='*60}")
        print(f"{severity_emoji} [{severity.upper()}] {issue}")
        print(f"{'='*60}")

        if commands:
            print("\n📋 修复命令:")
            for i, cmd in enumerate(commands, 1):
                print(f"  {i}. {cmd}")

        if risks:
            print("\n⚠️  潜在风险:")
            for r in risks:
                print(f"  - {r}")

        if rollback:
            print("\n🔄 回滚命令:")
            for cmd in rollback:
                print(f"  - {cmd}")

        if self.auto_yes:
            print("\n✅ 自动确认 (--auto-yes)")
            return True

        if self.dry_run:
            print("\n🔍 模拟执行 (--dry-run)，跳过")
            return False

        while True:
            choice = input("\n执行修复? [y]执行 [s]跳过 [q]退出: ").strip().lower()
            if choice == "y":
                return True
            elif choice == "s":
                return False
            elif choice == "q":
                print("用户退出")
                sys.exit(0)

    def execute_commands(self, commands: list, rollback_commands: list) -> dict:
        """执行一组命令"""
        results = []
        all_success = True

        for cmd in commands:
            entry = {
                "command": cmd,
                "start_time": datetime.now().isoformat(),
            }

            if self.dry_run:
                entry["status"] = "dry-run"
                entry["stdout"] = "(dry-run mode, not executed)"
                results.append(entry)
                continue

            try:
                # 替换占位符
                cmd_resolved = cmd.replace("{{target}}", self.target)

                result = subprocess.run(
                    cmd_resolved,
                    shell=True,
                    capture_output=True,
                    text=True,
                    timeout=300,
                    env={**os.environ, "RESCUE_TARGET": self.target}
                )

                entry["status"] = "success" if result.returncode == 0 else "failed"
                entry["returncode"] = result.returncode
                entry["stdout"] = result.stdout[:2000] if result.stdout else ""
                entry["stderr"] = result.stderr[:2000] if result.stderr else ""
                entry["end_time"] = datetime.now().isoformat()

                if result.returncode != 0:
                    all_success = False
                    print(f"  ❌ 命令失败: {cmd}")
                    print(f"     错误: {result.stderr[:200]}")

                    # 尝试回滚
                    if rollback_commands:
                        print("  🔄 尝试回滚...")
                        for rb_cmd in rollback_commands:
                            subprocess.run(rb_cmd, shell=True, capture_output=True, timeout=120)

            except subprocess.TimeoutExpired:
                entry["status"] = "timeout"
                entry["error"] = "命令执行超时 (300s)"
                all_success = False
                print(f"  ⏱️  命令超时: {cmd}")

            except Exception as e:
                entry["status"] = "error"
                entry["error"] = str(e)
                all_success = False
                print(f"  💥 执行异常: {e}")

            results.append(entry)

        return {"success": all_success, "results": results}

    def execute_plan(self, analysis: dict) -> dict:
        """执行完整修复方案"""
        findings = analysis.get("findings", [])
        report = {
            "start_time": datetime.now().isoformat(),
            "dry_run": self.dry_run,
            "auto_yes": self.auto_yes,
            "target": self.target,
            "total_findings": len(findings),
            "executed": 0,
            "skipped": 0,
            "failed": 0,
            "details": []
        }

        print(f"\n{'='*60}")
        print(f"🔧 修复方案执行器")
        print(f"{'='*60}")
        print(f"目标系统: {self.target}")
        print(f"问题总数: {len(findings)}")
        print(f"模式: {'模拟执行' if self.dry_run else ('自动确认' if self.auto_yes else '手动确认')}")

        # 按严重程度排序: critical > high > medium > low
        severity_order = {"critical": 0, "high": 1, "medium": 2, "low": 3}
        findings_sorted = sorted(findings, key=lambda f: severity_order.get(f.get("severity", "low"), 3))

        for i, finding in enumerate(findings_sorted, 1):
            fix = finding.get("fix", {})
            commands = fix.get("commands", [])

            if not commands:
                print(f"\n⏭️  [{i}/{len(findings)}] {finding.get('issue')} — 无自动修复命令（需手动处理）")
                report["skipped"] += 1
                continue

            print(f"\n处理 [{i}/{len(findings)}]...")

            if not self.confirm(finding):
                print("  ⏭️  跳过")
                report["skipped"] += 1
                report["details"].append({
                    "issue": finding.get("issue"),
                    "action": "skipped",
                    "severity": finding.get("severity")
                })
                continue

            print("  ▶ 执行修复...")
            result = self.execute_commands(commands, fix.get("rollback_commands", []))

            if result["success"]:
                print("  ✅ 修复成功")
                report["executed"] += 1
            else:
                print("  ❌ 修复失败")
                report["failed"] += 1

            report["details"].append({
                "issue": finding.get("issue"),
                "action": "executed",
                "severity": finding.get("severity"),
                "result": result
            })

            self.executed.append(result)

        report["end_time"] = datetime.now().isoformat()

        # 汇总
        print(f"\n{'='*60}")
        print(f"📊 修复结果汇总")
        print(f"{'='*60}")
        print(f"  ✅ 执行成功: {report['executed']}")
        print(f"  ⏭️  用户跳过: {report['skipped']}")
        print(f"  ❌ 执行失败: {report['failed']}")

        return report


def main():
    import argparse

    parser = argparse.ArgumentParser(description="执行系统修复方案")
    parser.add_argument("analysis_file", help="分析结果 JSON 文件")
    parser.add_argument("--auto-yes", "-y", action="store_true", help="自动确认所有修复")
    parser.add_argument("--dry-run", "-n", action="store_true", help="模拟执行，不真正修改")
    parser.add_argument("--target", "-t", default="/mnt/rescue-target", help="目标系统路径")
    parser.add_argument("--output", "-o", help="修复报告输出路径")
    parser.add_argument("--filter-severity", choices=["critical", "high", "medium", "low"],
                       help="只执行指定严重度及以上的修复")

    args = parser.parse_args()

    # 加载分析结果
    with open(args.analysis_file) as f:
        analysis = json.load(f)

    # 过滤
    if args.filter_severity:
        severity_order = {"critical": 0, "high": 1, "medium": 2, "low": 3}
        min_level = severity_order[args.filter_severity]
        analysis["findings"] = [
            f for f in analysis.get("findings", [])
            if severity_order.get(f.get("severity", "low"), 3) <= min_level
        ]
        print(f"过滤: 仅 {args.filter_severity} 及以上 ({len(analysis['findings'])} 条)")

    # 执行
    executor = RepairExecutor(
        auto_yes=args.auto_yes,
        dry_run=args.dry_run,
        target=args.target
    )
    report = executor.execute_plan(analysis)

    # 保存报告
    report_json = json.dumps(report, ensure_ascii=False, indent=2)
    if args.output:
        with open(args.output, "w") as f:
            f.write(report_json)
        print(f"\n📄 修复报告: {args.output}")
    else:
        print(f"\n{report_json}")


if __name__ == "__main__":
    main()
