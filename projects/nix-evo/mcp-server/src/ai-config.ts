/**
 * AI Configuration Generator - MCP side
 *
 * Translates natural language prompts into NixOS configuration requests
 * and formats the agent's response for the AI model.
 */

/**
 * Build the system context prompt for LLM-based config generation.
 * This would be used when calling an external LLM API.
 */
export function buildNixGenerationPrompt(userPrompt: string, existingConfig?: string): string {
  const sections = [
    `You are a NixOS configuration generator. Given a natural language description, generate valid NixOS configuration.`,
    ``,
    `Rules:`,
    `- Output ONLY valid NixOS configuration code`,
    `- Use modern NixOS module syntax (nixos-options style)`,
    `- Prefer declarative configuration over imperative scripts`,
    `- Include comments explaining non-obvious choices`,
    `- Never include flake.nix or other file-level boilerplate unless asked`,
    `- For dangerous operations (firewall, bootloader, disk), add a warning comment`,
    ``,
  ];

  if (existingConfig) {
    sections.push(`Existing configuration to modify:`);
    sections.push(`\`\`\`nix`);
    sections.push(existingConfig);
    sections.push(`\`\`\``);
    sections.push(``);
    sections.push(`Modify the existing config to satisfy the request. Output the complete modified config.`);
  } else {
    sections.push(`Generate a configuration snippet for the following request.`);
  }

  sections.push(``);
  sections.push(`Request: ${userPrompt}`);

  return sections.join("\n");
}

/**
 * Known NixOS patterns for template-based fallback.
 * Mirrors the Rust-side patterns in ai_config.rs.
 */
export const NIX_PATTERNS = [
  {
    keywords: ["nginx", "web server"],
    snippet: `services.nginx.enable = true;`,
    risk: "moderate",
  },
  {
    keywords: ["docker", "container"],
    snippet: `virtualisation.docker.enable = true;`,
    risk: "safe",
  },
  {
    keywords: ["ssh", "openssh"],
    snippet: `services.openssh.enable = true;`,
    risk: "safe",
  },
  {
    keywords: ["postgresql", "postgres"],
    snippet: `services.postgresql.enable = true;`,
    risk: "moderate",
  },
  {
    keywords: ["redis"],
    snippet: `services.redis.enable = true;`,
    risk: "safe",
  },
  {
    keywords: ["firewall", "port"],
    snippet: `networking.firewall.allowedTCPPorts = [ 80 443 ];`,
    risk: "dangerous",
  },
];

/**
 * Quick template match for common patterns (no LLM needed).
 */
export function quickMatch(prompt: string): { snippet: string; risk: string } | null {
  const lower = prompt.toLowerCase();
  for (const pattern of NIX_PATTERNS) {
    if (pattern.keywords.some(kw => lower.includes(kw))) {
      return { snippet: pattern.snippet, risk: pattern.risk };
    }
  }
  return null;
}
