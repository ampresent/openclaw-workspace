# 实验日志

## 2026-04-10 00:00-00:40 — 首次搭建 + 镜像拉取测试

### 环境搭建
- SSH 到 118.195.219.157 成功（CentOS Stream 10）
- 安装 bootc + image-builder + osbuild + skopeo + ostree + podman + qemu-kvm
- 遇到 OpenSSL 3.2.2 不兼容 bootc，通过 `dnf update openssl` 升级到 3.5.5 解决

### 镜像拉取测试

| 镜像 | 注册表 | 结果 |
|------|--------|------|
| `centos:latest` | ccr.ccs.tencentyun.com | ✅ CentOS 8 (239MB) |
| `centos:stream9` | ccr.ccs.tencentyun.com | ❌ manifest unknown |
| `centos:stream10` | ccr.ccs.tencentyun.com | ❌ manifest unknown |
| `centos-bootc:stream9` | quay.io | ❌ 网络超时 |
| `bootc-image-builder` | quay.io | ❌ 网络超时 |
| `centos:*` | docker.io | ❌ 网络超时 |
| `centos:*` | registry.cn-hangzhou.aliyuncs.com | ❌ 403 |
| `rockylinux:*` | ccr.ccs.tencentyun.com | ❌ 404 |
| `almalinux:*` | ccr.ccs.tencentyun.com | ❌ 404 |
| `centos-bootc:*` | ghcr.io | ❌ 403 |
| `fedora-bootc:41` | registry.fedoraproject.org | ✅ (1.86GB) |
| `fedora-bootc:42` | registry.fedoraproject.org | ✅ (1.88GB) |

### 构建测试
- 用 `centos:latest` (CentOS 8) 构建自定义镜像成功
- 但 CentOS 8 repos 已 EOL，无法安装新软件（vault.centos.org 可以但慢）
- 使用 `image-builder build qcow2 --bootc-ref fedora-bootc:41` 成功生成 qcow2
  - 虚拟大小: 10GB
  - 实际大小: 1.2GB

### QEMU 启动测试
- ❌ `-bios /usr/share/edk2/ovmf/OVMF_CODE.fd` — 直接用 OVMF fd 作为 BIOS 参数会报错
- ❌ `-bios /usr/share/seabios/bios-256k.bin` — SeaBIOS 启动但 qcow2 未完整引导（待调试）
- 正确用法：OVMF 需用 `-drive if=pflash,format=raw,readonly=on,file=...` 方式挂载

### 关键结论
1. **国内获取 bootc 镜像的唯一可行方案**：`registry.fedoraproject.org/fedora-bootc`
2. **不需要 bootc-image-builder 容器**：系统级 `image-builder` 命令可直接用 `--bootc-ref` 构建
3. **EL10 系基础镜像在国内不可用**：腾讯云 registry 无 Rocky/Alma，quay.io 无网络
4. **下一步**：解决 QEMU UEFI 固件挂载方式，完成 VM 启动验证

---
*记录: 2026-04-10 00:40*

## 2026-04-10 01:10-01:26 — QEMU UEFI 启动 + SSH 探索

### UEFI 启动（成功 ✅）
- 正确方式：`-drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd` + `-drive if=pflash,format=raw,file=OVMF_VARS.fd`
- 需要 CODE（只读）+ VARS（可写）两个 pflash 驱动
- 错误方式：`-bios OVMF_CODE.fd` 会报 "could not load PC BIOS"
- 错误方式：`-bios bios-256k.bin`（SeaBIOS）能启动但 qcow2 引导不完整

### VM 启动完整流程验证（成功 ✅）
- UEFI → GRUB 2.12 → Fedora Linux 41.20251120.0 → kernel 6.17.8 → systemd → multi-user.target
- SSH 服务启动、网络就绪 (ens3: 10.0.2.15)
- serial 输出完整，到达 `fedora login:` prompt
- 使用 `-snapshot` 模式保护原始 qcow2

