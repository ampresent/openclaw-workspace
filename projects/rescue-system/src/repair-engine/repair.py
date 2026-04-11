#!/usr/bin/env python3
"""
repair.py — 修复引擎主入口
整合诊断 → 分析 → 确认 → 执行 的完整流程
"""
import argparse
import json
import os
import subprocess
import sys
from datetime import datetime

# 路径
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_DIR = os.path.join(SCRIPT_DIR, "..", "..")
DIAG_SCRIPT = os.path.join(PROJECT_DIR, "src", "diagnostics", "rescue-diag")


def run_diagnostics(target, modules="all", output_dir="/tmp/rescue-diag-output"):
    """运行诊断"""
    env = os.environ.copy()
    env["RESCUE_OUTPUT_DIR"] = output_dir
    env["RESCUE_TARGET"] = target

    cmd = [DIAG_SCRIPT, modules, target]
    print(f"🔍 运行诊断: {' '.join(cmd)}")

    result = subprocess.run(cmd, capture_output=True, text=True, env=env, timeout=600)
    if result.returncode != 0 and result.stderr:
        print(f"⚠️  诊断警告:\n{result.stderr}", file=sys.stderr)

    report_path = os.path.join(output_dir, "report.json")
    if not os.path.exists(report_path):
        # 单模块模式
        report_path = os.path.join(output_dir, f"{modules}.json")

    if not os.path.exists(report_path):
        print("❌ 诊断报告未生成", file=sys.stderr)
        sys.exit(1)

    return report_path


def run_analysis(report_path, output_path=None):
    """调用模型分析"""
    analyzer = os.path.join(SCRIPT_DIR, "analyzer.py")
    cmd = ["python3", analyzer, report_path]
    if output_path:
        cmd += ["--output", output_path]

    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"❌ 分析失败:\n{result.stderr}", file=sys.stderr)
        sys.exit(1)

    if output_path:
        print(result.stderr, file=sys.stderr)  # 摘要信息
        return output_path
    else:
        return json.loads(result.stdout)


def run_executor(analysis_path, **kwargs):
    """执行修复"""
    executor = os.path.join(SCRIPT_DIR, "executor.py")
    cmd = ["python3", executor, analysis_path]

    if kwargs.get("auto_yes"):
        cmd.append("--auto-yes")
    if kwargs.get("dry_run"):
        cmd.append("--dry-run")
    if kwargs.get("target"):
        cmd += ["--target", kwargs["target"]]
    if kwargs.get("filter_severity"):
        cmd += ["--filter-severity", kwargs["filter_severity"]]
    if kwargs.get("output"):
        cmd += ["--output", kwargs["output"]]

    subprocess.run(cmd)


def main():
    parser = argparse.ArgumentParser(
        description="Rescue System — 系统故障诊断与修复",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
使用示例:
  # 完整流程: 诊断 → 分析 → 修复
  python3 repair.py --target /mnt/rescue-target

  # 只做诊断 + 分析，不修复
  python3 repair.py --target /mnt/rescue-target --no-repair

  # 自动修复 critical 和 high 级别问题
  python3 repair.py --target /mnt/rescue-target --auto-yes --filter-severity high

  # 模拟执行
  python3 repair.py --target /mnt/rescue-target --dry-run

  # 跳过诊断，直接分析已有报告
  python3 repair.py --report /tmp/rescue-diag-output/report.json
        """
    )

    parser.add_argument("--target", "-t", default="/mnt/rescue-target",
                       help="目标系统挂载点 (默认 /mnt/rescue-target)")
    parser.add_argument("--report", "-r",
                       help="已有诊断报告路径（跳过诊断）")
    parser.add_argument("--modules", "-m", default="all",
                       help="诊断模块 (默认 all)")
    parser.add_argument("--no-repair", action="store_true",
                       help="只做诊断和分析，不修复")
    parser.add_argument("--auto-yes", "-y", action="store_true",
                       help="自动确认修复")
    parser.add_argument("--dry-run", "-n", action="store_true",
                       help="模拟执行")
    parser.add_argument("--filter-severity",
                       choices=["critical", "high", "medium", "low"],
                       help="只修复指定级别及以上")
    parser.add_argument("--output-dir", "-d", default="/tmp/rescue-reports",
                       help="报告输出目录")

    args = parser.parse_args()

    os.makedirs(args.output_dir, exist_ok=True)
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")

    print("=" * 60)
    print("🚑 Rescue System — 故障诊断与修复")
    print("=" * 60)
    print(f"目标系统: {args.target}")
    print(f"输出目录: {args.output_dir}")
    print()

    # Step 1: 诊断
    if args.report:
        report_path = args.report
        print(f"📄 使用已有报告: {report_path}")
    else:
        diag_output = os.path.join(args.output_dir, f"diag_{timestamp}")
        os.makedirs(diag_output, exist_ok=True)
        report_path = run_diagnostics(args.target, args.modules, diag_output)
        print(f"📄 诊断报告: {report_path}")

    # Step 2: 分析
    analysis_path = os.path.join(args.output_dir, f"analysis_{timestamp}.json")
    run_analysis(report_path, analysis_path)

    # 加载分析结果
    with open(analysis_path) as f:
        analysis = json.load(f)

    severity = analysis.get("overall_severity", "unknown")
    findings = analysis.get("findings", [])
    print(f"\n📋 发现 {len(findings)} 个问题，严重度: {severity}")

    for f in findings:
        emoji = {"critical": "🔴", "high": "🟠", "medium": "🟡", "low": "🟢"}.get(f.get("severity"), "⚪")
        print(f"  {emoji} [{f.get('severity','').upper()}] {f.get('issue')}")

    # Step 3: 修复
    if args.no_repair:
        print("\n⏭️  跳过修复 (--no-repair)")
        print(f"分析结果: {analysis_path}")
        print("手动执行修复:")
        print(f"  python3 src/repair-engine/executor.py {analysis_path}")
        return

    if not findings:
        print("\n✅ 无需修复")
        return

    repair_report_path = os.path.join(args.output_dir, f"repair_{timestamp}.json")
    run_executor(
        analysis_path,
        auto_yes=args.auto_yes,
        dry_run=args.dry_run,
        target=args.target,
        filter_severity=args.filter_severity,
        output=repair_report_path
    )
    print(f"\n📄 修复报告: {repair_report_path}")


if __name__ == "__main__":
    main()
