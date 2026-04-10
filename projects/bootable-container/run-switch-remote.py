#!/usr/bin/env python3
"""Write and execute bootc switch script on VM via remote host."""
import subprocess, sys

HOST = "118.195.219.157"
USER = "root"

def run(cmd, timeout=60):
    r = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=timeout)
    return r.stdout.strip(), r.stderr.strip(), r.returncode

# Step 1: Write script on remote host
script_content = """#!/bin/bash
bootc switch registry.fedoraproject.org/fedora-bootc:42 > /tmp/bootc-switch.log 2>&1
bootc status >> /tmp/bootc-switch.log 2>&1
"""

# Write script via base64 to avoid quoting issues
import base64
b64 = base64.b64encode(script_content.encode()).decode()

ssh_base = f'sshpass -p "Bebop4life&" ssh -o StrictHostKeyChecking=no {USER}@{HOST}'

# Write script on remote
out, err, rc = run(f'{ssh_base} "echo {b64} | base64 -d > /tmp/run-switch.sh"')
if rc != 0:
    print(f"Failed to write script on remote: {err}", file=sys.stderr)
    sys.exit(1)
print("Script written on remote host")

# SCP script to VM
out, err, rc = run(f'{ssh_base} "scp -i /tmp/bootc-key -o StrictHostKeyChecking=no -P 2222 /tmp/run-switch.sh root@127.0.0.1:/tmp/run-switch.sh"')
if rc != 0:
    print(f"Failed to SCP to VM: {err}", file=sys.stderr)
    sys.exit(1)
print("Script copied to VM")

# Execute script on VM
out, err, rc = run(f'{ssh_base} "ssh -i /tmp/bootc-key -o StrictHostKeyChecking=no -p 2222 root@127.0.0.1 \'chmod +x /tmp/run-switch.sh && nohup /tmp/run-switch.sh &\'"')
print(f"Execute result: rc={rc}, out={out}, err={err}")

# Verify process is running
out, err, rc = run(f'{ssh_base} "ssh -i /tmp/bootc-key -o StrictHostKeyChecking=no -p 2222 root@127.0.0.1 \'ps aux | grep bootc | grep -v grep\'"')
print(f"Process check: {out}")
