# runb 最佳实践

> runb 是纯 chroot 的轻量 OCI 容器运行时。本文档覆盖其核心能力、操作流程、以及 runb-tui 如何通过 TUI 控制 runb。

---

## 目录

1. [设计约束与适用边界](#设计约束与适用边界)
2. [容器生命周期](#容器生命周期)
3. [Overlay 热升级](#overlay-热升级)
4. [Layer 变更管理](#layer-变更管理)
5. [后端选型](#后端选型)
6. [runb-tui 操作映射](#runb-tui-操作映射)
7. [运维工作流](#运维工作流)
8. [故障排查](#故障排查)
9. [反模式](#反模式)

---

## 设计约束与适用边界

runb **不是** Docker/runc 的替代品。它的定位是：

| 有 | 没有 |
|----|------|
| chroot 文件系统隔离 | 无 namespace（PID/网络/UTS/IPC 共享宿主） |
| OCI 兼容生命周期 | 无 cgroup（无 CPU/内存/IO 限制） |
| bind mount overlay 持久化 | 无 seccomp（无 syscall 过滤） |
| Layer 变更跟踪与 rebase | 无 rootless 模式（需要 root） |
| clearenv 环境隔离 | 无镜像 registry 拉取 |

**适用场景：**
- 边缘设备 / IoT 上运行轻量服务
- OS 热升级（替换 rootfs，保留用户数据）
- 开发测试的快速沙箱
- 学习容器原理（1.3MB 单二进制，无魔法）

**不适用：**
- 多租户隔离（无 namespace，进程可见宿主全部 PID）
- 资源受限环境（无 cgroup，容器可以吃光 CPU/内存）
- 不信任的代码执行（无 seccomp/capabilities 限制）

---

## 容器生命周期

### 状态机

```
                  create
    [不存在] ──────────────→ [Created]
                                │
                              start
                                │
                                ▼
                           [Running]
                           ╱        ╲
                  process exit     stop (SIGTERM)
                        ╲            ╱
                         ▼          ▼
                       [Stopped]
                           │
                         delete
                           │
                           ▼
                       [不存在]
```

**关键行为：**
- `state` 命令会自动检测进程是否存活（`kill(pid, 0)`），进程退出后自动转换为 Stopped
- `start` 时自动挂载 overlay（如果 `runb.toml` 存在）
- `delete` 只允许删除 Stopped 状态的容器

### 标准操作

```bash
# 1. 准备 OCI bundle
mkdir -p /opt/bundles/nginx
cd /opt/bundles/nginx
# 写 config.json + 准备 rootfs

# 2. 创建容器
runb create nginx --bundle /opt/bundles/nginx

# 3. 启动（chroot + exec，自动挂载 overlay）
runb start nginx

# 4. 查看状态
runb state nginx
# {"id":"nginx","state":"running","pid":1234,...}

# 5. 停止（默认 SIGTERM）
runb stop nginx
# 可指定信号
runb stop nginx -s 9   # SIGKILL

# 6. 删除
runb delete nginx
```

### config.json 要点

runb 只关心 OCI config.json 中的这几个字段：

```jsonc
{
  "root": {
    "path": "/absolute/path/to/rootfs"   // 必须是绝对路径
  },
  "process": {
    "args": ["/bin/sh", "-c", "echo hello"],  // 要执行的命令
    "env": ["PATH=/bin:/usr/bin"],             // clearenv 后注入
    "cwd": "/"                                 // chroot 后的 cwd
  }
}
```

**注意：** runb 忽略 mounts、linux.namespaces、linux.cgroups 等字段。

---

## Overlay 热升级

### 原理

```
宿主机                              容器 rootfs
┌─────────────────┐               ┌─────────────────┐
│ /usr/local/bin/runb             │ /bin/ /usr/ /lib/...
│ /data/          │               │                 │
│   ├── home/     │ ──bind mount──│ /home/          │
│   │   └── user.json             │   └── user.json │
│   └── var/      │ ──bind mount──│ /var/           │
│       └── log/  │               │   └── log/      │
└─────────────────┘               └─────────────────┘
```

宿主机只保留 runb 二进制 + 持久化数据。rootfs 包含完整业务系统。
热升级 = 替换 rootfs，bind mount 的宿主数据不受影响。

### 为什么用 bind mount 而不是 symlink

symlink 在 chroot 内无法逃逸——内核会在 chroot 边界处截断 `..`。bind mount 是唯一能在 chroot 内访问宿主路径的轻量机制。

### 配置 runb.toml

放在 bundle 目录下，和 `config.json` 同级：

```toml
[overlay]
links = [
    { host = "/data/home", container = "/home" },
    { host = "/data/var",  container = "/var" },
]
```

**规则：**
- `host` 必须存在（否则 prepare 报错）
- `container` 路径不能重复（去重检查）
- prepare 时自动创建 container 端的挂载点目录
- 已挂载的路径跳过（幂等）

### 热升级完整流程

```bash
# ① 首次部署
runb create myos --bundle /bundle-v1
runb start myos

# ② 用户产生数据（写入持久化目录）
echo "user config" > /data/home/user.json

# ③ 准备新版本 bundle（修改 config.json 指向新 rootfs）
# bundle-v2/config.json → root.path = "/path/to/new-rootfs"

# ④ 热升级（自动: stop → teardown → delete → create → prepare → start）
runb upgrade myos --bundle /bundle-v2

# ⑤ 验证：数据完整保留
# /data/home/user.json 仍然存在，新 rootfs 已生效
```

### 手动 overlay 操作

```bash
# 单独挂载（不启动容器）
runb prepare myos

# 卸载（热升级前的清理步骤，upgrade 会自动做）
runb teardown myos

# 检查完整性（容器运行中才有意义，进程退出后 bind mount 自动清理）
runb verify myos
# All overlays OK
# 或
# ISSUE: NOT ACTIVE: /bundle/rootfs/home is not currently mounted (normal if container process has exited)
```

---

## Layer 变更管理

### 原理

```
Base Image (初始 rootfs 快照)
    │
    ├── Layer 001: +23 files, install myapp
    ├── Layer 002: ~1 file, update config
    ├── Layer 003: +5 files, add plugin
    │
    │  rebase: 替换底层 OS，重放 Layer 001→002→003
    │
    ▼
New OS + 用户修改 = 完整 rootfs
```

### 工作流

```bash
# ① 创建容器后，初始化 layer 跟踪（快照 base 镜像）
runb init-layer myapp
# 默认使用 diff 后端
# 指定后端：runb init-layer myapp --backend tar

# ② 在容器内做修改（安装软件、改配置等）

# ③ 提交变更
runb commit myapp -m "install nginx"
# 输出: Layer 001 committed: 0 changed, 23 added, 0 deleted

# ④ 继续修改并提交
runb commit myapp -m "add custom nginx.conf"

# ⑤ 查看所有层
runb layers myapp
# Layers (diff):
#   layer-001  +23 -0 ~0  123456 bytes  install nginx
#   layer-002  +0  -0 ~1  2048 bytes    add custom nginx.conf
# Total layer disk: 125504 bytes (122.6 KB)

# ⑥ 升级底层 OS（替换 base，重放所有层）
runb rebase myapp /new-alpine-rootfs
# 自动: 替换 base → apply layer-001 → apply layer-002
```

### Layer 存储结构

```
bundle/
├── config.json
├── runb.toml
├── rootfs/
└── layers/
    ├── base.sha256           # base 镜像文件哈希清单
    ├── layer-001/
    │   ├── meta.json         # {created_at, description, layer_number, stats}
    │   └── files/            # 变更文件（相对 rootfs 的路径）
    │       └── usr/local/bin/myapp
    └── layer-002/
        ├── meta.json
        └── files/
            └── etc/nginx/nginx.conf
```

---

## 后端选型

runb 支持 4 种 layer 后端，各有适用场景：

| 后端 | 速度 | 空间 | 适用场景 |
|------|------|------|---------|
| **diff** (默认) | 快 | 中 | 通用，简单可靠，无外部依赖 |
| **tar** | 中 | 小 (压缩) | 需要跨机器传输层 |
| **hardlink** | 最快 | 小 (共享) | 同机 rebase 频繁 |
| **git** | 慢 | 大 | 需要完整版本历史和 diff 能力 |

**推荐策略：**
- 默认用 `diff`——简单、小层、无需额外工具
- 需要传输层 → `tar`
- 高频 rebase → `hardlink`
- 调试/审计变更 → `git`

```bash
# 指定后端初始化
runb init-layer myapp --backend tar

# 后续 commit 自动检测已有后端
runb commit myapp -m "update"   # 自动使用 tar
```

---

## runb-tui 操作映射

runb-tui 将 CLI 命令映射为 TUI 快捷操作。以下是完整的操作对照：

### Containers 视图

| TUI 快捷键 | CLI 命令 | 说明 |
|------------|---------|------|
| `j/k` | — | 上下导航列表（本地 UI） |
| `s` | `runb start <id>` | 启动选中容器 |
| `k` | `runb stop <id>` | 停止选中容器（SIGTERM） |
| `d` | `runb delete <id>` | 删除选中容器 |
| `u` | `runb upgrade <id>` | 热升级（stop→teardown→delete→create→prepare→start） |
| `r` | — | 刷新列表（重新读取 /run/runb/） |
| 自动刷新 | `runb list` 逻辑 | 每 3 秒自动重新扫描 /run/runb/ 目录 |

### Layers 视图

| TUI 快捷键 | CLI 命令 | 说明 |
|------------|---------|------|
| `c` | — | 切换当前操作的容器 |
| `i` | `runb init-layer <id>` | 初始化 layer 跟踪 |
| `m` | `runb commit <id> -m "manual commit"` | 提交当前变更 |
| `b` | `runb bench <rootfs>` | 运行后端基准测试 |

### Overlay 视图

| TUI 快捷键 | CLI 命令 | 说明 |
|------------|---------|------|
| `c` | — | 切换当前操作的容器 |
| `p` | `runb prepare <id>` | 挂载 overlay |
| `t` | `runb teardown <id>` | 卸载 overlay |
| `v` | `runb verify <id>` | 验证 overlay 完整性 |

### System 视图

| TUI 快捷键 | CLI 命令 | 说明 |
|------------|---------|------|
| `h` | `runb --help` | 显示帮助 |

### 数据读取方式

runb-tui **不调用 CLI** 读取数据，而是直接读文件：

- **容器列表** → 扫描 `/run/runb/` 目录下的 `state.json`
- **容器详情** → 解析 `/run/runb/<id>/state.json`
- **Layer 列表** → 扫描 `<bundle>/layers/layer-*/meta.json`
- **Overlay 配置** → 解析 `<bundle>/runb.toml`

**写操作**才通过 `execSync('runb ...')` 调用 CLI。

### 实时性

| 数据 | 刷新方式 | 间隔 |
|------|---------|------|
| 容器列表 | setInterval + 读 /run/runb/ | 3 秒 |
| Layer 列表 | setInterval + 读 meta.json | 5 秒 |
| Overlay 配置 | useMemo + 依赖 containerId | 切换时刷新 |
| 操作结果 | setTimeout(refresh, 500) | 操作后 0.5 秒 |

---

## 运维工作流

### 场景一：边缘设备部署 + 热升级

```bash
# 初始部署
runb create edge-app --bundle /opt/bundles/v1
runb start edge-app

# 远程推送新版本 rootfs 到设备
# 更新 config.json 指向新 rootfs

# 通过 SSH 或 TUI 执行热升级
runb upgrade edge-app --bundle /opt/bundles/v2
# 停机时间 = stop + teardown + delete + create + prepare + start
# 通常 < 1 秒（无 namespace/cgroup 开销）
```

### 场景二：开发环境多容器管理

```bash
# 创建多个容器
runb create dev-nginx --bundle /bundles/nginx
runb create dev-redis --bundle /bundles/redis
runb create dev-app   --bundle /bundles/app

# 用 TUI 管理
cd projects/runb-tui && npm run dev
# Tab 1: 看状态、启动/停止
# Tab 2: 管理 layer（跟踪变更）
# Tab 3: 检查 overlay
```

### 场景三：OS Rebase（保留用户修改）

```bash
# 初始 setup
runb create mysystem --bundle /bundle-alpine320
runb init-layer mysystem
runb start mysystem

# 用户在容器内做了大量定制
runb commit mysystem -m "install dev tools"
runb commit mysystem -m "configure services"

# Alpine 3.21 发布，升级底层 OS
# 准备新的 rootfs + 更新 config.json
runb rebase mysystem /bundle-alpine321
# 用户修改完整保留，底层已更新
```

---

## 故障排查

### 容器启动失败

```bash
# 检查状态
runb state <id>

# 常见原因：
# 1. rootfs 路径不存在 → 检查 config.json root.path
# 2. process.args[0] 不存在 → 检查 rootfs 内是否有该二进制
# 3. 权限不足 → runb 需要 root 运行
```

### Overlay 挂载失败

```bash
runb verify <id>

# 常见原因：
# 1. host 路径不存在 → 先创建宿主目录
# 2. container 路径重复 → runb.toml 中去重
# 3. 权限不足 → 需要 root 执行 mount()
```

### Layer Rebase 失败

```bash
# 检查 layers 目录
ls <bundle>/layers/

# 常见原因：
# 1. 没有 init-layer → 先执行 init-layer
# 2. base.sha256 与当前 rootfs 不匹配 → 正常，rebase 会替换 base
# 3. apply 时文件冲突 → 新 base 中已有同名文件，层中的版本会覆盖
```

### 进程状态不同步

runb 通过 `kill(pid, 0)` 检测进程存活。如果进程变成僵尸：

```bash
# 手动清理
kill -9 <pid>
runb stop <id>     # 会自动 reap zombie
runb delete <id>
```

---

## 反模式

### 1. 把 runb 当 Docker 用

```bash
# ❌ 期望 namespace 隔离
runb create untrusted --bundle /untrusted-app
runb start untrusted
# 容器进程可以看到宿主所有 PID、使用宿主网络

# ✅ 只在信任的代码/场景使用
runb create dev-sandbox --bundle /dev-env
# 用于开发、测试、边缘设备——不用于多租户
```

### 2. 忘记挂载 overlay

```bash
# ❌ 用户数据写在 rootfs 内部
runb create myapp --bundle /bundle-v1
runb start myapp
echo "data" > /run/runb/myapp/rootfs/data/file.txt
runb upgrade myapp --bundle /bundle-v2
# file.txt 丢失！被新 rootfs 覆盖

# ✅ 数据写在宿主持久化目录
# 配置 runb.toml: { host = "/data", container = "/data" }
echo "data" > /data/file.txt
runb upgrade myapp --bundle /bundle-v2
# file.txt 保留！
```

### 3. 不初始化 layer 就 commit

```bash
# ❌ 直接 commit
runb commit myapp -m "changes"
# ERROR: No layers directory, run init-layer first

# ✅ 先 init
runb init-layer myapp
# ... 做修改 ...
runb commit myapp -m "changes"
```

### 4. 在 Running 状态删除容器

```bash
# ❌
runb delete myapp
# ERROR: Cannot delete running container 'myapp'

# ✅ 先 stop
runb stop myapp
runb delete myapp
```

### 5. overlay host 路径不存在

```bash
# ❌ runb.toml 指向不存在的宿主目录
# { host = "/data/nonexistent", container = "/data" }
runb prepare myapp
# ERROR: Host path does not exist: '/data/nonexistent'

# ✅ 先创建宿主目录
mkdir -p /data/nonexistent
runb prepare myapp
```

### 6. 热升级时 overlay 没 teardown

```bash
# ❌ 手动 delete → create（不走 upgrade）
runb stop myapp
runb delete myapp
runb create myapp --bundle /bundle-v2
# 旧的 bind mount 可能残留（MNT_DETACH 延迟卸载）

# ✅ 使用 upgrade（自动处理 teardown）
runb upgrade myapp --bundle /bundle-v2

# 或手动分步时确保 teardown
runb stop myapp
runb teardown myapp
runb delete myapp
runb create myapp --bundle /bundle-v2
runb prepare myapp
runb start myapp
```

---

## 速查表

| 操作 | 命令 | 前置条件 |
|------|------|---------|
| 创建容器 | `runb create <id> --bundle <path>` | config.json + rootfs 存在 |
| 启动 | `runb start <id>` | 状态 = Created |
| 停止 | `runb stop <id>` | 状态 = Running |
| 删除 | `runb delete <id>` | 状态 = Stopped |
| 热升级 | `runb upgrade <id> --bundle <new>` | 任意状态 |
| 初始化层 | `runb init-layer <id>` | 容器已创建 |
| 提交层 | `runb commit <id> -m "msg"` | 已 init-layer |
| 查看层 | `runb layers <id>` | 已 init-layer |
| Rebase | `runb rebase <id> <new-rootfs>` | 已 init-layer |
| 挂载 overlay | `runb prepare <id>` | runb.toml + host 路径存在 |
| 卸载 overlay | `runb teardown <id>` | 已挂载 |
| 验证 overlay | `runb verify <id>` | runb.toml 存在 |

---

*2026-04-09 · runb + runb-tui*
