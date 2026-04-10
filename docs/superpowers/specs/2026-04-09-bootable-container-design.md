# Bootable Container Runtime Environment — 设计文档

**创建：** 2026-04-09
**状态：** Draft — 待用户 review

---

## 1. 目标

设计一个基于 bootc（OCI 镜像 → 可启动系统）的公司业务运行时环境，替代传统裸机/VM 的 OS 管理方式。

**核心能力矩阵：**

| # | 能力 | 优先级 | 说明 |
|---|------|--------|------|
| 1 | 热升级 | P0 | bootc 原子 image swap，服务层独立滚动 |
| 2 | 配置漂移检测 | P0 | 实际运行状态 vs 镜像声明状态的 diff |
| 3 | 变更预演 | P0 | 升级前在 QEMU 隔离环境验证 |
| 4 | 变更移植 | P0 | 跨环境/跨机器的精准文件/服务同步 |
| 5 | 回滚 | P0 | 升级失败自动回退到上一个已知好的 image |
| 6 | 镜像签名/验证 | P1 | 防止未授权镜像运行 |
| 7 | 审计日志 | P1 | 谁在何时对何机器做了什么变更 |
| 8 | 健康监控 | P1 | OS 层 + 容器层统一健康状态 |
| 9 | 灰度发布 | P1 | canary/ring 策略，不一次性推全量 |
| 10 | 紧急热补丁 | P2 | 重大漏洞快速 patch，不走完整 rebuild |
| 11 | 配置管理 | P1 | 密钥/配置与 image 分离管理 |
| 12 | 灾备恢复 | P2 | 机器挂了能从 image 快速重建 |

---

## 2. 架构概览

```
Git (source of truth)
  ├── base/Containerfile
  ├── overlay/team-*/Containerfile
  ├── services/*/compose.yaml
  ├── configs/{base,env,secrets}/
  └── inventory/{hosts,groups}.toml

CI/CD Pipeline
  ├── build → podman build → bootc image
  ├── sign → cosign 签名
  ├── push → OCI registry
  └── deploy → ring-based rollout

运行时（每台机器）
  ├── bootc 管理宿主机 OS
  ├── podman 运行 services
  ├── drift-check agent 定期扫描
  └── upgrade-agent 执行升级
```

---

## 3. 仓库结构

```
runtime-infra/
├── base/
│   ├── Containerfile              # 统一基础镜像
│   └── overlay/
│       └── team-foo/
│           └── Containerfile      # FROM base-image + 团队组件
├── services/
│   ├── api-server/
│   │   └── compose.yaml
│   ├── redis/
│   │   └── compose.yaml
│   └── ...
├── configs/
│   ├── base.toml
│   ├── env/
│   │   ├── dev.toml
│   │   ├── staging.toml
│   │   └── prod.toml
│   └── secrets/                   # encrypted (git-crypt / SOPS)
├── inventory/
│   ├── hosts.toml                 # IP, 角色, 环境, 当前 image 版本
│   └── groups.toml                # 分组定义
├── scripts/
│   ├── drift-check.sh
│   ├── upgrade-agent.sh
│   └── preview-upgrade.sh
└── .github/workflows/
    ├── build-image.yml
    └── deploy.yml
```

**设计原则：**
- Git 是唯一真相来源
- 所有变更必须经过 PR review
- 配置与镜像分离：镜像管"装什么"，配置管"怎么跑"
- 密钥加密存储，不进明文

---

## 4. 升级流程

### 4.1 Ring-based 渐进发布

