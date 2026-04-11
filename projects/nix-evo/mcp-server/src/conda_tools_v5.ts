/**
 * Conda V5 MCP tools for nix-evo
 *
 * The final frontier of conda integration:
 * - conda_simulate: Dry-run simulation — "what if I install X?"
 * - conda_watch: Monitor conda-forge for updates to pinned packages
 * - conda_watch_check: Run an immediate version check
 * - conda_compare: Compare N environments side by side with compatibility scoring
 * - conda_build: Build custom conda packages (conda-build/boa)
 * - conda_build_status: Check build status and history
 * - conda_version_commit: Snapshot environment state (git-like commit)
 * - conda_version_log: View environment version history
 * - conda_cloud_sync: Sync environments to S3/GCS/MinIO
 */

import { Tool } from "@modelcontextprotocol/sdk/types.js";

// ─── Tool Definitions ─────────────────────────────────────────────────

export const CONDA_TOOLS_V5: Tool[] = [
  // ── Environment Simulation ─────────────────────────────────────────
  {
    name: "conda_simulate",
    description:
      "🧪 环境模拟：在不实际执行的情况下预测安装/删除/更新的结果。" +
      "类似「如果我安装 X 会怎样？」的 dry-run 模拟。" +
      "返回预测的变更列表、依赖树、冲突检测和风险评估。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "目标环境名称" },
        packages: {
          type: "array",
          items: { type: "string" },
          description: "要模拟操作的包列表",
        },
        action: {
          type: "string",
          enum: ["install", "remove", "update", "update-all"],
          description: "操作类型（默认 install）",
          default: "install",
        },
        channels: {
          type: "array",
          items: { type: "string" },
          description: "额外的 conda 频道",
        },
        python_version: { type: "string", description: "Python 版本（可选）" },
      },
      required: ["env", "packages"],
    },
  },

  // ── Conda-Forge Watch ──────────────────────────────────────────────
  {
    name: "conda_watch",
    description:
      "🔄 查看监控配置：获取当前 conda-forge 版本监控的配置和固定包列表。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
      },
    },
  },
  {
    name: "conda_watch_check",
    description:
      "🔄 立即检查：对所有固定包运行 conda-forge 版本检查。" +
      "返回每个包的最新版本、是否有更新、版本跳跃级别（major/minor/patch）。" +
      "自动生成类似 PR 的变更提案。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
      },
    },
  },

  // ── Environment Comparison Matrix ──────────────────────────────────
  {
    name: "conda_compare",
    description:
      "📊 环境对比矩阵：对比 N 个 conda 环境的差异。" +
      "显示版本差异、缺失/多余包、兼容性评分。" +
      "支持两两对比和全局兼容性评分。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        envs: {
          type: "array",
          items: { type: "string" },
          description: "要对比的环境名称列表（至少 2 个）",
          minItems: 2,
        },
      },
      required: ["envs"],
    },
  },

  // ── Conda Build Automation ─────────────────────────────────────────
  {
    name: "conda_build",
    description:
      "🏭 构建自定义 conda 包：使用 conda-build 或 boa 构建包。" +
      "指定 recipe 路径，可选频道、Python 版本和构建变量。" +
      "返回构建状态和输出产物路径。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        recipe_path: { type: "string", description: "meta.yaml 所在的 recipe 目录路径" },
        output_dir: { type: "string", description: "输出目录（默认 /tmp/nix-evo-builds）" },
        use_boa: { type: "boolean", description: "使用 boa 而非 conda-build（如果可用）" },
        channels: {
          type: "array",
          items: { type: "string" },
          description: "额外的 conda 频道",
        },
        python_version: { type: "string", description: "目标 Python 版本" },
        build_vars: {
          type: "object",
          additionalProperties: { type: "string" },
          description: "构建变量（键值对）",
        },
      },
      required: ["recipe_path"],
    },
  },
  {
    name: "conda_build_status",
    description:
      "🏭 构建状态：获取构建历史和状态。可指定 build_id 查看特定构建详情。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        build_id: { type: "string", description: "构建 ID（可选，不指定则返回全部历史）" },
      },
    },
  },

  // ── Environment Versioning ─────────────────────────────────────────
  {
    name: "conda_version_commit",
    description:
      "🧬 环境版本提交：像 git commit 一样捕获当前环境状态。" +
      "记录所有包版本、Python 版本和指纹。" +
      "可选 tag 标记和 commit message。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "环境名称" },
        message: { type: "string", description: "提交信息" },
        tag: { type: "string", description: "标签（如 'v1.0', 'stable'）" },
      },
      required: ["env"],
    },
  },
  {
    name: "conda_version_log",
    description:
      "🧬 环境版本日志：查看环境的版本历史记录。" +
      "显示每次提交的时间、包数量、消息和标签。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "环境名称" },
        limit: { type: "number", description: "返回的记录数量（默认 50）" },
      },
      required: ["env"],
    },
  },

  // ── Cloud Sync ─────────────────────────────────────────────────────
  {
    name: "conda_cloud_sync",
    description:
      "🌐 云端同步：将 conda 环境状态同步到云存储（S3/GCS/MinIO）。" +
      "支持 push（上传）、pull（下载）和双向同步。" +
      "可同步单个环境或全部环境。dry_run 模式预览不执行。",
    inputSchema: {
      type: "object",
      properties: {
        host: { type: "string", description: "服务器标识" },
        env: { type: "string", description: "要同步的环境名称（不指定则同步全部）" },
        direction: {
          type: "string",
          enum: ["push", "pull", "both"],
          description: "同步方向（默认 push）",
          default: "push",
        },
        force: { type: "boolean", description: "强制覆盖远程" },
        dry_run: { type: "boolean", description: "仅预览不执行" },
      },
    },
  },
];

// ─── API Call Handlers ────────────────────────────────────────────────

interface CondaClient {
  post: (path: string, body: any) => Promise<any>;
  get: (path: string, params?: Record<string, string>) => Promise<any>;
}

export function createCondaV5Handlers(client: CondaClient) {
  return {
    conda_simulate: (args: any) => client.post("/api/conda/simulate", args),
    conda_watch: (args: any) => client.get("/api/conda/watch"),
    conda_watch_check: (args: any) => client.post("/api/conda/watch/check", args),
    conda_compare: (args: any) => client.post("/api/conda/compare", args),
    conda_build: (args: any) => client.post("/api/conda/build", args),
    conda_build_status: (args: any) => {
      const params: Record<string, string> = {};
      if (args.build_id) params.build_id = args.build_id;
      return client.get("/api/conda/build/status", params);
    },
    conda_version_commit: (args: any) => client.post("/api/conda/version/commit", args),
    conda_version_log: (args: any) => {
      const params: Record<string, string> = { env: args.env };
      if (args.limit) params.limit = String(args.limit);
      return client.get("/api/conda/version/log", params);
    },
    conda_cloud_sync: (args: any) => client.post("/api/conda/cloud/sync", args),
  };
}
