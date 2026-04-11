#!/usr/bin/env bash
# network.sh — P2 网络诊断
set -euo pipefail

python3 -c "
import json, subprocess, socket

checks = {}

# Interfaces
try:
    out = subprocess.run(['ip', '-j', 'addr'], capture_output=True, text=True, timeout=5)
    checks['interfaces'] = json.loads(out.stdout)
except: checks['interfaces'] = []

# DNS
nameservers = []
try:
    with open('/etc/resolv.conf') as f:
        for line in f:
            if line.startswith('nameserver'):
                nameservers.append(line.split()[1])
except: pass

# DNS test
dns_test = {}
for host in ['localhost', 'baidu.com']:
    try:
        ip = socket.getaddrinfo(host, None, socket.AF_INET)[0][4][0]
        dns_test[host] = {'resolved': True, 'ip': ip}
    except Exception as e:
        dns_test[host] = {'resolved': False, 'error': str(e)}
checks['dns'] = {'nameservers': nameservers, 'resolution_test': dns_test}

# Listening ports
try:
    out = subprocess.run(['ss', '-tlnp'], capture_output=True, text=True, timeout=5)
    checks['listening_ports'] = out.stdout.strip().split('\n')[:20]
except: checks['listening_ports'] = []

# Network errors
net_errors = []
try:
    out = subprocess.run(['dmesg'], capture_output=True, text=True, timeout=10)
    for line in out.stdout.split('\n'):
        if any(k in line.lower() for k in ['eth', 'wlan', 'net', 'nic']):
            if 'error' in line.lower():
                net_errors.append(line.strip())
except: pass
checks['network_errors'] = net_errors[-10:]

print(json.dumps({
    'module': 'network',
    'priority': 'P2',
    'status': 'error' if not nameservers else ('warning' if net_errors else 'ok'),
    'checks': checks
}, ensure_ascii=False, indent=2))
"
