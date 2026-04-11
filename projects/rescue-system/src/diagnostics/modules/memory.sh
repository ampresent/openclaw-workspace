#!/usr/bin/env bash
# memory.sh — P1 内存诊断
set -euo pipefail

python3 -c "
import json, subprocess

checks = {}
# Memory usage
try:
    out = subprocess.run(['free', '-b'], capture_output=True, text=True, timeout=5)
    mem = {}
    for line in out.stdout.strip().split('\n')[1:]:
        p = line.split()
        key = p[0].rstrip(':')
        mem[key] = {'total': int(p[1]), 'used': int(p[2]), 'free': int(p[3])}
    checks['memory_usage'] = mem
except: checks['memory_usage'] = {}

# OOM events
oom_events = []
try:
    out = subprocess.run(['dmesg'], capture_output=True, text=True, timeout=10)
    for line in out.stdout.split('\n'):
        if any(k in line.lower() for k in ['oom', 'out of memory', 'killed process']):
            oom_events.append(line.strip())
except: pass
checks['oom_events'] = {'count': len(oom_events), 'recent': oom_events[-10:]}

# Swap
try:
    out = subprocess.run(['swapon', '--show'], capture_output=True, text=True, timeout=5)
    checks['swap'] = out.stdout.strip() or 'none'
except: checks['swap'] = 'none'

# Pressure
try:
    with open('/proc/pressure/memory') as f:
        checks['pressure'] = f.read().strip()
except: checks['pressure'] = 'not available'

oom_count = len(oom_events)
print(json.dumps({
    'module': 'memory',
    'priority': 'P1',
    'status': 'error' if oom_count > 0 else 'ok',
    'checks': checks
}, ensure_ascii=False, indent=2))
"