### SSH 登录阻塞（待解决）
- Fedora bootc 镜像无默认 root 密码
- `virt-customize --root-password` 失败：libguestfs 无法识别 ostree 布局
  - `virt-filesystems` 能看到 /dev/sda2-4
  - guestfish mount /dev/sda4 后只有 `.bootc-aleph.json`、`boot/`、`ostree/`
  - 无 `/etc/shadow`（ostree deploy 路径不同）
- cloud-init ISO 方案：远程缺 genisoimage
- **推荐方案**：在 Containerfile 中 `RUN echo 'root:bootc' | chpasswd`，重新 build 镜像 + 重新生成 qcow2

### 新安装工具
- `guestfs-tools-1.54.0`（virt-customize, virt-filesystems, guestfish）
- `libguestfs-xfs-1:1.58.1`
- 需设置 `LIBGUESTFS_BACKEND=direct`（无 libvirt daemon）

---
*记录: 2026-04-10 01:26*

## 2026-04-10 01:30-02:04 — 自定义镜像重建 + SSH 密码认证探索

### 自定义镜像重新构建（成功 ✅）
- Containerfile（v1）基于 `registry.fedoraproject.org/fedora-bootc:41`
- 安装 vim curl openssh-server sudo
- `RUN echo root:Bebop4life | chpasswd`
- `RUN systemctl enable sshd`
- `podman build -t localhost/my-bootc-fedora:latest` 成功
- `image-builder build qcow2 --bootc-ref localhost/my-bootc-fedora:latest` 成功生成 1.2GB qcow2

### QEMU 启动（成功 ✅）
- 同样使用 OVMF pflash 方案
- Fedora 41 完整引导：UEFI → GRUB → kernel → systemd → multi-user → login prompt
- sshd 服务启动（serial log 确认 `Started sshd.service`）

### SSH 登录失败（待修复 ❌）
- 从远程 `ssh -p 2222 root@127.0.0.1` 连接成功（端口通）
- 输入密码 `Bebop4life` → Permission denied
- 输入密码 `Bebop4life&` → Permission denied
- **推测根因**：Fedora bootc 默认 sshd_config 中 `PasswordAuthentication no`，chpasswd 设了密码但 sshd 拒绝密码登录
- **修复方案**：Containerfile v2 中增加：
  ```
  RUN sed -i 's/^#*PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config && \
      sed -i 's/^#*PermitRootLogin.*/PermitRootLogin yes/' /etc/ssh/sshd_config
  ```
- v2 Containerfile 已写好到远程 `/tmp/Containerfile.v2`，待 build

### 新安装工具
- `guestfs-tools-1.54.0`（virt-customize, virt-filesystems, guestfish）
- 需设置 `LIBGUESTFS_BACKEND=direct`（无 libvirt daemon）

---

## 2026-04-10 02:08-02:37 — sshd 修复 + image-builder 调试

### Containerfile v2 修复 sshd 密码认证

**根因**：Fedora bootc 的 sshd_config 中 `#PasswordAuthentication no` 注释状态 + drop-in 文件无 `PasswordAuthentication` 设置 → 默认禁用密码登录。

**修复**：在 `/etc/ssh/sshd_config.d/02-password-auth.conf` 中写入：
```
PasswordAuthentication yes
PermitRootLogin yes
```

⚠️ 不要用 `sed -i` 修改主 sshd_config —— 要用 drop-in 文件。

### image-builder `/` 和 `:` 参数解析 Bug

**现象**：`image-builder build qcow2 --bootc-ref localhost/my-bootc-fedora:v2` 报 `error: accepts 1 arg(s), received 2`

**根因**：cobra/pflag 库对含 `/` 和 `:` 的 flag 值解析错误。`--bootc-ref a/b:c` 中 `/` 和 `:` 导致 pflag 把值的一部分解析为独立的 positional arg。

**修复**：给整个 `--flag=value` 加双引号保护：
```bash
# ❌ 失败
image-builder build qcow2 --bootc-ref localhost/my-bootc-fedora:v2
# ✅ 成功
image-builder build qcow2 "--bootc-ref=localhost/my-bootc-fedora:v2" "--bootc-default-fs=ext4" --output-dir /tmp/bootc-output
```

