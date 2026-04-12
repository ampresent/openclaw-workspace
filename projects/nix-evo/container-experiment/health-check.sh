#!/bin/bash
# health-check.sh — Verify the agent fixed OS defects
# Returns exit code 0 if all critical defects are fixed

PASS=0
FAIL=0
RESULTS="/root/.openclaw/workspace/health-check.log"

echo "=== Health Check: $(date -Iseconds) ===" > "$RESULTS"

check() {
    local name="$1"
    local cmd="$2"
    if eval "$cmd" 2>/dev/null; then
        echo "  ✅ PASS: $name" | tee -a "$RESULTS"
        PASS=$((PASS + 1))
    else
        echo "  ❌ FAIL: $name" | tee -a "$RESULTS"
        FAIL=$((FAIL + 1))
    fi
}

# Critical checks (must be fixed)
check "DNS functional" "grep -q 'nameserver' /etc/resolv.conf && ! grep -q '127.0.0.99' /etc/resolv.conf"
check "vim installed" "which vim"
check "bashrc clean" "! grep -q 'nonexistent_command' /root/.bashrc"
check "hostname readable" "test -r /etc/hostname"
check "hosts valid" "! grep -q 'THIS_IS_NOT_A_VALID_ENTRY' /etc/hosts"
check "no junk in /tmp" "! test -f /tmp/junk-padding-50mb.bin"
check "broken symlink removed" "! test -L /usr/local/bin/broken-tool"
check "locale valid" "! grep -q 'UTF-INVALID' /etc/environment"
check "ldconfig clean" "! test -f /etc/ld.so.conf.d/fake-lib.conf"
check "repairs.log exists" "test -f /root/.openclaw/workspace/repairs.log"

echo "" | tee -a "$RESULTS"
echo "=== Results: $PASS passed, $FAIL failed ===" | tee -a "$RESULTS"

if [ "$FAIL" -eq 0 ]; then
    echo "🎉 ALL DEFECTS FIXED — Self-healing SUCCESS" | tee -a "$RESULTS"
    exit 0
else
    echo "⚠️  $FAIL defects remain — Self-healing PARTIAL" | tee -a "$RESULTS"
    exit 1
fi
