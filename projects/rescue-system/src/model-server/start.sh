#!/usr/bin/env bash
# start.sh — 启动本地模型服务 (llama.cpp)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
CONFIG_FILE="$PROJECT_DIR/config/rescue.toml"

# 默认值
LLAMA_DIR="$PROJECT_DIR/third_party/llama.cpp"
MODEL_FILE="$PROJECT_DIR/models/qwen2.5-7b-instruct-q4_k_m.gguf"
HOST="127.0.0.1"
PORT="8081"
CONTEXT_SIZE="4096"
THREADS=$(nproc)

# 读取配置文件 (如果有)
if [ -f "$CONFIG_FILE" ]; then
    # 简单的 TOML 解析
    while IFS='=' read -r key value; do
        key=$(echo "$key" | tr -d ' ')
        value=$(echo "$value" | tr -d ' "')
        case "$key" in
            model_path) MODEL_FILE="$value" ;;
            host) HOST="$value" ;;
            port) PORT="$value" ;;
            context_size) CONTEXT_SIZE="$value" ;;
            threads) THREADS="$value" ;;
            llama_dir) LLAMA_DIR="$value" ;;
        esac
    done < <(grep -E '^\s*[a-z_]+\s*=' "$CONFIG_FILE" 2>/dev/null)
fi

# 检查
if [ ! -f "$LLAMA_DIR/build/bin/llama-server" ]; then
    echo "❌ llama-server 未找到: $LLAMA_DIR/build/bin/llama-server"
    echo "   运行: bash scripts/setup-dev.sh"
    exit 1
fi

if [ ! -f "$MODEL_FILE" ]; then
    echo "❌ 模型文件未找到: $MODEL_FILE"
    echo "   运行: bash scripts/setup-dev.sh"
    exit 1
fi

echo "🚀 启动模型服务"
echo "   模型: $(basename "$MODEL_FILE")"
echo "   地址: http://$HOST:$PORT"
echo "   上下文: $CONTEXT_SIZE tokens"
echo "   线程: $THREADS"
echo ""

exec "$LLAMA_DIR/build/bin/llama-server" \
    -m "$MODEL_FILE" \
    -c "$CONTEXT_SIZE" \
    -t "$THREADS" \
    --host "$HOST" \
    --port "$PORT" \
    --log-disable \
    "$@"
