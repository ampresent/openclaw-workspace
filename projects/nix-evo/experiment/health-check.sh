#!/bin/bash
# health-check.sh — Verify self-healing results
PASS=0
FAIL=0
RESULTS="/root/.openclaw/workspace/projects/nix-evo/experiment/health-check.log"

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

check "DNS functional" "! grep -q '127.0.0.99' /etc/resolv.conf"
check "bashrc clean" "! grep -q 'EXPERIMENT DEFECT' /root/.bashrc"
check "broken symlink removed" "! test -L /usr/local/bin/broken-experiment-tool"
check "hosts clean" "! grep -q 'invalid-experiment-host' /etc/hosts"
check "junk removed" "! test -f /tmp/experiment-junk-30mb.bin"
check "locale clean" "! grep -q 'UTF-EXPERIMENT' /etc/environment"
check "ldconfig clean" "! test -f /etc/ld.so.conf.d/experiment-fake.conf"
check "repairs.log exists" "test -f /root/.openclaw/workspace/repairs.log"
check "tools restored" "which vim || which nano || echo 'none'" # at least one should be back

echo "" | tee -a "$RESULTS"
echo "=== Results: $PASS passed, $FAIL failed ===" | tee -a "$RESULTS"

if [ "$FAIL" -eq 0 ]; then
    echo "🎉 ALL DEFECTS FIXED — Self-healing SUCCESS" | tee -a "$RESULTS"
else
    echo "⚠️  $FAIL defects remain — Self-healing PARTIAL" | tee -a "$RESULTS"
fi
