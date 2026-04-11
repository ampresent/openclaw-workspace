/**
 * nix-evo MCP Server - Unit Tests
 *
 * Run: npx tsx src/index.test.ts
 */

// Test hosts.toml parsing
interface HostEntry {
  url: string;
  token?: string;
  ssh_tunnel?: string;
  description?: string;
}

function parseHostsToml(content: string): Record<string, HostEntry> {
  const hosts: Record<string, HostEntry> = {};
  let currentSection: string | null = null;

  for (const rawLine of content.split("\n")) {
    const line = rawLine.replace(/#.*$/, "").trim();
    if (!line) continue;
    const sectionMatch = line.match(/^\[hosts\.([a-zA-Z0-9_-]+)\]$/);
    if (sectionMatch) {
      currentSection = sectionMatch[1];
      hosts[currentSection] = { url: "" };
      continue;
    }
    const kvMatch = line.match(/^([a-zA-Z_]+)\s*=\s*"(.*)"$/);
    if (kvMatch && currentSection) {
      const [, key, value] = kvMatch;
      if (key === "url") hosts[currentSection].url = value;
      else if (key === "token") hosts[currentSection].token = value;
      else if (key === "ssh_tunnel") hosts[currentSection].ssh_tunnel = value;
      else if (key === "description") hosts[currentSection].description = value;
    }
  }

  return hosts;
}

// Tests
let passed = 0;
let failed = 0;

function assert(condition: boolean, msg: string) {
  if (condition) {
    passed++;
    console.log(`  ✅ ${msg}`);
  } else {
    failed++;
    console.error(`  ❌ ${msg}`);
  }
}

function test(name: string, fn: () => void) {
  console.log(`\n📋 ${name}`);
  fn();
}

// ─── hosts.toml parsing tests ────────────────────────────────────────

test("parse hosts.toml with multiple hosts", () => {
  const toml = `
# nix-evo hosts configuration
[hosts.default]
url = "http://127.0.0.1:7890"
token = "secret123"

[hosts.production]
url = "http://prod-server:7890"
token = "prod-token"
ssh_tunnel = "admin@prod-server:7890"
description = "Production server"

[hosts.staging]
url = "http://127.0.0.1:7891"
`;
  const hosts = parseHostsToml(toml);

  assert(Object.keys(hosts).length === 3, "parsed 3 hosts");
  assert(hosts.default.url === "http://127.0.0.1:7890", "default url correct");
  assert(hosts.default.token === "secret123", "default token correct");
  assert(hosts.production.url === "http://prod-server:7890", "production url correct");
  assert(hosts.production.ssh_tunnel === "admin@prod-server:7890", "ssh_tunnel parsed");
  assert(hosts.production.description === "Production server", "description parsed");
  assert(hosts.staging.url === "http://127.0.0.1:7891", "staging url correct");
  assert(hosts.staging.token === undefined, "staging has no token");
});

test("parse hosts.toml handles comments", () => {
  const toml = `
# This is a comment
[hosts.test]
# Another comment
url = "http://localhost:7890" # inline comment
token = "abc123"
`;
  const hosts = parseHostsToml(toml);
  assert(hosts.test.url === "http://localhost:7890", "url parsed with comments");
  assert(hosts.test.token === "abc123", "token parsed with inline comment");
});

test("parse hosts.toml handles empty file", () => {
  const hosts = parseHostsToml("");
  assert(Object.keys(hosts).length === 0, "empty file returns empty object");
});

test("parse hosts.toml with special characters in token", () => {
  const toml = `
[hosts.secure]
url = "http://localhost:7890"
token = "abc-123_XYZ.456"
`;
  const hosts = parseHostsToml(toml);
  assert(hosts.secure.token === "abc-123_XYZ.456", "special chars in token preserved");
});

// ─── Summary ─────────────────────────────────────────────────────────

console.log(`\n${"─".repeat(40)}`);
console.log(`Results: ${passed} passed, ${failed} failed`);

if (failed > 0) {
  process.exit(1);
}
