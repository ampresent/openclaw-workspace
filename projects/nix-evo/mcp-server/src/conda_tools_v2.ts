/**
 * Conda V2 MCP tools for nix-evo
 *
 * Extended tools covering V2 features:
 * - python_envs: unified view of ALL Python env types
 * - env_sync: export/sync environments between machines
 * - env_test: run smoke tests on environments
 * - resolve_package: cross-source package resolver
 * - cache_status / cache_clean: build cache management
 * - env_health: comprehensive health check
 */

import { Tool } from "@modelcontextprotocol/sdk/types.js";

// ─── Tool Definitions ─────────────────────────────────────────────────

export const CONDA_TOOLS_V2: Tool[] = [
  {
    name: "python_envs",
    description:
      "列出系统上所有 Python 环境（conda, venv, poetry, pipenv, pdm, uv 等）。" +
      "显示跨环境的包冲突检测。全面了解服务器上的 Python 生态。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
      },
      required: [],
    },
  },
  {
    name: "env_sync",
    description:
      "导出/同步 conda 环境状态。支持多种格式：environment.yml, conda-lock, pip freeze, requirements.txt。" +
      "用于在机器之间迁移环境或创建可复现的快照。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "源环境名称" },
        format: {
          type: "string",
          enum: ["conda-pack", "conda-lock", "pip-freeze", "requirements", "environment-yml", "explicit"],
          description: "导出格式，默认 environment-yml",
          default: "environment-yml",
        },
        target_name: { type: "string", description: "目标环境名称（默认同源名称）" },
        include_pip: { type: "boolean", description: "包含 pip 包", default: true },
      },
      required: ["env"],
    },
  },
  {
    name: "env_test",
    description:
      "在 conda 环境上运行冒烟测试。验证包导入、CUDA 可用性、pytest 等。" +
      "自动检测环境类型（ML/数据科学/通用）并选择合适的测试套件。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "环境名称" },
        suite: {
          type: "string",
          enum: ["auto", "default", "ml", "data-science"],
          description: "测试套件，auto 自动检测",
          default: "auto",
        },
        timeout: { type: "number", description: "超时秒数", default: 60 },
      },
      required: ["env"],
    },
  },
  {
    name: "resolve_package",
    description:
      "跨源包解析器：检查包是否在 nixpkgs、conda-forge、PyPI 上可用。" +
      "比较版本，推荐最佳来源。特别适用于判断是否用 conda 还是 nix 管理某个包。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        package: { type: "string", description: "包名称，如 numpy, pytorch, git" },
      },
      required: ["package"],
    },
  },
  {
    name: "cache_status",
    description:
      "查看 conda 包缓存状态：总大小、文件数、过期条目、镜像配置。" +
      "用于管理磁盘空间和离线环境。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
      },
      required: [],
    },
  },
  {
    name: "cache_clean",
    description:
      "清理 conda 包缓存。移除过期的 tarballs、未使用的包。" +
      "支持 dry-run 模式预览将释放的空间。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        dry_run: { type: "boolean", description: "仅预览，不实际删除", default: false },
        remove_tarballs: { type: "boolean", description: "清理 tarballs", default: true },
        remove_packages: { type: "boolean", description: "清理未使用包", default: true },
      },
      required: [],
    },
  },
  {
    name: "env_health",
    description:
      "综合健康检查：运行诊断、漂移检测、冲突分析，生成整体健康报告。" +
      "相当于一键检查整个 conda 生态的状态。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "指定环境名称（可选，不指定则检查全部）" },
      },
      required: [],
    },
  },
];

// ─── Output Formatters ────────────────────────────────────────────────

