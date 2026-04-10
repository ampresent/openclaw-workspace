# Bootable Container — 项目状态

> **用途**：下次 session 继续工作时，读此文件即可了解全部上下文，无需重复探索。

## 最后更新：2026-04-10 11:26

---

## 远程测试后端

| 项目 | 值 |
|------|------|
| IP | `118.195.219.157` |
| 用户 | `root` |
| 密码 | 见 `ssh-cmd.py` |
| OS | CentOS Stream 10 (Coughlan), kernel 6.12.0 |
| 资源 | 8 核 / 30GB RAM / 50GB 磁盘 |
| SSH 工具 | `projects/bootable-container/ssh-cmd.py`（同步）, `ssh-bg.py`（异步） |

### 连接方式

```bash
cd ~/.openclaw/workspace/projects/bootable-container
python3 ssh-cmd.py "你想执行的命令"
```

---

## 已安装工具链（远程机器）

全部通过 `dnf install` 安装，无需手动编译。

| 组件 | 版本 | 二进制路径 |
|------|------|-----------|
| bootc | 1.14.1 | `/usr/bin/bootc` |
| image-builder | 51 | `/usr/bin/image-builder` |
| osbuild | 174 | `/usr/bin/osbuild` |
| ostree | 2025.7 | `/usr/bin/ostree` |
| skopeo | 1.22.0 | `/usr/bin/skopeo` |
| podman | 5.8.0 | `/usr/bin/podman` |
| qemu-kvm | 10.1.0 | `/usr/libexec/qemu-kvm` |
| rpm-ostree | 2026.1 | `/usr/bin/rpm-ostree` |
| OpenSSL | 3.5.5 | `/usr/bin/openssl` |

**⚠️ 重要**：bootc 依赖 OpenSSL 3.5.5+，安装后必须 `dnf update openssl openssl-libs`。

---

## 已拉取镜像

| 镜像 | 大小 | 状态 |
|------|------|------|
| `registry.fedoraproject.org/fedora-bootc:41` | 1.86GB | ✅ 可用于构建 qcow2 |
| `registry.fedoraproject.org/fedora-bootc:42` | 1.88GB | ✅ 最新 |
| `localhost/my-bootc-fedora:v2` | ~1.94GB | ✅ 自定义构建（含 sshd 修复） |
| `localhost/my-bootc:latest` | 351MB | ✅ CentOS 8 基础（旧版） |

---

## 已生成产物

| 文件 | 大小 | 位置 | 镜像版本 |
|------|------|------|----------|
| `bootc-fedora-41-qcow2-x86_64.qcow2` | ~1.2GB | 远程 `/tmp/bootc-output/` | v3 (含 SSH key)

---

## 完成进度

### Phase 1: 环境准备 ✅ 全部完成

所有工具已安装，基础镜像已拉取。

### Phase 2: 构建自定义 bootc 镜像 ✅ 完成 (v2)

- Fedora bootc 41 基础
- Containerfile 含 sshd drop-in 密码认证配置
- SSH key 公钥注入（`bootc-key.pub`）
- `podman build -t localhost/my-bootc-fedora:v2`

### Phase 3: 生成磁盘镜像 ✅ 完成

**已解决的关键问题**：`image-builder` 的 cobra/pflag 库对含 `/` 和 `:` 的 flag 值解析有 bug。

```bash
# ❌ 错误 — cobra 会把 / 或 : 后面的部分解析为独立的 positional arg
image-builder build qcow2 --bootc-ref localhost/my-bootc-fedora:v2

# ✅ 正确 — 给整个 flag=value 加双引号保护
image-builder build qcow2 "--bootc-ref=localhost/my-bootc-fedora:v2" "--bootc-default-fs=ext4" --output-dir /tmp/bootc-output
```

### Phase 4: QEMU 启动验证 ✅ 基本完成

**已解决**：
- ✅ QEMU UEFI 固件加载（OVMF pflash 方案）
- ✅ Fedora 41 完整引导到 login prompt
- ✅ SSH 服务启动
- ✅ qcow2 重新生成（v3 镜像，含 sshd 修复 + SSH key 注入）
- ✅ **SSH key 认证登录成功**（PAM 密码认证在 ostree 布局下不可用，key 认证绕过）
- ✅ `bootc status` 验证 — 确认 VM 运行 `localhost/my-bootc-fedora:v3`
- ✅ `bootc switch` 到 Fedora 42 完成 — staged 镜像已就绪，待 reboot 激活

### Phase 5: 进阶探索 ❌ 未开始

- bootc update 测试（等待 switch 完成）
- runb 集成探索
- EL10 系镜像构建

---

## 已知问题与修复

| 问题 | 根因 | 修复 |
|------|------|------|
| image-builder 报 `accepts 1 arg(s), received 2` | cobra/pflag 对含 `/` 和 `:` 的值解析错误 | 给 `--flag=value` 加双引号 |
| sshd_config sed 不生效 | Fedora bootc 用 drop-in 覆盖主配置 | 在 `sshd_config.d/` 创建 `02-password-auth.conf` |
| SSH 密码登录 `PAM:bad_ident` | ostree 布局下 PAM 兼容性问题 | SSH key 认证（Containerfile 注入公钥，私钥上传到远程 `/tmp/bootc-key`） | ✅ 已修复 |
| virt-customize 无法修改 ostree 镜像 | libguestfs 不识别 ostree OS | 所有定制必须在 Containerfile 中完成 |
| quay.io 国内不可访问 | 网络限制 | 用 `registry.fedoraproject.org` 替代 |

---

## SSH Key 认证

本地密钥对：`projects/bootable-container/bootc-key`（ed25519）
公钥已注入镜像（Containerfile COPY bootc-key.pub → authorized_keys）

```bash
# 从远程宿主机 SSH 到 VM
ssh -i bootc-key -p 2222 -o StrictHostKeyChecking=no root@127.0.0.1
```

---

## 最新进展 (2026-04-10 12:13)

### bootc switch 到 Fedora 42 — 成功 ✅

- `bootc switch registry.fedoraproject.org/fedora-bootc:42` 完成
- 65 层 / 1.0 GB，部署耗时约 19 分钟（QEMU 用户态网络）
- Fedora 42.20260409.0 已 staged，待 reboot 激活
- **当前 VM 状态**：
  - booted: `localhost/my-bootc-fedora:v3` (Fedora 41)
  - staged: `registry.fedoraproject.org/fedora-bootc:42`

### ⚠️ 下一步：reboot 验证

当前 QEMU 使用 `-snapshot` 参数，reboot 会丢弃所有写入（包括 staged 镜像）。需要：
1. 杀掉当前 QEMU 进程
2. 用 **去掉 `-snapshot`** 的命令重启 QEMU
3. SSH 进 VM 执行 `reboot`
4. 验证 `bootc status` 显示 Fedora 42 已 boot

---

## 下次继续时的检查清单

1. [x] SSH 连通性测试
2. [x] 检查远程机器状态：镜像、qcow2 文件是否还在
3. [x] 用 SSH key 登录 VM 验证成功
4. [x] 在 VM 内执行 `bootc status` 确认 bootc 状态
5. [x] 等 `bootc switch` 完成（Fedora 42 staged ✅）
6. [ ] 重启 QEMU（去掉 -snapshot）→ reboot VM → 验证 Fedora 42 升级
7. [ ] 测试 bootc update 流程（pull 新镜像 → reboot → 验证）
8. [ ] 探索 runb 集成可能性
9. [ ] 更新文档并 commit + push

---

*更新: 2026-04-10 12:13*
