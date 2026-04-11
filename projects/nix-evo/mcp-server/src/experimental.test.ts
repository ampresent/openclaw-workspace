/**
 * Unit tests for nix-evo MCP Server experimental tools
 * 
 * Run with: npx tsx src/experimental.test.ts
 * Or: npm test (if configured)
 */

import { describe, it, expect, beforeEach, vi, Mock } from "vitest";

// ─── Mock fetch globally ──────────────────────────────────────────────

const mockFetch = vi.fn();
(globalThis as any).fetch = mockFetch;

// ─── Import after mocking ─────────────────────────────────────────────

import {
  getExperimentalTools,
  handleExperimentalTool,
} from "./experimental.js";

// ─── Test fixtures ────────────────────────────────────────────────────

const mockHosts = {
  default: {
    url: "http://127.0.0.1:7890",
  },
};

const mockHostsMulti = {
  prod: { url: "http://prod:7890", token: "secret123" },
  staging: { url: "http://staging:7890" },
};

// ─── Tool definitions ─────────────────────────────────────────────────

describe("getExperimentalTools", () => {
  it("returns 4 tools", () => {
    const tools = getExperimentalTools(mockHosts);
    expect(tools).toHaveLength(4);
  });

  it("includes dashboard_subscribe", () => {
    const tools = getExperimentalTools(mockHosts);
    const tool = tools.find((t) => t.name === "dashboard_subscribe");
    expect(tool).toBeDefined();
    expect(tool!.description).toContain("实时");
  });

  it("includes audit_query", () => {
    const tools = getExperimentalTools(mockHosts);
    const tool = tools.find((t) => t.name === "audit_query");
    expect(tool).toBeDefined();
    expect(tool!.inputSchema.properties).toHaveProperty("action");
    expect(tool!.inputSchema.properties).toHaveProperty("limit");
  });

  it("includes healer_status", () => {
    const tools = getExperimentalTools(mockHosts);
    const tool = tools.find((t) => t.name === "healer_status");
    expect(tool).toBeDefined();
  });

  it("includes flake_convert", () => {
    const tools = getExperimentalTools(mockHosts);
    const tool = tools.find((t) => t.name === "flake_convert");
    expect(tool).toBeDefined();
    expect(tool!.inputSchema.properties).toHaveProperty("channel");
    expect(tool!.inputSchema.properties).toHaveProperty("hostname");
  });

  it("shows host options for multi-host setup", () => {
    const tools = getExperimentalTools(mockHostsMulti);
    const tool = tools.find((t) => t.name === "dashboard_subscribe");
    expect(tool!.inputSchema.properties.host.description).toContain("prod");
    expect(tool!.inputSchema.properties.host.description).toContain("staging");
  });
});

// ─── dashboard_subscribe ──────────────────────────────────────────────

describe("dashboard_subscribe", () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it("fetches snapshot and formats output", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        cpu_usage_pct: 45.2,
        memory: { usage_pct: 62.5 },
        load_avg: [1.2, 0.8, 0.5],
        services: [
          { name: "nginx.service", active: "active", sub: "running" },
          { name: "sshd.service", active: "active", sub: "running" },
        ],
      }),
    });

    const result = await handleExperimentalTool(
      "dashboard_subscribe",
      {},
      mockHosts
    );

    expect(result.content).toHaveLength(2); // formatted + JSON
    expect(result.content[0].text).toContain("CPU");
    expect(result.content[0].text).toContain("45.2%");
    expect(result.content[0].text).toContain("nginx.service");
    expect(result.isError).toBeUndefined();
  });

  it("handles API errors", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 500,
      statusText: "Internal Server Error",
      text: async () => "systemctl failed",
    });

    const result = await handleExperimentalTool(
      "dashboard_subscribe",
      {},
      mockHosts
    );

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("错误");
  });

  it("sends correct API request", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ cpu_usage_pct: 0 }),
    });

    await handleExperimentalTool("dashboard_subscribe", {}, mockHosts);

    expect(mockFetch).toHaveBeenCalledTimes(1);
    const url = mockFetch.mock.calls[0][0];
    expect(url).toContain("/api/snapshot");
  });
});

// ─── audit_query ──────────────────────────────────────────────────────

describe("audit_query", () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it("queries audit logs and formats output", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        total: 100,
        returned: 2,
        log_path: "/root/.nix-evo/audit.log",
        entries: [
          {
            timestamp: "1712899200",
            action: "config_apply",
            method: "POST",
            path: "/api/config/apply",
            params_hash: "abc123",
            client_ip: "127.0.0.1",
            result: "success",
            duration_ms: 150,
          },
        ],
      }),
    });

    const result = await handleExperimentalTool(
      "audit_query",
      { action: "config_apply", limit: 10 },
      mockHosts
    );

    expect(result.content[0].text).toContain("审计日志");
    expect(result.content[0].text).toContain("100");
    expect(result.content[0].text).toContain("config_apply");
  });

  it("passes filter params to API", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ total: 0, returned: 0, entries: [] }),
    });

    await handleExperimentalTool(
      "audit_query",
      { action: "rollback", path: "/api/rollback", limit: 25 },
      mockHosts
    );

    const url = mockFetch.mock.calls[0][0];
    expect(url).toContain("action=rollback");
    expect(url).toContain("path=%2Fapi%2Frollback");
    expect(url).toContain("limit=25");
  });

  it("shows empty state", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ total: 0, returned: 0, entries: [] }),
    });

    const result = await handleExperimentalTool("audit_query", {}, mockHosts);

    expect(result.content[0].text).toContain("无记录");
  });
});

