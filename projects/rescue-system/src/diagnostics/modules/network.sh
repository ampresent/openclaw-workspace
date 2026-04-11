#!/usr/bin/env bash
# network.sh — P2 网络诊断
set -euo pipefail

TARGET="${1:-/mnt/rescue-target}"

# 网络接口
interfaces=$(ip -j addr 2>/dev/null || echo "[]")

# 路由表
routes=$(ip -j route 2>/dev/null || echo "[]")

# DNS 配置
dns_config=""
if [ -f "$TARGET/etc/resolv.conf" ]; then
    dns_config=$(cat "$TARGET/etc/resolv.conf" 2>/dev/null || cat /etc/resolv.conf)
else
    dns_config=$(cat /etc/resolv.conf 2>/dev/null || echo "not found")
fi

# DNS 解析测试
dns_test=$(python3 -c "
import socket, json
results = {}
for host in ['localhost', 'google.com', 'baidu.com']:
    try:
        ip = socket.getaddrinfo(host, None, socket.AF_INET)[0][4][0]
        results[host] = {'resolved': True, 'ip': ip}
    except Exception as e:
        results[host] = {'resolved': False, 'error': str(e)}
print(json.dumps(results))
" 2>/dev/null || echo "{}")

# 监听端口
listening=$(ss -tlnp 2>/dev/null | head -30 || netstat -tlnp 2>/dev/null | head -30 || echo "unavailable")

# 防火墙状态
firewall=""
if command -v iptables &>/dev/null; then
    firewall=$(iptables -L -n 2>/dev/null | head -30 || echo "iptables failed")
elif command -v nft &>/dev/null; then
    firewall=$(nft list ruleset 2>/dev/null | head -30 || echo "nft failed")
fi

# 网络错误
net_errors=$(dmesg 2>/dev/null | grep -iE '(eth|wlan|net|nic|link).*error' | tail -10 || echo "")

python3 -c "
import json

dns_lines = [l.strip() for l in '''$dns_config'''.split('\n') if l.strip() and not l.startswith('#')]
nameservers = [l.split()[1] for l in dns_lines if l.startswith('nameserver')]
net_err = [l.strip() for l in '''$net_errors'''.split('\n') if l.strip()]

print(json.dumps({
    'module': 'network',
    'priority': 'P2',
    'status': 'error' if not nameservers else ('warning' if net_err else 'ok'),
    'checks': {
        'interfaces': json.loads('''$interfaces''' if '''$interfaces''' != '[]' else '[]'),
        'routes': json.loads('''$routes''' if '''$routes''' != '[]' else '[]'),
        'dns': {
            'nameservers': nameservers,
            'resolv_conf': dns_lines[:10],
            'resolution_test': json.loads('''$dns_test''' if '''$dns_test''' != '{}' else '{}')
        },
        'listening_ports': '''$listening'''.strip().split('\n')[:20],
        'network_errors': net_err
    }
}, indent=2))
"
