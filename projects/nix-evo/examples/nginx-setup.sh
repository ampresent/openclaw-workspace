#!/bin/bash
# Example: Setting up nginx with nix-evo API
# Run against a nix-evo-agent instance

AGENT="http://127.0.0.1:7890"

echo "=== Step 1: Check system health ==="
curl -s "$AGENT/api/snapshot" | jq '.hostname, .recent_failures'

echo "=== Step 2: Read current config ==="
curl -s "$AGENT/api/config" | jq -r '.content' | head -20

echo "=== Step 3: Validate nginx config ==="
curl -s -X POST "$AGENT/api/config/validate" \
  -H "Content-Type: application/json" \
  -d '{"config": "services.nginx.enable = true;"}' | jq '.summary.risk_level'

echo "=== Step 4: Apply (commented out - uncomment to execute) ==="
# curl -s -X POST "$AGENT/api/config/apply" \
#   -H "Content-Type: application/json" \
#   -d '{"config": "services.nginx.enable = true;", "message": "Enable nginx web server"}' | jq
