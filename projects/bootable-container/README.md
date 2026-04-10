# Bootable Container 实验

## 目标

探索 bootc 方案：将 OCI 容器镜像直接转换为可启动的 QEMU/KVM 磁盘镜像（qcow2），验证从「容器定义」到「可启动 VM」的完整闭环。

## 核心流程

```
Containerfile (FROM fedora-bootc:41 + 自定义软件包 + 密码 + sshd 配置)
  ↓ podman build → OCI 镜像 (localhost/my-bootc-fedora:latest)
  ↓ image-builder build qcow2 --bootc-ref → qcow2 磁盘镜像
  ↓ QEMU TCG (OVMF UEFI) → 完整 Linux OS 启动 → SSH 可达
```

## 测试环境

| 项目 | 值 |
|------|------|
| 主机 | 118.195.219.157 (腾讯云) |
| OS | CentOS Stream 10 (Coughlan), kernel 6.12.0 |
| 资源 | 8 核 / 30GB RAM / 50GB 磁盘 |
| 软件模拟 | QEMU TCG（纯软件模拟，无需 KVM） |

### 已安装工具链

| 组件 | 版本 | 用途 |
|------|------|------|
| bootc | 1.14.1 | OCI 镜像 → 可启动系统的框架 |
| image-builder | 51 | 从 bootc OCI 镜像生成 qcow2 |
| osbuild | 174 | 底层镜像构建引擎 |
| ostree | 2025.7 | 不可变文件系统树管理 |
| skopeo | 1.22.0 | OCI 镜像传输/检查 |
| podman | 5.8.0 | 容器构建和管理 |
| QEMU | 10.1.0 | 纯软件模拟（TCG） |
| rpm-ostree | 2026.1 | RPM + OSTree 组合 |
| guestfs-tools | 1.54.0 | virt-customize / guestfish（LIBGUESTFS_BACKEND=direct） |

## 国内镜像源可用性（2026-04-10 实测）

### ✅ 可用

| 镜像 | 注册表 | 备注 |
|------|--------|------|
| `centos:latest` (CentOS 8) | `ccr.ccs.tencentyun.com/library/` | 可拉取，但 repos 已 EOL |
| `fedora-bootc:41` | `registry.fedoraproject.org/` | **完整 bootc 镜像，可直接构建 qcow2** |
| `fedora-bootc:42` | `registry.fedoraproject.org/` | 同上 |

### ❌ 不可用

| 镜像 | 注册表 | 原因 |
|------|--------|------|
| `centos-bootc:stream9/10` | `quay.io/` | 国内网络不通 |
| `bootc-image-builder` | `quay.io/` | 国内网络不通 |
| `centos:*` | `docker.io/` | 国内网络不通 |
| `rockylinux:*` / `almalinux:*` | `ccr.ccs.tencentyun.com/` | 404 不存在 |
| `centos-bootc:*` | `ghcr.io/` | 403 被拒 |

**结论**：国内获取 bootc 镜像的唯一可行源是 `registry.fedoraproject.org/fedora-bootc`。

## 完整操作指南

### 1. 环境准备

```bash
ssh root@118.195.219.157

# 安装工具链
dnf install -y bootc image-builder osbuild ostree skopeo podman qemu-kvm
dnf update -y openssl openssl-libs  # bootc 依赖 OpenSSL 3.5.5+
```

### 2. 拉取基础镜像

```bash
podman pull registry.fedoraproject.org/fedora-bootc:41
```

### 3. 构建自定义 bootc 镜像

```dockerfile
# Containerfile
FROM registry.fedoraproject.org/fedora-bootc:41

# 安装软件包
RUN dnf install -y vim curl openssh-server sudo && dnf clean all

# 启用 SSH 密码登录（Fedora bootc 默认禁用）
RUN sed -i 's/^#*PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config && \
    sed -i 's/^#*PermitRootLogin.*/PermitRootLogin yes/' /etc/ssh/sshd_config

# 启用 sshd 服务
RUN systemctl enable sshd

# 设置 root 密码
RUN echo root:YOUR_PASSWORD | chpasswd
```

```bash
podman build -t localhost/my-bootc-fedora:latest -f Containerfile .
```

**⚠️ 关键注意事项**：
- 必须修改 `sshd_config` 启用 `PasswordAuthentication yes`，否则即使设了密码也无法 SSH 登录
- `systemctl enable sshd` 在 bootc Containerfile 中有效
- ostree 布局下无传统的 `/etc/shadow`，`virt-customize --root-password` 无法识别 OS

### 4. 生成 qcow2 磁盘镜像

```bash
image-builder build qcow2 \
  --bootc-ref localhost/my-bootc-fedora:latest \
  --output-dir /tmp/bootc-output \
  --bootc-default-fs ext4
```