export function formatPythonEnvs(result: any): string {
  const parts: string[] = [];

  parts.push(`🐍 Python 环境总览 (共 ${result.total_envs || 0} 个)\n`);

  // Summary by type
  const summary = result.summary_by_type || {};
  if (Object.keys(summary).length > 0) {
    parts.push(`按类型分布: ${Object.entries(summary).map(([t, c]) => `${t}(${c})`).join(', ')}\n`);
  }

  // System Python
  const sys = result.system_python;
  if (sys) {
    parts.push(`系统 Python: ${sys.version} at ${sys.path}`);
    const tools = [
      sys.has_uv ? 'uv' : null,
      sys.has_poetry ? 'poetry' : null,
      sys.has_pipenv ? 'pipenv' : null,
      sys.has_pdm ? 'pdm' : null,
      sys.has_pipx ? 'pipx' : null,
    ].filter(Boolean);
    if (tools.length > 0) {
      parts.push(`  可用工具: ${tools.join(', ')}`);
    }
    parts.push('');
  }

  // Environments
  const envs = result.environments || [];
  for (const env of envs) {
    const active = env.is_active ? ' 🟢' : '';
    const py = env.python_version ? ` Python ${env.python_version}` : '';
    const size = env.disk_usage_mb ? ` · ${env.disk_usage_mb}MB` : '';
    parts.push(`• ${env.name}${active} [${env.env_type}]${py}${size}`);
    parts.push(`  ${env.path}`);
  }

  // Conflicts
  const conflicts = result.conflicts || [];
  if (conflicts.length > 0) {
    parts.push(`\n⚠️  跨环境冲突 (${conflicts.length}):`);
    for (const c of conflicts) {
      const icon = c.severity === 'error' ? '❌' : c.severity === 'warning' ? '⚠️' : 'ℹ️';
      const versions = c.installations.map((i: any) => `${i.environment}(${i.version})`).join(', ');
      parts.push(`  ${icon} ${c.package_name}: ${versions}`);
    }
  }

  return parts.join('\n');
}

export function formatEnvSync(result: any): string {
  const parts: string[] = [];

  parts.push(`🔄 环境同步结果:`);
  parts.push(`   源环境: ${result.source_env}`);
  parts.push(`   格式: ${result.format_used}`);
  parts.push(`   导出包数: ${result.packages_exported}\n`);

  if (result.recreate_command) {
    parts.push(`重建命令:`);
    parts.push(`  ${result.recreate_command}\n`);
  }

  if (result.warnings?.length > 0) {
    parts.push(`⚠️  警告:`);
    for (const w of result.warnings) parts.push(`  • ${w}`);
    parts.push('');
  }

  if (result.exported_content) {
    parts.push(`导出内容:\n\`\`\`\n${result.exported_content.slice(0, 500)}${result.exported_content.length > 500 ? '\n...' : ''}\n\`\`\``);
  }

  return parts.join('\n');
}

export function formatEnvTest(result: any): string {
  const parts: string[] = [];

  const icon = result.overall_pass ? '✅' : '❌';
  parts.push(`${icon} 环境测试: ${result.environment}`);
  parts.push(`   通过: ${result.passed}/${result.total_tests} · 耗时: ${result.duration_ms}ms\n`);

  for (const r of result.results) {
    const ri = r.passed ? '✅' : '❌';
    parts.push(`  ${ri} ${r.description} (${r.duration_ms}ms)`);
    if (!r.passed && r.error) {
      parts.push(`     错误: ${r.error.slice(0, 100)}`);
    }
  }

  if (result.recommendations?.length > 0) {
    parts.push(`\n💡 建议:`);
    for (const r of result.recommendations) parts.push(`  • ${r}`);
  }

  return parts.join('\n');
}

export function formatResolvePackage(result: any): string {
  const parts: string[] = [];

  parts.push(`📦 包解析: ${result.package_name}\n`);

  const sources = [
    { name: 'nixpkgs', data: result.nixpkgs },
    { name: 'conda-forge', data: result.conda_forge },
    { name: 'PyPI', data: result.pypi },
  ];

  for (const { name, data } of sources) {
    if (data?.available) {
      parts.push(`  ✅ ${name}: ${data.version || '?'}`);
      if (data.size_mb) parts.push(`     大小: ${data.size_mb.toFixed(1)} MB`);
    } else {
      parts.push(`  ❌ ${name}: 不可用`);
    }
  }

  const rec = result.recommendation;
  if (rec) {
    parts.push(`\n推荐: ${rec.preferred_source} (${rec.confidence})`);
    parts.push(`  ${rec.use_case}`);
  }

  if (result.conflicts?.length > 0) {
    parts.push(`\n⚠️  冲突:`);
    for (const c of result.conflicts) parts.push(`  • ${c}`);
  }

  if (result.compatibility_notes?.length > 0) {
    parts.push(`\n📝 兼容性说明:`);
    for (const n of result.compatibility_notes) parts.push(`  • ${n}`);
  }

  return parts.join('\n');
}

