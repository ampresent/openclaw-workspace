#!/usr/bin/env bash
# disk.sh — P0 磁盘诊断
# 输出 JSON 格式诊断数据
set -euo pipefail

TARGET="${1:-/mnt/rescue-target}"

# 安全执行命令，失败返回空
safe() { eval "$@" 2>/dev/null || echo ""; }

# 磁盘空间
df_output=$(df -h --output=source,fstype,size,used,avail,pcent,target 2>/dev/null || df -h)

# 磁盘块设备
lsblk_json=$(lsblk -J -o NAME,SIZE,TYPE,FSTYPE,MOUNTPOINT,ROTA,MODEL,SERIAL 2>/dev/null || echo '{"blockdevices":[]}')

# 磁盘错误检查 (dmesg)
disk_errors=$(dmesg 2>/dev/null | grep -iE '(error|fail|corrupt|bad|I/O error|sector)' | grep -iE '(sd[a-z]|nvme|disk|ata|scsi)' | tail -20 || echo "")

# SMART 状态
smart_status="{}"
for dev in /dev/sd? /dev/nvme?n?; do
    [ -b "$dev" ] || continue
    dev_name=$(basename "$dev")
    smart_result=$(smartctl -H "$dev" 2>/dev/null | grep -i "result" || echo "unavailable")
    smart_status=$(echo "$smart_status" | python3 -c "
import sys, json
d = json.load(sys.stdin) if sys.stdin.readable() else {}
d['$dev_name'] = '''$smart_result'''.strip()
print(json.dumps(d))
" 2>/dev/null || echo "$smart_status")
done

# 文件系统检查 (只读)
fsck_results="{}"
for dev in /dev/sd?* /dev/nvme?n?p*; do
    [ -b "$dev" ] || continue
    dev_name=$(basename "$dev")
    fstype=$(blkid -o value -s TYPE "$dev" 2>/dev/null || echo "unknown")
    # 只做只读检查
    if [ "$fstype" = "ext4" ] || [ "$fstype" = "ext3" ]; then
        check_result=$(e2fsck -n "$dev" 2>&1 | tail -3 || echo "check failed")
    elif [ "$fstype" = "xfs" ]; then
        check_result="xfs_repair -n needed (not run in diag mode)"
    else
        check_result="unsupported fstype: $fstype"
    fi
    fsck_results=$(echo "$fsck_results" | python3 -c "
import sys, json
d = json.load(sys.stdin) if sys.stdin.readable() else {}
d['$dev_name'] = {'fstype': '$fstype', 'result': '''$check_result'''.strip()}
print(json.dumps(d))
" 2>/dev/null || echo "$fsck_results")
done

# inode 使用情况
inode_usage=$(df -i 2>/dev/null | awk 'NR>1 && $5+0 > 80 {print "{\"mount\":\""$6"\",\"ipcent\":\""$5"\",\"iused\":\""$3"\",\"ifree\":\""$4"\"}"}' | paste -sd',' || echo "")

# 目标系统磁盘空间 (如果挂载了)
target_disk=""
if [ -d "$TARGET" ] && mountpoint -q "$TARGET" 2>/dev/null; then
    target_disk=$(df -h "$TARGET" | tail -1)
fi

# 生成 JSON
cat <<EOF
{
  "module": "disk",
  "priority": "P0",
  "status": "$([ -z "$disk_errors" ] && echo 'ok' || echo 'warning')",
  "checks": {
    "filesystem_usage": $(echo "$df_output" | python3 -c "
import sys, json
lines = sys.stdin.read().strip().split('\n')
result = []
for l in lines[1:]:
    parts = l.split()
    if len(parts) >= 6:
        result.append({'device': parts[0], 'fstype': parts[1], 'size': parts[2],
                       'used': parts[3], 'avail': parts[4], 'pcent': parts[5],
                       'mount': parts[-1]})
print(json.dumps(result))
" 2>/dev/null || echo "[]"),
    "block_devices": $lsblk_json,
    "disk_errors": $(echo "$disk_errors" | python3 -c "
import sys, json
lines = [l.strip() for l in sys.stdin if l.strip()]
print(json.dumps(lines))
" 2>/dev/null || echo "[]"),
    "smart_status": $smart_status,
    "fsck_results": $fsck_results,
    "high_inode_usage": [$inode_usage],
    "target_disk": "$target_disk"
  }
}
EOF
