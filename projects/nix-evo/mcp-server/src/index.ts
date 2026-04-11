import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  Tool,
} from "@modelcontextprotocol/sdk/types.js";

// ─── Config ───────────────────────────────────────────────────────────────

interface HostConfig {
  agent_url: string;
  token?: string;
}

const AGENT_URL = process.env.NIX_EVO_AGENT_URL || "http://127.0.0.1:7890";
const API_TOKEN = process.env.NIX_EVO_TOKEN || "";

// ─── Agent API client ─────────────────────────────────────────────────────

async function agentGet(path: string, params: Record<string, string> = {}): Promise<any> {
  const url = new URL(path, AGENT_URL);
  for (const [k, v] of Object.entries(params)) {
    if (v) url.searchParams.set(k, v);
  }

  const headers: Record<string, string> = { Accept: "application/json" };
  if (API_TOKEN) headers["Authorization"] = `Bearer ${API_TOKEN}`;

  const res = await fetch(url.toString(), { headers });
  if (!res.ok) throw new Error(`Agent API error: ${res.status} ${res.statusText}`);
  return res.json();
}

async function agentPost(path: string, body: any): Promise<any> {
  const url = new URL(path, AGENT_URL);

  const headers: Record<string, string> = {
    Accept: "application/json",
    "Content-Type": "application/json",
  };
  if (API_TOKEN) headers["Authorization"] = `Bearer ${API_TOKEN}`;

  const res = await fetch(url.toString(), {
    method: "POST",
    headers,
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`Agent API error: ${res.status} ${res.statusText}`);
  return res.json();
}

// ─── Tool definitions ─────────────────────────────────────────────────────

const TOOLS: Tool[] = [
  {
    name: "system_snapshot",
    description: "获取 NixOS 服务器的全局状态快照（服务、磁盘、内存、最近失败）",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
      },
      required: [],
    },
  },
  {
    name: "service_logs",
    description: "获取指定 systemd 服务的 journalctl 日志",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        unit: { type: "string", description: "服务名，如 nginx.service" },
        lines: { type: "number", description: "日志行数，默认 50", default: 50 },
      },
      required: ["unit"],
    },
  },
  {
    name: "config_read",
    description: "读取 NixOS 配置源码文件",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        path: { type: "string", description: "配置文件路径，默认 /etc/nixos/configuration.nix" },
      },
      required: [],
    },
  },
  {
    name: "package_info",
    description: "查询已安装包的详细信息",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        name: { type: "string", description: "包名，如 nginx" },
      },
      required: ["name"],
    },
  },
  {
    name: "generation_diff",
    description: "对比两个 NixOS generation 的差异（包增删、服务变更）",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        from: { type: "string", description: "起始 generation 编号" },
        to: { type: "string", description: "目标 generation 编号" },
      },
      required: [],
    },
  },
  {
    name: "config_validate",
    description: "Dry-run 验证 NixOS 配置变更，返回影响摘要和风险评估",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        config: { type: "string", description: "新的 NixOS 配置内容" },
      },
      required: ["config"],
    },
  },
  {
    name: "config_apply",
    description: "应用 NixOS 配置变更（执行 nixos-rebuild switch）",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        config: { type: "string", description: "NixOS 配置内容（可选，不传则使用当前配置文件）" },
        message: { type: "string", description: "变更说明，记录到 generation 注释" },
      },
      required: [],
    },
  },
  {
    name: "rollback_list",
    description: "列出可用的 NixOS generation",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
      },
      required: [],
    },
  },
  {
    name: "rollback_apply",
    description: "回滚到指定的 NixOS generation",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
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

function addRiskWarning(result: any): string {
  const summary = result.summary || {};
  const level = summary.risk_level || "unknown";
  const reasons = summary.risk_reasons || [];

  let text = JSON.stringify(result, null, 2);

  if (level !== "safe") {
    const warning = [
      "",
      "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
      `⚠️  风险评估: ${formatRiskBadge(level)}`,
      ...reasons.map((r: string) => `  • ${r}`),
      level === "dangerous" ? "\n⛔ 此变更风险较高，请谨慎确认！" : "",
      "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
    ].filter(Boolean).join("\n");

    text += warning;
  }

  return text;
}

// ─── Main server ──────────────────────────────────────────────────────────

async function main() {
  const server = new Server(
    { name: "nix-evo", version: "0.1.0" },
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
      let result: any;

      switch (name) {
        case "system_snapshot":
          result = await agentGet("/api/snapshot", { host: a.host as string });
          break;

        case "service_logs":
          result = await agentGet("/api/logs", {
            host: a.host as string,
            unit: a.unit as string,
            lines: String(a.lines || 50),
          });
          break;

        case "config_read":
          result = await agentGet("/api/config", {
            host: a.host as string,
            path: a.path as string,
          });
          break;

        case "package_info":
          result = await agentGet("/api/package", {
            host: a.host as string,
            name: a.name as string,
          });
          break;

        case "generation_diff":
          result = await agentGet("/api/generations", {
            host: a.host as string,
            from: a.from as string,
            to: a.to as string,
          });
          break;

        case "config_validate":
          result = await agentPost("/api/config/validate", {
            host: a.host,
            config: a.config,
          });
          result = { ...result, _formatted: addRiskWarning(result) };
          break;

        case "config_apply":
          result = await agentPost("/api/config/apply", {
            host: a.host,
            config: a.config,
            message: a.message,
          });
          break;

        case "rollback_list":
          result = await agentGet("/api/generations", { host: a.host as string });
          break;

        case "rollback_apply":
          result = await agentPost("/api/rollback", {
            host: a.host,
            target: a.target,
          });
          break;

        default:
          throw new Error(`Unknown tool: ${name}`);
      }

      // Use formatted version for validate, raw JSON for others
      const text = result._formatted || JSON.stringify(result, null, 2);

      return {
        content: [{ type: "text" as const, text }],
      };
    } catch (error) {
      const msg = error instanceof Error ? error.message : String(error);
      return {
        content: [{ type: "text" as const, text: `Error: ${msg}` }],
        isError: true,
      };
    }
  });

  // Start stdio transport
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("nix-evo MCP server running on stdio");
}

main().catch((err) => {
  console.error("Fatal:", err);
  process.exit(1);
});
