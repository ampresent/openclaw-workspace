#!/bin/bash
# inject-defects.sh — Inject recoverable OS defects for the experiment
# All defects are logged and have corresponding fix commands
# Designed for host-level testing (non-destructive)

set -e

EXPERIMENT_DIR="/root/.openclaw/workspace/projects/nix-evo/experiment"
LOGFILE="$EXPERIMENT_DIR/defects-injected.log"
FIXSCRIPT="$EXPERIMENT_DIR/fix-all.sh"

echo "=== Defect injection started: $(date -Iseconds) ===" > "$LOGFILE"
echo "#!/bin/bash" > "$FIXSCRIPT"
echo "# Auto-generated fix script" >> "$FIXSCRIPT"
echo "set -e" >> "$FIXSCRIPT"
echo "" >> "$FIXSCRIPT"

DEFECTS=()

# --- Defect 1: Break resolv.conf (BACKUP first) ---
echo "[1] Breaking DNS configuration..."
cp /etc/resolv.conf /etc/resolv.conf.pre-experiment 2>/dev/null || true
echo "nameserver 127.0.0.99" > /etc/resolv.conf
DEFECTS+=("broken-dns: resolv.conf points to 127.0.0.99")
echo "  - resolv.conf -> 127.0.0.99 (broken)" >> "$LOGFILE"
echo "cp /etc/resolv.conf.pre-experiment /etc/resolv.conf" >> "$FIXSCRIPT"

# --- Defect 2: Corrupt .bashrc (APPEND, don't replace) ---
echo "[2] Corrupting shell configuration..."
cp /root/.bashrc /root/.bashrc.pre-experiment 2>/dev/null || true
cat >> /root/.bashrc << 'BROKEN'

# === EXPERIMENT DEFECT START ===
export PATH="/nonexistent/experiment/bin:$PATH"
alias ls='ls --color=auto --invalid-experiment-flag'
export PS1='$(missing_command_for_experiment)\u@\h:\w\$ '
source /opt/nonexistent/experiment/setup.sh
# === EXPERIMENT DEFECT END ===
BROKEN
DEFECTS+=("broken-bashrc: 4 errors injected (bad PATH, alias, PS1, source)")
echo "  - .bashrc corrupted with 4 errors" >> "$LOGFILE"
echo "cp /root/.bashrc.pre-experiment /root/.bashrc 2>/dev/null || true" >> "$FIXSCRIPT"

# --- Defect 3: Create broken symlink ---
echo "[3] Creating broken symlinks..."
ln -sf /usr/local/bin/tool_does_not_exist_experiment /usr/local/bin/broken-experiment-tool
DEFECTS+=("broken-symlink: /usr/local/bin/broken-experiment-tool -> nonexistent")
echo "  - /usr/local/bin/broken-experiment-tool -> nonexistent" >> "$LOGFILE"
echo "rm -f /usr/local/bin/broken-experiment-tool" >> "$FIXSCRIPT"

# --- Defect 4: Corrupt /etc/hosts (BACKUP first) ---
echo "[4] Corrupting /etc/hosts..."
cp /etc/hosts /etc/hosts.pre-experiment 2>/dev/null || true
cat >> /etc/hosts << 'BROKEN'

# EXPERIMENT DEFECT
999.999.999.999 invalid-experiment-host
THIS_LINE_IS_NOT_VALID_IP_ENTRY
BROKEN
DEFECTS+=("broken-hosts: 2 invalid entries in /etc/hosts")
echo "  - /etc/hosts has 2 invalid entries" >> "$LOGFILE"
echo "cp /etc/hosts.pre-experiment /etc/hosts" >> "$FIXSCRIPT"

# --- Defect 5: Fill /tmp with junk ---
echo "[5] Creating disk pressure..."
dd if=/dev/zero of=/tmp/experiment-junk-30mb.bin bs=1M count=30 2>/dev/null
DEFECTS+=("disk-pressure: 30MB junk file in /tmp/experiment-junk-30mb.bin")
echo "  - /tmp/experiment-junk-30mb.bin (30MB)" >> "$LOGFILE"
echo "rm -f /tmp/experiment-junk-30mb.bin" >> "$FIXSCRIPT"

# --- Defect 6: Break locale ---
echo "[6] Breaking locale..."
cp /etc/environment /etc/environment.pre-experiment 2>/dev/null || true
echo 'export LANG="xx_XX.UTF-EXPERIMENT"' >> /etc/environment
DEFECTS+=("broken-locale: invalid LANG in /etc/environment")
echo "  - Invalid LANG in /etc/environment" >> "$LOGFILE"
echo "cp /etc/environment.pre-experiment /etc/environment 2>/dev/null || true" >> "$FIXSCRIPT"

# --- Defect 7: Create fake broken ldconfig entry ---
echo "[7] Breaking ldconfig..."
echo "/opt/fake-experiment-lib-path" > /etc/ld.so.conf.d/experiment-fake.conf
DEFECTS+=("broken-ldconfig: fake library path in ld.so.conf.d")
echo "  - /etc/ld.so.conf.d/experiment-fake.conf" >> "$LOGFILE"
echo "rm -f /etc/ld.so.conf.d/experiment-fake.conf" >> "$FIXSCRIPT"

# --- Defect 8: Remove common tools (if installed) ---
echo "[8] Removing commonly needed tools..."
REMOVED_TOOLS=""
for tool in vim nano htop; do
    if which $tool 2>/dev/null; then
        TOOL_PATH=$(which $tool)
        mv "$TOOL_PATH" "${TOOL_PATH}.experiment-hidden"
        REMOVED_TOOLS="$REMOVED_TOOLS $tool"
    fi
done
if [ -n "$REMOVED_TOOLS" ]; then
    DEFECTS+=("missing-tools:${REMOVED_TOOLS}")
    echo "  - Hidden tools: ${REMOVED_TOOLS}" >> "$LOGFILE"
    # Fix: restore from hidden
    for tool in vim nano htop; do
        TOOL_PATH=$(which $tool 2>/dev/null | sed "s/\.experiment-hidden//")
        if [ -f "${TOOL_PATH}.experiment-hidden" ]; then
            echo "mv '${TOOL_PATH}.experiment-hidden' '${TOOL_PATH}'" >> "$FIXSCRIPT"
        fi
    done
else
    echo "  - vim/nano/htop not installed, skipping" >> "$LOGFILE"
fi

# Make fix script executable
chmod +x "$FIXSCRIPT"

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
echo "" >> "$LOGFILE"
echo "Fix script: $FIXSCRIPT" >> "$LOGFILE"

echo ""
echo "Defects logged to: $LOGFILE"
echo "Emergency fix script: $FIXSCRIPT"
echo "To emergency-restore: bash $FIXSCRIPT"