```
开发者 push → Containerfile 变更
  ↓
CI: podman build → bootc image → cosign sign → push to registry
  ↓
CD: 读 inventory.toml，按 ring 渐进部署
  ┌────────────────────────────────────────────┐
  │ Ring 0: 预演环境（1 台 VM）                  │
  │   - QEMU TCG 启动临时 VM                    │
  │   - bootc upgrade → 健康检查                │
  │   - pass → 进入 Ring 1                      │
  │   - fail → 中止 + 告警通知                  │
  ├────────────────────────────────────────────┤
  │ Ring 1: canary（5% 机器）                   │
  │   - bootc upgrade → 观察 30min             │
  │   - 自动指标检查：CPU/内存/错误率            │
  │   - pass → 进入 Ring 2                      │
  │   - fail → 自动 rollback + 告警            │
  ├────────────────────────────────────────────┤
  │ Ring 2: 50%                               │
  │   - 需人工确认                              │
  ├────────────────────────────────────────────┤
  │ Ring 3: 全量                               │
  │   - 需人工确认                              │
  └────────────────────────────────────────────┘
```

### 4.2 回滚机制

- `bootc rollback` — 切换回上一个 image（boot 级别，1 分钟内生效）
- 支持自动回滚：Ring 0/1 升级后健康检查失败 → 自动 rollback
- 支持手动回滚：`bootc rollback --target <image-hash>`

### 4.3 紧急热补丁

- 不走完整 CI/CD 流程
- 在现有 image 基础上做 `podman exec` 级别的临时修复
- 同时创建后续完整 rebuild 的 PR
- 补丁操作记录到审计日志

---

## 5. 变更检查（Drift Detection）

### 5.1 机制

每台机器上运行 drift-check agent（cron，每 5 分钟）：

```
drift-check agent
  │
  ├─ 声明状态（从 image 提取）
  │   ├── /etc/ 下文件 hash
  │   ├── 已安装包列表
  │   └── services/compose.yaml 中定义的容器
  │
  ├─ 实际状态（从运行系统采集）
  │   ├── /etc/ 下文件 hash
  │   ├── rpm -qa 包列表
  │   └── podman ps 运行中容器
  │
  └─ 输出 diff 报告
```

### 5.2 告警分级

| 级别 | 颜色 | 条件 | 动作 |
|------|------|------|------|
| Info | 🔵 | 配置有变更但不影响安全 | 记录日志 |
| Warning | 🟡 | 偏离声明，功能可能正常 | 通知运维 |
| Critical | 🔴 | 安全相关配置被改（sshd_config, firewall） | 立即告警 + 可选自动修复 |

### 5.3 输出格式

```json
{
  "host": "web-01",
  "image_hash": "sha256:abc123...",
  "timestamp": "2026-04-09T23:00:00Z",
  "drifts": [
    {
      "file": "/etc/nginx/nginx.conf",
      "severity": "warning",
      "detail": "mtime differs from image"
    },
    {
      "package": "tmux",
      "severity": "info",
      "detail": "installed but not in image declaration"
    }
  ]
}
```

---

## 6. 变更移植（Cherry-pick）

### 6.1 场景

测试环境验证了一个变更，需要移植到 staging/production。

### 6.2 流程

```
1. drift-check 已捕获 A 机器上的变更 diff
2. 运维执行：
   bootc cherry-pick --from web-test-01 --file /etc/nginx/nginx.conf --to web-staging
3. 工具执行：
   a. 从源机器获取文件内容
   b. 用 test-nginx.sh 验证语法（或用户自定义验证脚本）
   c. 验证通过 → 写入目标机器
   d. 记录审计日志
```

### 6.3 约束

- 移植前必须有验证步骤（不能裸拷贝）
- 移植操作记录到审计日志，关联变更来源
- 支持批量移植：`--to-group web-servers`
- 支持 dry-run 模式预览

---

## 7. 变更预演（Preview）

### 7.1 利用 QEMU TCG 做隔离验证

```
preview-upgrade.sh --target ring-1 --image registry/repo:new-v2
  │
  ├─ 1. 启动临时 VM（QEMU TCG，不需要 KVM）
  ├─ 2. 在 VM 中执行 bootc upgrade 到目标 image
  ├─ 3. 跑自动化检查套件：
  │   ├── 所有 declared services 是否启动
  │   ├── 关键端口是否可达
  │   ├── 配置文件 hash 是否匹配声明
  │   ├── 自定义检查脚本（项目可扩展）
  │   └── 性能基准对比（可选）
  ├─ 4. 生成预演报告
  └─ 5. 销毁临时 VM
```

