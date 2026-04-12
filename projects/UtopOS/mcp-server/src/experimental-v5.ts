/**
 * nix-evo MCP Server — Experimental V5 Features
 * 
 * Tools for: Config DNA Evolution, Config Theater, Blockchain Audit,
 * Collaborative Editing, Benchmarking, Topology Map.
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

export function getExperimentalV5Tools(hosts: Record<string, HostEntry>) {
  const hostNames = Object.keys(hosts);
  const hostParamDesc =
    hostNames.length > 1
      ? `服务器标识。可用: ${hostNames.join(", ")}`
      : "服务器标识";

  return [
    // ── 1. Config DNA Evolution ──────────────────────────────────
    {
      name: "dna_evolve",
      description: "🧬 NixOS 配置基因进化 — 将配置视为 DNA，通过遗传算法（变异、交叉、选择）优化配置。可优化构建速度、磁盘占用、安全评分和启动时间。",
      inputSchema: {
        type: "object",
        properties: {
          genes: {
            type: "array",
            description: "配置基因列表。每个基因是一个可配置选项。",
            items: {
              type: "object",
              properties: {
                name: { type: "string", description: "选项名，如 services.nginx.enable" },
                value: {
                  type: "object",
                  description: "基因值：{type: 'Bool'|'Int'|'Float'|'String'|'List', value: ...}",
                },
                category: {
                  type: "string",
                  enum: ["Security", "Performance", "Services", "Network", "Storage", "Boot", "Kernel", "Other"],
                },
                mutable: { type: "boolean", description: "是否可变异", default: true },
              },
              required: ["name", "value", "category"],
            },
          },
          population_size: { type: "number", description: "种群大小，默认 20", default: 20 },
          generations: { type: "number", description: "进化代数，默认 10", default: 10 },
          mutation_rate: { type: "number", description: "变异率 0-1，默认 0.15", default: 0.15 },
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler(args: any) {
        const data = await agentPost(hosts, args.host, "/api/dna/evolve", {
          genes: args.genes,
          config: {
            population_size: args.population_size || 20,
            generations: args.generations || 10,
            mutation_rate: args.mutation_rate || 0.15,
            crossover_rate: 0.7,
            elite_count: 4,
          },
        });

        const best = data.best_genome;
        const fitness = best?.fitness;

        return {
          content: [{
            type: "text",
            text: [
              `## 🧬 Config DNA Evolution Complete`,
              ``,
              `**状态**: ${data.status}`,
              `**完成代数**: ${data.generations_completed}`,
              `**种群大小**: ${data.population_summary?.length || 0}`,
              fitness ? `**最优适应度**: ${fitness.composite.toFixed(3)}` : "",
              fitness ? [
                ``,
                `### 📊 最优个体 Fitness`,
                `- 🔨 构建速度: ${fitness.build_speed.toFixed(3)}`,
                `- 💾 磁盘优化: ${fitness.disk_size.toFixed(3)}`,
                `- 🔒 安全评分: ${fitness.security.toFixed(3)}`,
                `- ⚡ 启动时间: ${fitness.boot_time.toFixed(3)}`,
                `- 🏆 综合评分: ${fitness.composite.toFixed(3)}`,
              ].join("\n") : "",
              best ? `\n**最优基因组**: ${best.id} (${best.genes.length} genes)` : "",
            ].filter(Boolean).join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    {
      name: "dna_population",
      description: "🧬 查看当前进化种群状态 — 获取所有基因组及其适应度评分。",
      inputSchema: {
        type: "object",
        properties: {
          limit: { type: "number", description: "返回数量限制", default: 20 },
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/dna/population", {
          limit: String(args.limit || 20),
        });

        const genomeList = (data.genomes || []).map((g: any, i: number) => {
          const f = g.fitness;
          return [
            `### ${i + 1}. ${g.label || g.id}`,
            `- 基因数: ${g.gene_count}`,
            f ? `- 适应度: ${f.composite.toFixed(3)} (安全:${f.security.toFixed(2)} 速度:${f.build_speed.toFixed(2)} 磁盘:${f.disk_size.toFixed(2)} 启动:${f.boot_time.toFixed(2)})` : "",
          ].filter(Boolean).join("\n");
        }).join("\n\n");

        return {
          content: [{
            type: "text",
            text: [
              `## 🧬 DNA Population (${data.population_size} individuals)`,
              ``,
              genomeList || "_Empty population. Run dna_evolve with genes to seed._",
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 2. Config Theater ────────────────────────────────────────
    {
      name: "theater_record",
      description: "🎭 记录配置变更场景 — 将每次配置变更记录为一个「场景」，支持回放、撤销和分支。",
      inputSchema: {
        type: "object",
        properties: {
          description: { type: "string", description: "变更描述" },
          diff: {
            type: "object",
            description: "配置差异",
            properties: {
              added: { type: "array", items: { type: "object", properties: { key: { type: "string" }, value: { type: "string" } } } },
              removed: { type: "array", items: { type: "object", properties: { key: { type: "string" }, value: { type: "string" } } } },
              modified: { type: "array", items: { type: "object", properties: { key: { type: "string" }, old_value: { type: "string" }, new_value: { type: "string" } } } },
            },
          },
          author: { type: "string", description: "变更者" },
          tags: { type: "array", items: { type: "string" }, description: "标签" },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["description", "diff"],
      },
      async handler(args: any) {
        const data = await agentPost(hosts, args.host, "/api/theater/record", {
          description: args.description,
          diff: args.diff,
          author: args.author,
          tags: args.tags,
        });
        return {
          content: [{
            type: "text",
            text: [
              `## 🎭 Scene Recorded`,
              ``,
              `- **ID**: ${data.id}`,
              `- **Act**: ${data.act} | **Scene**: ${data.scene_number}`,
              `- **Description**: ${data.description}`,
              `- **Time**: ${data.timestamp}`,
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    {
      name: "theater_replay",
      description: "🎭 回放配置变更历史 — 按时间顺序逐步展示所有配置变更，支持指定范围。",
      inputSchema: {
        type: "object",
        properties: {
          from: { type: "number", description: "起始场景号" },
          to: { type: "number", description: "结束场景号" },
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler(args: any) {
        const params: Record<string, string> = {};
        if (args.from) params.from = String(args.from);
        if (args.to) params.to = String(args.to);
        const data = await agentGet(hosts, args.host, "/api/theater/replay", params);

        const scenesText = (data.scenes || []).map((s: any) =>
          `  🎬 Scene ${s.scene_number} (Act ${s.act}): ${s.description} [${s.applied ? "✓ applied" : "✗ pending"}]`
        ).join("\n");

        const diff = data.cumulative_diff;
        return {
          content: [{
            type: "text",
            text: [
              `## 🎭 Replay: Scenes ${data.from_scene} → ${data.to_scene}`,
              ``,
              `**Total scenes**: ${data.total_scenes}`,
              ``,
              scenesText || "  _No scenes in range_",
              diff ? [
                ``,
                `### Cumulative Changes`,
                diff.added?.length ? `  +${diff.added.length} added` : "",
                diff.removed?.length ? `  -${diff.removed.length} removed` : "",
                diff.modified?.length ? `  ~${diff.modified.length} modified` : "",
              ].filter(Boolean).join("\n") : "",
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    {
      name: "theater_undo",
      description: "🎭 撤销单个配置场景 — 不是回滚到上一个状态，而是精确撤销某一次变更。",
      inputSchema: {
        type: "object",
        properties: {
          scene_id: { type: "string", description: "要撤销的场景 ID" },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["scene_id"],
      },
      async handler(args: any) {
        const data = await agentPost(hosts, args.host, "/api/theater/undo", {
          scene_id: args.scene_id,
        });
        return {
          content: [{
            type: "text",
            text: [
              `## 🎭 Scene Undone`,
              ``,
              `- **Undone**: ${data.undone_scene?.description}`,
              `- **Remaining**: ${data.remaining_scenes} scenes`,
              `- **Inverse diff**: +${data.inverse_diff?.added?.length || 0} / -${data.inverse_diff?.removed?.length || 0}`,
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    {
      name: "theater_branch",
      description: "🎭 创建配置分支 — 在某个场景处创建「如果当时走了另一条路」的平行时间线。",
      inputSchema: {
        type: "object",
        properties: {
          fork_scene_id: { type: "string", description: "分叉点场景 ID" },
          name: { type: "string", description: "分支名称" },
          description: { type: "string", description: "分支描述" },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["fork_scene_id", "name"],
      },
      async handler(args: any) {
        const data = await agentPost(hosts, args.host, "/api/theater/branch", {
          fork_scene_id: args.fork_scene_id,
          name: args.name,
          description: args.description || "",
        });
        return {
          content: [{
            type: "text",
            text: [
              `## 🎭 Branch Created`,
              ``,
              `- **Branch**: ${data.name} (${data.id})`,
              `- **Fork from**: ${data.fork_scene_id}`,
              `- **Base scenes**: ${data.scenes?.length || 0}`,
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 3. Blockchain Audit ──────────────────────────────────────
    {
      name: "chain_verify",
      description: "🔗 验证配置区块链完整性 — 检查所有区块的哈希链是否完好，检测任何篡改。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/chain/verify");
        const status = data.valid ? "✅ 链完整" : "❌ 检测到篡改";
        const errors = (data.errors || []).map((e: string) => `  ⚠️ ${e}`).join("\n");
        return {
          content: [{
            type: "text",
            text: [
              `## 🔗 Blockchain Verification`,
              ``,
              `**状态**: ${status}`,
              `**区块数**: ${data.total_blocks}`,
              `**验证时间**: ${data.verified_at}`,
              data.errors?.length ? `\n**错误**:\n${errors}` : "",
            ].filter(Boolean).join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    {
      name: "chain_history",
      description: "🔗 查看配置变更链历史 — 获取所有哈希链接的配置变更记录。",
      inputSchema: {
        type: "object",
        properties: {
          limit: { type: "number", description: "返回数量", default: 20 },
          action: { type: "string", description: "按操作类型过滤" },
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/chain/history", {
          limit: String(args.limit || 20),
          action: args.action || "",
        });

        const blocksText = (data.blocks || []).map((b: any) =>
          `  #${b.index} [${b.data.action}] ${b.data.description} — ${b.timestamp.slice(0, 19)}\n    hash: ${b.hash.slice(0, 16)}... prev: ${b.previous_hash.slice(0, 16)}...`
        ).join("\n\n");

        const stats = data.stats;
        return {
          content: [{
            type: "text",
            text: [
              `## 🔗 Config Chain History`,
              ``,
              stats ? `**Total blocks**: ${stats.total_blocks} | **Since**: ${stats.first_block?.slice(0, 10)}` : "",
              ``,
              blocksText || "  _No blocks yet_",
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 4. Benchmarking ──────────────────────────────────────────
    {
      name: "bench_run",
      description: "🎯 运行配置基准测试 — 测量启动时间、构建时间、磁盘占用、安全评分等，并生成对比报告。",
      inputSchema: {
        type: "object",
        properties: {
          label: { type: "string", description: "测试标签" },
          metrics: {
            type: "array",
            items: { type: "string" },
            description: "要测试的指标: boot_time, build_time, disk_size, service_startup, security_score, memory_usage",
          },
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler(args: any) {
        const data = await agentPost(hosts, args.host, "/api/bench/run", {
          label: args.label,
          metrics: args.metrics,
        });

        const metricsText = (data.metrics || []).map((m: any) =>
          `  📏 **${m.name}**: ${m.value.toFixed(1)} ${m.unit}`
        ).join("\n");

        const summary = data.summary;
        return {
          content: [{
            type: "text",
            text: [
              `## 🎯 Benchmark Complete`,
              ``,
              `**Run**: ${data.id} — ${data.label}`,
              `**Status**: ${data.status}`,
              ``,
              metricsText,
              summary ? [
                ``,
                `### 📊 Summary`,
                `- 🏆 Grade: **${summary.overall_grade}**`,
                summary.boot_time_ms ? `- ⚡ Boot: ${summary.boot_time_ms.toFixed(0)}ms` : "",
                summary.build_time_ms ? `- 🔨 Build: ${(summary.build_time_ms / 1000).toFixed(1)}s` : "",
                summary.disk_size_mb ? `- 💾 Disk: ${summary.disk_size_mb.toFixed(0)}MB` : "",
                summary.security_score ? `- 🔒 Security: ${summary.security_score.toFixed(0)}/100` : "",
              ].filter(Boolean).join("\n") : "",
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    {
      name: "bench_compare",
      description: "🎯 对比两次基准测试 — 检测配置变更是改进还是回退，包含统计显著性分析。",
      inputSchema: {
        type: "object",
        properties: {
          baseline: { type: "string", description: "基准测试 ID" },
          current: { type: "string", description: "当前测试 ID" },
          host: { type: "string", description: hostParamDesc },
        },
        required: ["baseline", "current"],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/bench/compare", {
          baseline: args.baseline,
          current: args.current,
        });

        const verdictEmoji = { improved: "✅ 改进", regressed: "❌ 回退", neutral: "➖ 持平" }[data.verdict] || "❓";
        const deltasText = (data.deltas || []).map((d: any) => {
          const arrow = d.direction === "better" ? "↑" : d.direction === "worse" ? "↓" : "→";
          const sig = d.significant ? " ⚡" : "";
          return `  ${arrow} **${d.name}**: ${d.baseline_value.toFixed(1)} → ${d.current_value.toFixed(1)} (${d.delta_pct > 0 ? "+" : ""}${d.delta_pct.toFixed(1)}%)${sig}`;
        }).join("\n");

        return {
          content: [{
            type: "text",
            text: [
              `## 🎯 Benchmark Comparison`,
              ``,
              `**Verdict**: ${verdictEmoji}`,
              `**Improvements**: ${data.improvement_count} | **Regressions**: ${data.regression_count}`,
              ``,
              `### Deltas`,
              deltasText || "  _No comparable metrics_",
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 5. Topology Map ──────────────────────────────────────────
    {
      name: "topology_map",
      description: "🗺️ 获取 NixOS 服务拓扑图 — 自动发现所有服务、端口、依赖关系和网络连接。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler(args: any) {
        const data = await agentGet(hosts, args.host, "/api/topology");

        const nodesText = (data.nodes || []).map((n: any) => {
          const typeKey = typeof n.node_type === "object" ? Object.keys(n.node_type)[0] : n.node_type;
          const icon = { Service: "⚙️", Database: "🗄️", ReverseProxy: "🔀", Cache: "⚡", Queue: "📨", Storage: "💾", Network: "🌐" }[typeKey] || "📦";
          const ports = (n.ports || []).map((p: any) => p.port).join(", ");
          return `  ${icon} **${n.name}** [${n.status}]${ports ? ` :${ports}` : ""}`;
        }).join("\n");

        const edgesText = (data.edges || []).map((e: any) =>
          `  🔗 ${e.from} → ${e.to} (${e.edge_type}${e.label ? `: ${e.label}` : ""}) [${e.health}]`
        ).join("\n");

        return {
          content: [{
            type: "text",
            text: [
              `## 🗺️ NixOS Topology — ${data.hostname}`,
              ``,
              `**Services** (${data.nodes?.length || 0}):`,
              nodesText || "  _No services discovered_",
              ``,
              `**Connections** (${data.edges?.length || 0}):`,
              edgesText || "  _No connections discovered_",
              ``,
              `🌐 Interactive: http://localhost:7890/topology`,
            ].join("\n"),
          }],
          structuredContent: data,
        };
      },
    },

    // ── 6. Collab Info ───────────────────────────────────────────
    {
      name: "collab_info",
      description: "🌊 协作编辑信息 — 查看实时协作编辑的 WebSocket 端点和使用说明。",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string", description: hostParamDesc },
        },
        required: [],
      },
      async handler() {
        return {
          content: [{
            type: "text",
            text: [
              `## 🌊 Real-Time Collaborative Config Editing`,
              ``,
              `基于 WebSocket 的实时协作编辑，类似 Google Docs for NixOS config。`,
              ``,
              `### WebSocket 端点`,
              `\`ws://<host>/api/collab/ws\``,
              ``,
              `### 协议`,
              `1. 连接后发送 init 消息: \`{"type":"init","client_id":"alice","client_name":"Alice"}\``,
              `2. 收到 Sync 消息获取当前文档和版本号`,
              `3. 发送 Operation 消息进行编辑: \`{"type":"operation","op":{"id":"op1","client_id":"alice","revision":0,"operation":{"op":"Insert","pos":5,"text":"hello"},"timestamp":"..."}}\``,
              `4. 发送 Cursor 消息同步光标位置`,
              ``,
              `### 功能`,
              `- ✅ Operational Transformation (OT) 冲突解决`,
              `- ✅ 多人同时编辑，自动合并`,
              `- ✅ 光标位置实时同步`,
              `- ✅ 带颜色标识的远程光标`,
              `- ✅ 编辑历史和版本追踪`,
            ].join("\n"),
          }],
          structuredContent: {
            websocket: "/api/collab/ws",
            protocol: "operational-transformation",
            features: ["ot-conflict-resolution", "cursor-tracking", "multi-peer", "version-history"],
          },
        };
      },
    },
  ];
}
