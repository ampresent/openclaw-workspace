#!/usr/bin/env bash
# services.sh — P1 服务诊断
set -euo pipefail

python3 -c "
import json, subprocess

checks = {}

# Failed services
failed = []
try:
    out = subprocess.run(['systemctl', 'list-units', '--state=failed', '--no-pager', '--no-legend'],
                        capture_output=True, text=True, timeout=10)
    failed = [l.strip() for l in out.stdout.strip().split('\n') if l.strip()]
except: pass
checks['failed_services'] = failed[:20]

# Critical services
critical = ['sshd', 'systemd-networkd', 'NetworkManager', 'systemd-resolved', 'cron', 'docker']
svc_status = {}
for svc in critical:
    active = subprocess.run(['systemctl', 'is-active', svc], capture_output=True, text=True, timeout=5)
    enabled = subprocess.run(['systemctl', 'is-enabled', svc], capture_output=True, text=True, timeout=5)
    svc_status[svc] = {'active': active.stdout.strip(), 'enabled': enabled.stdout.strip()}
checks['critical_services'] = svc_status

# Recent errors
errors = []
try:
    out = subprocess.run(['journalctl', '-p', 'err', '--since', '24 hours ago', '--no-pager', '-q'],
                        capture_output=True, text=True, timeout=15)
    errors = [l.strip() for l in out.stdout.strip().split('\n') if l.strip()][-30:]
except: pass
checks['recent_errors'] = errors

print(json.dumps({
    'module': 'services',
    'priority': 'P1',
    'status': 'error' if len(failed) > 5 else ('warning' if failed else 'ok'),
    'checks': checks
}, ensure_ascii=False, indent=2))
"
