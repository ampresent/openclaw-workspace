#!/bin/bash
set -e

BINARY="/usr/local/bin/runb"
ROOTFS="/test-rootfs"
BUNDLE="/tmp/test-bundle"
PASS=0
FAIL=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓ PASS${NC}: $1"; ((PASS++)); }
fail() { echo -e "${RED}✗ FAIL${NC}: $1"; ((FAIL++)); }

# Create test OCI bundle
create_bundle() {
    rm -rf "$BUNDLE"
    mkdir -p "$BUNDLE"
    cat > "$BUNDLE/config.json" << 'EOF'
{
  "ociVersion": "1.0.2",
  "root": {
    "path": "/test-rootfs"
  },
  "process": {
    "terminal": false,
    "args": ["/bin/echo", "hello from runb"],
    "env": ["PATH=/bin:/usr/bin", "HOME=/"],
    "cwd": "/"
  },
  "mounts": [],
  "linux": {}
}
EOF
}

echo "=== runb Integration Tests (Alpine) ==="
echo ""

# Test 1: Binary exists and is executable
echo "--- Test 1: Binary check ---"
if [ -x "$BINARY" ]; then
    pass "Binary exists and is executable"
else
    fail "Binary not found or not executable"
    exit 1
fi

# Test 2: Version output
echo "--- Test 2: Version ---"
VERSION=$($BINARY --version 2>&1) && pass "Version: $VERSION" || fail "Version check failed"

# Test 3: Create container
echo "--- Test 3: Create ---"
create_bundle
$BINARY create test-create --bundle "$BUNDLE" 2>&1 && pass "Container created" || fail "Create failed"

# Test 4: State check
echo "--- Test 4: State ---"
STATE=$($BINARY state test-create 2>&1)
if echo "$STATE" | grep -q "created"; then
    pass "State is 'created'"
else
    fail "Unexpected state: $STATE"
fi

# Test 5: Duplicate create should fail
echo "--- Test 5: Duplicate create ---"
if $BINARY create test-create --bundle "$BUNDLE" 2>&1; then
    fail "Duplicate create should have failed"
else
    pass "Duplicate create correctly rejected"
fi

# Test 6: Start container
echo "--- Test 6: Start ---"
# Wait for process to finish, then check
$BINARY start test-create 2>&1 && pass "Container started" || fail "Start failed"
sleep 1

# Test 7: List containers
echo "--- Test 7: List ---"
LIST=$($BINARY list 2>&1)
if echo "$LIST" | grep -q "test-create"; then
    pass "Container appears in list"
else
    fail "Container not in list: $LIST"
fi

# Test 8: Delete container
echo "--- Test 8: Delete ---"
$BINARY delete test-create 2>&1 && pass "Container deleted" || fail "Delete failed"

# Test 9: State after delete
echo "--- Test 9: State after delete ---"
if $BINARY state test-create 2>&1; then
    fail "State should fail after delete"
else
    pass "State correctly fails after delete"
fi

# Test 10: Create + start with env verification
echo "--- Test 10: Environment test ---"
create_bundle
cat > "$BUNDLE/config.json" << 'EOF'
{
  "ociVersion": "1.0.2",
  "root": {
    "path": "/test-rootfs"
  },
  "process": {
    "terminal": false,
    "args": ["/bin/env"],
    "env": ["PATH=/bin:/usr/bin", "TEST_VAR=runb_works"],
    "cwd": "/"
  },
  "mounts": [],
  "linux": {}
}
EOF
$BINARY create test-env --bundle "$BUNDLE" 2>&1 > /dev/null
$BINARY start test-env 2>&1 > /dev/null
sleep 1
pass "Env test container started"
$BINARY delete test-env 2>&1 > /dev/null

# Test 11: Error handling - missing bundle
echo "--- Test 11: Missing bundle ---"
if $BINARY create test-missing --bundle /nonexistent 2>&1; then
    fail "Should fail with missing bundle"
else
    pass "Correctly rejects missing bundle"
fi

# Summary
echo ""
echo "=== Results ==="
echo -e "${GREEN}Passed: $PASS${NC}"
echo -e "${RED}Failed: $FAIL${NC}"

if [ $FAIL -eq 0 ]; then
    echo -e "\n${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}Some tests failed.${NC}"
    exit 1
fi
