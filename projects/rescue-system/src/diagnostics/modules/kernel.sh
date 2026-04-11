#!/usr/bin/env bash
# kernel.sh — P3 内核诊断
set -euo pipefail

TARGET="${1:-/mnt/rescue-target}"

# 内核版本
kernel_version=$(uname -r)

# 内核 panic/错误
kernel_errors=$(dmesg 2>/dev/null | grep -iE '(panic|bug|oops|taint|call trace)' | tail -20 || echo "")

# 已加载模块
loaded_modules=$(lsmod 2>/dev/null | wc -l || echo 0)

# 内核参数
kernel_params=$(cat /proc/cmdline 2>/dev/null || echo "unknown")

# 内核 taint 状态
taint=$(cat /proc/sys/kernel/tainted 2>/dev/null || echo "0")

# 目标系统内核 (如果有)
target_kernels=$(ls "$TARGET"/boot/vmlinuz-* 2>/dev/null | while read f; do
    basename "$f"
done | tr '\n' ',' || echo "none")

# 系统日志中的严重错误
critical_logs=""
if [ -d "$TARGET/var/log" ]; then
    critical_logs=$(grep -rih "panic\|oops\|bug:" "$TARGET"/var/log/{messages,kern.log,dmesg,syslog} 2>/dev/null | tail -20 || echo "")
fi

# kdump 状态
kdump_status="unknown"
if command -v kdumpctl &>/dev/null; then
    kdump_status=$(kdumpctl status 2>/dev/null || echo "kdump check failed")
fi

python3 -c "
import json

k_err = [l.strip() for l in '''$kernel_errors'''.split('\n') if l.strip()]
c_log = [l.strip() for l in '''$critical_logs'''.split('\n') if l.strip()]
taint_val = int('''$taint''') if '''$taint'''.strip().isdigit() else 0

taint_desc = []
taint_flags = {
    1: 'proprietary module', 2: 'module was force loaded', 4: 'kernel running on SMP',
    8: 'module force unloaded', 16: 'processor reported MCE', 32: 'bad page',
    64: 'user requested taint', 128: 'died recently', 256: 'ACPI override',
    512: 'unsigned kernel'
}
for bit, desc in taint_flags.items():
    if taint_val & bit:
        taint_desc.append(desc)

print(json.dumps({
    'module': 'kernel',
    'priority': 'P3',
    'status': 'error' if any('panic' in e.lower() for e in k_err) else ('warning' if k_err or taint_val > 0 else 'ok'),
    'checks': {
        'version': '$kernel_version',
        'taint': {
            'value': taint_val,
            'flags': taint_desc
        },
        'cmdline': '$kernel_params'.strip(),
        'loaded_modules': $loaded_modules,
        'target_kernels': '$target_kernels'.strip(),
        'kernel_errors': k_err,
        'critical_logs': c_log,
        'kdump': '$kdump_status'.strip()
    }
}, indent=2))
"
