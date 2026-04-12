/**
 * nix-evo MCP Server — Experimental V3 Features
 * 
 * Tools for: Nix expression interpreter, multi-language support,
 * security scanner, interactive config builder, capacity planning,
 * GitOps bridge, and plugin system.
 */

// ─── Agent API helpers (reuse patterns from v2) ──────────────────

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

// ─── Tool definitions ──────────────────────────────────────────────────

export function getExperimentalV3Tools(hosts: Record<string, HostEntry>) {
  const hostNames = Object.keys(hosts);
  const hostParamDesc =
    hostNames.length > 1
      ? `服务器标识。可用: ${hostNames.join(", ")}`
      : "服务器标识";

  return [
    {
      name: "nix_eval_check",
      description: "Nix 表达式语法检查 — 解析并检查 Nix 表达式的语法正确性，返回 AST。不执行外部命令，纯内存解析。",
      inputSchema: {
        type: "object",
        properties: {
          expression: {
            type: "string",
            description: "要检查的 Nix 表达式，例如：'{ services.nginx.enable = true; }'",
          },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["expression"],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/nix/check", {
          expression: args.expression,
        });

        const astSummary = data.ast ? JSON.stringify(data.ast, null, 2).slice(0, 500) : "N/A";

        return {
          content: [
            {
              type: "text",
              text: [
                `## Nix 表达式语法检查`,
                ``,
                `**状态**: ${data.success ? "✅ 语法正确" : "❌ 语法错误"}`,
                `**Tokens**: ${data.tokens_parsed}`,
                `**耗时**: ${data.eval_time_us}μs`,
                data.error ? `\n**错误**: ${data.error}` : "",
                ``,
                `### AST (摘要)`,
                "```json",
                astSummary,
                "```",
              ].filter(Boolean).join("\n"),
            },
          ],
          structuredContent: data,
        };
      },
    },

    {
      name: "nix_eval_run",
      description: "Nix 表达式求值 — 解析并求值 Nix 表达式，返回结果值。支持 let、if、attrset、list、字符串拼接等。",
      inputSchema: {
        type: "object",
        properties: {
          expression: {
            type: "string",
            description: "要求值的 Nix 表达式",
          },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["expression"],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/nix/eval", {
          expression: args.expression,
        });

        return {
          content: [
            {
              type: "text",
              text: [
                `## Nix 表达式求值`,
                ``,
                `**表达式**: \`${args.expression}\``,
                `**结果**: \`${JSON.stringify(data.value)}\``,
                `**耗时**: ${data.eval_time_us}μs`,
                data.error ? `\n**错误**: ${data.error}` : "",
              ].filter(Boolean).join("\n"),
            },
          ],
          structuredContent: data,
        };
      },
    },

    {
      name: "i18n_translate",
      description: "翻译 NixOS 错误信息 — 将 NixOS 错误信息或 dry-build 输出翻译为指定语言。支持 zh-CN、ja-JP、de-DE、fr-FR。",
      inputSchema: {
        type: "object",
        properties: {
          text: {
            type: "string",
            description: "要翻译的文本（错误信息或构建输出）",
          },
          lang: {
            type: "string",
            description: "目标语言: zh-CN, ja-JP, de-DE, fr-FR",
            enum: ["zh-CN", "ja-JP", "de-DE", "fr-FR"],
          },
          mode: {
            type: "string",
            description: "翻译模式: error（错误信息）或 build（构建输出）",
            enum: ["error", "build"],
          },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["text", "lang"],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/i18n/translate", {
          text: args.text,
          lang: args.lang,
          mode: args.mode || "error",
        });

        const langNames: Record<string, string> = {
          "zh-CN": "简体中文", "ja-JP": "日本語", "de-DE": "Deutsch", "fr-FR": "Français",
        };

        return {
          content: [
            {
              type: "text",
              text: [
                `## 🌍 翻译结果 (${langNames[args.lang] || args.lang})`,
                ``,
                `**原文**: ${data.original}`,
                `**翻译**: ${data.translated}`,
                `**匹配**: ${data.matched ? "✅ 已翻译" : "⚠️ 未找到匹配的翻译规则"}`,
              ].join("\n"),
            },
          ],
          structuredContent: data,
        };
      },
    },

    {
      name: "security_scan",
      description: "安全扫描 — 扫描 configuration.nix 检查安全问题。检查防火墙、SSH、密码策略、开放端口、内核安全等。",
      inputSchema: {
        type: "object",
        properties: {
          config_path: {
            type: "string",
            description: "配置文件路径，默认 /etc/nixos/configuration.nix",
          },
          host: { type: "string", description: hostParamDesc },
        },
      },
      async handler(args: any) {
        const params: Record<string, string> = {};
        if (args.config_path) params.config_path = args.config_path;

        const data = await agentGet(hosts, args.host, "/api/security/scan", params);

        const scoreEmoji = data.score >= 80 ? "🟢" : data.score >= 60 ? "🟡" : data.score >= 40 ? "🟠" : "🔴";

        let findingsText = data.findings
          .slice(0, 10)
          .map((f: any) => {
            const sev = { critical: "🔴", high: "🟠", medium: "🟡", low: "🔵", info: "ℹ️" }[f.severity] || "❓";
            return `${sev} **${f.severity.toUpperCase()}** [${f.category}] ${f.title}\n   ${f.description}\n   💡 ${f.recommendation}`;
          })
          .join("\n\n");

        return {
          content: [
            {
              type: "text",
              text: [
                `## 🔒 Security Scan Report`,
                ``,
                `${scoreEmoji} **安全评分: ${data.score}/100**`,
                `**主机**: ${data.hostname}`,
                `**扫描时间**: ${data.scan_time}`,
                ``,
                `### 摘要`,
                `- 🔴 Critical: ${data.summary.critical}  |  🟠 High: ${data.summary.high}  |  🟡 Medium: ${data.summary.medium}  |  🔵 Low: ${data.summary.low}`,
                `- 🔌 开放端口: ${data.summary.open_ports.join(", ") || "无"}`,
                `- 📦 检查服务: ${data.summary.services_checked}  |  检查包: ${data.summary.packages_checked}`,
                ``,
                `### 发现的问题`,
                findingsText || "✅ 未发现安全问题",
              ].join("\n"),
            },
          ],
          structuredContent: data,
        };
      },
    },

    {
      name: "config_builder_status",
      description: "配置构建器状态 — 查看交互式配置构建器的 WebSocket 连接数和可用服务列表。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
      },
      async handler(args: any) {
        return {
          content: [
            {
              type: "text",
              text: [
                `## 🎮 Interactive Config Builder`,
                ``,
                `WebSocket 端点: \`ws://<host>/api/config-builder/ws\``,
                `Web UI: \`http://<host>/builder\``,
                ``,
                `### 工作流程`,
                `1. **Welcome** → 连接并获取服务列表`,
                `2. **Select Services** → 选择要启用的服务`,
                `3. **Configure Ports** → 配置端口`,
                `4. **Set Options** → 设置服务选项`,
                `5. **Review** → 预览生成的配置`,
                `6. **Apply** → 应用配置`,
                ``,
                `### WebSocket 命令`,
                `- \`{"action": "start"\}\` — 开始构建`,
                `- \`{"action": "select", "services": ["nginx", "openssh"]}\` — 选择服务`,
                `- \`{"action": "preview"\}\` — 生成配置预览`,
                `- \`{"action": "apply"\}\` — 应用配置`,
              ].join("\n"),
            },
          ],
          structuredContent: { status: "available", websocket: "/api/config-builder/ws" },
        };
      },
    },

    {
      name: "capacity_forecast",
      description: "容量预测 — 分析磁盘、内存、CPU 使用情况，预测资源耗尽时间，提供优化建议。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/capacity/forecast");

        const diskText = data.disk.mount_points
          .map((mp: any) => {
            const bar = renderBar(mp.usage_percent);
            const days = mp.days_until_full ? ` (预计 ${mp.days_until_full} 天后满)` : "";
            return `${mp.path}: ${bar} ${mp.usage_percent.toFixed(1)}% (${mp.used_gb.toFixed(1)}G / ${mp.total_gb.toFixed(1)}G)${days}`;
          })
          .join("\n");

        const recText = data.recommendations
          .map((r: any) => {
            const sev = { low: "🔵", medium: "🟡", high: "🟠", critical: "🔴" }[r.severity] || "❓";
            return `${sev} **${r.resource}**: ${r.action}\n   ${r.details}`;
          })
          .join("\n\n");

        return {
          content: [
            {
              type: "text",
              text: [
                `## 📊 Capacity Forecast`,
                ``,
                `**时间**: ${data.timestamp}`,
                ``,
                `### 💾 磁盘`,
                diskText,
                ``,
                `Nix Store: ${data.disk.nix_store_size_gb.toFixed(1)} GB` +
                  (data.disk.gc_savings_gb ? ` (清理可释放 ~${data.disk.gc_savings_gb.toFixed(1)} GB)` : ""),
                ``,
                `### 🧠 内存`,
                `${renderBar(data.memory.usage_percent)} ${data.memory.usage_percent.toFixed(1)}% (${data.memory.used_gb.toFixed(1)}G / ${data.memory.total_gb.toFixed(1)}G)`,
                `Swap: ${data.memory.swap_used_gb.toFixed(1)}G / ${data.memory.swap_total_gb.toFixed(1)}G`,
                ``,
                `### ⚡ CPU`,
                `Cores: ${data.cpu.cores} | Load: ${data.cpu.load_1m} / ${data.cpu.load_5m} / ${data.cpu.load_15m}`,
                `Per-core load: ${data.cpu.load_per_core.toFixed(2)}`,
                ``,
                data.recommendations.length > 0 ? `### 💡 建议\n\n${recText}` : "### ✅ 资源状态良好",
              ].join("\n"),
            },
          ],
          structuredContent: data,
        };
      },
    },

    {
      name: "gitops_status",
      description: "GitOps 状态 — 查看 GitOps 配置、当前 commit、待部署变更和部署状态。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/gitops/status");

        const stateEmoji: Record<string, string> = {
          idle: "⏸️", pulling: "⬇️", validating: "✅", deploying: "🚀", success: "✅",
        };
        const emoji = stateEmoji[data.deploy_state] || "❓";

        return {
          content: [
            {
              type: "text",
              text: [
                `## 🔄 GitOps Bridge`,
                ``,
                `**状态**: ${data.configured ? "✅ 已配置" : "⚠️ 未配置"}`,
                data.repo_url ? `**仓库**: ${data.repo_url}` : "",
                data.branch ? `**分支**: ${data.branch}` : "",
                `**部署状态**: ${emoji} ${data.deploy_state}`,
                ``,
                data.current_commit ? [
                  `### 当前 Commit`,
                  `- \`${data.current_commit.short_hash}\` ${data.current_commit.message}`,
                  `- Author: ${data.current_commit.author}`,
                  `- Time: ${data.current_commit.timestamp}`,
                ].join("\n") : "",
                ``,
                data.pending_commits.length > 0
                  ? `### 待部署 (${data.pending_commits.length})\n${data.pending_commits.map((c: any) => `- \`${c.short_hash}\` ${c.message}`).join("\n")}`
                  : "",
                data.last_deploy
                  ? `### 最近部署\n- Commit: \`${data.last_deploy.commit_hash.slice(0, 8)}\`\n- 结果: ${data.last_deploy.success ? "✅ 成功" : "❌ 失败"}\n- 耗时: ${data.last_deploy.duration_secs.toFixed(1)}s`
                  : "",
              ].filter(Boolean).join("\n"),
            },
          ],
          structuredContent: data,
        };
      },
    },

    {
      name: "plugins_list",
      description: "插件列表 — 列出已加载的插件及其状态。插件目录: ~/.nix-evo/plugins/",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/plugins");

        const statusEmoji: Record<string, string> = {
          loaded: "✅", unloaded: "⏸️",
        };

        return {
          content: [
            {
              type: "text",
              text: [
                `## 🧩 Plugin System`,
                ``,
                `**插件目录**: \`${data.plugin_dir}\``,
                `**总计**: ${data.total} | **已加载**: ${data.loaded} | **失败**: ${data.failed}`,
                ``,
                data.plugins.length > 0
                  ? data.plugins.map((p: any) => {
                      const emoji = p.status === "loaded" ? "✅" : "❌";
                      const err = p.last_error ? `\n   ⚠️ ${p.last_error}` : "";
                      return `${emoji} **${p.name}** v${p.version} — \`${p.path}\`${err}`;
                    }).join("\n")
                  : "📭 未发现插件。将 .so 文件放入 ~/.nix-evo/plugins/ 目录即可自动加载。",
                ``,
                `### 插件开发`,
                `插件需导出以下 C 函数:`,
                `- \`nix_evo_plugin_init()\` → 返回插件名`,
                `- \`nix_evo_plugin_version()\` → 返回版本`,
                `- \`nix_evo_plugin_handle_request(method, path, body)\` → 返回响应`,
                `- \`nix_evo_plugin_health_check()\` → 返回 "ok" 或错误`,
                `- \`nix_evo_plugin_cleanup()\` → 清理资源`,
              ].join("\n"),
            },
          ],
          structuredContent: data,
        };
      },
    },
  ];
}

// ─── Helpers ──────────────────────────────────────────────────────

function renderBar(percent: number, width = 20): string {
  const filled = Math.round((percent / 100) * width);
  const empty = width - filled;
  const char = percent >= 90 ? "█" : percent >= 70 ? "▓" : "░";
  return `[${"█".repeat(filled)}${"░".repeat(empty)}]`;
}
