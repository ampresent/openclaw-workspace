#!/usr/bin/env bash
#
# demo.sh — nix-evo GCC 编译修复工作流演示
#
# 场景：NixOS 服务器上 legacy-network 模块因 GCC 14+ 的
#       -Wimplicit-function-declaration 升级为 error 而编译失败。
#       通过 nix-evo 诊断 → 定位 → 应用 GCC overlay → 验证 → 执行。
#
# 前置条件：
#   - mock_agent.py 在运行（本脚本会自动启动）
#   - curl, jq 可用
#
# 用法：
#   bash demo.sh          # 正常运行
#   bash demo.sh --record # 录制 asciinema

set -euo pipefail

MOCK_PORT=7890
AGENT_URL="http://127.0.0.1:${MOCK_PORT}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MOCK_PID=""

# 颜色
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

cleanup() {
    if [ -n "$MOCK_PID" ] && kill -0 "$MOCK_PID" 2>/dev/null; then
        kill "$MOCK_PID" 2>/dev/null
        wait "$MOCK_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

step() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}  步骤 $1：$2${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

pause() {
    echo ""
    echo -e "${YELLOW}按 Enter 继续...${NC}"
    read -r
}

header() {
    echo -e "${BOLD}$1${NC}"
}

info() {
    echo -e "${GREEN}  ✓ $1${NC}"
}

warn() {
    echo -e "${YELLOW}  ⚠ $1${NC}"
}

fail() {
    echo -e "${RED}  ✗ $1${NC}"
}

# ============================================================
# 开始演示
# ============================================================

echo ""
echo -e "${BOLD}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║    nix-evo · GCC 编译修复工作流演示                       ║${NC}"
echo -e "${BOLD}║                                                         ║${NC}"
echo -e "${BOLD}║    场景：legacy-network 因 GCC 14+ 编译失败              ║${NC}"
echo -e "${BOLD}║    目标：诊断 → 应用 GCC overlay → 验证 → 修复           ║${NC}"
echo -e "${BOLD}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""

# ── 启动模拟服务器 ──────────────────────────────────────────

step "0" "启动 nix-evo-agent（模拟模式）"

info "在端口 ${MOCK_PORT} 启动模拟 agent 服务"
python3 "${SCRIPT_DIR}/mock_agent.py" $MOCK_PORT &
MOCK_PID=$!
sleep 1

# 验证 agent 是否运行
if curl -sf "${AGENT_URL}/health" > /dev/null 2>&1; then
    info "Agent 已就绪"
    curl -s "${AGENT_URL}/health" | jq .
else
    fail "Agent 启动失败"
    exit 1
fi

pause

# ── 第 1 步：检查系统状态 ──────────────────────────────────

step "1" "system_snapshot — 检查服务器全局状态"

info "调用 GET /api/snapshot 获取系统快照"
info "发现：legacy-network-build.service 状态为 failed"

curl -s "${AGENT_URL}/api/snapshot" | jq '{
    hostname,
    nixos_version,
    services: [.services[] | select(.active == "failed") | {name, active, sub}],
    recent_failures: [.recent_failures[] | {unit, since}]
}'

echo ""
header "最近失败服务的日志摘要："
curl -s "${AGENT_URL}/api/snapshot" | jq -r '.recent_failures[0].log_excerpt'

pause

# ── 第 2 步：查看详细日志 ──────────────────────────────────

step "2" "service_logs — 深入查看构建失败日志"

info "调用 GET /api/logs?unit=legacy-network-build.service"
info "定位到错误：implicit declaration of function 'memcpy'"

curl -s "${AGENT_URL}/api/logs?unit=legacy-network-build.service&lines=20" | jq -r '.logs[]'

echo ""
fail "关键错误：legacy_network.c:42:5 — memcpy 未声明（缺少 <string.h>）"
warn "GCC 14+ 将 -Wimplicit-function-declaration 默认升级为 error"

pause

