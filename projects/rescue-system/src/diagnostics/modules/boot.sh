#!/usr/bin/env bash
# boot.sh — P0 启动诊断
set -euo pipefail

TARGET="${1:-/mnt/rescue-target}"

python3 - "$TARGET" <<'PYEOF'
import sys, json, subprocess, os, glob

target = sys.argv[1] if len(sys.argv) > 1 else "/mnt/rescue-target"
checks = {}

# EFI
if os.path.isdir("/sys/firmware/efi"):
    efi_vars_count = len(os.listdir("/sys/firmware/efi/efivars")) if os.path.isdir("/sys/firmware/efi/efivars") else 0
    boot_entries = 0
    try:
        out = subprocess.run(["efibootmgr"], capture_output=True, text=True, timeout=5)
        boot_entries = out.stdout.count("Boot")
    except: pass
    checks["firmware_mode"] = {"mode": "UEFI", "efi_vars": efi_vars_count, "boot_entries": boot_entries}
else:
    checks["firmware_mode"] = {"mode": "BIOS/Legacy"}

# GRUB
grub_cfg = ""
for p in [f"{target}/boot/grub/grub.cfg", f"{target}/boot/grub2/grub.cfg"]:
    if os.path.isfile(p):
        grub_cfg = p
        break

grub_status = "found" if grub_cfg else "missing"
grub_entries = []
if grub_cfg:
    try:
        with open(grub_cfg) as f:
            for line in f:
                if line.startswith("menuentry") or line.startswith("submenu"):
                    grub_entries.append(line.split("'")[1] if "'" in line else line.strip()[:80])
    except: pass
checks["grub_status"] = grub_status
checks["grub_entries"] = grub_entries

# Kernels
kernels = []
for f in sorted(glob.glob(f"{target}/boot/vmlinuz-*")):
    try: kernels.append({"file": f, "size": os.path.getsize(f)})
    except: pass
checks["kernels"] = kernels

# Initrds
initrds = []
for pattern in [f"{target}/boot/initr*-*", f"{target}/boot/initramfs-*"]:
    for f in sorted(glob.glob(pattern)):
        try: initrds.append({"file": f, "size": os.path.getsize(f)})
        except: pass
checks["initrds"] = initrds

# Boot mount
boot_mount = ""
esp_mount = ""
try:
    mounts = subprocess.run(["mount"], capture_output=True, text=True, timeout=5).stdout
    for line in mounts.split("\n"):
        if " /boot " in line: boot_mount = line.strip()
        if " /boot/efi " in line or " /efi " in line: esp_mount = line.strip()
except: pass
checks["boot_partition"] = {"boot_mount": boot_mount, "esp_mount": esp_mount}

# Boot errors
boot_errors = []
try:
    jdir = f"{target}/var/log/journal"
    if os.path.isdir(jdir):
        out = subprocess.run(["journalctl", "-D", jdir, "-b", "-p", "err", "--no-pager", "-q"],
                           capture_output=True, text=True, timeout=10)
        boot_errors = [l for l in out.stdout.strip().split("\n") if l.strip()][-30:]
except: pass
checks["boot_errors"] = boot_errors

# fstab
fstab_issues = []
fstab_path = os.path.join(target, "etc", "fstab")
if os.path.isfile(fstab_path):
    try:
        with open(fstab_path) as f:
            for i, line in enumerate(f, 1):
                line = line.strip()
                if not line or line.startswith("#"): continue
                parts = line.split()
                if len(parts) < 6:
                    fstab_issues.append({"line": i, "issue": "incomplete entry"})
                    continue
                dev = parts[0]
                if dev.startswith("UUID="):
                    uuid = dev[5:]
                    r = subprocess.run(["blkid", "-U", uuid], capture_output=True, timeout=5)
                    if r.returncode != 0:
                        fstab_issues.append({"line": i, "issue": "UUID not found", "uuid": uuid})
    except: pass
checks["fstab_issues"] = fstab_issues

result = {
    "module": "boot",
    "priority": "P0",
    "status": "error" if grub_status == "missing" else ("warning" if fstab_issues else "ok"),
    "checks": checks
}
print(json.dumps(result, ensure_ascii=False, indent=2))
PYEOF
