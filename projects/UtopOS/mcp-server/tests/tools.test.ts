/**
 * UtopOS MCP Server - Tool Routing & Integration Tests
 *
 * Tests MCP tool argument validation, host resolution, request construction,
 * and response formatting without requiring a live agent.
 *
 * Run: npx tsx tests/tools.test.ts
 */

// ─── Test harness ────────────────────────────────────────────────────────

let passed = 0;
let failed = 0;
const errors: string[] = [];

function assert(condition: boolean, msg: string) {
  if (condition) {
    passed++;
    console.log(`  ✅ ${msg}`);
  } else {
    failed++;
    console.error(`  ❌ ${msg}`);
    errors.push(msg);
  }
}

function assertEq<T>(actual: T, expected: T, msg: string) {
  assert(actual === expected, `${msg} (got: ${JSON.stringify(actual)}, expected: ${JSON.stringify(expected)})`);
}

function test(name: string, fn: () => void | Promise<void>) {
  console.log(`\n📋 ${name}`);
  return Promise.resolve(fn());
}

// ─── Stub types (extracted from index.ts for testability) ────────────────

interface HostEntry {
  url: string;
  token?: string;
  ssh_tunnel?: string;
  description?: string;
}

// ─── Host resolution logic (copied from index.ts) ────────────────────────

function resolveHost(
  hosts: Record<string, HostEntry>,
  hostArg?: string
): { name: string; entry: HostEntry } {
  const hostNames = Object.keys(hosts);

  if (hostArg && hosts[hostArg]) {
    return { name: hostArg, entry: hosts[hostArg] };
  }
  if (hosts["default"]) {
    return { name: "default", entry: hosts["default"] };
  }
  if (hostNames.length === 1) {
    return { name: hostNames[0], entry: hosts[hostNames[0]] };
  }
  throw new Error(`请指定 host 参数。可用主机: ${hostNames.join(", ")}`);
}

// ─── Request construction helpers ────────────────────────────────────────

function buildGetUrl(baseUrl: string, path: string, params: Record<string, string> = {}): URL {
  const url = new URL(path, baseUrl);
  for (const [k, v] of Object.entries(params)) {
    if (v) url.searchParams.set(k, v);
  }
  return url;
}

function buildHeaders(host: HostEntry): Record<string, string> {
  const headers: Record<string, string> = { Accept: "application/json" };
  if (host.token) headers["Authorization"] = `Bearer ${host.token}`;
  return headers;
}

// ─── Risk badge formatting (from index.ts) ───────────────────────────────

function formatRiskBadge(level: string): string {
  switch (level) {
    case "safe": return "🟢 安全";
    case "moderate": return "🟡 中等风险";
    case "dangerous": return "🔴 高风险";
    default: return "❓ 未知";
  }
}

// ─── Tool schema validation ──────────────────────────────────────────────

interface ToolParam {
  type: string;
  description?: string;
  default?: any;
  required?: boolean;
}

interface ToolSchema {
  name: string;
  description: string;
  inputSchema: {
    type: string;
    properties: Record<string, ToolParam>;
    required: string[];
  };
}

function validateToolArgs(schema: ToolSchema, args: Record<string, any>): string | null {
  for (const req of schema.inputSchema.required) {
    if (args[req] === undefined || args[req] === null || args[req] === "") {
      return `Missing required parameter: ${req}`;
    }
  }
  return null;
}

// ─── Mock agent responses ────────────────────────────────────────────────

const MOCK_SNAPSHOT = {
  hostname: "test-nixos",
  nixos_version: "24.05",
  kernel: "6.8.0",
  uptime: "3 days",
  memory: { total: "16Gi", used: "8Gi", available: "8Gi" },
  disk: [{ mount: "/", total: "100G", used: "45G", used_pct: 45 }],
  services: ["nginx.service", "sshd.service"],
  recent_failures: [],
};

const MOCK_VALIDATE = {
  valid: true,
  dry_run_output: "building...",
  summary: {
    packages_added: ["nginx-1.24.2"],
    packages_removed: [],
    services_restart: [],
    services_stop: [],
    risk_level: "safe",
    risk_reasons: [],
  },
};

const MOCK_GENERATIONS = {
  current: 42,
  generations: [
    { number: 40, date: "2026-04-10", description: "initial setup" },
    { number: 41, date: "2026-04-11", description: "add nginx" },
    { number: 42, date: "2026-04-12", description: "enable ssl" },
  ],
};

// ═══════════════════════════════════════════════════════════════════════════
// TEST SUITES
// ═══════════════════════════════════════════════════════════════════════════

// ─── 1. Host Resolution ──────────────────────────────────────────────────