**调试过程**：
- `a/b` → 通过（无 `:`）
- `a/b:c` → 失败（`/` + `:` 组合触发）
- `--bootc-ref=test` → 通过（无特殊字符）
- strace 确认 execve 参数正确 → 是 cobra/pflag 库层的 bug

### SSH 密码登录 PAM:bad_ident 问题

**现象**：VM 启动成功，sshd 运行，但 `ssh -p 2222 root@127.0.0.1` 密码登录失败。

**审计日志**：
```
PAM:bad_ident grantors=? acct="?" res=failed
op=login acct="root" res=failed
```

**分析**：
- sshd_config.d/02-password-auth.conf 正确（已确认）
- shadow 文件密码哈希正确（su 在容器内验证密码有效）
- PAM password-auth 配置标准
- 问题可能是 ostree 布局下 PAM 模块读取用户数据库异常

**解决方案**：改用 SSH key 认证绕过 PAM 密码验证。Containerfile v3 中 COPY bootc-key.pub → authorized_keys。

### qcow2 重新生成成功

- 镜像 `localhost/my-bootc-fedora:v2` 构建成功
- qcow2 重新生成成功（约 100 秒 osbuild 流水线）
- QEMU 启动 → Fedora 41 完整引导到 login prompt ✅

---
*记录: 2026-04-10 02:37*

## 2026-04-10 11:03-11:26 — v3 镜像重建 + SSH key 验证 + bootc switch 测试

### 背景
Gateway 重启后恢复项目上下文，从 STATUS.md 的"下次检查清单"继续。

### v3 镜像确认
- 远程已有 `localhost/my-bootc-fedora:v3`（含 sshd_config.d drop-in + SSH 公钥注入）
- 但之前运行的 VM 使用的是 v2 版 qcow2（无 SSH key）
- 需要用 v3 重新生成 qcow2

### SSH 私钥上传
- 本地 `projects/bootable-container/bootc-key`（ed25519 私钥）上传到远程 `/tmp/bootc-key`
- `chmod 600 /tmp/bootc-key`

### qcow2 重新生成（v3）
- `image-builder build qcow2 "--bootc-ref=localhost/my-bootc-fedora:v3" "--bootc-default-fs=ext4" --output-dir /tmp/bootc-output`
- 用时约 100 秒，成功生成 1.2GB qcow2
- 注意：需先 `kill` 旧 QEMU 进程

### SSH key 认证 — 突破 ✅
- 旧 QEMU 杀掉，用 v3 qcow2 重新启动 QEMU
- 等待约 100 秒 VM 完全引导
- `ssh -i /tmp/bootc-key -p 2222 root@127.0.0.1` → **成功！**
- `uname -a` → `Linux fedora 6.17.8-100.fc41.x86_64`
- **关键**：PAM 密码认证在 ostree 布局下确实不可用，SSH key 是正确方案

### bootc status 验证 ✅
```
apiVersion: org.containers.bootc/v1
kind: BootcHost
status:
  booted:
    image:
      image: localhost/my-bootc-fedora:v3
      version: 41.20251120.0
```

### bootc switch 测试（进行中 ⏳）
- 命令：`bootc switch registry.fedoraproject.org/fedora-bootc:42`
- 需要拉取 65 层 / 1.0 GB
- QEMU TCG + 用户态网络，速度较慢
- 运行约 10 分钟仍未完成，进程仍活跃（PID 1130，CPU 83.8%）
- **预估**：还需 10-15 分钟完成拉取

### 经验总结
1. **每次重启 VM（新 qcow2）后**，远程宿主机的 `~/.ssh/known_hosts` 会冲突，需 `ssh-keygen -R [127.0.0.1]:2222`
2. **pexpect 嵌套 SSH** 时，prompt 匹配容易出错，`bootc status` 等长输出命令需要更长 timeout
3. **bootc switch 通过 QEMU 用户态网络**拉取大镜像非常慢，考虑后续在宿主机预先拉取镜像，再通过 virtio-9p 或内部 registry 传递

