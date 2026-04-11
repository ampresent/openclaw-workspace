#!/usr/bin/env bash
# boot.sh — P0 启动诊断
set -euo pipefail

TARGET="${1:-/mnt/rescue-target}"

# EFI 引导信息
efi_info="{}"
if [ -d /sys/firmware/efi ]; then
    efi_info=$(cat <<EFI_JSON
{
  "mode": "UEFI",
  "efi_vars": $(ls /sys/firmware/efi/efivars 2>/dev/null | wc -l),
  "boot_entries": $(efibootmgr 2>/dev/null | grep -c "Boot" || echo "0")
}
EFI_JSON
)
else
    efi_info='{"mode": "BIOS/Legacy"}'
fi

# GRUB 配置检查
grub_status="unknown"
grub_cfg=""
if [ -f "$TARGET/boot/grub/grub.cfg" ]; then
    grub_cfg="$TARGET/boot/grub/grub.cfg"
    grub_status="found"
elif [ -f "$TARGET/boot/grub2/grub.cfg" ]; then
    grub_cfg="$TARGET/boot/grub2/grub.cfg"
    grub_status="found"
else
    grub_status="missing"
fi

# GRUB 菜单项
grub_entries="[]"
if [ -n "$grub_cfg" ]; then
    grub_entries=$(grep -E "^menuentry|^submenu" "$grub_cfg" 2>/dev/null | \
        sed "s/['\"}].*//" | python3 -c "
import sys, json
entries = [l.strip() for l in sys.stdin if l.strip()]
print(json.dumps(entries))
" 2>/dev/null || echo "[]")
fi

# 内核文件检查
kernels="[]"
kernels=$(ls "$TARGET"/boot/vmlinuz-* 2>/dev/null | while read f; do
    size=$(stat -c%s "$f" 2>/dev/null || echo 0)
    echo "{\"file\": \"$f\", \"size\": $size}"
done | python3 -c "
import sys, json
items = [json.loads(l) for l in sys.stdin if l.strip()]
print(json.dumps(items))
" 2>/dev/null || echo "[]")

# initrd/initramfs 检查
initrds="[]"
initrds=$(ls "$TARGET"/boot/initr*-* "$TARGET"/boot/initramfs-* 2>/dev/null | while read f; do
    size=$(stat -c%s "$f" 2>/dev/null || echo 0)
    echo "{\"file\": \"$f\", \"size\": $size}"
done | python3 -c "
import sys, json
items = [json.loads(l) for l in sys.stdin if l.strip()]
print(json.dumps(items))
" 2>/dev/null || echo "[]")

# 引导分区挂载状态
boot_mount=$(mount | grep " /boot " || echo "")
esp_mount=$(mount | grep -E " /boot/efi | /efi " || echo "")

# systemd 启动日志 (如果目标系统有 journalctl)
boot_errors="[]"
if [ -d "$TARGET/var/log/journal" ]; then
    boot_errors=$(journalctl -D "$TARGET/var/log/journal" -b -p err --no-pager -q 2>/dev/null | tail -30 | \
        python3 -c "
import sys, json
lines = [l.strip() for l in sys.stdin if l.strip()]
print(json.dumps(lines))
" 2>/dev/null || echo "[]")
fi

# fstab 检查
fstab_issues="[]"
if [ -f "$TARGET/etc/fstab" ]; then
    fstab_issues=$(python3 -c "
import json, subprocess, os

issues = []
target = '$TARGET'
fstab_path = os.path.join(target, 'etc/fstab')

with open(fstab_path) as f:
    for i, line in enumerate(f, 1):
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        parts = line.split()
        if len(parts) < 6:
            issues.append({'line': i, 'issue': 'incomplete entry', 'content': line})
            continue
        dev, mnt, fstype = parts[0], parts[1], parts[2]
        # Check if device exists
        if dev.startswith('/dev/') and not os.path.exists(dev):
            # Might be on the target system
            target_dev = os.path.join(target, dev.lstrip('/'))
            if not os.path.exists(target_dev):
                issues.append({'line': i, 'issue': 'device not found', 'device': dev})
        elif dev.startswith('UUID='):
            uuid = dev[5:]
            try:
                subprocess.run(['blkid', '-U', uuid], capture_output=True, timeout=5)
            except:
                issues.append({'line': i, 'issue': 'UUID not found', 'uuid': uuid})

print(json.dumps(issues))
" 2>/dev/null || echo "[]")
fi

cat <<EOF
{
  "module": "boot",
  "priority": "P0",
  "status": "$([ "$grub_status" = "missing" ] && echo 'error' || echo 'ok')",
  "checks": {
    "firmware_mode": $efi_info,
    "grub_status": "$grub_status",
    "grub_entries": $grub_entries,
    "kernels": $kernels,
    "initrds": $initrds,
    "boot_partition": {
      "boot_mount": "$(echo $boot_mount)",
      "esp_mount": "$(echo $esp_mount)"
    },
    "boot_errors": $boot_errors,
    "fstab_issues": $fstab_issues
  }
}
EOF
