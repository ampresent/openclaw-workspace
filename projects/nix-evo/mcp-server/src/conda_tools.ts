/**
 * Conda environment MCP tools for nix-evo
 *
 * Provides 6 conda-related tools that wrap the agent's conda API endpoints:
 * - conda_list_envs: list all conda environments
 * - conda_env_info: detailed info about one environment
 * - conda_install: install packages into an environment
 * - conda_export: export environment.yml
 * - conda_drift: compare actual vs declared state
 * - conda_lock: generate lockfile from current state
 */

import { Tool } from "@modelcontextprotocol/sdk/types.js";

// ─── Tool Definitions ─────────────────────────────────────────────────

export const CONDA_TOOLS: Tool[] = [
  {
    name: "conda_list_envs",
    description:
      "列出服务器上所有 conda/micromamba 环境。包含环境名称、路径、Python 版本、包数量。" +
      "管理数据科学/ML 服务器时首先调用此工具。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
      },
      required: [],
    },
  },
  {
    name: "conda_env_info",
    description:
      "获取指定 conda 环境的详细信息：已安装包列表、Python 版本、磁盘占用、环境配置文件。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "环境名称，如 ml-project" },
      },
      required: ["env"],
    },
  },
  {
    name: "conda_install",
    description:
      "在指定 conda 环境中安装包。支持 conda-forge、pip 包。" +
      "示例: packages: [\"numpy\", \"pandas\", \"scikit-learn\"]",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "目标环境名称" },
        packages: {
          type: "array",
          items: { type: "string" },
          description: "要安装的包列表",
        },
      },
      required: ["env", "packages"],
    },
  },
  {
    name: "conda_export",
    description:
      "导出 conda 环境的 environment.yml。支持两种格式：标准 environment.yml 或带包 URL 的 explicit 格式。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "要导出的环境名称" },
        explicit: {
          type: "boolean",
          description: "使用 explicit 格式（带完整包 URL），默认 false",
          default: false,
        },
      },
      required: ["env"],
    },
  },
  {
    name: "conda_drift",
    description:
      "检测 conda 环境与 environment.yml 之间的偏差。" +
      "找出：已安装但未声明的包、声明但未安装的包、版本不匹配。" +
      "用于 CI/CD 和环境一致性检查。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "环境名称" },
        yml: { type: "string", description: "environment.yml 文件路径" },
      },
      required: ["env", "yml"],
    },
  },
  {
    name: "conda_lock",
    description:
      "生成 conda-lock.yml 锁文件，锁定当前环境的精确版本。" +
      "支持平台指定（linux-64, linux-aarch64, osx-64, osx-arm64）。" +
      "用于构建可复现的环境。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "环境名称（不指定则从 environment.yml 生成）" },
        yml: { type: "string", description: "environment.yml 路径（与 env 二选一）" },
        platforms: {
          type: "array",
          items: { type: "string" },
          description: "目标平台列表，默认 linux-64",
          default: ["linux-64"],
        },
        filename: {
          type: "string",
          description: "输出文件名，默认 conda-lock.yml",
          default: "conda-lock.yml",
        },
      },
      required: [],
    },
  },
];

// ─── Output Formatters ────────────────────────────────────────────────

export function formatCondaEnvList(result: any): string {
  const parts: string[] = [];

  parts.push(`📦 Conda 环境 (后端: ${result.backend})\n`);

  const envs = result.environments || [];
  if (envs.length === 0) {
    parts.push("  没有发现 conda 环境");
    return parts.join("\n");
  }

  for (const env of envs) {
    const active = env.is_active ? " ← 活跃" : "";
    const py = env.python_version ? ` (Python ${env.python_version})` : "";
    const pkg_count = env.package_count !== null ? ` · ${env.package_count} 包` : "";
    parts.push(`  • ${env.name}${py}${pkg_count}${active}`);
    parts.push(`    ${env.path}`);
  }

  // Warnings
  const warnings = result.warnings || [];
  if (warnings.length > 0) {
    parts.push("\n⚠️  诊断警告:");
    for (const w of warnings) {
      const icon = w.level === "error" ? "❌" : w.level === "warning" ? "⚠️" : "ℹ️";
      parts.push(`  ${icon} [${w.environment}] ${w.message}`);
    }
  }

  return parts.join("\n");
}

