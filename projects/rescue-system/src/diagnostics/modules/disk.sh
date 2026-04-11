#!/usr/bin/env bash
# disk.sh — P0 磁盘诊断
set -euo pipefail

python3 -c "
import json, subprocess, os, glob

checks = {}
# Filesystem usage
try:
    out = subprocess.run(['df', '-h', '--output=source,fstype,size,used,avail,pcent,target'],
                        capture_output=True, text=True, timeout=10)
    fs_list = []
    for line in out.stdout.strip().split('\n')[1:]:
        p = line.split()
        if len(p) >= 7:
            fs_list.append({'device': p[0], 'fstype': p[1], 'size': p[2],
                           'used': p[3], 'avail': p[4], 'pcent': p[5], 'mount': p[6]})
    checks['filesystem_usage'] = fs_list
except: checks['filesystem_usage'] = []

# Block devices
try:
    out = subprocess.run(['lsblk', '-J', '-o', 'NAME,SIZE,TYPE,FSTYPE,MOUNTPOINT,ROTA,MODEL'],
                        capture_output=True, text=True, timeout=10)
    checks['block_devices'] = json.loads(out.stdout)
except: checks['block_devices'] = {'blockdevices': []}

# Disk errors
disk_errors = []
try:
    out = subprocess.run(['dmesg'], capture_output=True, text=True, timeout=10)
    for line in out.stdout.split('\n'):
        if any(k in line.lower() for k in ['error', 'fail', 'corrupt', 'i/o error']):
            if any(d in line.lower() for d in ['sd', 'nvme', 'disk', 'ata', 'scsi']):
                disk_errors.append(line.strip())
except: pass
checks['disk_errors'] = disk_errors[-20:]

# High inode usage
high_inode = []
try:
    out = subprocess.run(['df', '-i'], capture_output=True, text=True, timeout=10)
    for line in out.stdout.strip().split('\n')[1:]:
        p = line.split()
        if len(p) >= 6 and p[4].rstrip('%').isdigit() and int(p[4].rstrip('%')) > 80:
            high_inode.append({'mount': p[5], 'ipcent': p[4]})
except: pass
checks['high_inode_usage'] = high_inode

has_errors = bool(disk_errors)
print(json.dumps({
    'module': 'disk',
    'priority': 'P0',
    'status': 'warning' if has_errors else 'ok',
    'checks': checks
}, ensure_ascii=False, indent=2))
"
