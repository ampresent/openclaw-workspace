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

// ─── hosts.toml parsing ─────────────────────────────────────────────────

interface HostEntry {
  url: string;
  token?: string;
  ssh_tunnel?: string; // e.g. "user@remote-host:7890"
  description?: string;
}

const CONFIG_PATH = join(
  process.env.XDG_CONFIG_HOME || join(homedir(), ".config"),
  "nix-evo",
  "hosts.toml"
);

/**
 * Minimal TOML parser for hosts.toml structure.
 * Supports: [hosts.name], key = "value", # comments
 */
function parseHostsToml(content: string): Record<string, HostEntry> {
  const hosts: Record<string, HostEntry> = {};
  let currentSection: string | null = null;

  for (const rawLine of content.split("\n")) {
    const line = rawLine.replace(/#.*$/, "").trim();
    if (!line) continue;

    // Section header: [hosts.name]
    const sectionMatch = line.match(/^\[hosts\.([a-zA-Z0-9_-]+)\]$/);
    if (sectionMatch) {
      currentSection = sectionMatch[1];
      hosts[currentSection] = { url: "" };
      continue;
    }

    // Key-value
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

  // Fallback: env vars for single-host setup
  const envUrl = process.env.NIX_EVO_AGENT_URL || "http://127.0.0.1:7890";
  const envToken = process.env.NIX_EVO_TOKEN;
  return {
    default: {
      url: envUrl,
      ...(envToken ? { token: envToken } : {}),
    },
  };
}

// ─── Config ─────────────────────────────────────────────────────────────

const hosts = loadHosts();
const hostNames = Object.keys(hosts);

console.error(`Loaded ${hostNames.length} host(s): ${hostNames.join(", ")}`);
if (!existsSync(CONFIG_PATH)) {
  console.error(`Note: No hosts.toml found at ${CONFIG_PATH}, using env vars`);
}

/**
 * Resolve a host from the tool call argument.
 * If no host specified, uses "default" or the only available host.
 */
function resolveHost(hostArg?: string): HostEntry {
  // Explicit host name
  if (hostArg && hosts[hostArg]) {
    return hosts[hostArg];
  }
  // Default
  if (hosts["default"]) {
    return hosts["default"];
  }
  // Only one host available — use it
  if (hostNames.length === 1) {
    return hosts[hostNames[0]];
  }
  // Multiple hosts, no default — error
  throw new Error(
    `请指定 host 参数。可用主机: ${hostNames.join(", ")}`
  );
}

// ─── Agent API client ───────────────────────────────────────────────────

async function agentGet(
  host: HostEntry,
  path: string,
  params: Record<string, string> = {}
): Promise<any> {
  const url = new URL(path, host.url);
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
  host: HostEntry,
  path: string,
  body: any
): Promise<any> {
  const url = new URL(path, host.url);

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
];

// ─── Risk assessment layer (MCP-side) ─────────────────────────────────────

function formatRiskBadge(level: string): string {
  switch (level) {
    case "safe":
      return "🟢 安全";
    case "moderate":
      return "🟡 中等风险";
    case "dangerous":
      return "🔴 高风险";
    default:
      return "❓ 未知";
  }
}

function formatValidateOutput(result: any): string {
  const parts: string[] = [];

  // Status line
  parts.push(result.valid ? "✅ 验证通过" : "❌ 验证失败");

  // Summary
  const s = result.summary || {};
  if (s.packages_added?.length) {
    parts.push(`\n📦 将安装 (${s.packages_added.length}):`);
    // Show first 10
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

  // Risk assessment
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

  // Disk warnings
  const diskWarning = result.disk?.find((d: any) => d.used_pct > 80);
  if (diskWarning) {
    parts.push(`\n⚠️  磁盘使用率高: ${diskWarning.mount} (${diskWarning.used_pct}%)`);
  }

  // Failed services
  if (result.recent_failures?.length > 0) {
    parts.push(`\n❌ 失败的服务 (${result.recent_failures.length}):`);
    for (const f of result.recent_failures) {
      parts.push(`  • ${f.unit}`);
      if (f.log_excerpt) parts.push(`    ${f.log_excerpt.slice(0, 120)}`);
    }
  }

  // Memory
  if (result.memory) {
    parts.push(`\n💾 内存: ${result.memory.used} / ${result.memory.total} (可用: ${result.memory.available})`);
  }

  // Running services count
  if (result.services?.length) {
    parts.push(`\n🟢 运行中: ${result.services.length} 个服务`);
  }

  return parts.join("\n");
}

function formatGenerations(result: any): string {
  const parts: string[] = [];
  parts.push(`📋 NixOS Generation 历史 (当前: ${result.current})\n`);

  // Show last 10
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
    { name: "nix-evo", version: "0.2.0" },
    { capabilities: { tools: {} } }
  );

  // List tools
  server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: TOOLS,
  }));

  // Call tool
  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;
    const a = args || {};

    try {
      const host = resolveHost(a.host as string | undefined);
      let result: any;

      switch (name) {
        case "system_snapshot":
          result = await agentGet(host, "/api/snapshot", { host: a.host as string });
          break;

        case "service_logs":
          result = await agentGet(host, "/api/logs", {
            host: a.host as string,
            unit: a.unit as string,
            lines: String(a.lines || 50),
          });
          break;

        case "config_read":
          result = await agentGet(host, "/api/config", {
            host: a.host as string,
            path: a.path as string,
          });
          break;

        case "package_info":
          result = await agentGet(host, "/api/package", {
            host: a.host as string,
            name: a.name as string,
          });
          break;

        case "generation_diff":
          result = await agentGet(host, "/api/generations", {
            host: a.host as string,
            from: a.from as string,
            to: a.to as string,
          });
          break;

        case "config_validate":
          result = await agentPost(host, "/api/config/validate", {
            host: a.host,
            config: a.config,
          });
          break;

        case "config_apply":
          result = await agentPost(host, "/api/config/apply", {
            host: a.host,
            config: a.config,
            message: a.message,
          });
          break;

        case "rollback_list":
          result = await agentGet(host, "/api/generations", { host: a.host as string });
          break;

        case "rollback_apply":
          result = await agentPost(host, "/api/rollback", {
            host: a.host,
            target: a.target,
          });
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

  // Start stdio transport
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("nix-evo MCP server v0.2.0 running on stdio");
}

main().catch((err) => {
  console.error("Fatal:", err);
  process.exit(1);
});