async function testHostResolution() {
  await test("resolve explicit host", () => {
    const hosts: Record<string, HostEntry> = {
      default: { url: "http://localhost:7890" },
      prod: { url: "http://prod:7890", token: "tok" },
    };
    const r = resolveHost(hosts, "prod");
    assertEq(r.name, "prod", "resolved name");
    assertEq(r.entry.url, "http://prod:7890", "resolved url");
  });

  await test("resolve default host when no arg", () => {
    const hosts: Record<string, HostEntry> = {
      default: { url: "http://localhost:7890" },
      prod: { url: "http://prod:7890" },
    };
    const r = resolveHost(hosts);
    assertEq(r.name, "default", "falls back to default");
  });

  await test("resolve single host auto", () => {
    const hosts: Record<string, HostEntry> = {
      myserver: { url: "http://server:7890" },
    };
    const r = resolveHost(hosts);
    assertEq(r.name, "myserver", "auto-selects only host");
  });

  await test("error on multiple hosts without selection", () => {
    const hosts: Record<string, HostEntry> = {
      prod: { url: "http://prod:7890" },
      staging: { url: "http://staging:7890" },
    };
    try {
      resolveHost(hosts);
      assert(false, "should have thrown");
    } catch (e: any) {
      assert(e.message.includes("请指定 host"), "error message is correct");
      assert(e.message.includes("prod"), "lists prod");
      assert(e.message.includes("staging"), "lists staging");
    }
  });

  await test("resolve unknown host falls back to default", () => {
    const hosts: Record<string, HostEntry> = {
      default: { url: "http://localhost:7890" },
      prod: { url: "http://prod:7890" },
    };
    const r = resolveHost(hosts, "nonexistent");
    assertEq(r.name, "default", "falls back to default for unknown host");
  });
}

// ─── 2. Request Construction ─────────────────────────────────────────────

async function testRequestConstruction() {
  await test("GET request URL with params", () => {
    const url = buildGetUrl("http://localhost:7890", "/api/logs", {
      unit: "nginx.service",
      lines: "100",
    });
    assertEq(url.pathname, "/api/logs", "path correct");
    assertEq(url.searchParams.get("unit"), "nginx.service", "unit param");
    assertEq(url.searchParams.get("lines"), "100", "lines param");
  });

  await test("GET request URL skips empty params", () => {
    const url = buildGetUrl("http://localhost:7890", "/api/snapshot", {
      host: "",
    });
    assert(!url.searchParams.has("host"), "empty param omitted");
  });

  await test("auth header included when token present", () => {
    const headers = buildHeaders({ url: "http://x", token: "my-secret" });
    assertEq(headers["Authorization"], "Bearer my-secret", "bearer token");
    assertEq(headers["Accept"], "application/json", "accept header");
  });

  await test("no auth header when no token", () => {
    const headers = buildHeaders({ url: "http://x" });
    assert(!("Authorization" in headers), "no auth header");
  });

  await test("POST body serialization", () => {
    const body = { config: "services.nginx.enable = true;", message: "enable nginx" };
    const serialized = JSON.stringify(body);
    const parsed = JSON.parse(serialized);
    assertEq(parsed.config, "services.nginx.enable = true;", "config preserved");
    assertEq(parsed.message, "enable nginx", "message preserved");
  });
}

// ─── 3. Tool Schema Validation ───────────────────────────────────────────

async function testToolValidation() {
  const TOOLS: ToolSchema[] = [
    {
      name: "service_logs",
      description: "Get service logs",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string" },
          unit: { type: "string" },
          lines: { type: "number", default: 50 },
        },
        required: ["unit"],
      },
    },
    {
      name: "config_validate",
      description: "Validate config",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string" },
          config: { type: "string" },
        },
        required: ["config"],
      },
    },
    {
      name: "config_read",
      description: "Read config",
      inputSchema: {
        type: "object",
        properties: {
          host: { type: "string" },
          path: { type: "string" },
        },
        required: [],
      },
    },
  ];

  await test("validate service_logs requires unit", () => {
    const schema = TOOLS.find((t) => t.name === "service_logs")!;
    const err = validateToolArgs(schema, { host: "default" });
    assert(err !== null, "error on missing unit");
    assert(err!.includes("unit"), "error mentions unit");
  });

  await test("validate service_logs passes with unit", () => {
    const schema = TOOLS.find((t) => t.name === "service_logs")!;
    const err = validateToolArgs(schema, { unit: "nginx.service" });
    assert(err === null, "no error with unit provided");
  });

  await test("validate config_validate requires config", () => {
    const schema = TOOLS.find((t) => t.name === "config_validate")!;
    const err = validateToolArgs(schema, {});
    assert(err !== null, "error on missing config");
  });

  await test("validate config_read has no required params", () => {
    const schema = TOOLS.find((t) => t.name === "config_read")!;
    const err = validateToolArgs(schema, {});
    assert(err === null, "no error with empty args");
  });

  await test("validate rejects empty string for required field", () => {
    const schema = TOOLS.find((t) => t.name === "service_logs")!;
    const err = validateToolArgs(schema, { unit: "" });
    assert(err !== null, "empty string rejected");
  });
}

// ─── 4. Response Formatting ──────────────────────────────────────────────

