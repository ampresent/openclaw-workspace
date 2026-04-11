#!/usr/bin/env bash
# setup-dev.sh — 开发环境搭建
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
LLAMA_DIR="$PROJECT_DIR/third_party/llama.cpp"
MODELS_DIR="$PROJECT_DIR/models"

echo "=== Rescue System 开发环境搭建 ==="

# 1. 克隆并编译 llama.cpp
if [ ! -d "$LLAMA_DIR" ]; then
    echo "[1/3] 克隆 llama.cpp..."
    git clone --depth 1 https://github.com/ggerganov/llama.cpp.git "$LLAMA_DIR"
else
    echo "[1/3] llama.cpp 已存在，跳过"
fi

echo "[2/3] 编译 llama.cpp (CPU only, 多线程优化)..."
cd "$LLAMA_DIR"
cmake -B build -DLLAMA_NATIVE=ON -DLLAMA_BLAS=ON -DLLAMA_BLAS_VENDOR=OpenBLAS 2>/dev/null || \
cmake -B build -DLLAMA_NATIVE=ON
cmake --build build -j"$(nproc)"

# 2. 下载模型
echo "[3/3] 下载千问模型..."
mkdir -p "$MODELS_DIR"

# Qwen2.5-7B GGUF 量化版本
MODEL_FILE="$MODELS_DIR/qwen2.5-7b-instruct-q4_k_m.gguf"
if [ ! -f "$MODEL_FILE" ]; then
    echo "下载 Qwen2.5-7B-Instruct Q4_K_M (ModelScope 镜像)..."
    curl -L --connect-timeout 30 --max-time 1200 -o "$MODEL_FILE" \
        "https://modelscope.cn/models/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/master/qwen2.5-7b-instruct-q4_k_m.gguf"
else
    echo "模型已存在: $MODEL_FILE"
fi

echo ""
echo "=== 搭建完成 ==="
echo "模型路径: $MODEL_FILE"
echo "llama.cpp: $LLAMA_DIR"
echo ""
echo "启动模型服务:"
echo "  $LLAMA_DIR/build/bin/llama-server -m $MODEL_FILE -c 4096 -t $(nproc) --host 127.0.0.1 --port 8081"
