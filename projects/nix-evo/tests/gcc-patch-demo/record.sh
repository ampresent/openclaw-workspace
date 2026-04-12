#!/usr/bin/env bash
#
# record.sh — 使用 asciinema 录制 nix-evo GCC 修复演示
#
# 用法：
#   bash record.sh              # 录制到本地文件
#   bash record.sh --upload     # 录制并上传到 asciinema.org

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RECORD_DIR="${SCRIPT_DIR}/recordings"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RECORD_FILE="${RECORD_DIR}/gcc-patch-demo-${TIMESTAMP}.cast"

mkdir -p "$RECORD_DIR"

echo ""
echo "═══════════════════════════════════════════════════"
echo "  nix-evo GCC 修复演示 — asciinema 录制"
echo "═══════════════════════════════════════════════════"
echo ""
echo "录制文件：${RECORD_FILE}"
echo ""

if [ "${1:-}" = "--upload" ]; then
    echo "录制并上传到 asciinema.org..."
    asciinema rec --title "nix-evo: GCC 编译修复工作流 (implicit-function-declaration)" \
                  --command "bash ${SCRIPT_DIR}/demo-auto.sh" \
                  --idle-time-limit 5
else
    echo "录制到本地文件..."
    asciinema rec "$RECORD_FILE" \
                  --title "nix-evo: GCC 编译修复工作流 (implicit-function-declaration)" \
                  --command "bash ${SCRIPT_DIR}/demo-auto.sh" \
                  --idle-time-limit 5
fi

echo ""
echo "═══════════════════════════════════════════════════"
echo "  录制完成"
echo "═══════════════════════════════════════════════════"
echo ""

if [ "${1:-}" != "--upload" ]; then
    echo "录制文件：${RECORD_FILE}"
    echo ""
    echo "播放：  asciinema play ${RECORD_FILE}"
    echo "上传：  asciinema upload ${RECORD_FILE}"
    echo ""
    echo "也可以拖动到 https://asciinema.org 手动上传"
    echo ""
    echo "文件大小：$(du -h "$RECORD_FILE" | cut -f1)"
fi