# ── 第 3 步：读取配置 ──────────────────────────────────────

step "3" "config_read — 读取 NixOS 配置"

info "调用 GET /api/config 读取 configuration.nix"

curl -s "${AGENT_URL}/api/config" | jq -r '.content'

pause

# ── 第 4 步：展示问题代码 ──────────────────────────────────

step "4" "查看问题源码 — legacy_network.c"

info "在 configuration.nix 中找到 legacy-network 构建脚本"
info "源码位置：/opt/legacy-network/legacy_network.c"

echo ""
header "问题代码（第 42 行）："
echo ""
grep -n "memcpy" "${SCRIPT_DIR}/legacy_network.c" | head -5
echo ""
warn "根因：源码遗漏了 #include <string.h>"
warn "memcpy() 被当作隐式声明的函数"

pause

# ── 第 5 步：初始验证失败 ──────────────────────────────────

step "5" "config_validate（修复前）— 验证当前配置"

info "调用 POST /api/config/validate — 用当前配置做 dry-build"

curl -s -X POST "${AGENT_URL}/api/config/validate" \
    -H "Content-Type: application/json" \
    -d '{"config": "{ config, pkgs, ... }: { services.nginx.enable = true; }"}' | jq .

fail "dry-build 失败！legacy-network 仍然无法编译"
warn "直接重新构建无法解决问题，需要修改 GCC 编译行为"

pause

# ── 第 6 步：应用 GCC overlay 修复 ─────────────────────────

step "6" "config_validate（修复后）— 应用 GCC 容错 overlay"

header "修复方案：为 legacy-network 包定制 GCC，将 -Werror=implicit-function-declaration 降级为 warning"
echo ""
header "NixOS 配置变更（将注入 configuration.nix）："
echo ""
cat "${SCRIPT_DIR}/nixos-gcc-tolerance-overlay.nix"
echo ""

info "调用 POST /api/config/validate 验证修复后的配置"

# 发送包含 GCC overlay 的修复配置
python3 -c "
import json
with open('${SCRIPT_DIR}/legacy-network-fixed.nix') as f:
    config = f.read()
print(json.dumps({'config': config}))
" > /tmp/nix-evo-validate-req.json

curl -s -X POST "${AGENT_URL}/api/config/validate" \
    -H "Content-Type: application/json" \
    -d @/tmp/nix-evo-validate-req.json | jq .

info "dry-build 通过！风险评估：safe"

pause

# ── 第 7 步：执行配置变更 ──────────────────────────────────

step "7" "config_apply — 应用配置变更"

info "调用 POST /api/config/apply — 执行 nixos-rebuild switch"

curl -s -X POST "${AGENT_URL}/api/config/apply" \
    -H "Content-Type: application/json" \
    -d '{"message": "GCC overlay: 容忍 legacy-network implicit-function-declaration"}' \
    | jq .

info "配置已生效！generation 43"
info "legacy-network 使用容错 GCC 编译成功"

pause

# ── 第 8 步：验证结果 ──────────────────────────────────────

step "8" "验证修复 — 再次检查系统状态"

info "调用 GET /api/generations 确认新 generation"

curl -s "${AGENT_URL}/api/generations" | jq .

echo ""
info "✅ 修复完成！流程总结："
echo ""
header "  1. system_snapshot   → 发现 legacy-network-build.service failed"
header "  2. service_logs      → 定位到 implicit declaration of memcpy"
header "  3. config_read       → 确认构建脚本使用系统 GCC"
header "  4. config_validate   → dry-build 确认问题存在"
header "  5. 应用 GCC overlay  → 降级 -Werror=implicit-function-declaration"
header "  6. config_validate   → dry-build 通过，风险评估 safe"
header "  7. config_apply      → generation 43，配置生效"
header "  8. 回滚保障          → 随时可用 nixos-rebuild switch --rollback"

echo ""
echo -e "${GREEN}${BOLD}演示结束。${NC}"
echo ""
