/**
 * nix-evo MCP Server — Experimental V4 Features
 * 
 * Tools for: AI-Powered Nix Doctor, Service Orchestration Composer,
 * Predictive Failure Detection, Config Streaming, Cross-Distro Compatibility,
 * System Health Score.
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

export function getExperimentalV4Tools(hosts: Record<string, HostEntry>) {
  const hostNames = Object.keys(hosts);
  const hostParamDesc =
    hostNames.length > 1
      ? `服务器标识。可用: ${hostNames.join(", ")}`
      : "服务器标识";

  return [
    {
      name: "nix_doctor",
      description: "NixOS 错误诊断 — 将 NixOS 构建错误、服务失败等错误信息发送给 AI Doctor，匹配知识库中的已知问题并返回修复建议。",
      inputSchema: {
        type: "object",
        properties: {
          error_message: {
            type: "string",
            description: "NixOS 错误信息（构建错误、服务失败、求值错误等）",
          },
          context: {
            type: "string",
            description: "可选的额外上下文信息",
          },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["error_message"],
      },
      async handler(args: any) {
        const data = await agentPost(hosts, args.host, "/api/doctor/diagnose", {
          error_message: args.error_message,
          context: args.context,
        });

        if (!data.matches || data.matches.length === 0) {
          return {
            content: [{
              type: "text",
              text: [
                `## 🧠 Nix Doctor — 未找到匹配的诊断`,
                ``,
                `**错误信息**: ${data.input.slice(0, 200)}`,
                ``,
                `### 💡 建议`,
                ...data.suggestions.map((s: string) => `- ${s}`),
                ``,
                `知识库共有 ${data.total_kb_entries} 条记录。`,
              ].join("\n"),
            }],
            structuredContent: data,
          };
        }

        const matchesText = data.matches.slice(0, 5).map((m: any, i: number) => {
          const sevEmoji = { critical: "🔴", high: "🟠", medium: "🟡", low: "🔵" }[m.severity] || "❓";
          const cmds = m.commands.map((c: string) => `\`${c}\``).join("\n  ");
          return [
            `### ${i + 1}. ${sevEmoji} ${m.title} (${Math.round(m.confidence * 100)}% 匹配)`,
            `- **严重程度**: ${m.severity} | **类别**: ${m.category}`,
            `- **描述**: ${m.description}`,
            `- **解决方案**: ${m.solution}`,
            m.commands.length ? `- **命令**:\n  ${cmds}` : "",
            m.docs_url ? `- 📖 [文档](${m.docs_url})` : "",
          ].filter(Boolean).join("\n");
        }).join("\n\n");

        return {
          content: [{
            type: "text",
            text: [
              `## 🧠 Nix Doctor 诊断结果`,
              ``,
              `**输入错误**: ${data.input.slice(0, 200)}${data.input.length > 200 ? "..." : ""}`,
              `**找到 ${data.matches.length} 个匹配**`,
              ``,
              matchesText,
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    {
      name: "compose_services",
      description: "服务编排 — 定义多服务部署组合，计算依赖排序，生成等效 NixOS 配置。类似 docker-compose 但用于 NixOS。",
      inputSchema: {
        type: "object",
        properties: {
          services: {
            type: "array",
            description: "服务列表，每个服务包含 name, depends_on, ports 等",
            items: {
              type: "object",
              properties: {
                name: { type: "string" },
                package: { type: "string" },
                enable: { type: "boolean" },
                depends_on: { type: "array", items: { type: "string" } },
              },
            },
          },
          action: {
            type: "string",
            enum: ["plan", "deploy", "validate"],
            description: "操作类型: plan（规划）, deploy（部署）, validate（验证）",
          },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["services"],
      },
      async handler(args: any) {
        const composition = {
          name: "mcp-composition",
          version: "1.0.0",
          description: "Created via MCP",
          services: args.services.map((s: any) => ({
            name: s.name,
            package: s.package || null,
            enable: s.enable !== false,
            depends_on: s.depends_on || [],
            health_check: null,
            restart_policy: "on-failure",
            scaling: null,
            env: {},
            ports: s.ports || [],
            config_options: {},
          })),
        };

        const data = await agentPost(hosts, args.host, "/api/compose", {
          composition,
          action: args.action || "plan",
        });

        const planText = data.startup_plan
          ? data.startup_plan.layers.map((layer: string[], i: number) =>
              `Layer ${i + 1}: ${layer.join(", ")}`
            ).join("\n")
          : "N/A";

        const warningsText = data.validation_warnings.length > 0
          ? data.validation_warnings.map((w: string) => `⚠️ ${w}`).join("\n")
          : "✅ 无警告";

        return {
          content: [{
            type: "text",
            text: [
              `## 🎵 Service Composition — ${data.action}`,
              ``,
              `### 启动顺序 (拓扑排序)`,
              planText,
              ``,
              `### 验证`,
              warningsText,
              ``,
              data.nixos_config ? `### 生成的 NixOS 配置\n\`\`\`nix\n${data.nixos_config.slice(0, 800)}\n\`\`\`` : "",
            ].filter(Boolean).join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    {
      name: "predict_alerts",
      description: "预测性故障检测 — 分析系统指标趋势，预测磁盘满、内存不足、服务降级等问题，提前预警。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/predict/alerts");

        const riskEmoji = data.risk_score >= 70 ? "🔴" : data.risk_score >= 40 ? "🟡" : "🟢";

        const alertsText = data.alerts.length > 0
          ? data.alerts.map((a: any) => {
              const sevEmoji = { critical: "🔴", warning: "🟡", info: "ℹ️" }[a.severity] || "❓";
              const time = a.estimated_time ? ` (预计 ${a.estimated_time})` : "";
              const recs = a.recommended_actions.map((r: string) => `  - ${r}`).join("\n");
              return `${sevEmoji} **${a.title}**${time}\n  ${a.description}\n${recs}`;
            }).join("\n\n")
          : "✅ 无预警";

        return {
          content: [{
            type: "text",
            text: [
              `## 🔮 Predictive Failure Detection`,
              ``,
              `${riskEmoji} **风险评分**: ${data.risk_score.toFixed(1)}/100`,
              ``,
              `### 系统概况`,
              `- 💾 磁盘: ${data.system_summary.disk_usage_percent.toFixed(1)}%`,
              `- 🧠 内存: ${data.system_summary.memory_usage_percent.toFixed(1)}%`,
              `- ⚡ CPU 负载: ${data.system_summary.cpu_load_1m}`,
              `- ⏱️ 运行时间: ${data.system_summary.uptime_hours.toFixed(1)} 小时`,
              data.system_summary.failed_services.length
                ? `- ❌ 失败服务: ${data.system_summary.failed_services.join(", ")}`
                : "",
              ``,
              `### 预警 (${data.alerts.length})`,
              alertsText,
            ].filter(Boolean).join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    {
      name: "compat_translate",
      description: "跨发行版兼容转换 — 将 NixOS 配置概念翻译为 Ubuntu/Debian/Fedora/Arch 等其他发行版的等效配置。",
      inputSchema: {
        type: "object",
        properties: {
          nixos_config: {
            type: "string",
            description: "NixOS 配置文本（含 services.*.enable = true）",
          },
          target_distro: {
            type: "string",
            enum: ["ubuntu", "debian", "fedora", "arch", "alpine"],
            description: "目标发行版",
          },
          services: {
            type: "array",
            items: { type: "string" },
            description: "可选：手动指定要转换的服务列表",
          },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["nixos_config", "target_distro"],
      },
      async handler(args: any) {
        const data = await agentPost(hosts, args.host, "/api/compat/translate", {
          nixos_config: args.nixos_config,
          target_distro: args.target_distro,
          services: args.services,
        });

        const pkgText = data.package_mapping.map((p: any) =>
          `- \`${p.nixos_package}\` → \`${p.target_package}\` (${p.match_type})`
        ).join("\n");

        const configsText = data.translated_configs
          .filter((c: any) => c.config_type !== "script")
          .map((c: any) => `### ${c.filename}\n\`\`\`\n${c.content.slice(0, 400)}\n\`\`\``)
          .join("\n\n");

        const scriptConfig = data.translated_configs.find((c: any) => c.config_type === "script");

        return {
          content: [{
            type: "text",
            text: [
              `## 🧬 Cross-Distro Translation → ${data.target_distro}`,
              ``,
              `### 包名映射`,
              pkgText,
              ``,
              `### Systemd 单元文件`,
              configsText,
              ``,
              scriptConfig ? `### 安装脚本 (${scriptConfig.filename})\n\`\`\`bash\n${scriptConfig.content.slice(0, 600)}\n\`\`\`` : "",
              ``,
              data.warnings.length ? `### ⚠️ 警告\n${data.warnings.map((w: string) => `- ${w}`).join("\n")}` : "",
              data.notes.length ? `### 📝 注意\n${data.notes.map((n: string) => `- ${n}`).join("\n")}` : "",
            ].filter(Boolean).join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    {
      name: "health_score",
      description: "系统健康评分 — 综合评估服务、磁盘、内存、安全、配置质量、更新状态，给出 0-100 分。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/health/score");

        const scoreEmoji = data.overall_score >= 80 ? "🟢" : data.overall_score >= 60 ? "🟡" : "🔴";

        const factorsText = data.factors.map((f: any) => {
          const statusEmoji = { good: "✅", warning: "⚠️", critical: "🔴" }[f.status] || "❓";
          const recs = f.recommendations.length
            ? "\n  " + f.recommendations.map((r: string) => `💡 ${r}`).join("\n  ")
            : "";
          return `${statusEmoji} **${f.name}**: ${Math.round(f.score)}/100 — ${f.details}${recs}`;
        }).join("\n\n");

        return {
          content: [{
            type: "text",
            text: [
              `## 🏆 System Health Score`,
              ``,
              `${scoreEmoji} **总分: ${data.overall_score}/100** (Grade ${data.grade})`,
              ``,
              `### 各项指标`,
              factorsText,
              ``,
              `### 概要`,
              `- ✅ 良好: ${data.summary.good_factors} | ⚠️ 警告: ${data.summary.warning_factors} | 🔴 严重: ${data.summary.critical_factors}`,
              data.summary.top_issue ? `- ⚠️ 首要问题: ${data.summary.top_issue}` : "",
            ].filter(Boolean).join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    {
      name: "stream_config_status",
      description: "配置流状态 — 查看配置文件实时流（WebSocket）的端点和监控路径。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
      },
      async handler(args: any) {
        return {
          content: [{
            type: "text",
            text: [
              `## 🌊 NixOS Config Streaming`,
              ``,
              `实时监控 NixOS 配置文件变更的 WebSocket 端点。`,
              ``,
              `### WebSocket 端点`,
              `\`ws://<host>/api/stream/config\``,
              ``,
              `### 监控路径`,
              `- \`/etc/nixos/\` — 主配置目录`,
              `- \`/etc/nix/\` — Nix 配置`,
              ``,
              `### 功能`,
              `- 实时文件变更通知 (created/modified/deleted)`,
              `- Git commit 信息自动关联`,
              `- Diff 预览`,
              `- 自动启动，无需配置`,
              ``,
              `### 客户端连接示例`,
              `\`\`\`javascript`,
              `const ws = new WebSocket('ws://host/api/stream/config');`,
              `ws.onmessage = (e) => console.log(JSON.parse(e.data));`,
              `\`\`\``,
            ].join("\n"),
          }],
          structuredContent: {
            websocket: "/api/stream/config",
            watch_paths: ["/etc/nixos", "/etc/nix"],
            auto_started: true,
          },
        };
      },
    },
  ];
}