- 不要同时使用 `--bootc-ref` 和 `--distro`（会冲突）
- 输出约 1.2GB（虚拟 10GB），分区为 ostree 布局（/dev/sda2-4）

### 5. QEMU UEFI 启动（已验证）

```bash
# 准备 OVMF VARS（可写副本）
cp /usr/share/edk2/ovmf/OVMF_VARS.fd /tmp/OVMF_VARS.fd

# 启动 VM
/usr/libexec/qemu-kvm \
  -M pc -cpu Haswell-v4 -smp 2 -m 4096 \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2/ovmf/OVMF_CODE.fd \
  -drive if=pflash,format=raw,file=/tmp/OVMF_VARS.fd \
  -drive file=/tmp/bootc-output/bootc-fedora-41-qcow2-x86_64.qcow2,format=qcow2,if=virtio \
  -netdev user,id=net0,hostfwd=tcp::2222-:22 \
  -device virtio-net-pci,netdev=net0 \
  -nographic -serial file:/tmp/qemu-serial.log -snapshot
```

**关键参数说明**：
- `-drive if=pflash,readonly=on,file=OVMF_CODE.fd` — UEFI 固件代码（只读）
- `-drive if=pflash,file=OVMF_VARS.fd` — UEFI 变量存储（需可写副本）
- `-snapshot` — 保护原始 qcow2 不被修改
- `-serial file:/tmp/qemu-serial.log` — 串口输出到文件（非交互模式）
- `-nographic` — 无图形，纯串口
- `hostfwd=tcp::2222-:22` — 主机 2222 端口转发到 VM 的 22

**❌ 错误用法**：
- `-bios OVMF_CODE.fd` → 报错 "could not load PC BIOS"
- `-bios bios-256k.bin`（SeaBIOS）→ 能启动但 ostree 引导不完整

### 6. SSH 连入 VM

```bash
# 从 QEMU 宿主机执行
ssh -p 2222 root@127.0.0.1

# 验证
uname -a
bootc status
cat /etc/os-release
```

### 7. bootc 升级测试（在 VM 内）

```bash
# 拉取新镜像并切换
bootc switch registry.fedoraproject.org/fedora-bootc:42
reboot
```

## 架构要点

### ostree 分区布局

bootc qcow2 使用 ostree 不可变文件系统，分区结构：

```
/dev/sda1  — EFI 系统分区 (ESP)
/dev/sda2  — boot 分区
/dev/sda3  — ostree 物理根（不可变）
/dev/sda4  — ostree 部署根（实际挂载为 /）
```

根目录结构：
```
/
├── .bootc-aleph.json   # bootc 元数据
├── boot/               # 内核/initramfs
└── ostree/
    ├── deploy/         # 部署的系统树
    ├── repo/           # ostree 仓库
    └── boot.1/         # 引导条目
```

### 与传统方案的差异

| 维度 | 传统 ISO 安装 | bootc 容器 |
|------|--------------|------------|
| 定义方式 | kickstart / cloud-init | Containerfile |
| 更新方式 | dnf update（增量） | bootc switch（原子替换） |
| 回滚 | 困难 | ostree 自动保留旧部署 |
| 镜像格式 | qcow2 / raw 直接构建 | OCI → qcow2 两步转换 |
| 可复现性 | 低（状态漂移） | 高（不可变文件系统） |

## 已知限制

| 问题 | 影响 | 状态 |
|------|------|------|
| quay.io 国内不可访问 | 无法拉取 centos-bootc | 用 Fedora 源绕过 |
| EL10 基础镜像不可用 | 无法构建 RHEL10 系 bootc | 待调研 |
| Fedora bootc sshd 默认禁用密码登录 | chpasswd 设了密码仍无法 SSH | Containerfile 中 sed 修改 sshd_config |
| ostree 布局下 virt-customize 不可用 | 无法事后修改镜像 | 所有定制必须在 Containerfile 中完成 |

## 文件清单

```
projects/bootable-container/
├── README.md          ← 本文件（方案设计 + 操作指南）
├── TODO.md            ← 任务进度跟踪
├── STATUS.md          ← 断点续传状态快照
├── EXPERIMENTS.md     ← 实验日志（按时间记录每次尝试）
├── Containerfile      ← 自定义 bootc 镜像定义（本地参考版）
├── ssh-cmd.py         ← 远程同步执行（SSH wrapper）
└── ssh-bg.py          ← 远程异步执行
```

## 参考

- [bootc 官方文档](https://containers.github.io/bootc/)
- [Fedora bootc 文档](https://docs.fedoraproject.org/en-US/bootc/)
- [osbuild / image-builder](https://www.osbuild.org/)
- [Fedora bootc 镜像](https://registry.fedoraproject.org/repo/fedora-bootc)

---
*创建: 2026-04-09 | 更新: 2026-04-10 02:05*
