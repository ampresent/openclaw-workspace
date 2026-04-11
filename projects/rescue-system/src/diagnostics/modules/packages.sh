#!/usr/bin/env bash
# packages.sh — P2 包管理器诊断
set -euo pipefail
TARGET="${1:-/mnt/rescue-target}"

python3 - "$TARGET" <<'PYEOF'
import json, subprocess, os, sys

target = sys.argv[1] if len(sys.argv) > 1 else "/mnt/rescue-target"
checks = {}

# Detect package manager
pkg_mgr = "unknown"
for path, name in [("etc/debian_version", "apt"), ("etc/redhat-release", "yum"), ("etc/arch-release", "pacman")]:
    if os.path.isfile(os.path.join(target, path)):
        pkg_mgr = name
        break
checks["package_manager"] = pkg_mgr

# DB status
db_errors = []
if pkg_mgr == "apt":
    try:
        r = subprocess.run(["chroot", target, "dpkg", "--audit"], capture_output=True, text=True, timeout=30)
        db_errors = [l for l in r.stdout.strip().split("\n") if l.strip()][:30]
    except: pass
elif pkg_mgr == "yum":
    try:
        r = subprocess.run(["chroot", target, "rpm", "-Va", "--nomtime", "--nosize"], capture_output=True, text=True, timeout=60)
        db_errors = [l for l in r.stdout.strip().split("\n") if l.strip()][:30]
    except: pass
checks["db_status"] = "warning" if db_errors else "ok"
checks["db_errors"] = db_errors

# Lock files
locks = []
for lock in ["var/lib/dpkg/lock", "var/lib/apt/lists/lock", "var/cache/apt/archives/lock", "var/lib/rpm/.rpm.lock"]:
    fp = os.path.join(target, lock)
    if os.path.isfile(fp):
        try:
            r = subprocess.run(["fuser", fp], capture_output=True, text=True, timeout=5)
            if r.returncode == 0:
                locks.append(f"LOCKED: {fp}")
        except: pass
checks["lock_status"] = locks

print(json.dumps({
    "module": "packages",
    "priority": "P2",
    "status": "error" if locks else ("warning" if db_errors else "ok"),
    "checks": checks
}, ensure_ascii=False, indent=2))
PYEOF
