#!/usr/bin/env bash
#
# nix-evo-agent integration test suite
#
# Run against a live agent instance to verify all endpoints work.
#
# Usage:
#   ./integration_test.sh [BASE_URL]
#
# Example:
#   ./integration_test.sh http://127.0.0.1:7890

set -euo pipefail

BASE_URL="${1:-http://127.0.0.1:7890}"
PASS=0
FAIL=0
TOTAL=0

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

assert_status() {
    local desc="$1"
    local actual="$2"
    local expected="$3"
    TOTAL=$((TOTAL + 1))

    if [ "$actual" = "$expected" ]; then
        echo -e "  ${GREEN}✓${NC} $desc (HTTP $actual)"
        PASS=$((PASS + 1))
    else
        echo -e "  ${RED}✗${NC} $desc (expected $expected, got $actual)"
        FAIL=$((FAIL + 1))
    fi
}

assert_contains() {
    local desc="$1"
    local body="$2"
    local pattern="$3"
    TOTAL=$((TOTAL + 1))

    if echo "$body" | grep -q "$pattern"; then
        echo -e "  ${GREEN}✓${NC} $desc"
        PASS=$((PASS + 1))
    else
        echo -e "  ${RED}✗${NC} $desc (missing: $pattern)"
        FAIL=$((FAIL + 1))
    fi
}

assert_json_field() {
    local desc="$1"
    local body="$2"
    local field="$3"
    TOTAL=$((TOTAL + 1))

    if echo "$body" | python3 -c "import sys,json; d=json.load(sys.stdin); assert '$field' in d" 2>/dev/null; then
        echo -e "  ${GREEN}✓${NC} $desc"
        PASS=$((PASS + 1))
    else
        echo -e "  ${RED}✗${NC} $desc (missing JSON field: $field)"
        FAIL=$((FAIL + 1))
    fi
}

echo ""
echo -e "${YELLOW}╔══════════════════════════════════════════════╗${NC}"
echo -e "${YELLOW}║  nix-evo-agent Integration Tests             ║${NC}"
echo -e "${YELLOW}║  Target: $BASE_URL"
echo -e "${YELLOW}╚══════════════════════════════════════════════╝${NC}"
echo ""

# ─── Health check ─────────────────────────────────────────────────────

echo "Health Check"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/health")
BODY=$(echo "$RESPONSE" | head -n1)
STATUS=$(echo "$RESPONSE" | tail -n1)
assert_status "GET /health" "$STATUS" "200"
assert_contains "Health returns ok" "$BODY" "ok"
echo ""

# ─── Audit endpoints ─────────────────────────────────────────────────

echo "Audit Trail"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/api/audit?limit=10")
BODY=$(echo "$RESPONSE" | head -n1)
STATUS=$(echo "$RESPONSE" | tail -n1)
assert_status "GET /api/audit" "$STATUS" "200"
assert_json_field "Audit has total field" "$BODY" "total"
assert_json_field "Audit has entries field" "$BODY" "entries"
assert_json_field "Audit has log_path field" "$BODY" "log_path"

RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/api/audit/stats")
BODY=$(echo "$RESPONSE" | head -n1)
STATUS=$(echo "$RESPONSE" | tail -n1)
assert_status "GET /api/audit/stats" "$STATUS" "200"
assert_json_field "Stats has action_counts" "$BODY" "action_counts"
echo ""

# ─── Healer status ───────────────────────────────────────────────────

echo "Self-Healer"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/api/healer/status")
BODY=$(echo "$RESPONSE" | head -n1)
STATUS=$(echo "$RESPONSE" | tail -n1)
assert_status "GET /api/healer/status" "$STATUS" "200"
assert_json_field "Healer has running" "$BODY" "running"
assert_json_field "Healer has rules" "$BODY" "rules"
assert_json_field "Healer has service_states" "$BODY" "service_states"
echo ""

# ─── Flake converter ────────────────────────────────────────────────

echo "Flake Converter"
FLAKE_BODY='{"config_content":"{ networking.hostName = \"testserver\"; services.nginx.enable = true; }","hostname":"testserver","channel":"nixos-24.05"}'
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST -H "Content-Type: application/json" -d "$FLAKE_BODY" "$BASE_URL/api/flake/convert")
BODY=$(echo "$RESPONSE" | head -n1)
STATUS=$(echo "$RESPONSE" | tail -n1)
assert_status "POST /api/flake/convert" "$STATUS" "200"
assert_json_field "Flake has flake_nix" "$BODY" "flake_nix"
assert_json_field "Flake has detected_channel" "$BODY" "detected_channel"
assert_json_field "Flake has detected_hostname" "$BODY" "detected_hostname"
assert_contains "Flake contains nixosConfigurations" "$BODY" "nixosConfigurations"
assert_contains "Flake contains testserver" "$BODY" "testserver"
echo ""

# ─── Config Diff ────────────────────────────────────────────────────

echo "Config Diff"
DIFF_BODY='{"config_a":"{ services.nginx.enable = true; }","config_b":"{ services.nginx.enable = true; services.redis.enable = true; }"}'
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST -H "Content-Type: application/json" -d "$DIFF_BODY" "$BASE_URL/api/config/diff")
BODY=$(echo "$RESPONSE" | head -n1)
STATUS=$(echo "$RESPONSE" | tail -n1)
assert_status "POST /api/config/diff" "$STATUS" "200"
assert_json_field "Diff has unified_diff" "$BODY" "unified_diff"
assert_json_field "Diff has structured" "$BODY" "structured"
assert_json_field "Diff has risk_assessment" "$BODY" "risk_assessment"
assert_contains "Diff detects redis addition" "$BODY" "redis"
echo ""

# ─── Service Dependency Graph ───────────────────────────────────────

echo "Dependency Graph"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/api/deps?focus=sshd.service&depth=2")
BODY=$(echo "$RESPONSE" | head -n1)
STATUS=$(echo "$RESPONSE" | tail -n1)
# 200 or 500 is acceptable (500 if sshd isn't available in test env)
assert_status "GET /api/deps" "$STATUS" "200"
echo ""

# ─── 404 for unknown routes ─────────────────────────────────────────

echo "Error Handling"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/api/nonexistent")
STATUS=$(echo "$RESPONSE" | tail -n1)
assert_status "Unknown route returns 404" "$STATUS" "404"
echo ""

# ─── Summary ────────────────────────────────────────────────────────

echo -e "${YELLOW}════════════════════════════════════════════════${NC}"
echo -e "  Total: $TOTAL  ${GREEN}Pass: $PASS${NC}  ${RED}Fail: $FAIL${NC}"
echo -e "${YELLOW}════════════════════════════════════════════════${NC}"
echo ""

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}All tests passed! ✨${NC}"
    exit 0
else
    echo -e "${RED}$FAIL test(s) failed${NC}"
    exit 1
fi
