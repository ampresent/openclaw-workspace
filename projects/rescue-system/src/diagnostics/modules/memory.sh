#!/usr/bin/env bash
# memory.sh — P1 内存诊断
set -euo pipefail

TARGET="${1:-/mnt/rescue-target}"

# 基础内存信息
mem_info=$(free -b)

# 详细内存
mem_detail=$(cat /proc/meminfo)

# OOM 历史
oom_events=$(dmesg 2>/dev/null | grep -i "oom\|out of memory\|killed process" | tail -20 || echo "")

# Swap 使用
swap_info=$(swapon --show 2>/dev/null || echo "none")

# 大页内存
hugepages=$(cat /proc/meminfo 2>/dev/null | grep -i huge || echo "")

# 内存压力 (如果有 pressure stall info)
memory_pressure=$(cat /proc/pressure/memory 2>/dev/null || echo "not available")

# NUMA 信息
numa_info=""
if command -v numactl &>/dev/null; then
    numa_info=$(numactl --hardware 2>/dev/null || echo "numactl failed")
fi

# 目标系统 OOM 日志
target_oom=""
if [ -d "$TARGET/var/log" ]; then
    target_oom=$(grep -rih "oom\|out of memory" "$TARGET"/var/log/{messages,syslog,kern.log,dmesg} 2>/dev/null | tail -20 || echo "")
fi

python3 -c "
import json, os

# Parse free output
mem_lines = '''$mem_info'''.strip().split('\n')
mem_dict = {}
for line in mem_lines:
    parts = line.split()
    if parts and parts[0] in ('Mem:', 'Swap:', 'buff/cache:'):
        key = parts[0].rstrip(':').lower().replace('/', '_')
        mem_dict[key] = {
            'total': int(parts[1]) if len(parts) > 1 else 0,
            'used': int(parts[2]) if len(parts) > 2 else 0,
            'free': int(parts[3]) if len(parts) > 3 else 0,
        }

oom_lines = [l.strip() for l in '''$oom_events'''.split('\n') if l.strip()]
oom_count = len(oom_lines)

print(json.dumps({
    'module': 'memory',
    'priority': 'P1',
    'status': 'error' if oom_count > 0 else ('warning' if mem_dict.get('mem', {}).get('free', 0) < 500000000 else 'ok'),
    'checks': {
        'memory_usage': mem_dict,
        'oom_events': {
            'count': oom_count,
            'recent': oom_lines[-10:] if oom_lines else []
        },
        'swap': '''$swap_info'''.strip() or 'none',
        'pressure': '''$memory_pressure'''.strip()
    }
}, indent=2))
"