export function formatCondaEnvInfo(result: any): string {
  const parts: string[] = [];

  parts.push(`📦 环境: ${result.environment || result.name}`);
  parts.push(`   路径: ${result.path || "未知"}`);
  if (result.python_version) {
    parts.push(`   Python: ${result.python_version}`);
  }
  parts.push(`   包数量: ${result.count || result.packages?.length || 0}`);

  if (result.packages?.length > 0) {
    parts.push(`\n已安装包 (显示前 30 个):`);
    const shown = result.packages.slice(0, 30);
    for (const pkg of shown) {
      parts.push(`  ${pkg.name} ${pkg.version}`);
    }
    if (result.packages.length > 30) {
      parts.push(`  ... 及其他 ${result.packages.length - 30} 个`);
    }
  }

  return parts.join("\n");
}

export function formatCondaInstall(result: any): string {
  const parts: string[] = [];

  if (result.success) {
    parts.push(`✅ 在 ${result.environment} 中操作完成`);
    if (result.changed) {
      parts.push(`   已变更: ${result.packages.join(", ")}`);
    } else {
      parts.push(`   无变更（包已是最新）`);
    }
  } else {
    parts.push(`❌ 操作失败: ${result.packages.join(", ")}`);
  }

  return parts.join("\n");
}

export function formatCondaExport(result: any): string {
  const parts: string[] = [];

  parts.push(`📄 环境导出: ${result.environment}`);
  parts.push(`   格式: ${result.format}\n`);
  parts.push("```yaml");
  parts.push(result.content);
  parts.push("```");

  return parts.join("\n");
}

export function formatCondaDrift(result: any): string {
  const parts: string[] = [];
  const drift = result.drift;

  parts.push(`🔍 漂移检测: ${result.environment}`);
  parts.push(`   声明文件: ${result.yml_path}\n`);

  if (!drift.has_drift) {
    parts.push("✅ 环境与声明文件一致，无漂移");
    return parts.join("\n");
  }

  if (drift.extra_packages?.length > 0) {
    parts.push(`\n📦 多余包 (已安装但未声明, ${drift.extra_packages.length}):`);
    for (const p of drift.extra_packages.slice(0, 15)) parts.push(`  + ${p}`);
    if (drift.extra_packages.length > 15) {
      parts.push(`  ... 及其他 ${drift.extra_packages.length - 15} 个`);
    }
  }

  if (drift.missing_packages?.length > 0) {
    parts.push(`\n❓ 缺失包 (已声明但未安装, ${drift.missing_packages.length}):`);
    for (const p of drift.missing_packages) parts.push(`  - ${p}`);
  }

  if (drift.version_mismatches?.length > 0) {
    parts.push(`\n🔄 版本不匹配 (${drift.version_mismatches.length}):`);
    for (const m of drift.version_mismatches) {
      parts.push(`  ${m.name}: 声明 ${m.declared} vs 实际 ${m.installed}`);
    }
  }

  return parts.join("\n");
}

export function formatCondaLock(result: any): string {
  const parts: string[] = [];

  parts.push(`🔒 conda-lock 生成结果:`);
  parts.push(`   输出文件: ${result.filename}`);
  parts.push(`   平台: ${(result.platforms || []).join(", ")}`);
  parts.push(`   包数量: ${result.package_count || "未知"}`);

  if (result.success) {
    parts.push(`\n✅ 锁文件已生成`);
  } else {
    parts.push(`\n❌ 生成失败`);
  }

  return parts.join("\n");
}
