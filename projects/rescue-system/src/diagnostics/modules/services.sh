#!/usr/bin/env bash
# services.sh — P1 服务诊断
set -euo pipefail

TARGET="${1:-/mnt/rescue-target}"

# 失败的服务
failed_services=$(systemctl list-units --state=failed --no-pager --no-legend 2>/dev/null || echo "")

# 目标系统失败服务
target_failed=""
if [ -d "$TARGET/run/systemd" ]; then
    target_failed=$(systemctl --root="$TARGET" list-units --state=failed --no-pager --no-legend 2>/dev/null || echo "")
fi

# 关键服务状态
critical_services="sshd systemd-networkd NetworkManager systemd-resolved crond cron docker"
service_status="{}"
for svc in $critical_services; do
    status=$(systemctl is-active "$svc" 2>/dev/null || echo "not-found")
    enabled=$(systemctl is-enabled "$svc" 2>/dev/null || echo "unknown")
    service_status=$(python3 -c "
import sys, json
d = json.loads('''$service_status''') if '''$service_status''' != '{}' else {}
d['$svc'] = {'active': '$status', 'enabled': '$enabled'}
print(json.dumps(d))
" 2>/dev/null || echo "$service_status")
done

# 最近的 journal 错误
recent_errors=$(journalctl -p err --since "24 hours ago" --no-pager -q 2>/dev/null | tail -30 || echo "")

# 崩溃的进程
crashed=$(coredumpctl list --no-pager 2>/dev/null | tail -10 || echo "coredumpctl not available")

python3 -c "
import json

failed = [l.strip() for l in '''$failed_services'''.split('\n') if l.strip()]
target = [l.strip() for l in '''$target_failed'''.split('\n') if l.strip()]
errors = [l.strip() for l in '''$recent_errors'''.split('\n') if l.strip()]

total_failed = len(failed) + len(target)

print(json.dumps({
    'module': 'services',
    'priority': 'P1',
    'status': 'error' if total_failed > 5 else ('warning' if total_failed > 0 else 'ok'),
    'checks': {
        'failed_services': {
            'host_count': len(failed),
            'host': failed[:20],
            'target_count': len(target),
            'target': target[:20]
        },
        'critical_services': json.loads('''$service_status''' if '''$service_status''' != '{}' else '{}'),
        'recent_errors': errors[-20:],
        'crashes': '''$crashed'''.strip()
    }
}, indent=2))
"
