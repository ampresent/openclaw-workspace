#!/usr/bin/env python3
"""Run command on remote host via SSH, return stdout+stderr."""
import sys
import pexpect

HOST = "118.195.219.157"
USER = "root"
PASS = "Bebop4life&"

def ssh_run(cmd, timeout=600):
    child = pexpect.spawn(
        f'ssh -o StrictHostKeyChecking=no -o ServerAliveInterval=15 {USER}@{HOST}',
        timeout=timeout
    )
    child.expect('password:')
    child.sendline(PASS)
    child.expect(r'[\$#]\s*')
    child.sendline(cmd)
    child.expect(r'[\$#]\s*', timeout=timeout)
    output = child.before.decode()
    child.sendline('exit')
    child.expect(pexpect.EOF)
    return output

if __name__ == "__main__":
    cmd = " ".join(sys.argv[1:]) or "echo OK"
    output = ssh_run(cmd)
    print(output)