export function formatCacheStatus(result: any): string {
  const parts: string[] = [];

  parts.push(`💾 缓存状态:`);
  parts.push(`   后端: ${result.backend}`);
  parts.push(`   缓存目录: ${result.cache_dir}`);
  parts.push(`   总大小: ${result.total_size_mb} MB\n`);

  const pkgs = result.package_cache;
  parts.push(`📦 包缓存: ${pkgs.size_mb} MB · ${pkgs.file_count} 文件`);
  if (pkgs.oldest_file_age_days !== null) {
    parts.push(`   最旧: ${pkgs.oldest_file_age_days} 天前`);
  }

  const stale = result.stale_entries || [];
  if (stale.length > 0) {
    const staleTotal = stale.reduce((s: number, e: any) => s + e.size_mb, 0);
    parts.push(`\n🧹 过期条目: ${stale.length} 个 (${staleTotal} MB)`);
    parts.push(`   运行 cache_clean 可释放 ${result.cleanup_savings_mb || staleTotal} MB`);
  }

  const mirrors = result.mirrors || [];
  if (mirrors.length > 0) {
    parts.push(`\n🪞 镜像:`);
    for (const m of mirrors) {
      parts.push(`  • ${m.name}: ${m.url} ${m.is_local ? '(本地)' : ''}`);
    }
  }

  return parts.join('\n');
}

export function formatCacheClean(result: any): string {
  const parts: string[] = [];

  if (result.dry_run) {
    parts.push(`🔍 缓存清理预览:`);
  } else {
    parts.push(`🧹 缓存清理完成:`);
  }
  parts.push(`   释放空间: ${result.space_freed_mb} MB`);
  if (!result.dry_run) {
    parts.push(`   清理 tarballs: ${result.tarballs_removed}`);
    parts.push(`   清理包: ${result.packages_removed}`);
  }

  if (result.actions?.length > 0) {
    parts.push(`\n操作:`);
    for (const a of result.actions) parts.push(`  ✓ ${a}`);
  }

  if (result.errors?.length > 0) {
    parts.push(`\n❌ 错误:`);
    for (const e of result.errors) parts.push(`  • ${e}`);
  }

  return parts.join('\n');
}

export function formatEnvHealth(result: any): string {
  const parts: string[] = [];

  const diag = result.diagnostics || result;
  parts.push(`🏥 环境健康报告\n`);

  const envs = diag.environments || [];
  for (const env of envs) {
    let score = 100;
    if (env.conflicts?.length) score -= env.conflicts.length * 10;
    if (env.outdated?.length) score -= Math.min(env.outdated.length * 2, 30);
    if (!env.has_environment_yml) score -= 15;
    score = Math.max(0, Math.min(100, score));

    const icon = score >= 80 ? '🟢' : score >= 50 ? '🟡' : '🔴';
    parts.push(`${icon} ${env.name}: ${score}% (${env.package_count} 包, ${env.disk_usage_mb || '?'} MB)`);
    if (env.python_version) parts.push(`   Python ${env.python_version}`);
    if (env.conflicts?.length) parts.push(`   ⚠️ ${env.conflicts.length} 个冲突`);
    if (env.outdated?.length) parts.push(`   📦 ${env.outdated.length} 个过期包`);
  }

  const warnings = diag.warnings || [];
  if (warnings.length > 0) {
    parts.push(`\n⚠️  警告:`);
    for (const w of warnings) {
      const icon = w.level === 'error' ? '❌' : w.level === 'warning' ? '⚠️' : 'ℹ️';
      parts.push(`  ${icon} [${w.environment}] ${w.message}`);
    }
  }

  if (result.drift) {
    const d = result.drift.drift || result.drift;
    if (d.has_drift) {
      parts.push(`\n🔄 漂移检测: 有差异`);
      if (d.extra_packages?.length) parts.push(`  多余: ${d.extra_packages.slice(0, 5).join(', ')}`);
      if (d.missing_packages?.length) parts.push(`  缺失: ${d.missing_packages.join(', ')}`);
    }
  }

  return parts.join('\n');
}
