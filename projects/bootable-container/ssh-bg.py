#!/usr/bin/env python3
"""SSH wrapper for bootable-container project background commands."""
import sys
import subprocess

HOST = "118.195.219.157"
USER = "root"

def ssh_cmd_bg(cmd, timeout=300):
    """Run a command on remote server via sshpass."""
    ssh_cmd = (
        f'sshpass -p "Bebop4life&" ssh -o StrictHostKeyChecking=no '
        f'-o ServerAliveInterval=15 {USER}@{HOST} "{cmd}"'
    )
    result = subprocess.run(ssh_cmd, shell=True, capture_output=True, text=True, timeout=timeout)
    return result.stdout, result.stderr, result.returncode

if __name__ == "__main__":
    cmd = " ".join(sys.argv[1:]) or "echo OK"
    stdout, stderr, code = ssh_cmd_bg(cmd, timeout=600)
    if stdout:
        print(stdout)
    if stderr:
        print(stderr, file=sys.stderr)
    sys.exit(code)
