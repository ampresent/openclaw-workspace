/**
 * Conda V3 MCP tools for nix-evo
 *
 * Advanced conda features:
 * - env_fingerprint: environment identity hashing & comparison
 * - env_migrate: cross-tool migration (conda↔micromamba, pip→conda, etc.)
 * - env_repair: diagnose & auto-fix broken environments
 * - pkg_risk: package risk assessment & popularity
 * - env_templates: pre-built environment templates
 * - env_push / env_pull: remote environment sync
 */

import { Tool } from "@modelcontextprotocol/sdk/types.js";

// ─── Tool Definitions ─────────────────────────────────────────────────

export const CONDA_TOOLS_V3: Tool[] = [
  {
    name: "env_fingerprint",
    description:
      "🧬 环境指纹：为 conda 环境生成唯一哈希（基于包+版本+Python版本）。" +
      "可用于检测两个机器上的环境是否"完全相同"，或跟踪环境随时间的演变。" +
      "支持保存快照用于历史追踪。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "环境名称" },
        save: { type: "boolean", description: "保存指纹快照用于历史追踪", default: false },
      },
      required: ["env"],
    },
  },
  {
    name: "env_fingerprint_compare",
    description:
      "🧬 指纹对比：比较两个环境的指纹，报告差异（包数量、版本差异、相似度评分）。" +
      "可用于验证不同机器上环境的一致性。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env_a: { type: "string", description: "第一个环境名称" },
        env_b: { type: "string", description: "第二个环境名称" },
      },
      required: ["env_a", "env_b"],
    },
  },
  {
    name: "env_fingerprint_history",
    description:
      "🧬 指纹历史：查看环境的指纹变化历史。需要之前使用 save=true 保存过快照。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "环境名称" },
      },
      required: ["env"],
    },
  },
  {
    name: "env_migrate",
    description:
      "🔀 环境迁移助手：在不同工具和格式之间迁移环境。" +
      "支持: conda↔micromamba, pip→conda, requirements.txt→environment.yml, " +
      "environment.yml→conda-lock.yml 等。自动检测 pip-only 包并查找 conda 等价物。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        source: {
          type: "string",
          enum: ["conda", "micromamba", "pip", "requirements.txt", "environment.yml", "conda-lock.yml"],
          description: "迁移来源",
        },
        target: {
          type: "string",
          enum: ["conda", "micromamba", "environment.yml", "conda-lock.yml", "requirements.txt"],
          description: "迁移目标",
        },
        env_name: { type: "string", description: "环境名称（某些迁移类型需要）" },
        file_path: { type: "string", description: "源文件路径（从文件迁移时需要）" },
        dry_run: { type: "boolean", description: "仅预览不执行", default: false },
      },
      required: ["source", "target"],
    },
  },
  {
    name: "env_repair",
    description:
      "🏥 环境修复引擎：诊断和修复损坏的 conda 环境。" +
      "检测问题包括：缺失的 .so 共享库、损坏的元数据、版本冲突、孤立的 dist-info。" +
      "支持自动修复模式。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "环境名称" },
        auto_fix: { type: "boolean", description: "自动修复检测到的问题", default: false },
        check_shared_libs: { type: "boolean", description: "检查共享库完整性", default: true },
        check_metadata: { type: "boolean", description: "检查元数据完整性", default: true },
        check_conflicts: { type: "boolean", description: "检查版本冲突", default: true },
      },
      required: ["env"],
    },
  },
  {
    name: "pkg_risk",
    description:
      "📈 包风险评估：查询 conda-forge/PyPI 获取包的下载量、最后更新时间、维护者数量。" +
      "生成风险评分：过期维护、单一维护者、无许可证等风险因素。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        package: { type: "string", description: "包名称" },
      },
      required: ["package"],
    },
  },
  {
    name: "pkg_risk_batch",
    description:
      "📈 批量风险评估：一次评估多个包的风险，识别高风险包。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        packages: { type: "array", items: { type: "string" }, description: "包名称列表" },
      },
      required: ["packages"],
    },
  },
  {
    name: "env_templates",
    description:
      "🎯 环境模板：查看所有预置环境模板。包含 ML-GPU、数据科学、Web 开发、生物信息等。" +
      "每个模板包含固定版本的包列表，确保可复现性。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
      },
      required: [],
    },
  },
  {
    name: "env_provision",
    description:
      "🎯 环境一键部署：从模板一键创建环境。支持自定义 Python 版本、添加额外包。" +
      "模板：ml-gpu, data-science, web-dev, bioinformatics, deep-learning, jupyter。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        template: { type: "string", description: "模板名称，如 ml-gpu, data-science" },
        env_name: { type: "string", description: "环境名称（默认同模板名）" },
        python_version: { type: "string", description: "Python 版本（默认模板指定）" },
        extra_packages: { type: "array", items: { type: "string" }, description: "额外安装的包" },
        skip_optional: { type: "boolean", description: "跳过可选包", default: false },
        dry_run: { type: "boolean", description: "仅预览不执行", default: false },
      },
      required: ["template"],
    },
  },
  {
    name: "env_push",
    description:
      "🌐 推送环境：将本地 conda 环境推送到远程机器。" +
      "导出环境 → 发送到远程 → 在远程重建。支持通过 API 或 SSH 隧道。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "要推送的本地环境" },
        remote_url: { type: "string", description: "远程 nix-evo-agent API URL" },
        remote_token: { type: "string", description: "远程 API token" },
        remote_env_name: { type: "string", description: "远程环境名称（默认同本地）" },
        format: {
          type: "string",
          enum: ["environment-yml", "conda-lock", "explicit", "pip-freeze"],
          description: "导出格式",
          default: "environment-yml",
        },
      },
      required: ["env", "remote_url"],
    },
  },
  {
    name: "env_pull",
    description:
      "🌐 拉取环境：从远程机器拉取 conda 环境并在本地重建。" +
      "从远程获取环境导出 → 在本地创建。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        remote_url: { type: "string", description: "远程 nix-evo-agent API URL" },
        remote_token: { type: "string", description: "远程 API token" },
        remote_env: { type: "string", description: "远程环境名称" },
        local_env_name: { type: "string", description: "本地环境名称（默认同远程）" },
        overwrite: { type: "boolean", description: "覆盖已存在的环境", default: false },
      },
      required: ["remote_url", "remote_env"],
    },
  },
];

