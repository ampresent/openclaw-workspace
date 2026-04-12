#!/usr/bin/env bash
#
# demo-auto.sh — UtopOS GCC 编译修复演示（自动播放版，用于 asciinema 录制）

set -euo pipefail

MOCK_PORT=7890
AGENT_URL="http://127.0.0.1:${MOCK_PORT}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MOCK_PID=""

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
    sleep 1
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

header() {
    echo -e "${BOLD}$1${NC}"
}

# ============================================================

echo ""
echo -e "${BOLD}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║    UtopOS · GCC 编译修复工作流演示                       ║${NC}"
echo -e "${BOLD}║                                                         ║${NC}"
echo -e "${BOLD}║    场景：legacy-network 因 GCC 14+ 编译失败              ║${NC}"
echo -e "${BOLD}║    目标：诊断 → 应用 GCC overlay → 验证 → 修复           ║${NC}"
echo -e "${BOLD}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""
sleep 2

# ── 步骤 0：启动 agent ─────────────────────────────────────
step "0" "启动 UtopOS-agent（模拟模式）"

# 确保端口可用
fuser -k ${MOCK_PORT}/tcp 2>/dev/null || true
sleep 0.5

info "在端口 ${MOCK_PORT} 启动模拟 agent 服务"
python3 "${SCRIPT_DIR}/mock_agent.py" $MOCK_PORT &
MOCK_PID=$!
sleep 1

curl -sf "${AGENT_URL}/health" > /dev/null 2>&1 && info "Agent 已就绪" || fail "启动失败"
curl -s "${AGENT_URL}/health" | jq .
sleep 2

# ── 步骤 1：系统快照 ───────────────────────────────────────
step "1" "system_snapshot — 检查服务器全局状态"
info "调用 GET /api/snapshot 获取系统快照"
info "发现：legacy-network-build.service 状态为 failed"
echo ""
curl -s "${AGENT_URL}/api/snapshot" | jq '{
    hostname,
    nixos_version,
    services: [.services[] | select(.active == "failed") | {name, active, sub}],
    recent_failures: [.recent_failures[] | {unit, since}]
}'
echo ""
header "最近失败日志摘要："
curl -s "${AGENT_URL}/api/snapshot" | jq -r '.recent_failures[0].log_excerpt'
sleep 3

# ── 步骤 2：详细日志 ───────────────────────────────────────
step "2" "service_logs — 深入查看构建失败日志"
info "调用 GET /api/logs?unit=legacy-network-build.service"
info "定位到错误：implicit declaration of function 'memcpy'"
echo ""
curl -s "${AGENT_URL}/api/logs?unit=legacy-network-build.service&lines=20" | jq -r '.logs[]'
echo ""
fail "关键错误：legacy_network.c:42:5 — memcpy 未声明（缺少 <string.h>）"
warn "GCC 14+ 将 -Wimplicit-function-declaration 默认升级为 error"
sleep 3

# ── 步骤 3：读取配置 ───────────────────────────────────────
step "3" "config_read — 读取 NixOS 配置"
info "调用 GET /api/config 读取 configuration.nix"
echo ""
curl -s "${AGENT_URL}/api/config" | jq -r '.content'
sleep 3

# ── 步骤 4：问题源码 ───────────────────────────────────────
step "4" "查看问题源码 — legacy_network.c"
info "在 configuration.nix 中找到 legacy-network 构建配置"
info "源码位置：/opt/legacy-network/legacy_network.c"
echo ""
header "问题代码（第 42 行）："
echo ""
grep -n "memcpy" "${SCRIPT_DIR}/legacy_network.c" | head -5
echo ""
warn "根因：源码遗漏了 #include <string.h>"
warn "memcpy() 被当作隐式声明的函数"
sleep 3

# ── 步骤 5：验证失败 ───────────────────────────────────────
step "5" "config_validate（修复前）— 验证当前配置"
info "调用 POST /api/config/validate — 用当前配置做 dry-build"
echo ""
curl -s -X POST "${AGENT_URL}/api/config/validate" \
    -H "Content-Type: application/json" \
    -d '{"config": "{ config, pkgs, ... }: { services.nginx.enable = true; }"}' | jq .
echo ""
fail "dry-build 失败！legacy-network 仍然无法编译"
warn "需要修改 GCC 编译行为才能解决问题"
sleep 3

# ── 步骤 6：应用 GCC overlay ──────────────────────────────
step "6" "config_validate（修复后）— 应用 GCC 容错 overlay"
header "修复方案：为 legacy-network 定制 GCC，将 implicit-function-declaration 降级为 warning"
echo ""
header "NixOS 配置变更："
echo ""
cat "${SCRIPT_DIR}/nixos-gcc-tolerance-overlay.nix"
echo ""
sleep 2

info "调用 POST /api/config/validate 验证修复后的配置"
python3 -c "
import json
with open('${SCRIPT_DIR}/legacy-network-fixed.nix') as f:
    config = f.read()
print(json.dumps({'config': config}))
" > /tmp/UtopOS-validate-req.json
echo ""
curl -s -X POST "${AGENT_URL}/api/config/validate" \
    -H "Content-Type: application/json" \
    -d @/tmp/UtopOS-validate-req.json | jq .
echo ""
info "dry-build 通过！风险评估：safe"
sleep 3

# ── 步骤 7：执行配置变更 ───────────────────────────────────
step "7" "config_apply — 应用配置变更"
info "调用 POST /api/config/apply — 执行 nixos-rebuild switch"
echo ""
curl -s -X POST "${AGENT_URL}/api/config/apply" \
    -H "Content-Type: application/json" \
    -d '{"message": "GCC overlay: 容忍 legacy-network implicit-function-declaration"}' | jq .
echo ""
info "配置已生效！generation 43"
info "legacy-network 使用容错 GCC 编译成功"
sleep 2

# ── 步骤 8：验证结果 ───────────────────────────────────────
step "8" "验证修复 — 再次检查系统状态"
info "调用 GET /api/generations 确认新 generation"
echo ""
curl -s "${AGENT_URL}/api/generations" | jq .
echo ""
info "✅ 修复完成！流程总结："
echo ""
header "  1. system_snapshot   → 发现 legacy-network-build.service failed"
header "  2. service_logs      → 定位到 implicit declaration of memcpy"
header "  3. config_read       → 确认构建脚本使用系统 GCC"
header "  4. config_validate   → dry-build 确认问题存在"
header "  5. GCC overlay       → 降级 -Werror=implicit-function-declaration"
header "  6. config_validate   → dry-build 通过，风险评估 safe"
header "  7. config_apply      → generation 43，配置生效"
header "  8. 回滚保障          → nixos-rebuild switch --rollback"
echo ""
echo -e "${GREEN}${BOLD}演示结束。${NC}"
sleep 2
