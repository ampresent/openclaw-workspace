/**
 * nix-evo MCP Server — Experimental V6 Features
 * 
 * Tools for: Time-Travel Debugging, Chaos Engineering, Pattern Library,
 * Config Impact Analysis, Distributed Config Sync, Mobile-First API.
 */

// ─── Agent API helpers ───────────────────────────────────────────

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
  if (keys.length === 1) return hosts[keys[keys.length - 1]];
  throw new Error(`请指定 host 参数。可用: ${keys.join(", ")}`);
}

// ─── Tool definitions ────────────────────────────────────────────

export function getExperimentalV6Tools(hosts: Record<string, HostEntry>) {
  const hostNames = Object.keys(hosts);
  const hostParamDesc =
    hostNames.length > 1
      ? `服务器标识。可用: ${hostNames.join(", ")}`
      : "服务器标识";

  return [
    // ── 1. Time-Travel: Snapshot ─────────────────────────────────
    {
      name: "timetravel_snapshot",
      description: "🕰️ 创建系统状态快照 — 记录服务、磁盘、内存、网络的当前状态，用于时间旅行回溯。",
      inputSchema: {
        type: "object",
        properties: {
          label: { type: "string", description: "快照标签（可选），如 'before-nginx-upgrade'" },
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler(args: any) {
        const data = await agentPost(hosts, args.host, "/api/timetravel/snapshot", {
          label: args.label,
        });

        const failedSvc = (data.services || []).filter((s: any) => s.status === "failed");
        return {
          content: [{
            type: "text",
            text: [
              `## 🕰️ Snapshot Captured`,
              ``,
              `**ID**: \`${data.id}\``,
              `**Time**: ${data.timestamp}`,
              `**Label**: ${data.label || "(none)"}`,
              ``,
              `📊 **State**:`,
              `- Services: ${data.services?.length || 0} (${failedSvc.length} failed)`,
              `- Packages: ${data.packages?.length || 0}`,
              `- Disk mounts: ${data.disk_usage?.length || 0}`,
              `- Open ports: ${data.network?.open_ports?.length || 0}`,
              `- Config hash: \`${data.config_hash}\``,
              failedSvc.length > 0 ? [
                ``,
                `⚠️ **Failed services**:`,
                ...failedSvc.map((s: any) => `  - ${s.name}`),
              ].join("\n") : "",
            ].filter(Boolean).join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 2. Time-Travel: Diff ─────────────────────────────────────
    {
      name: "timetravel_diff",
      description: "🕰️ 对比两个快照 — 查看服务状态、包变更、磁盘使用、开放端口的变化。",
      inputSchema: {
        type: "object",
        properties: {
          from: { type: "string", description: "起始快照 ID" },
          to: { type: "string", description: "目标快照 ID" },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["from", "to"],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/timetravel/diff", {
          from: args.from,
          to: args.to,
        });

        const svcText = (data.services_changed || []).map((s: any) =>
          `  🔄 ${s.name}: ${s.from_status} → ${s.to_status}`
        ).join("\n");

        const diskText = (data.disk_changes || []).map((d: any) =>
          `  💾 ${d.mount}: ${d.from_pct.toFixed(0)}% → ${d.to_pct.toFixed(0)}% (${d.delta_gb > 0 ? "+" : ""}${d.delta_gb.toFixed(1)}GB)`
        ).join("\n");

        return {
          content: [{
            type: "text",
            text: [
              `## 🕰️ Snapshot Comparison`,
              ``,
              `**From**: ${data.from_time} (\`${data.from_id}\`)`,
              `**To**: ${data.to_time} (\`${data.to_id}\`)`,
              `**Delta**: ${data.time_delta_secs}s`,
              ``,
              data.config_changed ? "⚠️ **Config file changed**\n" : "",
              data.services_changed?.length ? `**Service changes** (${data.services_changed.length}):\n${svcText}\n` : "",
              data.packages_added?.length ? `**Packages added** (${data.packages_added.length}): ${data.packages_added.slice(0, 10).join(", ")}${data.packages_added.length > 10 ? "..." : ""}\n` : "",
              data.packages_removed?.length ? `**Packages removed** (${data.packages_removed.length}): ${data.packages_removed.slice(0, 10).join(", ")}\n` : "",
              data.disk_changes?.length ? `**Disk changes**:\n${diskText}\n` : "",
              `**Memory delta**: ${data.memory_delta_mb > 0 ? "+" : ""}${data.memory_delta_mb}MB`,
              data.open_ports_added?.length ? `**Ports opened**: ${data.open_ports_added.join(", ")}\n` : "",
              data.open_ports_removed?.length ? `**Ports closed**: ${data.open_ports_removed.join(", ")}\n` : "",
            ].filter(Boolean).join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 3. Time-Travel: Replay ───────────────────────────────────
    {
      name: "timetravel_replay",
      description: "🕰️ 回放快照序列 — 查看系统在一段时间内的状态变化轨迹。",
      inputSchema: {
        type: "object",
        properties: {
          from: { type: "number", description: "起始时间 (unix epoch, 可选)" },
          to: { type: "number", description: "结束时间 (unix epoch, 可选)" },
          limit: { type: "number", description: "最大帧数，默认100", default: 100 },
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/timetravel/replay", {
          from: args.from?.toString() || "",
          to: args.to?.toString() || "",
          limit: args.limit?.toString() || "100",
        });

        const framesText = (data.frames || []).map((f: any) => {
          const status = f.failed_services?.length > 0 ? "❌" : "✅";
          return `  ${status} ${f.timestamp} ${f.label || ""} — ${f.service_count} svc, mem ${f.memory_used_pct.toFixed(0)}%, disk ${f.disk_max_pct.toFixed(0)}%${f.failed_services?.length ? ` (${f.failed_services.join(", ")} failed)` : ""}`;
        }).join("\n");

        return {
          content: [{
            type: "text",
            text: [
              `## 🕰️ Replay (${data.frame_count} frames)`,
              ``,
              framesText || "  _No snapshots captured yet. Use timetravel_snapshot first._",
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 4. Chaos: Start Experiment ───────────────────────────────
    {
      name: "chaos_start",
      description: "🎲 启动混沌实验 — 有意地破坏系统（安全地）来测试恢复能力。",
      inputSchema: {
        type: "object",
        properties: {
          scenario: {
            type: "string",
            description: "场景 ID",
            enum: ["service-kill", "network-partition", "disk-pressure", "cpu-stress", "config-corrupt"],
          },
          target: { type: "string", description: "目标（服务名/路径/百分比，可选）" },
          duration: { type: "number", description: "持续秒数", default: 10 },
          intensity: { type: "number", description: "强度 0.0-1.0", default: 0.5 },
          auto_recover: { type: "boolean", description: "自动恢复", default: true },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["scenario"],
      },
      async handler(args: any) {
        const data = await agentPost(hosts, args.host, "/api/chaos/start", {
          scenario: args.scenario,
          target: args.target,
          duration_secs: args.duration || 10,
          intensity: args.intensity ?? 0.5,
          auto_recover: args.auto_recover ?? true,
        });

        const statusEmoji = { passed: "✅", recovered: "🔄", failed: "❌", running: "⏳" }[data.status] || "❓";
        const recoveryText = data.recovery_action ? `\n**Recovery**: ${data.recovery_action}` : "";
        const obsText = (data.observations || []).map((o: any) =>
          `  ${o.breached ? "🔴" : "🟢"} ${o.metric}: ${o.value.toFixed(1)} (threshold: ${o.threshold})`
        ).join("\n");

        return {
          content: [{
            type: "text",
            text: [
              `## 🎲 Chaos Experiment ${statusEmoji}`,
              ``,
              `**ID**: ${data.experiment_id}`,
              `**Status**: ${data.status}`,
              `**Started**: ${data.started_at}`,
              `**Ended**: ${data.ended_at || "running..."}`,
              recoveryText,
              obsText ? `\n**Observations**:\n${obsText}` : "",
            ].filter(Boolean).join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 5. Chaos: Report ─────────────────────────────────────────
    {
      name: "chaos_report",
      description: "🎲 获取混沌实验报告 — 查看系统韧性评分和所有实验历史。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/chaos/report");

        const scoreEmoji = data.resilience_score >= 80 ? "🛡️" : data.resilience_score >= 50 ? "⚠️" : "💀";

        return {
          content: [{
            type: "text",
            text: [
              `## 🎲 Chaos Engineering Report`,
              ``,
              `${scoreEmoji} **Resilience Score**: ${data.resilience_score}/100`,
              ``,
              `📊 **Experiments**: ${data.total_experiments} total`,
              `  ✅ Passed: ${data.passed}`,
              `  🔄 Recovered: ${data.recovered}`,
              `  ❌ Failed: ${data.failed}`,
              `  ⏱️ Avg recovery: ${data.avg_recovery_ms}ms`,
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 6. Patterns: Search ──────────────────────────────────────
    {
      name: "patterns_search",
      description: "🧩 搜索 NixOS 配置模式库 — 按用例、难度、安全等级查找常见配置模式。",
      inputSchema: {
        type: "object",
        properties: {
          query: { type: "string", description: "搜索关键词，如 'reverse proxy', 'database', 'firewall'" },
          category: {
            type: "string",
            description: "分类过滤",
            enum: ["WebServer", "Database", "Networking", "Security", "Monitoring", "Containers", "Storage", "Boot", "Desktop", "Development"],
          },
          difficulty: {
            type: "string",
            description: "难度过滤",
            enum: ["Beginner", "Intermediate", "Advanced", "Expert"],
          },
          security: {
            type: "string",
            description: "安全等级过滤",
            enum: ["Minimal", "Standard", "Hardened", "Paranoid"],
          },
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler(args: any) {
        const params: Record<string, string> = {};
        if (args.query) params.q = args.query;
        if (args.category) params.category = args.category;
        if (args.difficulty) params.difficulty = args.difficulty;
        if (args.security) params.security = args.security;

        const data = await agentGet(hosts, args.host, "/api/patterns", params);

        const patternsText = (data.patterns || []).map((p: any) => {
          const diffIcon = { Beginner: "🟢", Intermediate: "🟡", Advanced: "🟠", Expert: "🔴" }[p.difficulty] || "❓";
          const secIcon = { Minimal: "🔓", Standard: "🔒", Hardened: "🛡️", Paranoid: "🏰" }[p.security_rating] || "❓";
          return [
            `### ${diffIcon} ${p.name}`,
            `${p.description}`,
            `Category: ${p.category} | Difficulty: ${p.difficulty} | Security: ${secIcon} ${p.security_rating}`,
            `Tags: ${p.tags.join(", ")}`,
            `Use: ${p.use_cases.join(", ")}`,
            p.dependencies.length ? `Depends on: ${p.dependencies.join(", ")}` : "",
            `\nUse \`patterns_detail\` with id \`${p.id}\` for full Nix code.`,
          ].filter(Boolean).join("\n");
        }).join("\n---\n\n");

        return {
          content: [{
            type: "text",
            text: [
              `## 🧩 NixOS Pattern Library (${data.total} patterns)`,
              ``,
              data.categories?.length ? `**Categories**: ${data.categories.map((c: any) => `${c.name} (${c.count})`).join(", ")}\n` : "",
              patternsText || "  _No patterns match your criteria_",
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 7. Patterns: Detail ──────────────────────────────────────
    {
      name: "patterns_detail",
      description: "🧩 获取模式详情 — 包含完整解释和 Nix 配置代码。",
      inputSchema: {
        type: "object",
        properties: {
          id: { type: "string", description: "模式 ID，如 'nginx-reverse-proxy', 'hardened-ssh'" },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["id"],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, `/api/patterns/${args.id}`);

        return {
          content: [{
            type: "text",
            text: [
              `## 🧩 ${data.name}`,
              ``,
              data.explanation,
              ``,
              `### Nix Configuration`,
              `\`\`\`nix`,
              data.nix_code,
              `\`\`\``,
              data.dependencies?.length ? `\n**Dependencies**: ${data.dependencies.join(", ")}` : "",
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 8. Impact: Analyze ───────────────────────────────────────
    {
      name: "impact_analyze",
      description: "🔮 分析配置变更影响 — 在应用变更前预测哪些服务会受影响、需要级联修改什么。",
      inputSchema: {
        type: "object",
        properties: {
          changes: {
            type: "array",
            description: "要分析的变更列表",
            items: {
              type: "object",
              properties: {
                option: { type: "string", description: "NixOS 选项路径，如 'services.nginx.listen.port'" },
                old_value: { type: "string", description: "当前值（可选）" },
                new_value: { type: "string", description: "新值" },
              },
              required: ["option", "new_value"],
            },
          },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["changes"],
      },
      async handler(args: any) {
        const data = await agentPost(hosts, args.host, "/api/impact/analyze", {
          changes: args.changes,
        });

        const riskEmoji = { low: "🟢", medium: "🟡", high: "🔴" }[data.risk_level] || "❓";

        const directText = (data.direct_impacts || []).map((i: any) =>
          `  ${i.severity === "breaking" ? "🔴" : i.severity === "warning" ? "🟡" : "ℹ️"} **${i.target}** — ${i.description}`
        ).join("\n");

        const transitiveText = (data.transitive_impacts || []).map((i: any) =>
          `  → ${i.target}: ${i.description}`
        ).join("\n");

        const requiredText = (data.required_changes || []).map((c: any) =>
          `  📝 \`${c.option}\`: ${c.current_value} → ${c.suggested_value} (${c.reason})`
        ).join("\n");

        return {
          content: [{
            type: "text",
            text: [
              `## 🔮 Impact Analysis`,
              ``,
              `${riskEmoji} **Risk**: ${data.risk_level.toUpperCase()}`,
              data.summary,
              ``,
              data.warnings?.length ? `### ⚠️ Warnings\n${data.warnings.map((w: string) => `- ${w}`).join("\n")}\n` : "",
              directText ? `### Direct Impacts\n${directText}\n` : "",
              transitiveText ? `### Transitive Impacts\n${transitiveText}\n` : "",
              requiredText ? `### Required Changes\n${requiredText}\n` : "",
              data.recommendations?.length ? `### 💡 Recommendations\n${data.recommendations.map((r: string) => `- ${r}`).join("\n")}` : "",
            ].filter(Boolean).join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 9. Distributed Sync: Status ──────────────────────────────
    {
      name: "sync_status",
      description: "🌍 查看分布式配置同步状态 — 节点列表、版本向量、同步状态。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/sync/status");

        const syncIcon = data.all_in_sync ? "✅ All in sync" : "⚠️ Divergence detected";
        const nodesText = (data.nodes || []).map((n: any) =>
          `  ${n.status === "online" ? "🟢" : "🔴"} **${n.name}** (${n.id}) — ${n.status}${n.last_sync ? `, last sync: ${n.last_sync}` : ""}`
        ).join("\n");

        return {
          content: [{
            type: "text",
            text: [
              `## 🌍 Distributed Config Sync`,
              ``,
              `**Status**: ${syncIcon}`,
              `**Nodes**: ${data.node_count}`,
              `**Operations**: ${data.op_count}`,
              `**Config hash**: \`${data.config_hash || "N/A"}\``,
              ``,
              `### Nodes`,
              nodesText || "  _No nodes registered. Use sync_init to add nodes._",
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 10. Mobile: Status ───────────────────────────────────────
    {
      name: "mobile_status",
      description: "📱 获取移动端优化状态 — 超紧凑 JSON，适合手机端查看。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/mobile/status");

        const statusIcon = { o: "🟢 OK", w: "🟡 Warning", c: "🔴 Critical" }[data.s] || "❓";
        const uptimeStr = `${Math.floor(data.u / 86400)}d ${Math.floor((data.u % 86400) / 3600)}h`;

        return {
          content: [{
            type: "text",
            text: [
              `## 📱 Mobile Status`,
              ``,
              `**${data.h}** — ${statusIcon}`,
              ``,
              `⏱️ Uptime: ${uptimeStr}`,
              `💾 Memory: ${data.m}%`,
              `💿 Disk: ${data.d}%`,
              `📈 Load: ${data.l}`,
              `❌ Failed services: ${data.f}${data.fs?.length ? ` (${data.fs.join(", ")})` : ""}`,
              ``,
              `📊 Response size: ~${JSON.stringify(data).length} bytes (ultra-compact)`,
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },
  ];
}
