#!/bin/bash
# run-experiment.sh — Full experiment lifecycle
# 1. Build container
# 2. Inject defects
# 3. Start OpenClaw
# 4. Trigger agent self-healing via UtopOS skill
# 5. Verify results
# 6. Push to git every 5 minutes

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
CONTAINER_NAME="UtopOS-experiment"

echo "=== UtopOS Container Experiment ==="
echo "Project: $PROJECT_DIR"
echo ""

# Read MiMo API key from current openclaw config
MIMO_KEY=$(grep -o '"apiKey"[[:space:]]*:[[:space:]]*"[^"]*"' /root/.openclaw/openclaw.json | head -1 | sed 's/.*"apiKey"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/')
if [ -z "$MIMO_KEY" ]; then
    echo "ERROR: Could not extract MiMo API key from config"
    exit 1
fi
echo "MiMo API key: ${MIMO_KEY:0:8}..."

# Export for docker-compose
export MIMO_API_KEY="$MIMO_KEY"

# Step 1: Build
echo ""
echo "--- Step 1: Building container ---"
cd "$PROJECT_DIR"
docker compose -f container-experiment/docker-compose.yml build

# Step 2: Stop existing container if running
echo ""
echo "--- Step 2: Cleaning up old container ---"
docker rm -f "$CONTAINER_NAME" 2>/dev/null || true

# Step 3: Start container
echo ""
echo "--- Step 3: Starting container ---"
docker compose -f container-experiment/docker-compose.yml up -d

# Wait for container to be ready
echo "Waiting for container to start..."
sleep 10

# Step 4: Verify defects were injected
echo ""
echo "--- Step 4: Verifying defects ---"
docker exec "$CONTAINER_NAME" cat /root/.openclaw/workspace/defects-injected.log

# Step 5: Run the agent experiment
echo ""
echo "--- Step 5: Sending experiment prompt to agent ---"
# Copy experiment prompt into workspace
docker exec "$CONTAINER_NAME" cat /tmp/experiment-prompt.txt 2>/dev/null || true

echo ""
echo "Container is running. OpenClaw available at http://localhost:18789"
echo ""
echo "To trigger agent self-healing:"
echo "  docker exec -it $CONTAINER_NAME openclaw session"
echo ""
echo "To check health:"
echo "  docker exec $CONTAINER_NAME /health-check.sh"
echo ""
echo "To view logs:"
echo "  docker logs -f $CONTAINER_NAME"
