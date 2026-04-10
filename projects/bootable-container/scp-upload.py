#!/usr/bin/env python3
"""Upload file to remote host via SCP."""
import sys
import pexpect

HOST = "118.195.219.157"
USER = "root"
PASS = "Bebop4life&"

src = sys.argv[1]
dst = sys.argv[2]

child = pexpect.spawn(f'scp -o StrictHostKeyChecking=no {src} {USER}@{HOST}:{dst}', timeout=60)
child.expect('password:')
child.sendline(PASS)
child.expect(pexpect.EOF)
child.close()
print(f"Uploaded {src} -> {HOST}:{dst} (exit {child.exitstatus})")
sys.exit(child.exitstatus)
