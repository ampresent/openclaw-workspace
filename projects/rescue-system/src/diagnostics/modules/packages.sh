#!/usr/bin/env bash
# packages.sh — P2 包管理器诊断
set -euo pipefail

TARGET="${1:-/mnt/rescue-target}"

# 检测包管理器类型
pkg_manager="unknown"
if [ -f "$TARGET/etc/debian_version" ]; then
    pkg_manager="apt"
elif [ -f "$TARGET/etc/redhat-release" ]; then
    pkg_manager="yum"
elif [ -f "$TARGET/etc/arch-release" ]; then
    pkg_manager="pacman"
fi

# dpkg/rpm 状态检查
db_status="ok"
db_errors=""
if [ "$pkg_manager" = "apt" ]; then
    db_errors=$(chroot "$TARGET" dpkg --audit 2>/dev/null | head -30 || echo "")
    if [ -n "$db_errors" ]; then
        db_status="warning"
    fi
    # 检查中断的安装
    interrupted=$(chroot "$TARGET" dpkg --configure -a --dry-run 2>&1 | head -10 || echo "")
elif [ "$pkg_manager" = "yum" ]; then
    db_errors=$(chroot "$TARGET" rpm -Va --nomtime --nosize --nomode --nolinkto 2>/dev/null | head -30 || echo "")
    if [ -n "$db_errors" ]; then
        db_status="warning"
    fi
fi

# 锁文件检查
locks=""
for lock in "$TARGET/var/lib/dpkg/lock" "$TARGET/var/lib/apt/lists/lock" "$TARGET/var/cache/apt/archives/lock" "$TARGET/var/lib/rpm/.rpm.lock"; do
    [ -f "$lock" ] || continue
    if fuser "$lock" &>/dev/null; then
        locks+="LOCKED: $lock (in use)\n"
    fi
done

python3 -c "
import json

db_err = '''$db_errors'''.strip()
locks = '''$locks'''.strip()

print(json.dumps({
    'module': 'packages',
    'priority': 'P2',
    'status': 'error' if 'LOCKED' in locks else ('$db_status' if db_err else 'ok'),
    'checks': {
        'package_manager': '$pkg_manager',
        'db_status': '$db_status',
        'db_errors': db_err.split('\n')[:20] if db_err else [],
        'lock_status': locks.split('\n') if locks else [],
        'interrupted_installs': '''$interrupted'''.strip().split('\n')[:10] if '''$interrupted'''.strip() else []
    }
}, indent=2))
"
