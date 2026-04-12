#!/bin/bash
# entrypoint.sh — Container entry point for nix-evo experiment
set -e

echo "============================================"
echo "  nix-evo Container Experiment"
echo "  OpenClaw + Self-Healing OS Agent"
echo "============================================"
echo ""

# Step 1: Inject OS defects
echo "[1/4] Injecting OS defects..."
/inject-defects.sh
echo ""

# Step 2: Substitute API key
echo "[2/4] Configuring OpenClaw..."
if [ -n "$MIMO_API_KEY" ]; then
    sed -i "s|\${MIMO_API_KEY}|${MIMO_API_KEY}|g" /root/.openclaw/openclaw.json
    echo "  MiMo API key configured"
else
    echo "  WARNING: MIMO_API_KEY not set!"
fi

# Step 3: Initialize git in workspace
echo "[3/4] Initializing workspace git..."
cd /root/.openclaw/workspace
if [ ! -d .git ]; then
    git init
    git config user.email "nix-evo@experiment"
    git config user.name "nix-evo-experiment"
    git add -A
    git commit -m "Initial: container with injected defects"
fi

# Step 4: Start experiment
echo "[4/4] Starting experiment..."
echo ""

# Run the experiment script
/agent-experiment.sh

# Keep container running
echo ""
echo "Container is ready. OpenClaw gateway running."
echo "Connect via: http://localhost:18789"
echo ""
echo "To run health check: docker exec <container> /health-check.sh"
echo ""

# If first arg is "test", run health check after agent interaction
if [ "$1" = "test" ]; then
    echo "Waiting 60s for agent interaction..."
    sleep 60
    echo ""
    /health-check.sh
fi

# Keep alive
tail -f /dev/null
