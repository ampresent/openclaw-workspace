#!/usr/bin/env python3
"""SSH via subprocess + pty, no pexpect."""
import os, pty, subprocess, sys

HOST = "118.195.219.157"
USER = "root"
PASS = "Bebop4life&"

cmd = sys.argv[1] if len(sys.argv) > 1 else "echo OK"

# Use sshpass via expect-like trick
ssh_cmd = f'ssh -o StrictHostKeyChecking=no -o BatchMode=no {USER}@{HOST}'

# Write a helper expect script
import tempfile
with tempfile.NamedTemporaryFile(mode='w', suffix='.exp', delete=False) as f:
    f.write(f'''#!/usr/bin/expect -f
set timeout 600
spawn {ssh_cmd}
expect "password:"
send "{PASS}\\r"
expect "$ "
send "{cmd}\\r"
expect "$ "
send "exit\\r"
expect eof
''')
    exp_file = f.name

os.chmod(exp_file, 0o700)
result = subprocess.run(["expect", exp_file], capture_output=True, text=True, timeout=600)
os.unlink(exp_file)
print(result.stdout)
if result.stderr:
    print(result.stderr, file=sys.stderr)
sys.exit(result.returncode)
