#!/usr/bin/env python3
"""SSH wrapper for bootable-container project backend."""
import sys
import pexpect

HOST = "118.195.219.157"
USER = "root"
PASS = "Bebop4life&"

def ssh_cmd(cmd, timeout=600):
    child = pexpect.spawn(
        f'ssh -o StrictHostKeyChecking=no -o ServerAliveInterval=15 {USER}@{HOST} "{cmd}"',
        timeout=timeout
    )
    child.expect('password:')
    child.sendline(PASS)
    child.expect(pexpect.EOF)
    output = child.before.decode()
    child.close()
    return output, child.exitstatus

if __name__ == "__main__":
    cmd = " ".join(sys.argv[1:]) or "echo OK"
    output, code = ssh_cmd(cmd)
    print(output)
    sys.exit(code)