// ─── Output Formatters ────────────────────────────────────────────────

export function formatFingerprint(result: any): string {
  const parts: string[] = [];

  parts.push(`🧬 环境指纹: ${result.environment}`);
  parts.push(`   Hash: ${result.hash}`);
  parts.push(`   短Hash: ${result.short_hash}`);
  parts.push(`   Python: ${result.python_version || '?'}`);
  parts.push(`   包数: ${result.package_count}`);
  parts.push(`   后端: ${result.backend}`);
  if (result.platform) parts.push(`   平台: ${result.platform}`);
  if (result.channels?.length) parts.push(`   频道: ${result.channels.join(', ')}`);

  return parts.join('\n');
}

export function formatFingerprintCompare(result: any): string {
  const parts: string[] = [];

  const icon = result.identical ? '✅' : '❌';
  parts.push(`${icon} 环境对比: ${result.env_a} vs ${result.env_b}`);
  parts.push(`   Hash: ${result.hash_a} vs ${result.hash_b}`);
  parts.push(`   相似度: ${result.similarity_score}%`);
  parts.push(`   相同: ${result.identical ? '是' : '否'}\n`);

  if (result.version_diffs?.length) {
    parts.push(`📦 版本差异 (${result.version_diffs.length}):`);
    for (const d of result.version_diffs.slice(0, 10)) {
      parts.push(`  • ${d.name}: ${d.version_a} → ${d.version_b}`);
    }
    if (result.version_diffs.length > 10) parts.push(`  ... 及其他 ${result.version_diffs.length - 10} 个`);
  }

  if (result.only_in_a?.length) {
    parts.push(`\n🔵 仅在 ${result.env_a}: ${result.only_in_a.join(', ')}`);
  }
  if (result.only_in_b?.length) {
    parts.push(`🟢 仅在 ${result.env_b}: ${result.only_in_b.join(', ')}`);
  }

  return parts.join('\n');
}

