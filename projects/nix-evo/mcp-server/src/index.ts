import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  Tool,
} from "@modelcontextprotocol/sdk/types.js";
import { readFileSync, existsSync } from "fs";
import { join } from "path";
import { homedir } from "os";
import { ensureTunnel, cleanupTunnels } from "./ssh-tunnel.js";
import {
  CONDA_TOOLS,
  formatCondaEnvList,
  formatCondaEnvInfo,
  formatCondaInstall,
  formatCondaExport,
  formatCondaDrift,
  formatCondaLock,
} from "./conda_tools.js";
import {
  CONDA_TOOLS_V3,
  formatFingerprint,
  formatFingerprintCompare,
  formatFingerprintHistory,
  formatMigrate,
  formatRepair,
  formatPkgRisk,
  formatPkgRiskBatch,
  formatTemplates,
  formatProvision,
  formatRemoteSync,
} from "./conda_tools_v3.js";
import {
  CONDA_TOOLS_V4,
  handleCondaToolV4,
} from "./conda_tools_v4.js";

// ─── hosts.toml parsing ─────────────────────────────────────────────────

interface HostEntry {
  url: string;
  token?: string;
  ssh_tunnel?: string;
  description?: string;
}

const CONFIG_PATH = join(
  process.env.XDG_CONFIG_HOME || join(homedir(), ".config"),
  "nix-evo",
  "hosts.toml"
);

function parseHostsToml(content: string): Record<string, HostEntry> {
  const hosts: Record<string, HostEntry> = {};
  let currentSection: string | null = null;

  for (const rawLine of content.split("\n")) {
    const line = rawLine.replace(/#.*$/, "").trim();
    if (!line) continue;

    const sectionMatch = line.match(/^\[hosts\.([a-zA-Z0-9_-]+)\]$/);
    if (sectionMatch) {
      currentSection = sectionMatch[1];
      hosts[currentSection] = { url: "" };
      continue;
    }

    const kvMatch = line.match(/^([a-zA-Z_]+)\s*=\s*"(.*)"$/);
    if (kvMatch && currentSection) {
      const [, key, value] = kvMatch;
      if (key === "url") hosts[currentSection].url = value;
      else if (key === "token") hosts[currentSection].token = value;
      else if (key === "ssh_tunnel") hosts[currentSection].ssh_tunnel = value;
      else if (key === "description") hosts[currentSection].description = value;
    }
  }

  return hosts;
}

function loadHosts(): Record<string, HostEntry> {
  if (existsSync(CONFIG_PATH)) {
    try {
      const content = readFileSync(CONFIG_PATH, "utf-8");
      return parseHostsToml(content);
    } catch (e) {
      console.error(`Warning: Failed to parse ${CONFIG_PATH}: ${e}`);
    }
  }

  const envUrl = process.env.NIX_EVO_AGENT_URL || "http://127.0.0.1:7890";
  const envToken = process.env.NIX_EVO_TOKEN;
  return {
    default: {
      url: envUrl,
      ...(envToken ? { token: envToken } : {}),
    },
  };
}

const hosts = loadHosts();
const hostNames = Object.keys(hosts);

console.error(`Loaded ${hostNames.length} host(s): ${hostNames.join(", ")}`);
if (!existsSync(CONFIG_PATH)) {
  console.error(`Note: No hosts.toml found at ${CONFIG_PATH}, using env vars`);
}

function resolveHost(hostArg?: string): { name: string; entry: HostEntry } {
  if (hostArg && hosts[hostArg]) {
    return { name: hostArg, entry: hosts[hostArg] };
  }
  if (hosts["default"]) {
    return { name: "default", entry: hosts["default"] };
  }
  if (hostNames.length === 1) {
    return { name: hostNames[0], entry: hosts[hostNames[0]] };
  }
  throw new Error(
    `请指定 host 参数。可用主机: ${hostNames.join(", ")}`
  );
}

async function getEffectiveUrl(hostName: string, entry: HostEntry): Promise<string> {
  if (entry.ssh_tunnel) {
    return ensureTunnel(hostName, entry.ssh_tunnel, entry.url);
  }
  return entry.url;
}

// ─── Agent API client ───────────────────────────────────────────────────

async function agentGet(
  hostName: string,
  host: HostEntry,
  path: string,
  params: Record<string, string> = {}
): Promise<any> {
  const baseUrl = await getEffectiveUrl(hostName, host);
  const url = new URL(path, baseUrl);
  for (const [k, v] of Object.entries(params)) {
    if (v) url.searchParams.set(k, v);
  }

  const headers: Record<string, string> = { Accept: "application/json" };
  if (host.token) headers["Authorization"] = `Bearer ${host.token}`;

  const res = await fetch(url.toString(), { headers });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`Agent API ${res.status}: ${body || res.statusText}`);
  }
  return res.json();
}

