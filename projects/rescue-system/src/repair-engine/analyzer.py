#!/usr/bin/env python3
"""
analyzer.py — 调用本地小模型分析诊断报告，生成修复方案
"""
import json
import sys
import os
import urllib.request
import urllib.error

MODEL_API = os.environ.get("RESCUE_MODEL_API", "http://127.0.0.1:8081/v1/chat/completions")
MODEL_NAME = os.environ.get("RESCUE_MODEL_NAME", "")
SKILLS_DIR = os.path.join(os.path.dirname(__file__), "..", "..", "skills")

SYSTEM_PROMPT = """你是一个 Linux 系统故障诊断与修复专家。分析系统诊断报告（JSON），识别问题并给出修复方案。

输出严格 JSON 格式：
{"summary":"总述","overall_severity":"critical|high|medium|low","findings":[{"module":"模块","issue":"问题","severity":"级别","evidence":"证据","fix":{"description":"方案","commands":["命令"],"requires_confirmation":true,"rollback_commands":["回滚"],"risks":["风险"]}}],"manual_steps":["人工步骤"]}

规则：只输出JSON；不要编造问题；考虑救援环境（目标挂载在/mnt/rescue-target）"""


def load_skill_references():
    """加载 skill 知识库作为补充参考"""
    refs = []
    refs_dir = os.path.join(SKILLS_DIR, "sysadmin-toolbox", "references")
    if os.path.isdir(refs_dir):
        for fname in os.listdir(refs_dir):
            if fname.endswith(".md"):
                path = os.path.join(refs_dir, fname)
                try:
                    with open(path) as f:
                        content = f.read()
                        # 只取前 3000 字符避免上下文太长
                        refs.append(f"## {fname}\n{content[:1500]}")
                except Exception:
                    pass
    return "\n\n".join(refs[:2])  # 最多 2 个参考文件


def _resolve_model_name() -> str:
    """从 API 自动获取模型名称"""
    if MODEL_NAME:
        return MODEL_NAME
    try:
        req = urllib.request.Request(MODEL_API.replace("/chat/completions", "/models"))
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read())
            if data.get("models"):
                return data["models"][0]["name"]
    except Exception:
        pass
    return ""


def _truncate_report(report_dict: dict, max_chars: int = 6000) -> str:
    """截断过长的诊断报告，适配小模型上下文"""
    trimmed = dict(report_dict)
    for mod in trimmed.get("modules", []):
        checks = mod.get("checks", {})
        for k, v in list(checks.items()):
            if isinstance(v, list) and len(v) > 5:
                checks[k] = v[:5] + [f"... ({len(v)} total, truncated)"]
            elif isinstance(v, str) and len(v) > 300:
                checks[k] = v[:300] + "... (truncated)"
    text = json.dumps(trimmed, ensure_ascii=False, indent=2)
    if len(text) > max_chars:
        text = text[:max_chars] + "\n... (report truncated)"
    return text


def call_model(diagnosis_report: str, skill_refs: str) -> dict:
    """调用本地模型 API"""
    model_name = _resolve_model_name()

    user_message = f"""请分析以下系统诊断报告并给出修复方案：

## 诊断报告
```json
{diagnosis_report}
```

## 运维知识参考
{skill_refs if skill_refs else "（无额外参考）"}
"""

    messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "user", "content": user_message}
    ]
    payload_dict = {
        "messages": messages,
        "temperature": 0.1,
        "max_tokens": 4096,
    }
    if model_name:
        payload_dict["model"] = model_name

    payload = json.dumps(payload_dict).encode("utf-8")

    req = urllib.request.Request(
        MODEL_API,
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST"
    )

    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            result = json.loads(resp.read().decode("utf-8"))
            content = result["choices"][0]["message"]["content"]
            # 尝试提取 JSON（模型可能包裹在 ```json ``` 里）
            content = content.strip()
            if content.startswith("```"):
                content = content.split("\n", 1)[1]
                if content.endswith("```"):
                    content = content[:-3]
                content = content.strip()
            return json.loads(content)
    except urllib.error.URLError as e:
        print(f"❌ 无法连接模型服务: {e}", file=sys.stderr)
        print(f"   API 地址: {MODEL_API}", file=sys.stderr)
        print(f"   请先启动模型服务: bash src/model-server/start.sh", file=sys.stderr)
        sys.exit(1)
    except (json.JSONDecodeError, KeyError) as e:
        print(f"❌ 模型返回格式异常: {e}", file=sys.stderr)
        sys.exit(1)


def analyze(report_path: str) -> dict:
    """主分析流程"""
    # 1. 加载诊断报告
    with open(report_path) as f:
        report = json.load(f)

    # 2. 加载 skill 参考知识
    skill_refs = load_skill_references()

    # 3. 调用模型分析
    print("🧠 正在分析诊断报告...")
    report_text = _truncate_report(report)
    analysis = call_model(report_text, skill_refs)

    # 4. 附加元数据
    analysis["source_report"] = report_path
    analysis["model"] = MODEL_NAME

    return analysis


def main():
    if len(sys.argv) < 2:
        print("用法: analyzer.py <诊断报告.json> [--output 输出文件]")
        sys.exit(1)

    report_path = sys.argv[1]
    output_path = None
    if "--output" in sys.argv:
        idx = sys.argv.index("--output")
        if idx + 1 < len(sys.argv):
            output_path = sys.argv[idx + 1]

    if not os.path.exists(report_path):
        print(f"❌ 报告文件不存在: {report_path}", file=sys.stderr)
        sys.exit(1)

    analysis = analyze(report_path)

    # 输出
    result_json = json.dumps(analysis, ensure_ascii=False, indent=2)

    if output_path:
        with open(output_path, "w") as f:
            f.write(result_json)
        print(f"✅ 分析结果已保存: {output_path}")
    else:
        print(result_json)

    # 摘要输出到 stderr
    severity = analysis.get("overall_severity", "unknown")
    findings_count = len(analysis.get("findings", []))
    severity_emoji = {"critical": "🔴", "high": "🟠", "medium": "🟡", "low": "🟢"}.get(severity, "⚪")
    print(f"\n{severity_emoji} 总体严重度: {severity} | 发现问题: {findings_count}", file=sys.stderr)


if __name__ == "__main__":
    main()
