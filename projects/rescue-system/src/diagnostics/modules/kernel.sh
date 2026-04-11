#!/usr/bin/env bash
# kernel.sh — P3 内核诊断
set -euo pipefail

python3 -c "
import json, subprocess

checks = {}
checks['version'] = subprocess.run(['uname', '-r'], capture_output=True, text=True).stdout.strip()

# Taint
try:
    with open('/proc/sys/kernel/tainted') as f:
        taint_val = int(f.read().strip())
except: taint_val = 0

taint_flags = {1: 'proprietary module', 2: 'force loaded', 4: 'SMP', 8: 'force unloaded',
               16: 'MCE', 32: 'bad page', 64: 'user tainted', 128: 'died recently',
               256: 'ACPI override', 512: 'unsigned kernel'}
checks['taint'] = {'value': taint_val, 'flags': [d for b, d in taint_flags.items() if taint_val & b]}

# Cmdline
try:
    with open('/proc/cmdline') as f:
        checks['cmdline'] = f.read().strip()
except: checks['cmdline'] = 'unknown'

# Loaded modules
try:
    out = subprocess.run(['lsmod'], capture_output=True, text=True, timeout=5)
    checks['loaded_modules'] = len(out.stdout.strip().split('\n')) - 1
except: checks['loaded_modules'] = 0

# Kernel errors
kernel_errors = []
try:
    out = subprocess.run(['dmesg'], capture_output=True, text=True, timeout=10)
    for line in out.stdout.split('\n'):
        if any(k in line.lower() for k in ['panic', 'bug', 'oops', 'call trace']):
            kernel_errors.append(line.strip())
except: pass
checks['kernel_errors'] = kernel_errors[-20:]

has_panic = any('panic' in e.lower() for e in kernel_errors)
print(json.dumps({
    'module': 'kernel',
    'priority': 'P3',
    'status': 'error' if has_panic else ('warning' if kernel_errors or taint_val > 0 else 'ok'),
    'checks': checks
}, ensure_ascii=False, indent=2))
"
