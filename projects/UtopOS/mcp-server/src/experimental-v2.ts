/**
 * nix-evo MCP Server — Experimental V2 Features
 * 
 * Tools for: multi-cluster orchestrator, marketplace browser,
 * config dependency graph, generation timeline, smart rollback advisor,
 * and Prometheus metrics exporter.
 */

// ─── Agent API helpers (reuse from experimental.ts patterns) ──────────

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

async function agentGetText(
  hosts: Record<string, HostEntry>,
  hostArg: string | undefined,
  path: string,
  params: Record<string, string> = {}
): Promise<string> {
  const host = resolveHost(hosts, hostArg);
  const url = new URL(path, host.url);
  for (const [k, v] of Object.entries(params)) {
    if (v) url.searchParams.set(k, v);
  }
  const headers: Record<string, string> = {};
  if (host.token) headers["Authorization"] = `Bearer ${host.token}`;
  const res = await fetch(url.toString(), { headers });
  if (!res.ok) throw new Error(`Agent API ${res.status}: ${res.statusText}`);
  return res.text();
}

function resolveHost(
  hosts: Record<string, HostEntry>,
  hostArg?: string
): HostEntry {
  if (hostArg && hosts[hostArg]) return hosts[hostArg];
  if (hosts["default"]) return hosts["default"];
  const keys = Object.keys(hosts);
  if (keys.length === 1) return hosts[keys[keys.length - 1]];
  throw new Error(`请指定 host 参数。可用: ${keys.join(", ")}`);
}

// ─── Tool definitions ──────────────────────────────────────────────────

export function getExperimentalV2Tools(hosts: Record<string, HostEntry>) {
  const hostNames = Object.keys(hosts);
  const hostParamDesc =
    hostNames.length > 1
      ? `服务器标识。可用: ${hostNames.join(", ")}`
      : "服务器标识";

  return [
    {
      name: "cluster_deploy",
      description:
        "在多个 NixOS 服务器上执行部署命令。支持 fan-out（并行）、fan-in（收集结果）、rolling（滚动部署，失败即停）三种策略。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
          command: {
            type: "string",
            description: '要执行的命令，如 "nixos-rebuild switch"',
          },
          strategy: {
            type: "string",
            enum: ["fan-out", "fan-in", "rolling"],
            description: "部署策略，默认 fan-out",
            default: "fan-out",
          },
          nodes: {
            type: "array",
            items: { type: "string" },
            description: "目标节点名列表（不指定则全部）",
          },
          stop_on_failure: {
            type: "boolean",
            description: "rolling 策略下遇到失败是否停止，默认 true",
            default: true,
          },
          timeout_secs: {
            type: "number",
            description: "每个节点超时秒数，默认 300",
            default: 300,
          },
        },
        required: ["command"],
      },
    },
    {
      name: "cluster_status",
      description:
        "查看集群状态：节点列表、可达性、延迟、最近部署结果。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
    },
    {
      name: "marketplace_search",
      description:
        "搜索 Nix 包。通过 search.nixos.org API 搜索 nixpkgs，返回包名、版本、描述。支持 channel 过滤。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
          query: { type: "string", description: "搜索关键词" },
          channel: {
            type: "string",
            description: 'NixOS 频道，默认 "unstable"',
            default: "unstable",
          },
          limit: {
            type: "number",
            description: "结果数量上限，默认 10",
            default: 10,
          },
        },
        required: ["query"],
      },
    },
    {
      name: "deps_graph",
      description:
        "分析 configuration.nix 的服务依赖关系，生成依赖图。可导出 JSON 或 Graphviz DOT 格式。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
          config_path: {
            type: "string",
            description: "配置文件路径，默认 /etc/nixos/configuration.nix",
          },
          format: {
            type: "string",
            enum: ["json", "dot"],
            description: "输出格式，默认 json",
            default: "json",
          },
          depth: {
            type: "number",
            description: "依赖递归深度，默认 5",
            default: 5,
          },
        },
        required: [],
      },
    },
    {
      name: "timeline_view",
      description:
        "查看 NixOS 生成历史时间线。显示每个代的日期、描述、风险等级。支持对比任意两代。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
          limit: {
            type: "number",
            description: "显示最近 N 个生成，默认 20",
            default: 20,
          },
          compare_from: {
            type: "number",
            description: "对比起始代号（需配合 compare_to）",
          },
          compare_to: {
            type: "number",
            description: "对比终止代号",
          },
        },
        required: [],
      },
    },
    {
      name: "advisor_recommend",
      description:
        "智能回滚建议。分析最近的生成历史，基于服务健康度、运行时间、稳定性等指标，推荐最佳回滚目标（而非简单的"上一代"）。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
          lookback: {
            type: "number",
            description: "分析最近 N 个生成，默认 10",
            default: 10,
          },
          critical_services: {
            type: "array",
            items: { type: "string" },
            description: "需要优先考虑的关键服务列表",
          },
        },
        required: [],
      },
    },
    {
      name: "metrics_export",
      description:
        "导出 Prometheus 格式的监控指标。包含 API 调用数、响应时间、生成数量、自愈操作等指标，可直接接入 Grafana。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
    },
  ];
}