async function agentPost(
  hostName: string,
  host: HostEntry,
  path: string,
  body: any
): Promise<any> {
  const baseUrl = await getEffectiveUrl(hostName, host);
  const url = new URL(path, baseUrl);

  const headers: Record<string, string> = {
    Accept: "application/json",
    "Content-Type": "application/json",
  };
  if (host.token) headers["Authorization"] = `Bearer ${host.token}`;

  const res = await fetch(url.toString(), {
    method: "POST",
    headers,
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const respBody = await res.text().catch(() => "");
    throw new Error(`Agent API ${res.status}: ${respBody || res.statusText}`);
  }
  return res.json();
}

// ─── Tool definitions ─────────────────────────────────────────────────────

const hostParamDesc = hostNames.length > 1
  ? `服务器标识。可用: ${hostNames.join(", ")}`
  : "服务器标识";

const TOOLS: Tool[] = [
  {
    name: "system_snapshot",
    description: "获取 NixOS 服务器的全局状态快照（服务、磁盘、内存、最近失败）。诊断问题时第一步必调。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: hostParamDesc },
      },
      required: [],
    },
  },
  {
    name: "service_logs",
    description: "获取指定 systemd 服务的 journalctl 日志。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: hostParamDesc },
        unit: { type: "string", description: "服务名，如 nginx.service" },
        lines: { type: "number", description: "日志行数，默认 50", default: 50 },
      },
      required: ["unit"],
    },
  },
  {
    name: "config_read",
    description: "读取 NixOS 配置源码文件。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: hostParamDesc },
        path: { type: "string", description: "配置文件路径，默认 /etc/nixos/configuration.nix" },
      },
      required: [],
    },
  },
  {
    name: "package_info",
    description: "查询已安装包的详细信息。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: hostParamDesc },
        name: { type: "string", description: "包名，如 nginx" },
      },
      required: ["name"],
    },
  },
  {
    name: "generation_diff",
    description: "对比两个 NixOS generation 的差异（包增删、服务变更）。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: hostParamDesc },
        from: { type: "string", description: "起始 generation 编号" },
        to: { type: "string", description: "目标 generation 编号" },
      },
      required: [],
    },
  },
  {
    name: "config_validate",
    description: "Dry-run 验证 NixOS 配置变更，返回影响摘要和风险评估。在 config_apply 之前必须调用。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: hostParamDesc },
        config: { type: "string", description: "新的 NixOS 配置内容" },
      },
      required: ["config"],
    },
  },
  {
    name: "config_apply",
    description: "应用 NixOS 配置变更（执行 nixos-rebuild switch）。会生成新的 generation，支持回滚。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: hostParamDesc },
        config: { type: "string", description: "NixOS 配置内容（可选，不传则使用当前配置文件）" },
        message: { type: "string", description: "变更说明，记录到 generation 注释" },
      },
      required: [],
    },
  },
  {
    name: "rollback_list",
    description: "列出可用的 NixOS generation。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: hostParamDesc },
      },
      required: [],
    },
  },
  {
    name: "rollback_apply",
    description: "回滚到指定的 NixOS generation。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: hostParamDesc },
        target: { type: "number", description: "目标 generation 编号（不指定则回滚到上一个）" },
      },
      required: [],
    },
  },
  ...CONDA_TOOLS,
  ...CONDA_TOOLS_V3,
  ...CONDA_TOOLS_V4,
];

