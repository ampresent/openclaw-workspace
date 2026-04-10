#!/usr/bin/env python3
"""Test image-builder argument parsing via SSH."""
import subprocess, sys

HOST = "118.195.219.157"
USER = "root"
PASS = "Bebop4life&"

cmd = sys.argv[1] if len(sys.argv) > 1 else "echo OK"

result = subprocess.run(
    ["sshpass", "-p", PASS, "ssh", "-o", "StrictHostKeyChecking=no", f"{USER}@{HOST}", cmd],
    capture_output=True, text=True, timeout=30
)
print(result.stdout)
print(result.stderr, file=sys.stderr)
sys.exit(result.returncode)