### 7.2 预演报告结构

```json
{
  "image": "registry/repo:new-v2",
  "base_image": "registry/repo:v1",
  "preview_time": "2026-04-09T23:00:00Z",
  "duration_seconds": 45,
  "results": {
    "services_up": { "status": "pass", "details": "8/8 services running" },
    "ports_reachable": { "status": "pass", "details": "80,443,3306 OK" },
    "config_match": { "status": "warn", "details": "/etc/sysctl.conf differs" },
    "custom_checks": { "status": "pass", "details": "all 3 scripts passed" }
  },
  "recommendation": "proceed with caution (1 warning)"
}
```

---

## 8. 实验计划（Phase 分解）

### Phase 1: 环境准备 ✅

- [x] 确认 QEMU TCG 可用
- [x] 确认 podman + OVMF 已安装
- [ ] 拉取 base bootc image（`quay.io/centos-bootc/centos-bootc:stream9`）
- [ ] 验证 bootc-image-builder 容器可用

### Phase 2: 最小可运行 PoC

- [ ] 编写最小 Containerfile（base + 1 个自定义包）
- [ ] 构建 bootc image
- [ ] 用 QEMU TCG 启动 VM
- [ ] 在 VM 中用 podman 跑一个简单服务
- [ ] 验证 bootc upgrade 流程

### Phase 3: Drift Detection 原型

- [ ] 编写 drift-check 脚本（对比 /etc hash）
- [ ] 在 VM 中手动改配置 → 验证检测到漂移
- [ ] 输出 JSON 格式报告

### Phase 4: 变更预演

- [ ] QEMU 启动 VM → 执行 bootc upgrade → 跑检查
- [ ] 模拟升级失败 → 验证自动 rollback
- [ ] 生成预演报告

### Phase 5: 变更移植

- [ ] 实现 cherry-pick 基本功能（单文件移植）
- [ ] 实现验证步骤（升级前检查）
- [ ] 支持 dry-run

### Phase 6: CI/CD 集成

- [ ] GitHub Actions 构建 pipeline
- [ ] cosign 签名集成
- [ ] Ring-based 部署脚本

### Phase 7: 生产就绪

- [ ] 审计日志系统
- [ ] 健康监控集成
- [ ] 密钥管理（SOPS / git-crypt）
- [ ] 文档 + 运维手册

---

## 9. 技术栈汇总

| 组件 | 技术选型 | 说明 |
|------|----------|------|
| Base Image | centos-bootc:stream9 | OCI 镜像 → 可启动系统 |
| Image Builder | bootc-image-builder | OCI → qcow2/raw |
| 容器运行时 | podman | 服务层 |
| 测试 VM | QEMU TCG | 隔离环境，无需 KVM |
| 镜像签名 | cosign (Sigstore) | OCI 镜像签名验证 |
| 密钥管理 | SOPS + age | 配置加密存储 |
| CI/CD | GitHub Actions | 构建 + 部署 |
| 监控 | 待定 | 可选 Prometheus/node_exporter |
| 审计 | 自研轻量方案 | JSON 日志 + 可查询 |

---

## 10. 风险与限制

| 风险 | 缓解措施 |
|------|----------|
| bootc 生态较新，文档/社区支持有限 | 参考 Fedora bootc 官方文档，保留 fallback 到传统方案 |
| QEMU TCG 性能较差，预演耗时长 | 仅用于预演，生产用 KVM/裸机；预演可接受较慢 |
| OCI registry 网络依赖 | 可部署内网 registry (Harbor/Zot) |
| 密钥泄露风险 | SOPS 加密 + 签名 + 最小权限 |
| 升级回滚失败 | 多版本 image 保留，bootc 本身支持 rollback |

---

*待用户 review 后进入 implementation plan 阶段。*
