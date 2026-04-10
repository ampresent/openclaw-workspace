# TODO

## Phase 1: 环境准备 ✅

- [x] 确认使用 QEMU TCG（纯软件模拟，无需 KVM）
- [x] 安装 podman (5.8.0)
- [x] 安装 QEMU (10.1.0) + SeaBIOS + OVMF
- [x] 安装 bootc (1.14.1) + ostree + skopeo + osbuild + image-builder
- [x] 修复 OpenSSL 依赖（3.2.2 → 3.5.5）
- [x] 拉取 bootc 基础镜像 `registry.fedoraproject.org/fedora-bootc:41`
- [x] 拉取 bootc 基础镜像 `registry.fedoraproject.org/fedora-bootc:42`
- [ ] 拉取 CentOS Stream 10 / Rocky 10 / Alma 10 基础镜像（不可用，需其他方案）

## Phase 2: 构建自定义 bootc 镜像 ✅

- [x] 编写 Containerfile（基于 Fedora bootc / CentOS 8）
- [x] podman build 并验证镜像 (`localhost/my-bootc:latest`)

## Phase 3: 生成磁盘镜像 ✅

- [x] 运行 image-builder 生成 qcow2（Fedora 41 bootc）
- [x] 验证磁盘分区结构（10GB 虚拟 / 1.2GB 实际）
- [ ] 尝试 raw 格式对比

## Phase 4: QEMU 启动验证 ✅ 基本完成

- [x] 编写 QEMU 启动脚本
- [x] 解决固件兼容性问题 → **OVMF pflash 方案可行**
- [x] QEMU 成功启动 qcow2 镜像 → Fedora 41 完整引导到 login prompt
- [x] 自定义镜像（含 chpasswd + systemctl enable sshd）重新 build + 生成 qcow2
- [x] SSH key 认证登录验证成功（v3 镜像 + SSH 私钥上传）
- [x] 在 VM 内执行 `bootc status` 确认状态
- [x] bootc switch 到 Fedora 42（已完成，staged 镜像已就绪）
- [ ] reboot 验证升级后的系统
- [ ] 验证自定义软件包在新版本中保留

## Phase 5: 进阶探索

- [ ] 对比 bootc vs 传统 ISO 安装的 workflow
- [ ] 探索 bootc 与 runb 的结合可能性
- [ ] 评估生产可行性（国内镜像源限制）
- [ ] 尝试构建 EL10 系 bootc 镜像（CentOS Stream 10 / Rocky 10）
- [ ] 优化镜像传输速度（考虑宿主机内部 registry 或 virtio-9p）

## 已知阻塞

| 问题 | 影响 | 状态 |
|------|------|------|
| quay.io 国内不可访问 | 无法拉取 centos-bootc 和 bootc-image-builder | 已绕过（用 Fedora 源） |
| CentOS 8 EOL | dnf repos 不可用 | 不再使用 |
| Fedora bootc sshd_config 默认禁用密码认证 | chpasswd 设了密码但 SSH 登录仍被拒 | Containerfile 需加 sed 修改 sshd_config |
| EL10 基础镜像不可用 | 无法构建 RHEL10 系 bootc 镜像 | 待调研 |

---
*创建: 2026-04-09 | 更新: 2026-04-10 00:40*