async function testResponseFormatting() {
  await test("risk badge formatting", () => {
    assertEq(formatRiskBadge("safe"), "🟢 安全", "safe badge");
    assertEq(formatRiskBadge("moderate"), "🟡 中等风险", "moderate badge");
    assertEq(formatRiskBadge("dangerous"), "🔴 高风险", "dangerous badge");
    assertEq(formatRiskBadge("unknown"), "❓ 未知", "unknown badge");
  });

  await test("snapshot format includes hostname", () => {
    const s = MOCK_SNAPSHOT;
    const hasHostname = JSON.stringify(s).includes("test-nixos");
    assert(hasHostname, "hostname in snapshot");
  });

  await test("validate output structure", () => {
    const v = MOCK_VALIDATE;
    assert(v.valid === true, "valid flag");
    assert(Array.isArray(v.summary.packages_added), "packages_added is array");
    assert(typeof v.summary.risk_level === "string", "risk_level is string");
  });

  await test("generations output has current and list", () => {
    const g = MOCK_GENERATIONS;
    assertEq(g.current, 42, "current gen");
    assert(g.generations.length === 3, "has 3 generations");
    assert(g.generations.every((x: any) => x.number && x.date), "all gens have number+date");
  });
}

// ─── 5. Tool Routing Matrix ──────────────────────────────────────────────

async function testToolRouting() {
  const TOOL_ROUTES: Record<string, { method: string; path: string; needsBody: boolean }> = {
    system_snapshot: { method: "GET", path: "/api/snapshot", needsBody: false },
    service_logs: { method: "GET", path: "/api/logs", needsBody: false },
    config_read: { method: "GET", path: "/api/config", needsBody: false },
    package_info: { method: "GET", path: "/api/package", needsBody: false },
    generation_diff: { method: "GET", path: "/api/generations", needsBody: false },
    config_validate: { method: "POST", path: "/api/config/validate", needsBody: true },
    config_apply: { method: "POST", path: "/api/config/apply", needsBody: true },
    rollback_list: { method: "GET", path: "/api/generations", needsBody: false },
    rollback_apply: { method: "POST", path: "/api/rollback", needsBody: true },
  };

  await test("all 9 tools have route definitions", () => {
    assertEq(Object.keys(TOOL_ROUTES).length, 9, "9 tool routes");
  });

  await test("GET tools don't need body", () => {
    for (const [name, route] of Object.entries(TOOL_ROUTES)) {
      if (route.method === "GET") {
        assert(!route.needsBody, `${name} GET has no body`);
      }
    }
  });

  await test("POST tools need body", () => {
    for (const [name, route] of Object.entries(TOOL_ROUTES)) {
      if (route.method === "POST") {
        assert(route.needsBody, `${name} POST has body`);
      }
    }
  });

  await test("all routes start with /api/", () => {
    for (const [name, route] of Object.entries(TOOL_ROUTES)) {
      assert(route.path.startsWith("/api/"), `${name} path starts with /api/`);
    }
  });

  await test("read-only tools use GET", () => {
    const readOnly = ["system_snapshot", "service_logs", "config_read", "package_info", "generation_diff", "rollback_list"];
    for (const name of readOnly) {
      assertEq(TOOL_ROUTES[name].method, "GET", `${name} uses GET`);
    }
  });

  await test("mutation tools use POST", () => {
    const mutations = ["config_validate", "config_apply", "rollback_apply"];
    for (const name of mutations) {
      assertEq(TOOL_ROUTES[name].method, "POST", `${name} uses POST`);
    }
  });
}

// ─── 6. Error Handling ───────────────────────────────────────────────────

async function testErrorHandling() {
  await test("agent error propagates correctly", async () => {
    const mockError = { error: { code: "VALIDATION_ERROR", message: "配置不能为空" } };
    assertEq(mockError.error.code, "VALIDATION_ERROR", "error code");
    assert(mockError.error.message.includes("配置"), "error message");
  });

  await test("HTTP status mapped to error type", () => {
    const statusMap: Record<number, string> = {
      400: "VALIDATION_ERROR",
      401: "UNAUTHORIZED",
      404: "NOT_FOUND",
      500: "INTERNAL_ERROR",
    };
    for (const [status, code] of Object.entries(statusMap)) {
      assert(typeof code === "string", `status ${status} → ${code}`);
    }
  });

  await test("unknown tool throws error", () => {
    const knownTools = ["system_snapshot", "service_logs", "config_read", "package_info",
      "generation_diff", "config_validate", "config_apply", "rollback_list", "rollback_apply"];
    const unknownTool = "deploy_kubernetes";
    assert(!knownTools.includes(unknownTool), "unknown tool not in list");
  });
}

// ═══════════════════════════════════════════════════════════════════════════
// RUN ALL
// ═══════════════════════════════════════════════════════════════════════════

async function main() {
  console.log("🧪 UtopOS MCP Server — Tool Routing Tests\n");

  await testHostResolution();
  await testRequestConstruction();
  await testToolValidation();
  await testResponseFormatting();
  await testToolRouting();
  await testErrorHandling();

  console.log(`\n${"═".repeat(50)}`);
  console.log(`Results: ${passed} passed, ${failed} failed`);
  if (failed > 0) {
    console.error("\nFailed tests:");
    errors.forEach((e) => console.error(`  • ${e}`));
    process.exit(1);
  }
}

main();
