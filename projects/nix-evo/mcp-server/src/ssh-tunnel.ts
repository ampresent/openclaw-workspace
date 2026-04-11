import { execSync, spawn, ChildProcess } from "child_process";

interface TunnelEntry {
  process: ChildProcess;
  localPort: number;
  remoteHost: string;
  remotePort: number;
}

const tunnels = new Map<string, TunnelEntry>();

/**
 * Parse ssh_tunnel string: "user@host:port" or "user@host"
 * Returns { user, host, remotePort }
 */
function parseSshTunnel(tunnel: string): {
  user: string;
  host: string;
  remotePort: number;
} {
  // user@host:port
  const match = tunnel.match(/^(.+)@([^:]+):(\d+)$/);
  if (match) {
    return { user: match[1], host: match[2], remotePort: parseInt(match[3]) };
  }
  // user@host (default port 7890)
  const match2 = tunnel.match(/^(.+)@(.+)$/);
  if (match2) {
    return { user: match2[1], host: match2[2], remotePort: 7890 };
  }
  throw new Error(`Invalid ssh_tunnel format: ${tunnel}. Expected: user@host[:port]`);
}

/**
 * Check if a local port is available
 */
function isPortAvailable(port: number): boolean {
  try {
    execSync(`ss -tln | grep -q ':${port} '`, { stdio: "ignore" });
    return false; // Port is in use
  } catch {
    return true; // Port is free
  }
}

/**
 * Find an available local port starting from the given port
 */
function findAvailablePort(startPort: number): number {
  for (let port = startPort; port < startPort + 100; port++) {
    if (isPortAvailable(port)) return port;
  }
  throw new Error(`No available port found in range ${startPort}-${startPort + 100}`);
}

/**
 * Wait for a port to be ready (SSH tunnel established)
 */
async function waitForPort(port: number, timeoutMs: number = 10000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      execSync(`ss -tln | grep -q ':${port} '`, { stdio: "ignore" });
      return;
    } catch {
      await new Promise((r) => setTimeout(r, 200));
    }
  }
  throw new Error(`Timeout waiting for port ${port} to be ready`);
}

/**
 * Ensure an SSH tunnel is established for a given host config.
 * Returns the local port to connect to.
 */
export async function ensureTunnel(
  hostName: string,
  sshTunnel: string,
  remoteUrl: string
): Promise<string> {
  // If tunnel already exists and is alive, reuse it
  const existing = tunnels.get(hostName);
  if (existing && !existing.process.killed) {
    return `http://127.0.0.1:${existing.localPort}`;
  }

  const { user, host, remotePort } = parseSshTunnel(sshTunnel);

  // Find a local port
  const url = new URL(remoteUrl);
  const localPort = findAvailablePort(parseInt(url.port) || 7890);

  console.error(
    `Establishing SSH tunnel: ${user}@${host} → localhost:${localPort}:${host}:${remotePort}`
  );

  // Spawn SSH tunnel
  const ssh = spawn(
    "ssh",
    [
      "-L", `${localPort}:127.0.0.1:${remotePort}`,
      "-N", // no remote command
      "-f", // background
      "-o", "ExitOnForwardFailure=yes",
      "-o", "ConnectTimeout=10",
      "-o", "ServerAliveInterval=60",
      "-o", "ServerAliveCountMax=3",
      `${user}@${host}`,
    ],
    { stdio: ["ignore", "pipe", "pipe"] }
  );

  // Wait for tunnel to be established
  await new Promise<void>((resolve, reject) => {
    let stderr = "";
    ssh.stderr?.on("data", (d) => (stderr += d.toString()));
    ssh.on("close", (code) => {
      if (code !== 0) {
        reject(new Error(`SSH tunnel failed (exit ${code}): ${stderr}`));
      }
    });
    // Give it a moment
    setTimeout(resolve, 1000);
  });

  await waitForPort(localPort, 10000);

  // The -f flag means ssh backgrounds itself, so we don't track the child process
  // Instead, we just record the port
  tunnels.set(hostName, {
    process: ssh,
    localPort,
    remoteHost: host,
    remotePort,
  });

  console.error(`SSH tunnel ready: localhost:${localPort} → ${host}:${remotePort}`);
  return `http://127.0.0.1:${localPort}`;
}

/**
 * Cleanup all tunnels on shutdown
 */
export function cleanupTunnels(): void {
  for (const [name, entry] of tunnels) {
    try {
      if (!entry.process.killed) {
        entry.process.kill("SIGTERM");
      }
    } catch {
      // ignore
    }
    console.error(`Closed tunnel: ${name}`);
  }
  tunnels.clear();
}

// Cleanup on exit
process.on("SIGTERM", cleanupTunnels);
process.on("SIGINT", cleanupTunnels);
