/**
 * nix-evo MCP Server — Experimental Features
 * 
 * Tools for: live dashboard metrics, audit trail queries,
 * self-healer status, and Nix flakes conversion.
 */

// ─── Agent API helpers (reuse from index.ts patterns) ──────────────────

interface HostEntry {
  url: string;
  token?: string;
  ssh_tunnel?: string;
  description?: string;
}

async function agentGet(
  hosts: Record<string, HostEntry>,
  hostArg: string | undefined,
  path: string,
  params: Record<string, string> = {}
): Promise<any> {
  const host = resolveHost(hosts, hostArg);
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
  hosts: Record<string, HostEntry>,
  hostArg: string | undefined,
  path: string,
  body: any
): Promise<any> {
  const host = resolveHost(hosts, hostArg);
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

function resolveHost(
  hosts: Record<string, HostEntry>,
  hostArg?: string
): HostEntry {
  if (hostArg && hosts[hostArg]) return hosts[hostArg];
  if (hosts["default"]) return hosts["default"];
  const keys = Object.keys(hosts);
  if (keys.length === 1) return hosts[keys[0]];
  throw new Error(`请指定 host 参数。可用: ${keys.join(", ")}`);
}

// ─── Tool definitions ──────────────────────────────────────────────────

export function getExperimentalTools(hosts: Record<string, HostEntry>) {
  const hostNames = Object.keys(hosts);
  const hostParamDesc =
    hostNames.length > 1
      ? `服务器标识。可用: ${hostNames.join(", ")}`
      : "服务器标识";

  return [
    {
      name: "dashboard_subscribe",
      description:
        "订阅实时系统监控指标。返回当前系统快照（CPU、内存、磁盘、服务状态）。用于实时监控场景。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
          interval_secs: {
            type: "number",
            description: "采样间隔秒数，默认 3",
            default: 3,
          },
        },
        required: [],
      },
    },
    {
      name: "audit_query",
      description:
        "查询审计日志。记录了所有 API 调用的时间戳、操作、参数哈希和结果。用于安全审计和故障追溯。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
          action: {
            type: "string",
            description: "按操作类型过滤，如 config_apply, rollback",
          },
          path: {
            type: "string",
            description: "按 API 路径过滤",
          },
          limit: {
            type: "number",
            description: "返回条数上限，默认 50",
            default: 50,
          },
        },
        required: [],
      },
    },
    {
      name: "healer_status",
      description:
        "查看自愈代理状态。显示监控中的服务健康状况、故障计数、自动修复历史。用于排查服务自动重启问题。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
    },
    {
      name: "flake_convert",
      description:
        "将 configuration.nix 转换为 flake.nix。自动检测 NixOS 频道、硬件模块、服务配置和额外输入。生成可直接使用的 flake.nix 文件。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
          channel: {
            type: "string",
            description: 'NixOS 频道，如 "nixos-24.05"。不指定则自动检测。',
          },
          hostname: {
            type: "string",
            description: "主机名。不指定则从配置中检测。",
          },
          config_content: {
            type: "string",
            description: "configuration.nix 内容（可选，不传则从磁盘读取）",
          },
          extra_inputs: {
            type: "object",
            description: '额外的 flake inputs，如 {"home-manager": "github:nix-community/home-manager"}',
          },
        },
        required: [],
      },
    },
  ];
}

// ─── Tool handlers ─────────────────────────────────────────────────────

export async function handleExperimentalTool(
  name: string,
  args: Record<string, any>,
  hosts: Record<string, HostEntry>
): Promise<{ content: { type: string; text: string }[]; isError?: boolean }> {
  try {
    const host = args.host as string | undefined;
    let result: any;
    let text: string;

    switch (name) {
      case "dashboard_subscribe": {
        // Fetch a single snapshot from the dashboard metrics endpoint
        const params: Record<string, string> = {};
        if (args.interval_secs) params.interval_secs = String(args.interval_secs);
        result = await agentGet(hosts, host, "/api/snapshot", {});
        text = formatDashboardSnapshot(result);
        break;
      }

      case "audit_query": {
        const params: Record<string, string> = {};
        if (args.action) params.action = args.action;
        if (args.path) params.path = args.path;
        if (args.limit) params.limit = String(args.limit);
        result = await agentGet(hosts, host, "/api/audit", params);
        text = formatAuditResult(result);
        break;
      }

      case "healer_status": {
        result = await agentGet(hosts, host, "/api/healer/status", {});
        text = formatHealerStatus(result);
        break;
      }

      case "flake_convert": {
        const body: Record<string, any> = {};
        if (args.channel) body.channel = args.channel;
        if (args.hostname) body.hostname = args.hostname;
        if (args.config_content) body.config_content = args.config_content;
        if (args.extra_inputs) body.extra_inputs = args.extra_inputs;
        result = await agentPost(hosts, host, "/api/flake/convert", body);
        text = formatFlakeResult(result);
        break;
      }

      default:
        throw new Error(`Unknown experimental tool: ${name}`);
    }

    return {
      content: [
        {
          type: "text",
          text: `${text}\n\n---\n\n\`\`\`json\n${JSON.stringify(result, null, 2)}\n\`\`\``,
        },
      ],
    };
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    return {
      content: [{ type: "text", text: `错误: ${msg}` }],
      isError: true,
    };
  }
}

