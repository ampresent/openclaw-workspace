# Bootable Container — 项目状态

> **用途**：下次 session 继续工作时，读此文件即可了解全部上下文，无需重复探索。

## 最后更新：2026-04-10 16:42

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
| guestfs-tools | 1.54.0 | `/usr/bin/virt-customize` |
| OpenSSL | 3.5.5 | `/usr/bin/openssl` |

**⚠️ 重要**：bootc 依赖 OpenSSL 3.5.5+，安装后必须 `dnf update openssl openssl-libs`。

---

## 已拉取镜像

| 镜像 | 大小 | 状态 |
|------|------|------|
| `registry.fedoraproject.org/fedora-bootc:41` | 1.86GB | ✅ rollback 镜像 |
| `registry.fedoraproject.org/fedora-bootc:42` | 1.88GB | ✅ 当前 booted |
| `localhost/my-bootc-fedora:v2` | ~1.94GB | ✅ 自定义构建（含 sshd 修复） |
| `localhost/my-bootc-fedora:v3` | ~1.94GB | ✅ 含 sshd 修复 + SSH key 注入 |

---

## 已生成产物

| 文件 | 大小 | 位置 |
|------|------|------|
| `bootc-fedora-41-qcow2-x86_64.qcow2` | 2.9GB | 远程 `/tmp/bootc-output/` |

---

## 完成进度

### Phase 1: 环境准备 ✅
### Phase 2: 自定义 bootc 镜像 ✅
### Phase 3: 磁盘镜像生成 ✅
### Phase 4: QEMU 启动验证 ✅
### Phase 5: bootc 生命周期验证 ✅

已验证的完整生命周期：

| 操作 | 状态 | 详情 |
|------|------|------|
| `bootc switch` | ✅ | Fedora 41 (v3) → Fedora 42，65 层 / 1GB / ~19 分钟 |
| `bootc reboot` | ✅ | Fedora 42 启动成功 |
| `bootc rollback` | ✅ | Fedora 42 → Fedora 41 (v3)，reboot 后生效 |
| `bootc update --check` | ✅ | 机制正常，当前无新版本 |

**升级回滚完整循环**：
```
Fedora 41 (v3) → bootc switch → Fedora 42 → bootc rollback → Fedora 41 (v3)
```

当前 VM 状态（2026-04-10 13:37）：
- **booted**: `localhost/my-bootc-fedora:v3` (Fedora 41)
- **rollback**: `registry.fedoraproject.org/fedora-bootc:42` (Fedora 42)
- 可随时 `bootc switch` 回 Fedora 42

### PoC 进展（按 PLAN.md 顺序）

| 编号 | 方向 | 状态 | 交付物 |
|------|------|------|--------|
| E | 生产可行性评估 | ✅ 完成 | `PRODUCTION-FEASIBILITY.md` |
| F | 自建 bootc 镜像 | ⏳ 下一步 | `docs/bootc-image-from-scratch.md` |
| A | Drift Detection | 待做 | `scripts/drift-check.sh` |
| B | Ring 部署 | 待做 | `scripts/ring-deploy.sh` |
| C | CI/CD | 待做 | `.github/workflows/build-image.yml` |

### 待办

| 任务 | 状态 |
|------|------|
| EL10 系镜像构建 | ❌ 阻塞（quay.io 国内不可访问，无替代源） |

---

## 已知问题与修复

| 问题 | 根因 | 修复 |
|------|------|------|
| image-builder 报 `accepts 1 arg(s), received 2` | cobra/pflag 对含 `/` 和 `:` 的值解析错误 | 给 `--flag=value` 加双引号 |
| sshd_config sed 不生效 | Fedora bootc 用 drop-in 覆盖主配置 | `sshd_config.d/02-password-auth.conf` |
| SSH 密码登录 `PAM:bad_ident` | ostree 布局下 PAM 兼容性问题 | SSH key 认证绕过 |
| virt-customize 无法修改 ostree 镜像 | libguestfs 不识别 ostree OS | 所有定制在 Containerfile 中完成 |
| quay.io 国内不可访问 | 网络限制 | 用 `registry.fedoraproject.org` 替代 |
| `-snapshot` 导致 switch 数据丢失 | QEMU snapshot 模式丢弃写入 | 持久化操作必须去掉 `-snapshot` |

---

## SSH Key 认证

本地密钥对：`projects/bootable-container/bootc-key`（ed25519）
公钥已注入镜像（Containerfile COPY bootc-key.pub → authorized_keys）

```bash
ssh -i bootc-key -p 2222 -o StrictHostKeyChecking=no root@127.0.0.1
```

---

## 已知限制

| 问题 | 影响 | 状态 |
|------|------|------|
| quay.io 国内不可访问 | 无法拉取 centos-bootc | 用 Fedora 源绕过 |
| EL10 基础镜像不可用 | 无法构建 RHEL10 系 bootc | 阻塞 |
| Fedora bootc sshd 默认禁用密码登录 | chpasswd 设了密码仍无法 SSH | Containerfile 中 drop-in 修复 |
| ostree 布局下 virt-customize 不可用 | 无法事后修改镜像 | 所有定制必须在 Containerfile 中完成 |
| QEMU TCG 性能差 | bootc switch ~19 分钟 | 仅影响测试环境，生产用 KVM/裸机 |

---

*更新: 2026-04-10 13:37*