---
*记录: 2026-04-10 11:26*

## 2026-04-10 12:06-12:13 — bootc switch 完成 + 新 session 恢复

### 背景
Gateway 重启后新 session 启动，从 STATUS.md/EXPERIMENTS.md 恢复项目上下文。

### bootc switch 结果 — 成功 ✅
- `bootc switch registry.fedoraproject.org/fedora-bootc:42` 于 04:13 UTC 完成
- 总耗时约 19 分钟（03:54 → 04:13 UTC）
- 65 层拉取 + 部署（QEMU TCG + 用户态网络）
- `bootc status` 确认：
  - staged: `registry.fedoraproject.org/fedora-bootc:42` version `42.20260409.0`
  - booted: `localhost/my-bootc-fedora:v3` (Fedora 41)
- qcow2 从 1.2GB 增长到 2.7GB（含 staged 镜像数据）

### 待执行
- 重启 QEMU（去掉 `-snapshot`），reboot VM 验证 Fedora 42 升级

---
*记录: 2026-04-10 12:13*

## 2026-04-10 12:06-12:55 — Reboot 验证 + Fedora 42 升级成功

### 关键发现：-snapshot 导致数据丢失
- 之前所有 QEMU 操作（包括 bootc switch）都带 `-snapshot`，写入到 overlay 文件而非 qcow2
- 杀掉 QEMU 后 overlay 丢弃，staged Fedora 42 镜像丢失
- **教训**：需要持久化的操作必须在无 -snapshot 的 QEMU 中执行

### 修复过程
1. 杀掉旧 QEMU（PID 267962，带 -snapshot）
2. 重启 QEMU（PID 275823，无 -snapshot）
3. SSH 验证：staged=null，仅 Fedora 41
4. 重新执行 `bootc switch registry.fedoraproject.org/fedora-bootc:42`
   - 层已缓存（"No changes"），直接 Deploy
   - Deploy 耗时 4 分钟（QEMU TCG）
5. reboot VM → Fedora 42 (Adams) 启动成功 ✅
   - bootc status: booted=fedora-bootc:42, version=42.20260409.0
   - os-release: Fedora Linux 42 (Adams)

### Phase 5 完成
- bootc switch: ✅
- bootc reboot: ✅
- Fedora 41 → 42 原子升级验证通过

### 待探索
- bootc update（pull 更新版本 → reboot）


---
*记录: 2026-04-10 12:55*

## 2026-04-10 13:31-13:37 — bootc update 检查 + rollback 验证

### bootc update --check — 机制验证 ✅
- 命令：`bootc update --check`
- 结果：`No changes in: docker://registry.fedoraproject.org/fedora-bootc:42`
- 当前 booted 镜像已是最新，无可用更新
- **结论**：update 机制正常工作

### bootc rollback — 完整回滚验证 ✅

**操作流程**：
1. 执行 `bootc rollback` → `Next boot: rollback deployment`
2. reboot VM（约 90 秒，QEMU TCG）
3. SSH 验证：已回退到 Fedora 41 (my-bootc-fedora:v3)

**rollback 后状态**：
- booted: `localhost/my-bootc-fedora:v3` (Fedora 41.20251120.0)
- rollback: `registry.fedoraproject.org/fedora-bootc:42` (Fedora 42.20260409.0)

**关键发现**：
- rollback 后之前的 booted 自动成为新的 rollback 目标
- 可反复 rollback/switch 形成双版本跷跷板切换
- rollback 是 boot 级操作，reboot 后生效

### 完整升级/回滚循环
```
Fedora 41 (v3) → bootc switch → reboot → Fedora 42
  → bootc rollback → reboot → Fedora 41 (v3)
```

**Phase 5 全部完成**：
- ✅ bootc switch（跨版本升级）
- ✅ bootc reboot（新版本激活）
- ✅ bootc rollback（一键回滚）
- ✅ bootc update --check（更新检查）

---
*记录: 2026-04-10 13:37*