// ─── Risk assessment layer (MCP-side) ─────────────────────────────────────

function formatRiskBadge(level: string): string {
  switch (level) {
    case "safe": return "🟢 安全";
    case "moderate": return "🟡 中等风险";
    case "dangerous": return "🔴 高风险";
    default: return "❓ 未知";
  }
}

function formatValidateOutput(result: any): string {
  const parts: string[] = [];

  parts.push(result.valid ? "✅ 验证通过" : "❌ 验证失败");

  const s = result.summary || {};
  if (s.packages_added?.length) {
    parts.push(`\n📦 将安装 (${s.packages_added.length}):`);
    const shown = s.packages_added.slice(0, 10);
    for (const p of shown) parts.push(`  + ${p}`);
    if (s.packages_added.length > 10) {
      parts.push(`  ... 及其他 ${s.packages_added.length - 10} 个`);
    }
  }
  if (s.packages_removed?.length) {
    parts.push(`\n🗑️  将删除 (${s.packages_removed.length}):`);
    for (const p of s.packages_removed.slice(0, 10)) parts.push(`  - ${p}`);
  }
  if (s.services_restart?.length) {
    parts.push(`\n🔄 将重启 (${s.services_restart.length}):`);
    for (const svc of s.services_restart) parts.push(`  ⟳ ${svc}`);
  }
  if (s.services_stop?.length) {
    parts.push(`\n⏹️  将停止 (${s.services_stop.length}):`);
    for (const svc of s.services_stop) parts.push(`  ■ ${svc}`);
  }

  const level = s.risk_level || "unknown";
  const reasons = s.risk_reasons || [];
  parts.push(`\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  parts.push(`⚠️  风险评估: ${formatRiskBadge(level)}`);
  for (const r of reasons) parts.push(`  • ${r}`);
  if (level === "dangerous") {
    parts.push(`\n⛔ 此变更风险较高，请谨慎确认！`);
  }
  parts.push(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);

  return parts.join("\n");
}

function formatSnapshot(result: any): string {
  const parts: string[] = [];

  parts.push(`🖥️  ${result.hostname} (NixOS ${result.nixos_version})`);
  parts.push(`   内核: ${result.kernel} | 运行时间: ${result.uptime}`);

  const diskWarning = result.disk?.find((d: any) => d.used_pct > 80);
  if (diskWarning) {
    parts.push(`\n⚠️  磁盘使用率高: ${diskWarning.mount} (${diskWarning.used_pct}%)`);
  }

  if (result.recent_failures?.length > 0) {
    parts.push(`\n❌ 失败的服务 (${result.recent_failures.length}):`);
    for (const f of result.recent_failures) {
      parts.push(`  • ${f.unit}`);
      if (f.log_excerpt) parts.push(`    ${f.log_excerpt.slice(0, 120)}`);
    }
  }

  if (result.memory) {
    parts.push(`\n💾 内存: ${result.memory.used} / ${result.memory.total} (可用: ${result.memory.available})`);
  }

  if (result.services?.length) {
    parts.push(`\n🟢 运行中: ${result.services.length} 个服务`);
  }

  return parts.join("\n");
}

function formatGenerations(result: any): string {
  const parts: string[] = [];
  parts.push(`📋 NixOS Generation 历史 (当前: ${result.current})\n`);

  const gens = (result.generations || []).slice(-10).reverse();
  for (const g of gens) {
    const marker = g.number === result.current ? " ← 当前" : "";
    parts.push(`  ${g.number}. ${g.date} ${g.description}${marker}`);
  }

  if (result.diff) {
    const d = result.diff;
    if (d.packages_added?.length || d.packages_removed?.length) {
      parts.push(`\n🔄 包变更 (from→to):`);
      if (d.packages_added.length) parts.push(`  +${d.packages_added.length} 新增`);
      if (d.packages_removed.length) parts.push(`  -${d.packages_removed.length} 删除`);
    }
  }

  return parts.join("\n");
}

function formatRollbackList(result: any): string {
  const parts: string[] = [];
  parts.push(`📋 可用 Generation (当前: ${result.current})\n`);

  const gens = (result.generations || []).slice(-15).reverse();
  for (const g of gens) {
    const marker = g.number === result.current ? " ← 当前" : "";
    parts.push(`  ${g.number}. ${g.date} ${g.description}${marker}`);
  }

  return parts.join("\n");
}

// ─── Main server ──────────────────────────────────────────────────────────

async function main() {
  const server = new Server(
    { name: "nix-evo", version: "0.3.0" },
    { capabilities: { tools: {} } }
  );

  server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: TOOLS,
  }));

  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;
    const a = args || {};

    try {
      const { name: hostName, entry: host } = resolveHost(a.host as string | undefined);
      let result: any;

      switch (name) {
        // ─── NixOS tools ────────────────────────────────────────────
        case "system_snapshot":
          result = await agentGet(hostName, host, "/api/snapshot", { host: a.host as string });
          break;
        case "service_logs":
          result = await agentGet(hostName, host, "/api/logs", {
            host: a.host as string, unit: a.unit as string, lines: String(a.lines || 50),
          });
          break;
        case "config_read":
          result = await agentGet(hostName, host, "/api/config", {
            host: a.host as string, path: a.path as string,
          });
          break;
        case "package_info":
          result = await agentGet(hostName, host, "/api/package", {
            host: a.host as string, name: a.name as string,
          });
          break;
        case "generation_diff":
          result = await agentGet(hostName, host, "/api/generations", {
            host: a.host as string, from: a.from as string, to: a.to as string,
          });
          break;
        case "config_validate":
          result = await agentPost(hostName, host, "/api/config/validate", {
            host: a.host, config: a.config,
          });
          break;
        case "config_apply":
          result = await agentPost(hostName, host, "/api/config/apply", {
            host: a.host, config: a.config, message: a.message,
          });
          break;
        case "rollback_list":
          result = await agentGet(hostName, host, "/api/generations", { host: a.host as string });
          break;
        case "rollback_apply":
          result = await agentPost(hostName, host, "/api/rollback", {
            host: a.host, target: a.target,
          });
          break;

        // ─── Conda tools ────────────────────────────────────────────
        case "conda_list_envs":
          result = await agentGet(hostName, host, "/api/conda/envs", { host: a.host as string });
          break;
        case "conda_env_info":
          result = await agentGet(hostName, host, "/api/conda/packages", {
            host: a.host as string, env: a.env as string,
          });
          break;
        case "conda_install":
          result = await agentPost(hostName, host, "/api/conda/install", {
            env: a.env, packages: a.packages,
          });
          break;
        case "conda_export":
          result = await agentGet(hostName, host, "/api/conda/export", {
            host: a.host as string, env: a.env as string, explicit: String(a.explicit || false),
          });
          break;
        case "conda_drift":
          result = await agentGet(hostName, host, "/api/conda/drift", {
            host: a.host as string, env: a.env as string, yml: a.yml as string,
          });
          break;
        case "conda_lock":
          result = await agentPost(hostName, host, "/api/conda/lock", {
            env: a.env, yml: a.yml, platforms: a.platforms, filename: a.filename,
          });
          break;

        // V3 tools
        case "env_fingerprint":
          result = await agentPost(hostName, host, "/api/env/fingerprint", { env: a.env, save: a.save });
          break;
        case "env_fingerprint_compare":
          result = await agentPost(hostName, host, "/api/env/fingerprint/compare", { env_a: a.env_a, env_b: a.env_b });
          break;
        case "env_fingerprint_history":
          result = await agentGet(hostName, host, "/api/env/fingerprint/history", { env: a.env });
          break;
        case "env_migrate":
          result = await agentPost(hostName, host, "/api/env/migrate", { source: a.source, target: a.target, env_name: a.env_name, file_path: a.file_path, dry_run: a.dry_run });
          break;
        case "env_repair":
          result = await agentPost(hostName, host, "/api/env/repair", { env: a.env, auto_fix: a.auto_fix, check_shared_libs: a.check_shared_libs, check_metadata: a.check_metadata, check_conflicts: a.check_conflicts });
          break;
        case "pkg_risk":
          result = await agentGet(hostName, host, "/api/pkg/risk/" + a.package, {});
          break;
        case "pkg_risk_batch":
          result = await agentPost(hostName, host, "/api/pkg/risk/batch", { packages: a.packages });
          break;
        case "env_templates":
          result = await agentGet(hostName, host, "/api/env/templates", {});
          break;
        case "env_provision":
          result = await agentPost(hostName, host, "/api/env/provision", { template: a.template, env_name: a.env_name, python_version: a.python_version, extra_packages: a.extra_packages, skip_optional: a.skip_optional, dry_run: a.dry_run });
          break;
        case "env_push":
          result = await agentPost(hostName, host, "/api/env/push", { env: a.env, remote_host: { name: "remote", api_url: a.remote_url, api_token: a.remote_token }, remote_env_name: a.remote_env_name, format: a.format });
          break;
        case "env_pull":
          result = await agentPost(hostName, host, "/api/env/pull", { remote_host: { name: "remote", api_url: a.remote_url, api_token: a.remote_token }, remote_env: a.remote_env, local_env_name: a.local_env_name, overwrite: a.overwrite });
          break;

        // V4 tools
        case "env_branch":
          result = await agentPost(hostName, host, "/api/env/branch", { source: a.source, branch_name: a.branch_name, description: a.description });
          break;
        case "env_diff":
          result = await agentGet(hostName, host, "/api/env/diff", { env_a: a.env_a, env_b: a.env_b });
          break;
        case "env_merge":
          result = await agentPost(hostName, host, "/api/env/merge", { source: a.source, target: a.target, strategy: a.strategy });
          break;
        case "conda_sbom":
          result = await agentGet(hostName, host, "/api/conda/sbom", { env: a.env, format: a.format });
          break;
        case "conda_verify":
          result = await agentPost(hostName, host, "/api/conda/verify", { env: a.env, packages: a.packages });
          break;
        case "conda_to_nix":
          result = await agentPost(hostName, host, "/api/conda/to-nix", { env: a.env, environment_yml: a.environment_yml, output_dir: a.output_dir });
          break;
        case "conda_optimize":
          result = await agentGet(hostName, host, "/api/conda/optimize", { env: a.env, check_disk: a.check_disk });
          break;
        case "conda_multiarch":
          result = await agentGet(hostName, host, `/api/conda/multiarch/${a.env}`, { env: a.env, target_arch: a.target_arch });
          break;
        case "conda_analytics":
          result = await agentGet(hostName, host, "/api/conda/analytics", { env: a.env, impact_package: a.impact_package });
          break;

        default:
          throw new Error(`Unknown tool: ${name}`);
      }

      // Format output based on tool type
      let text: string;
      switch (name) {
        case "system_snapshot":
          text = `${formatSnapshot(result)}\n\n---\n\n\`\`\`json\n${JSON.stringify(result, null, 2)}\n\`\`\``;
          break;
        case "config_validate":
          text = `${formatValidateOutput(result)}\n\n---\n\n\`\`\`json\n${JSON.stringify(result, null, 2)}\n\`\`\``;
          break;
        case "generation_diff":
          text = `${formatGenerations(result)}\n\n---\n\n\`\`\`json\n${JSON.stringify(result, null, 2)}\n\`\`\``;
          break;
        case "rollback_list":
          text = `${formatRollbackList(result)}\n\n---\n\n\`\`\`json\n${JSON.stringify(result, null, 2)}\n\`\`\``;
          break;
        case "conda_list_envs":
          text = formatCondaEnvList(result);
          break;
        case "conda_env_info":
          text = formatCondaEnvInfo(result);
          break;
        case "conda_install":
          text = formatCondaInstall(result);
          break;
        case "conda_export":
          text = formatCondaExport(result);
          break;
        case "conda_drift":
          text = formatCondaDrift(result);
          break;
        case "conda_lock":
          text = formatCondaLock(result);
          break;

        // V3 formatters
        case "env_fingerprint":
          text = formatFingerprint(result);
          break;
        case "env_fingerprint_compare":
          text = formatFingerprintCompare(result);
          break;
        case "env_fingerprint_history":
          text = formatFingerprintHistory(result);
          break;
        case "env_migrate":
          text = formatMigrate(result);
          break;
        case "env_repair":
          text = formatRepair(result);
          break;
        case "pkg_risk":
          text = formatPkgRisk(result);
          break;
        case "pkg_risk_batch":
          text = formatPkgRiskBatch(result);
          break;
        case "env_templates":
          text = formatTemplates(result);
          break;
        case "env_provision":
          text = formatProvision(result);
          break;
        case "env_push":
        case "env_pull":
          text = formatRemoteSync(result);
          break;
        // V4 formatters
        case "env_branch":
          text = `✅ Environment branched: ${result.source} → ${result.branch_name}\n  Packages: ${result.package_count}\n  Path: ${result.path || 'unknown'}`;
          break;
        case "env_diff":
          text = `🔍 Environment Diff: ${result.env_a} vs ${result.env_b}\n  Similarity: ${result.similarity_percent}%\n  Only in ${result.env_a}: ${result.packages_only_in_a.length} packages\n  Only in ${result.env_b}: ${result.packages_only_in_b.length} packages\n  Version diffs: ${result.version_differences.length}\n  Channel diffs: ${result.channel_differences.length}`;
          if (result.packages_only_in_a.length) text += '\n\n  Only in ' + result.env_a + ':\n' + result.packages_only_in_a.slice(0, 10).map((p: any) => `    - ${p.name} ${p.version}`).join('\n');
          if (result.packages_only_in_b.length) text += '\n\n  Only in ' + result.env_b + ':\n' + result.packages_only_in_b.slice(0, 10).map((p: any) => `    + ${p.name} ${p.version}`).join('\n');
          break;
        case "env_merge":
          text = `🔀 Merge complete: ${result.source} → ${result.target} (strategy: ${result.strategy})\n  Added: ${result.packages_added.length} packages\n  Updated: ${result.packages_updated.length} packages\n  Conflicts: ${result.packages_conflict.length}`;
          if (result.packages_added.length) text += '\n\n  Added:\n' + result.packages_added.slice(0, 10).map((p: string) => `    + ${p}`).join('\n');
          break;
        case "conda_sbom":
          text = `🔐 SBOM (${result.format}) for ${result.environment}\n  Total: ${result.total_packages} packages\n  Trusted: ${result.trusted_packages}\n  Untrusted: ${result.untrusted_packages}`;
          if (result.untrusted_channels.length) text += '\n\n  ⚠️ Untrusted:\n' + result.untrusted_channels.slice(0, 10).map((p: any) => `    - ${p.package} (${p.channel}, risk: ${p.risk_level})`).join('\n');
          text += '\n\n  BOM: ' + JSON.stringify(result.bom, null, 2).slice(0, 500) + '...';
          break;
        case "conda_verify":
          text = `🔐 Verification: ${result.environment}\n  Checked: ${result.total_checked}\n  ✅ Verified: ${result.verified}\n  ❌ Failed: ${result.failed}`;
          if (result.failed > 0) text += '\n\n  Failed:\n' + result.results.filter((r: any) => !r.verified).map((r: any) => `    - ${r.name}: ${r.details}`).join('\n');
          break;
        case "conda_to_nix":
          text = `📦 Conda → Nix conversion complete\n  Output: ${result.output_dir}/flake.nix\n  Packages: ${result.total_packages} total, ${result.mapped_count} mapped`;
          if (result.unmapped_packages.length) text += '\n  ⚠️ Unmapped: ' + result.unmapped_packages.join(', ');
          text += '\n\n  Mapped packages:\n' + result.mapped_packages.slice(0, 15).map((p: any) => `    ${p.conda_name} → ${p.nix_name}${p.exact_match ? '' : ' (?)'}`).join('\n');
          break;
        case "conda_optimize":
          text = `🏃 Optimization Report: ${result.environment}\n  Health Score: ${result.health_score}/100\n  Packages: ${result.total_packages}${result.env_size_mb ? ' (' + result.env_size_mb + ' MB)' : ''}\n  Channels: ${result.dependency_stats.unique_channels}`;
          if (result.findings.length) text += '\n\n  Findings:\n' + result.findings.map((f: any) => `    ${f.severity === 'critical' ? '🔴' : f.severity === 'warning' ? '🟡' : 'ℹ️'} [${f.category}] ${f.message}`).join('\n');
          if (result.suggestions.length) text += '\n\n  Suggestions:\n' + result.suggestions.map((s: any) => `    💡 ${s.action}: ${s.rationale}${s.command ? '\n      $ ' + s.command : ''}`).join('\n');
          if (result.potentially_unused.length) text += '\n\n  Potentially unused: ' + result.potentially_unused.join(', ');
          break;
        case "conda_multiarch":
          text = `🌐 Multi-arch: ${result.environment} (${result.current_arch} → ${result.target_arch})\n  Migration feasible: ${result.migration_feasible ? '✅' : '❌'}\n  Score: ${result.migration_score}%\n  Available: ${result.available.length}/${result.total_packages}\n  Unavailable: ${result.unavailable.length}`;
          if (result.blockers.length) text += '\n\n  Blockers:\n' + result.blockers.map((b: string) => `    ❌ ${b}`).join('\n');
          if (result.suggestions.length) text += '\n\n  Suggestions:\n' + result.suggestions.map((s: string) => `    💡 ${s}`).join('\n');
          break;
        case "conda_analytics":
          text = `📊 Analytics${result.environment ? ': ' + result.environment : ' (all envs)'}\n  Packages: ${result.ecosystem_overview.total_packages} across ${result.ecosystem_overview.total_environments} envs\n  Channels: ${result.ecosystem_overview.unique_channels}\n  Python: ${result.ecosystem_overview.python_versions.join(', ') || 'N/A'}`;
          if (result.top_packages.length) text += '\n\n  🏆 Top packages:\n' + result.top_packages.slice(0, 10).map((p: any, i: number) => `    ${i + 1}. ${p.name} ${p.version} (score: ${p.importance_score}, rev deps: ${p.reverse_dep_count})`).join('\n');
          if (result.impact_analysis) {
            const ia = result.impact_analysis;
            text += `\n\n  💥 Impact of removing ${ia.target_package}:\n    Direct: ${ia.direct_dependents.length} | Transitive: ${ia.transitive_dependents.length} | Total: ${ia.total_affected}\n    ${ia.safe_to_remove ? '✅ Safe' : '❌ NOT safe'} (${ia.risk_level} risk)`;
          }
          if (result.risk_indicators.length) text += '\n\n  ⚠️ Risks:\n' + result.risk_indicators.map((r: any) => `    [${r.severity}] ${r.message}`).join('\n');
          break;

        default:
          text = JSON.stringify(result, null, 2);
      }

      return {
        content: [{ type: "text" as const, text }],
      };
    } catch (error) {
      const msg = error instanceof Error ? error.message : String(error);
      return {
        content: [{ type: "text" as const, text: `错误: ${msg}` }],
        isError: true,
      };
    }
  });

  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("nix-evo MCP server v0.3.0 running on stdio");
}

main().catch((err) => {
  console.error("Fatal:", err);
  process.exit(1);
});

