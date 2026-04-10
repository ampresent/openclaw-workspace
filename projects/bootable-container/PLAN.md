# Bootable Container — 后续开发计划

**状态**：Draft
**创建**：2026-04-10 14:01
**类型**：快速 PoC（1-2 session 完成）

## 当前基线

Phase 1-5 全部完成。已验证完整的 bootc 生命周期（build → qcow2 → boot → switch → rollback）。

## 计划总览

| 编号 | 方向 | 交付物 | 预估 session |
|------|------|--------|-------------|
| A | Drift Detection | drift-check.sh + 测试报告 | 1 |
| B | Ring 部署 | ring-deploy.sh + QEMU Ring 0 验证 | 1 |
| C | CI/CD | GitHub Actions workflow | 1 |
| E | 生产可行性评估 | PRODUCTION-FEASIBILITY.md | 1 |
| F | 自建 bootc 镜像 | Containerfile + 构建指南 | 1 |

## 执行顺序

```
E (评估先行，决定投入)
 ↓
F (自建镜像能力，解锁 EL10 路径)
 ↓
A (drift detection 核心能力)
 ↓
B (ring 部署，结合 A 的检测结果)
 ↓
C (CI/CD 自动化前面所有流程)
```

理由：E 先评估值不值得投入；F 解锁底层能力；A 和 B 是业务价值；C 是自动化收尾。

---

## PoC A: Drift Detection

### 目标
在 VM 中验证：手动修改配置 → 脚本检测到偏离 → 输出 JSON 报告。

### 方案

```
drift-check.sh --snapshot <输出文件>        # 采集当前状态
drift-check.sh --baseline <基线文件>        # 与基线对比，输出 drift 报告
```

**采集维度**：
1. `/etc` 关键文件 hash（sshd_config, passwd, shadow, resolv.conf 等）
2. 已安装包列表（`rpm -qa`）
3. 运行中 systemd 服务（`systemctl list-units --type=service --state=running`）
4. 监听端口（`ss -tlnp`）

**输出格式**：JSON，包含 file/package/service/port 四类 drift。

### 测试流程

1. 在 VM 内生成基线 snapshot
2. 手动修改 `/etc/ssh/sshd_config`（加一行注释）
3. 手动安装一个包（`dnf install -y htop`）
4. 跑 drift-check 对比 → 验证检测到 2 个 drift
5. 记录结果到 EXPERIMENTS.md

### 交付物
- `scripts/drift-check.sh`
- EXPERIMENTS.md 中的测试记录

---

## PoC B: Ring 部署

### 目标
验证 Ring 0（QEMU 预演）→ Ring 1（canary）→ 全量的渐进发布脚本。

### 方案

**ring-deploy.sh** 流程：

```
ring-deploy.sh --image <新镜像> --rings 0,1,all
```

| Ring | 动作 | 健康检查 | 失败处理 |
|------|------|----------|----------|
| 0 | QEMU TCG 启动临时 VM，bootc switch + reboot | SSH 连通 + bootc status + 自定义检查 | 中止 + 告警 |
| 1 | 目标机器 bootc switch + reboot（canary 5%） | 同上 + 观察 30 min | 自动 rollback |
| all | 全量机器 bootc switch + reboot | 同上 | 自动 rollback |

**PoC 范围**：只验证 Ring 0（QEMU 预演），因为目前只有 1 台远程机器。

### 测试流程

1. 在远程机器上执行 `ring-deploy.sh --image fedora-bootc:42 --ring 0`
2. 脚本自动：启动 QEMU → bootc switch → reboot → 健康检查 → 报告
3. 验证预演报告格式和退出码

### 交付物
- `scripts/ring-deploy.sh`
- EXPERIMENTS.md 中的测试记录

---

## PoC C: CI/CD Pipeline

### 目标
GitHub Actions workflow：push Containerfile 变更 → 自动构建 → 生成 qcow2 → 上传 artifact。

### 方案

**触发条件**：`base/Containerfile` 或 `overlay/*/Containerfile` 变更。

**Pipeline 步骤**：
1. Checkout
2. `podman build -t bootc-custom:$SHA -f base/Containerfile`
3. `podman save bootc-custom:$SHA | gzip > image.tar.gz`
4. Upload artifact（image tar + qcow2 元数据）

**限制**：GitHub Actions 无 KVM，qcow2 生成需在有 osbuild 的环境中。PoC 只做 build + artifact，不跑 image-builder。

### 交付物
- `.github/workflows/build-image.yml`

---

## PoC E: 生产可行性评估

### 目标
一份文档，回答「bootc 方案是否值得投入生产」。

### 评估维度

| 维度 | 评估内容 |
|------|----------|
| 镜像源 | 国内可用的 bootc 基础镜像有哪些？自建路径？ |
| 构建速度 | QEMU TCG 下 switch ~19 min，KVM 预估？裸机预估？ |
| 运维复杂度 | 对比传统 ISO 安装 / cloud-init 方案 |
| 回滚能力 | bootc rollback vs 传统方案 |
| 安全性 | 不可变文件系统、镜像签名、drift detection |
| 生态成熟度 | bootc 社区活跃度、文档质量、企业采用案例 |
| 国内落地 | 镜像源、网络、合规性 |

### 交付物
- `PRODUCTION-FEASIBILITY.md`

---

## PoC F: 自建 bootc 镜像

### 目标
不依赖 quay.io 的 centos-bootc 镜像，从普通 Fedora 镜像改造为 bootc 可用镜像。

### 核心挑战

bootc 镜像 = OCI 镜像 + ostree 文件系统布局 + bootc 元数据标签。
普通 Fedora 镜像无 ostree 布局。

### 方案一：基于 Fedora bootc 扩展（推荐，实际可行）

利用已有的 `registry.fedoraproject.org/fedora-bootc:41`（国内可达），在其上叠加自定义层：

```dockerfile
FROM registry.fedoraproject.org/fedora-bootc:41
# 添加自定义软件包、服务、配置
```

**结论**：这就是我们已经做的方案，已验证可行。

### 方案二：从普通 Fedora 镜像从零构建（探索性）

```dockerfile
FROM registry.fedoraproject.org/fedora:41
# 1. 安装 bootc + ostree 栈
RUN dnf install -y bootc ostree rpm-ostree ...
# 2. 初始化 ostree
# 3. 用 ostree commit 当前 rootfs
# 4. 设置 bootc OCI labels
```

**关键问题**：
- `ostree admin init-fs` 需要真实的 block device，容器内无法执行
- bootc 启动依赖 ostree 的 deploy 机制，不是简单的 chroot
- 没有已知方法从普通容器镜像生成 ostree 布局

**结论**：方案二在技术上不可行（bootc 的 ostree 依赖无法在容器 build 时满足）。

### F 的实际产出

**文档化两条路径的对比**：

| 路径 | 方案 | 可行性 | 适用场景 |
|------|------|--------|----------|
| F1 | 基于 fedora-bootc 扩展 | ✅ 已验证 | 短期落地 |
| F2 | 从普通 Fedora 构建 | ❌ 技术不可行 | — |
| F3 | 自建 ostree 基础镜像 | ⚠️ 需调研 | 长期自主可控 |

**F3 路径**（自建 ostree 基础镜像）：
- 使用 `rpm-ostree compose tree` 从 package manifest 构建 ostree repo
- 用 `ostree container encapsulate` 转为 OCI 镜像
- 这是 bootc 官方项目自身构建 base image 的方式
- 需要 EL10 的 package manifest（kickstart/treefile）

### 交付物
- `docs/bootc-image-from-scratch.md`（路径对比 + 构建指南）