export function formatFingerprintHistory(result: any): string {
  const parts: string[] = [];

  parts.push(`🧬 指纹历史: ${result.environment} (${result.count} 条记录)\n`);

  for (const snap of result.snapshots || []) {
    parts.push(`  ${snap.timestamp}: ${snap.short_hash} (${snap.package_count} 包)`);
    if (snap.python_version) parts.push(`    Python ${snap.python_version}`);
  }

  return parts.join('\n');
}

export function formatMigrate(result: any): string {
  const parts: string[] = [];

  const icon = result.success ? '✅' : '❌';
  parts.push(`${icon} 迁移: ${result.source} → ${result.target}`);
  parts.push(`   找到包: ${result.packages_found}`);
  parts.push(`   迁移包: ${result.packages_migrated}`);
  if (result.conda_equivalents_found > 0) {
    parts.push(`   conda 等价物: ${result.conda_equivalents_found}`);
  }

  if (result.pip_only_packages?.length) {
    parts.push(`\n⚠️  Pip-only 包 (${result.pip_only_packages.length}):`);
    for (const p of result.pip_only_packages.slice(0, 5)) {
      parts.push(`  • ${p.pip_name} ${p.pip_version} ${p.conda_available ? '(有conda)' : '(pip only)'}`);
    }
  }

  if (result.output_content) {
    const preview = result.output_content.slice(0, 400);
    parts.push(`\n📄 输出:\n\`\`\`\n${preview}${result.output_content.length > 400 ? '\n...' : ''}\n\`\`\``);
  }

  if (result.warnings?.length) {
    parts.push(`\n⚠️  警告:`);
    for (const w of result.warnings) parts.push(`  • ${w}`);
  }

  return parts.join('\n');
}

export function formatRepair(result: any): string {
  const parts: string[] = [];

  const icon = result.success ? '✅' : '⚠️';
  parts.push(`${icon} 环境修复: ${result.environment}`);
  parts.push(`   发现问题: ${result.issues_found}`);
  parts.push(`   已修复: ${result.issues_fixed}`);
  parts.push(`   耗时: ${result.duration_ms}ms\n`);

  const severity_icon = (s: string) => {
    switch (s) {
      case 'critical': return '🔴';
      case 'error': return '❌';
      case 'warning': return '⚠️';
      default: return 'ℹ️';
    }
  };

  for (const issue of result.issues || []) {
    const si = severity_icon(issue.severity);
    const fix = issue.fix_applied ? ' ✅ 已修复' : '';
    parts.push(`  ${si} [${issue.issue_type}] ${issue.description}${fix}`);
    if (issue.fix_command && !issue.fix_applied) {
      parts.push(`     修复: ${issue.fix_command}`);
    }
  }

  if (result.commands_executed?.length) {
    parts.push(`\n🔧 执行的命令:`);
    for (const c of result.commands_executed) parts.push(`  ${c}`);
  }

  return parts.join('\n');
}

export function formatPkgRisk(result: any): string {
  const parts: string[] = [];

  const level_icon = (l: string) => {
    switch (l) {
      case 'low': return '🟢';
      case 'medium': return '🟡';
      case 'high': return '🟠';
      case 'critical': return '🔴';
      default: return '⚪';
    }
  };

  const icon = level_icon(result.risk_level);
  parts.push(`${icon} 包风险: ${result.name}`);
  parts.push(`   风险评分: ${result.risk_score}/100 (${result.risk_level})`);
  parts.push(`   conda: ${result.conda_available ? '✅' : '❌'}  PyPI: ${result.pypi_available ? '✅' : '❌'}\n`);

  if (result.conda_info?.version) {
    parts.push(`📦 conda: ${result.conda_info.version} (${result.conda_info.channel || '?'})`);
  }
  if (result.pypi_info?.version) {
    parts.push(`📦 PyPI: ${result.pypi_info.version}`);
    if (result.pypi_info.license) parts.push(`   许可证: ${result.pypi_info.license}`);
    if (result.pypi_info.maintainer_count !== undefined) {
      parts.push(`   维护者: ${result.pypi_info.maintainer_count}`);
    }
  }

  if (result.risk_factors?.length) {
    parts.push(`\n⚠️  风险因素:`);
    for (const f of result.risk_factors) {
      parts.push(`  • [${f.factor_type}] ${f.description} (+${f.severity})`);
    }
  }

  parts.push(`\n💡 ${result.recommendation}`);

  return parts.join('\n');
}