// ─── Formatters ────────────────────────────────────────────────────────

function formatDashboardSnapshot(data: any): string {
  const parts: string[] = [];

  parts.push("📊 系统实时指标");
  parts.push(`   CPU: ${data.cpu_usage_pct?.toFixed(1) ?? "?"}%`);
  parts.push(`   内存: ${data.memory?.usage_pct?.toFixed(1) ?? "?"}%`);
  if (data.load_avg) {
    parts.push(`   负载: ${data.load_avg[0]} ${data.load_avg[1]} ${data.load_avg[2]}`);
  }

  if (data.services?.length) {
    parts.push("\n🔧 服务状态:");
    for (const svc of data.services) {
      const icon = svc.active === "active" ? "✅" : "❌";
      parts.push(`   ${icon} ${svc.name} (${svc.active}/${svc.sub})`);
    }
  }

  return parts.join("\n");
}

function formatAuditResult(data: any): string {
  const parts: string[] = [];

  parts.push(`📋 审计日志 (共 ${data.total} 条，返回 ${data.returned} 条)\n`);

  if (!data.entries?.length) {
    parts.push("  (无记录)");
    return parts.join("\n");
  }

  // Show last 20 entries
  const entries = data.entries.slice(-20).reverse();
  for (const e of entries) {
    const ts = new Date(Number(e.timestamp) * 1000).toISOString().slice(0, 19);
    parts.push(`  [${ts}] ${e.method} ${e.path}`);
    parts.push(`    action=${e.action} result=${e.result} ${e.duration_ms}ms`);
  }

  return parts.join("\n");
}

function formatHealerStatus(data: any): string {
  const parts: string[] = [];

  parts.push(`🩺 自愈代理状态: ${data.running ? "🟢 运行中" : "🔴 未运行"}`);
  parts.push(`   检查间隔: ${data.check_interval_secs}s`);
  parts.push(`   总修复次数: ${data.total_heal_actions}`);
  if (data.last_check) {
    parts.push(`   上次检查: ${data.last_check}`);
  }

  if (data.rules?.length) {
    parts.push("\n📏 规则:");
    for (const r of data.rules) {
      parts.push(`   • ${r.service}: ${r.max_failures} 次失败/${r.window_minutes}min → ${r.action} (冷却 ${r.cooldown_minutes}min)`);
    }
  }

  if (data.service_states?.length) {
    parts.push("\n📊 服务健康:");
    for (const s of data.service_states) {
      const icon = s.healthy ? "✅" : "❌";
      parts.push(`   ${icon} ${s.service} — 故障 ${s.failure_count} 次`);
      if (s.last_action) {
        parts.push(`     上次操作: ${s.last_action}`);
      }
    }
  }

  return parts.join("\n");
}

function formatFlakeResult(data: any): string {
  const parts: string[] = [];

  parts.push("❄️ Flake 转换结果");
  parts.push(`   检测到频道: ${data.detected_channel}`);
  parts.push(`   检测到主机名: ${data.detected_hostname}`);

  if (data.detected_inputs?.length) {
    parts.push(`   检测到 Inputs: ${data.detected_inputs.join(", ")}`);
  }

  if (data.warnings?.length) {
    parts.push("\n⚠️ 注意事项:");
    for (const w of data.warnings) {
      parts.push(`   • ${w}`);
    }
  }

  parts.push("\n📝 生成的 flake.nix:");
  parts.push("\`\`\`nix");
  parts.push(data.flake_nix);
  parts.push("\`\`\`");

  return parts.join("\n");
}
