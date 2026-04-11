#!/usr/bin/env bash
# healthcheck.sh — 检查模型服务是否就绪
set -euo pipefail

HOST="${RESCUE_MODEL_HOST:-127.0.0.1}"
PORT="${RESCUE_MODEL_PORT:-8081}"
TIMEOUT="${RESCUE_HEALTH_TIMEOUT:-5}"

URL="http://$HOST:$PORT/health"

for i in $(seq 1 30); do
    if curl -s --max-time "$TIMEOUT" "$URL" &>/dev/null; then
        echo "✅ 模型服务就绪 ($URL)"
        exit 0
    fi
    echo "⏳ 等待模型服务... ($i/30)"
    sleep 2
done

echo "❌ 模型服务超时未就绪"
exit 1