// ─── healer_status ────────────────────────────────────────────────────

describe("healer_status", () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it("shows running healer status", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        running: true,
        check_interval_secs: 30,
        total_heal_actions: 5,
        last_check: "1712899200",
        rules: [
          {
            service: "nginx.service",
            max_failures: 3,
            window_minutes: 5,
            action: "restart",
            cooldown_minutes: 10,
          },
        ],
        service_states: [
          { service: "nginx.service", healthy: true, failure_count: 0, last_action: null },
          { service: "sshd.service", healthy: true, failure_count: 0, last_action: "restart" },
        ],
      }),
    });

    const result = await handleExperimentalTool("healer_status", {}, mockHosts);

    expect(result.content[0].text).toContain("运行中");
    expect(result.content[0].text).toContain("nginx.service");
    expect(result.content[0].text).toContain("修复次数: 5");
  });

  it("shows stopped healer status", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        running: false,
        check_interval_secs: 30,
        total_heal_actions: 0,
        rules: [],
        service_states: [],
      }),
    });

    const result = await handleExperimentalTool("healer_status", {}, mockHosts);

    expect(result.content[0].text).toContain("未运行");
  });
});

// ─── flake_convert ────────────────────────────────────────────────────

describe("flake_convert", () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it("converts config and formats output", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        flake_nix: '{\n  description = "test";\n  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";\n}',
        detected_channel: "nixos-24.05",
        detected_hostname: "myhost",
        detected_inputs: ["nixpkgs"],
        detected_modules: [],
        warnings: [],
      }),
    });

    const result = await handleExperimentalTool(
      "flake_convert",
      { channel: "nixos-24.05" },
      mockHosts
    );

    expect(result.content[0].text).toContain("Flake 转换");
    expect(result.content[0].text).toContain("nixos-24.05");
    expect(result.content[0].text).toContain("nixpkgs.url");
  });

  it("shows warnings", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        flake_nix: "{}",
        detected_channel: "nixos-24.05",
        detected_hostname: "h",
        detected_inputs: [],
        detected_modules: [],
        warnings: ["检测到 <nixpkgs>"],
      }),
    });

    const result = await handleExperimentalTool("flake_convert", {}, mockHosts);

    expect(result.content[0].text).toContain("注意");
    expect(result.content[0].text).toContain("<nixpkgs>");
  });

  it("posts config content to API", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        flake_nix: "", detected_channel: "", detected_hostname: "",
        detected_inputs: [], detected_modules: [], warnings: [],
      }),
    });

    await handleExperimentalTool(
      "flake_convert",
      {
        config_content: "{ networking.hostName = 'test'; }",
        hostname: "override",
        extra_inputs: { agenix: "github:ryantm/agenix" },
      },
      mockHosts
    );

    expect(mockFetch).toHaveBeenCalledTimes(1);
    const callArgs = mockFetch.mock.calls[0];
    expect(callArgs[1].method).toBe("POST");

    const body = JSON.parse(callArgs[1].body);
    expect(body.config_content).toContain("test");
    expect(body.hostname).toBe("override");
    expect(body.extra_inputs.agenix).toContain("agenix");
  });
});

// ─── Multi-host resolution ────────────────────────────────────────────

describe("multi-host resolution", () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it("uses specified host", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ running: true }),
    });

    await handleExperimentalTool(
      "healer_status",
      { host: "prod" },
      mockHostsMulti
    );

    const url = mockFetch.mock.calls[0][0];
    expect(url).toContain("http://prod:7890");
  });

  it("adds auth token when present", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ running: true }),
    });

    await handleExperimentalTool(
      "healer_status",
      { host: "prod" },
      mockHostsMulti
    );

    const headers = mockFetch.mock.calls[0][1].headers;
    expect(headers.Authorization).toBe("Bearer secret123");
  });

  it("throws on unknown host", async () => {
    const result = await handleExperimentalTool(
      "healer_status",
      { host: "nonexistent" },
      mockHostsMulti
    );

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("请指定 host");
  });
});

// ─── Unknown tool ─────────────────────────────────────────────────────

describe("unknown tool handling", () => {
  it("returns error for unknown tool", async () => {
    const result = await handleExperimentalTool(
      "nonexistent_tool",
      {},
      mockHosts
    );

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("Unknown experimental tool");
  });
});
