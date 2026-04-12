# UtopOS container experiment
# OpenClaw in a container with deliberate OS defects
# Tests whether the agent proactively fixes OS issues

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=Asia/Shanghai

# === Layer 1: Base packages (INTENTIONALLY INCOMPLETE) ===
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    gnupg \
    && rm -rf /var/lib/apt/lists/*

# === Layer 2: Node.js 22 ===
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# === Layer 3: Install OpenClaw globally ===
RUN npm install -g openclaw@2026.3.12

# === Layer 4: Create workspace structure ===
RUN mkdir -p /root/.openclaw/workspace/skills/UtopOS \
    /root/.openclaw/workspace/memory \
    /root/.openclaw/workspace/projects

# === Layer 5: Copy skill ===
COPY skills/UtopOS/SKILL.md /root/.openclaw/workspace/skills/UtopOS/SKILL.md

# === Layer 6: Copy experiment scripts ===
COPY container-experiment/inject-defects.sh /inject-defects.sh
COPY container-experiment/agent-experiment.sh /agent-experiment.sh
COPY container-experiment/health-check.sh /health-check.sh
RUN chmod +x /inject-defects.sh /agent-experiment.sh /health-check.sh

# === Layer 7: OpenClaw config (MiMo API) ===
COPY container-experiment/openclaw.json /root/.openclaw/openclaw.json

# === Layer 8: Entry point ===
COPY container-experiment/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 18789

ENTRYPOINT ["/entrypoint.sh"]
