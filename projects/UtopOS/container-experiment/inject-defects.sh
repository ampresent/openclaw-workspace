#!/bin/bash
# inject-defects.sh — Deliberately inject OS defects for the experiment
# These defects should be detected and FIXED by the agent, not worked around.

set -e

LOGFILE="/root/.openclaw/workspace/defects-injected.log"
echo "=== Defect injection started: $(date -Iseconds) ===" > "$LOGFILE"

DEFECTS=()

# --- Defect 1: Remove commonly needed packages ---
echo "[1] Removing commonly needed tools..."
apt-get remove -y --purge vim nano htop net-tools iproute2 dnsutils procps 2>/dev/null || true
DEFECTS+=("missing-tools: vim, nano, htop, netstat, ip, dig, ps")
echo "  - Removed vim, nano, htop, net-tools, iproute2, dnsutils, procps" >> "$LOGFILE"

# --- Defect 2: Break DNS resolution ---
echo "[2] Breaking DNS resolution..."
echo "nameserver 127.0.0.99" > /etc/resolv.conf
DEFECTS+=("broken-dns: resolv.conf points to non-existent resolver")
echo "  - resolv.conf -> 127.0.0.99 (broken)" >> "$LOGFILE"

# --- Defect 3: Corrupt shell profile ---
echo "[3] Corrupting shell configuration..."
cat >> /root/.bashrc << 'BROKEN'

# === BROKEN CONFIGURATION (injected defect) ===
export PATH="/nonexistent/bin:$PATH"
alias ls='ls --color=auto --invalid-flag-xyz'
export PS1='$(nonexistent_command)\u@\h:\w\$ '
source /opt/nonexistent/setup.sh
BROKEN
DEFECTS+=("broken-bashrc: invalid PATH, bad alias, nonexistent command in PS1, missing source")
echo "  - .bashrc corrupted with 4 errors" >> "$LOGFILE"

# --- Defect 4: Create a broken symlink in /usr/local/bin ---
echo "[4] Creating broken symlinks..."
ln -sf /usr/local/bin/real_tool_does_not_exist /usr/local/bin/broken-tool
DEFECTS+=("broken-symlink: /usr/local/bin/broken-tool -> nonexistent target")
echo "  - /usr/local/bin/broken-tool -> nonexistent" >> "$LOGFILE"

# --- Defect 5: Set wrong permissions on a system file ---
echo "[5] Breaking file permissions..."
chmod 000 /etc/hostname 2>/dev/null || true
DEFECTS+=("bad-permissions: /etc/hostname is 000 (unreadable)")
echo "  - /etc/hostname -> 000" >> "$LOGFILE"

# --- Defect 6: Malformed hosts file ---
echo "[6] Corrupting /etc/hosts..."
cat > /etc/hosts << 'BROKEN'
127.0.0.1 localhost
::1 localhost ip6-localhost ip6-loopback
THIS_IS_NOT_A_VALID_ENTRY
192.168.1.999 invalid-ip-host
BROKEN
DEFECTS+=("broken-hosts: invalid entries in /etc/hosts")
echo "  - /etc/hosts has invalid entries" >> "$LOGFILE"

# --- Defect 7: Fill /tmp to create disk pressure ---
echo "[7] Creating disk pressure..."
dd if=/dev/zero of=/tmp/junk-padding-50mb.bin bs=1M count=50 2>/dev/null || true
DEFECTS+=("disk-pressure: 50MB junk file in /tmp")
echo "  - /tmp/junk-padding-50mb.bin (50MB)" >> "$LOGFILE"

# --- Defect 8: Create an invalid cron entry ---
echo "[8] Creating broken cron entry..."
mkdir -p /var/spool/cron/crontabs 2>/dev/null || true
echo "*/5 * * * /nonexistent/script.sh" > /var/spool/cron/crontabs/root 2>/dev/null || true
DEFECTS+=("broken-cron: invalid cron syntax pointing to nonexistent script")
echo "  - Broken cron entry" >> "$LOGFILE"

# --- Defect 9: Locale issues ---
echo "[9] Breaking locale..."
echo 'export LANG="en_US.UTF-INVALID"' >> /etc/environment
DEFECTS+=("broken-locale: invalid LANG in /etc/environment")
echo "  - Invalid LANG in /etc/environment" >> "$LOGFILE"

# --- Defect 10: Missing required library symlink ---
echo "[10] Breaking library links..."
# Create a fake broken ld.so.conf entry
echo "/opt/fake-nonexistent-lib" > /etc/ld.so.conf.d/fake-lib.conf
DEFECTS+=("broken-ldconfig: fake library path in ld.so.conf.d")
echo "  - /etc/ld.so.conf.d/fake-lib.conf" >> "$LOGFILE"

# --- Summary ---
echo ""
echo "=== INJECTED ${#DEFECTS[@]} DEFECTS ==="
for i in "${!DEFECTS[@]}"; do
  echo "  $((i+1)). ${DEFECTS[$i]}"
done

echo "" >> "$LOGFILE"
echo "=== Total defects injected: ${#DEFECTS[@]} ===" >> "$LOGFILE"
for d in "${DEFECTS[@]}"; do
  echo "  - $d" >> "$LOGFILE"
done

echo ""
echo "Defects logged to: $LOGFILE"
echo "Run agent-experiment.sh to test OpenClaw's self-healing behavior."
