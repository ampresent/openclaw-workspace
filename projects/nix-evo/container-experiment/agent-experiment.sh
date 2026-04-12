#!/bin/bash
# agent-experiment.sh — Run the nix-evo self-healing experiment
# This script:
# 1. Starts OpenClaw gateway
# 2. Sends a prompt that triggers the nix-evo skill
# 3. Verifies the agent detects and fixes OS defects
# 4. Records results

set -e

WORKSPACE="/root/.openclaw/workspace"
RESULTS="$WORKSPACE/experiment-results.log"
DEFECTS_LOG="$WORKSPACE/defects-injected.log"

echo "=== nix-evo Container Experiment ===" | tee "$RESULTS"
echo "Started: $(date -Iseconds)" | tee -a "$RESULTS"
echo "" | tee -a "$RESULTS"

# Phase 1: Verify defects are present
echo "--- Phase 1: Verifying injected defects ---" | tee -a "$RESULTS"

DEFECT_CHECKS=0
DEFECTS_FOUND=0

check_defect() {
    local name="$1"
    local cmd="$2"
    DEFECT_CHECKS=$((DEFECT_CHECKS + 1))
    if eval "$cmd" 2>/dev/null; then
        echo "  ✅ DEFECT ACTIVE: $name" | tee -a "$RESULTS"
        DEFECTS_FOUND=$((DEFECTS_FOUND + 1))
    else
        echo "  ⚠️  Defect not detected: $name" | tee -a "$RESULTS"
    fi
}

check_defect "DNS broken" "grep -q '127.0.0.99' /etc/resolv.conf"
check_defect "vim missing" "! which vim"
check_defect "bashrc corrupted" "grep -q 'nonexistent_command' /root/.bashrc"
check_defect "hostname unreadable" "! test -r /etc/hostname"
check_defect "/tmp junk" "test -f /tmp/junk-padding-50mb.bin"
check_defect "broken symlink" "test -L /usr/local/bin/broken-tool && ! test -e /usr/local/bin/broken-tool"
check_defect "invalid hosts" "grep -q 'THIS_IS_NOT_A_VALID_ENTRY' /etc/hosts"
check_defect "broken locale" "grep -q 'UTF-INVALID' /etc/environment"
check_defect "fake ldconfig" "test -f /etc/ld.so.conf.d/fake-lib.conf"
check_defect "broken cron" "test -f /var/spool/cron/crontabs/root"

echo "" | tee -a "$RESULTS"
echo "Defects active: $DEFECTS_FOUND / $DEFECT_CHECKS" | tee -a "$RESULTS"

# Phase 2: Start OpenClaw
echo "" | tee -a "$RESULTS"
echo "--- Phase 2: Starting OpenClaw gateway ---" | tee -a "$RESULTS"

# Start the gateway in background
openclaw gateway start &
GATEWAY_PID=$!
sleep 5

# Phase 3: Trigger the nix-evo skill via session
echo "--- Phase 3: Triggering agent self-healing ---" | tee -a "$RESULTS"
echo "Sending prompt: 'Read skills/nix-evo/SKILL.md. Execute the full system scan and fix ALL issues you find.'" | tee -a "$RESULTS"

# The experiment prompt — this should trigger the nix-evo skill
cat > /tmp/experiment-prompt.txt << 'PROMPT'
你是一个运行在容器中的 OpenClaw agent。

请读取 skills/nix-evo/SKILL.md 并严格按照其中的协议执行：

1. 执行完整的系统快照 (sys_scan)
2. 执行包管理器健康检查 (pkg_health)
3. 执行服务状态检查 (svc_check)
4. 执行配置完整性检查 (config_check)

对于发现的每一个问题：
- 不要绕过，不要用 workaround
- 直接修复根因
- 验证修复成功
- 将修复记录写入 /root/.openclaw/workspace/repairs.log

目标：让系统恢复到健康状态。
PROMPT

echo "" | tee -a "$RESULTS"
echo "Experiment prompt saved to /tmp/experiment-prompt.txt" | tee -a "$RESULTS"
echo "Agent will be triggered via OpenClaw sessions." | tee -a "$RESULTS"
echo "" | tee -a "$RESULTS"
echo "=== Phase 3 ready. Awaiting agent interaction. ===" | tee -a "$RESULTS"
echo "GATEWAY_PID=$GATEWAY_PID" >> "$RESULTS"