export function formatPkgRiskBatch(result: any): string {
  const parts: string[] = [];

  parts.push(`📈 批量风险评估 (${result.count} 个包)\n`);
  if (result.high_risk_count > 0) {
    parts.push(`🔴 高风险包: ${result.high_risk_count}\n`);
  }

  const level_icon = (l: string) => {
    switch (l) {
      case 'low': return '🟢';
      case 'medium': return '🟡';
      case 'high': return '🟠';
      case 'critical': return '🔴';
      default: return '⚪';
    }
  };

  for (const pkg of result.packages || []) {
    const icon = level_icon(pkg.risk_level);
    parts.push(`${icon} ${pkg.name}: ${pkg.risk_score}/100`);
  }

  return parts.join('\n');
}

export function formatTemplates(result: any): string {
  const parts: string[] = [];

  parts.push(`🎯 环境模板 (${result.count} 个)\n`);

  for (const t of result.templates || []) {
    const optional = t.packages?.filter((p: any) => p.optional).length || 0;
    const required = t.packages?.filter((p: any) => !p.optional).length || 0;
    parts.push(`📦 ${t.display_name} [${t.name}]`);
    parts.push(`   ${t.description}`);
    parts.push(`   Python ${t.python_version} · ~${t.estimated_size_mb}MB · ${required} 必选 + ${optional} 可选`);
    parts.push(`   标签: ${t.tags.join(', ')}`);
    parts.push('');
  }

  if (result.categories?.length) {
    parts.push(`分类: ${result.categories.join(', ')}`);
  }

  return parts.join('\n');
}

export function formatProvision(result: any): string {
  const parts: string[] = [];

  const icon = result.success ? '✅' : '❌';
  parts.push(`${icon} 环境部署: ${result.environment}`);
  parts.push(`   模板: ${result.template}`);
  parts.push(`   Python: ${result.python_version}`);
  parts.push(`   包数: ${result.packages_installed}`);
  parts.push(`   耗时: ${result.duration_ms}ms`);

  if (result.warnings?.length) {
    parts.push(`\n⚠️  警告:`);
    for (const w of result.warnings) parts.push(`  • ${w}`);
  }

  if (result.commands_executed?.length) {
    parts.push(`\n🔧 后续命令:`);
    for (const c of result.commands_executed) parts.push(`  ${c}`);
  }

  return parts.join('\n');
}

export function formatRemoteSync(result: any): string {
  const parts: string[] = [];

  const icon = result.success ? '✅' : '❌';
  const op = result.operation === 'push' ? '推送' : '拉取';
  parts.push(`${icon} 环境${op}: ${result.local_env} ↔ ${result.remote_env}`);
  parts.push(`   远程主机: ${result.remote_host}`);
  parts.push(`   格式: ${result.format_used}`);
  parts.push(`   包数: ${result.packages_transferred}`);
  parts.push(`   数据量: ${(result.bytes_transferred / 1024).toFixed(1)} KB`);
  parts.push(`   耗时: ${result.duration_ms}ms`);

  if (result.warnings?.length) {
    parts.push(`\n⚠️  警告:`);
    for (const w of result.warnings) parts.push(`  • ${w}`);
  }

  if (result.errors?.length) {
    parts.push(`\n❌ 错误:`);
    for (const e of result.errors) parts.push(`  • ${e}`);
  }

  if (result.commands_executed?.length) {
    parts.push(`\n🔧 命令:`);
    for (const c of result.commands_executed) parts.push(`  ${c}`);
  }

  return parts.join('\n');
}