// ─── Tool handlers ─────────────────────────────────────────────────────

export async function handleExperimentalV2Tool(
  name: string,
  args: Record<string, any>,
  hosts: Record<string, HostEntry>
): Promise<{ content: { type: string; text: string }[]; isError?: boolean }> {
  try {
    const host = args.host as string | undefined;
    let result: any;
    let text: string;

    switch (name) {
      case "cluster_deploy": {
        const body: Record<string, any> = { command: args.command };
        if (args.strategy) body.strategy = args.strategy;
        if (args.nodes) body.nodes = args.nodes;
        if (args.stop_on_failure !== undefined) body.stop_on_failure = args.stop_on_failure;
        if (args.timeout_secs) body.timeout_secs = args.timeout_secs;
        result = await agentPost(hosts, host, "/api/cluster/deploy", body);
        text = formatClusterDeploy(result);
        break;
      }

      case "cluster_status": {
        result = await agentGet(hosts, host, "/api/cluster/status");
        text = formatClusterStatus(result);
        break;
      }

      case "marketplace_search": {
        const params: Record<string, string> = { q: args.query };
        if (args.channel) params.channel = args.channel;
        if (args.limit) params.limit = String(args.limit);
        result = await agentGet(hosts, host, "/api/marketplace/search", params);
        text = formatMarketplace(result);
        break;
      }

      case "deps_graph": {
        const params: Record<string, string> = {};
        if (args.config_path) params.config_path = args.config_path;
        if (args.format) params.format = args.format;
        if (args.depth) params.depth = String(args.depth);
        if (args.format === "dot") {
          const dot = await agentGetText(hosts, host, "/api/deps/graph", params);
          text = `📊 Config Dependency Graph (DOT format)\n\n\`\`\`dot\n${dot}\n\`\`\``;
          result = { dot };
        } else {
          result = await agentGet(hosts, host, "/api/deps/graph", params);
          text = formatDepsGraph(result);
        }
        break;
      }

      case "timeline_view": {
        if (args.compare_from && args.compare_to) {
          result = await agentGet(hosts, host, "/api/timeline/compare", {
            from: String(args.compare_from),
            to: String(args.compare_to),
          });
          text = formatCompare(result, args.compare_from, args.compare_to);
        } else {
          const params: Record<string, string> = {};
          if (args.limit) params.limit = String(args.limit);
          result = await agentGet(hosts, host, "/api/timeline", params);
          text = formatTimeline(result);
        }
        break;
      }

      case "advisor_recommend": {
        const body: Record<string, any> = {};
        if (args.lookback) body.lookback = args.lookback;
        if (args.critical_services) body.critical_services = args.critical_services;
        result = await agentPost(hosts, host, "/api/advisor/recommend", body);
        text = formatAdvisor(result);
        break;
      }

      case "metrics_export": {
        const metricsText = await agentGetText(hosts, host, "/metrics");
        text = `📡 Prometheus Metrics\n\n\`\`\`\n${metricsText.slice(0, 3000)}${metricsText.length > 3000 ? '\n... (truncated)' : ''}\n\`\`\``;
        result = { metrics: metricsText };
        break;
      }

      default:
        throw new Error(`Unknown experimental-v2 tool: ${name}`);
    }

    const jsonStr = typeof result === "string" ? result : JSON.stringify(result, null, 2);
    return {
      content: [
        {
          type: "text",
          text: `${text}\n\n---\n\n\`\`\`json\n${jsonStr.slice(0, 2000)}${jsonStr.length > 2000 ? '\n... (truncated)' : ''}\n\`\`\``,
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

function formatClusterDeploy(data: any): string {
  const parts: string[] = [];
  parts.push(`🚀 集群部署结果 [${data.strategy}]`);
  parts.push(`   节点: ${data.total_nodes} | 完成: ${data.completed} | 失败: ${data.failed}`);
  parts.push("");

  if (data.results?.length) {
    for (const r of data.results) {
      const icon = r.success ? "✅" : "❌";
      parts.push(`   ${icon} ${r.node} (${r.duration_ms}ms)`);
      if (r.error) parts.push(`      Error: ${r.error.slice(0, 100)}`);
    }
  }

  return parts.join("\n");
}

function formatClusterStatus(data: any): string {
  const parts: string[] = [];
  parts.push(`🌐 集群状态 — ${data.node_count} 个节点`);
  parts.push("");

  if (data.nodes?.length) {
    for (const n of data.nodes) {
      const icon = n.reachable ? "🟢" : "🔴";
      const latency = n.latency_ms ? ` (${n.latency_ms}ms)` : "";
      parts.push(`   ${icon} ${n.name} — ${n.url}${latency}`);
    }
  } else {
    parts.push("   (无节点配置)");
  }

  if (data.last_deploy) {
    parts.push("");
    parts.push(`📦 最近部署: ${data.last_deploy.strategy} — ${data.last_deploy.finished_at || '进行中'}`);
  }

  return parts.join("\n");
}

function formatMarketplace(data: any): string {
  const parts: string[] = [];
  parts.push(`📦 搜索 "${data.query}" — ${data.count} 个结果 (${data.channel})`);
  parts.push("");

  if (data.packages?.length) {
    for (const pkg of data.packages.slice(0, 15)) {
      parts.push(`• ${pkg.name} (${pkg.version})`);
      if (pkg.description) parts.push(`  ${pkg.description.slice(0, 120)}`);
      if (pkg.license) parts.push(`  License: ${pkg.license}`);
      parts.push("");
    }
  }

  return parts.join("\n");
}

function formatDepsGraph(data: any): string {
  const parts: string[] = [];
  parts.push(`🔍 Config Dependency Graph — ${data.nodes?.length || 0} nodes, ${data.edges?.length || 0} edges`);
  parts.push("");

  // Group by kind
  const byKind: Record<string, any[]> = {};
  (data.nodes || []).forEach((n: any) => {
    if (!byKind[n.kind]) byKind[n.kind] = [];
    byKind[n.kind].push(n);
  });

  for (const [kind, nodes] of Object.entries(byKind)) {
    const icon = kind === "service" ? "🔧" : kind === "library" ? "📚" : kind === "runtime" ? "⚙️" : "🔨";
    parts.push(`${icon} ${kind.toUpperCase()}:`);
    for (const n of nodes) {
      const status = n.enabled ? "✅ enabled" : "○ available";
      parts.push(`   ${n.label} — ${status}`);
    }
    parts.push("");
  }

  if (data.edges?.length) {
    parts.push("🔗 Dependencies:");
    for (const e of data.edges.slice(0, 20)) {
      parts.push(`   ${e.from} → ${e.to} (${e.kind})`);
    }
    if (data.edges.length > 20) parts.push(`   ... and ${data.edges.length - 20} more`);
  }

  return parts.join("\n");
}

function formatTimeline(data: any): string {
  const parts: string[] = [];
  parts.push(`🧬 NixOS 生成时间线 — ${data.total} 个生成`);
  parts.push("");

  const gens = data.generations || [];
  const recent = gens.slice(-15);
  for (const g of recent) {
    const icon = g.is_current ? "▶" : "○";
    const risk = g.risk_level === "high" ? "🔴" : g.risk_level === "medium" ? "🟡" : "";
    parts.push(`${icon} #${g.number} ${g.date} ${risk}`);
    if (g.description) parts.push(`   ${g.description}`);
    if (g.nixos_version) parts.push(`   NixOS ${g.nixos_version} · Kernel ${g.kernel_version}`);
  }

  return parts.join("\n");
}

function formatCompare(data: any, from: number, to: number): string {
  const parts: string[] = [];
  parts.push(`⚖️ 对比 Generation #${from} → #${to}`);
  parts.push("");

  if (data.added_services?.length) {
    parts.push(`➕ 新增: ${data.added_services.join(", ")}`);
  }
  if (data.removed_services?.length) {
    parts.push(`➖ 移除: ${data.removed_services.join(", ")}`);
  }
  if (data.changed_packages?.length) {
    parts.push(`📦 变更: ${data.changed_packages.join(", ")}`);
  }
  if (data.config_diff_summary) {
    parts.push("");
    parts.push("📝 差异摘要:");
    parts.push(data.config_diff_summary);
  }

  return parts.join("\n");
}

function formatAdvisor(data: any): string {
  const parts: string[] = [];
  parts.push(`🎯 智能回滚建议`);
  parts.push(`   当前: Generation #${data.current_generation}`);
  parts.push(`   推荐: Generation #${data.recommended_generation} (置信度: ${(data.confidence * 100).toFixed(0)}%)`);
  parts.push("");
  parts.push(`📋 ${data.analysis_summary}`);
  parts.push("");

  if (data.candidates?.length) {
    parts.push("📊 候选排名:");
    for (const c of data.candidates.slice(0, 5)) {
      const bar = "█".repeat(Math.round(c.score * 20));
      parts.push(`   #${c.generation} [${bar}] ${c.score.toFixed(2)}`);
      if (c.reasons?.length) parts.push(`      ${c.reasons.join("; ")}`);
    }
  }

  return parts.join("\n");
}
