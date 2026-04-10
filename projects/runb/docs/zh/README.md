# runb — 轻量级容器运行时

> 纯 chroot 隔离，无 namespace，无 cgroup。适合边缘设备、简单沙箱、OS 热升级场景。

## 核心特性

| 特性 | 说明 |
|------|------|
| OCI 兼容生命周期 | create / start / stop / delete / state / list |
| 纯 chroot 隔离 | 无 Linux namespace，无 cgroup |
| Overlay 挂载 | bind mount 宿主持久化目录，支持热升级 |
| Layer 管理 | commit 用户修改为层，rebase 到新 OS |
| 环境清理 | clearenv 后注入 config.json 定义的环境变量 |
| 极小体积 | ~1.3MB（strip + LTO） |
| 无守护进程 | 单二进制，直接管理容器进程 |

## 快速开始

```bash
# 编译
cargo build --release

# 创建 OCI bundle
mkdir bundle && cd bundle
cat > config.json << 'EOF'
{
  "ociVersion": "1.0.2",
  "root": { "path": "/path/to/rootfs" },
  "process": {
    "terminal": false,
    "args": ["/bin/echo", "hello"],
    "env": ["PATH=/bin:/usr/bin", "HOME=/"],
    "cwd": "/"
  },
  "mounts": [],
  "linux": {}
}
EOF

# 创建并启动容器
runb create myapp --bundle .
runb start myapp

# 查询状态
runb state myapp

# 删除
runb delete myapp
```

## 命令参考

### 基础命令

| 命令 | 说明 |
|------|------|
| `runb create <id> --bundle <path>` | 从 OCI bundle 创建容器 |
| `runb start <id>` | 启动容器（chroot + exec） |
| `runb stop <id> [-s signal]` | 停止运行中的容器 |
| `runb delete <id>` | 删除已停止的容器 |
| `runb state <id>` | 查询容器状态 |
| `runb list` | 列出所有容器 |

### Overlay 命令

| 命令 | 说明 |
|------|------|
| `runb prepare <id>` | 手动挂载 overlay 目录 |
| `runb teardown <id>` | 手动卸载 overlay 目录 |
| `runb verify <id>` | 检查 overlay 挂载状态 |
| `runb upgrade <id> --bundle <path>` | 热升级（自动 teardown→delete→create→prepare→start） |

### Layer 命令

| 命令 | 说明 |
|------|------|
| `runb init-layer <id>` | 初始化层跟踪（快照 base 镜像） |
| `runb commit <id> -m "描述"` | 将当前 rootfs 变更提交为新层 |
| `runb layers <id>` | 列出所有层 |
| `runb rebase <id> <新rootfs>` | 替换底层 OS，重新应用用户层 |

## Overlay 热升级

### 设计理念

```
宿主机（空壳）                     容器 rootfs
├── /usr/local/bin/runb           ├── /bin/, /usr/, /lib/ ...
└── /data/                        ├── /home  ──bind mount──→  /data/home
    ├── home/                     └── /var   ──bind mount──→  /data/var
    │   └── user.txt (持久)       
    └── var/                      
        └── app.log (持久)        
```

宿主机只保留 runb 二进制和持久化数据。rootfs 包含完整业务操作系统。
热升级 = 替换 rootfs，持久化数据通过 bind mount 保留。

### 配置 runb.toml

```toml
[overlay]
links = [
    { host = "/data/home", container = "/home" },
    { host = "/data/var", container = "/var" },
]
```

### 热升级流程

```bash
# 1. 部署初始版本
runb create myos --bundle /bundle-v1
runb start myos

# 2. 用户写入数据到持久化目录
echo "important data" > /data/home/user.txt

# 3. 热升级到新版本（修改 config.json 指向新 rootfs 后）
runb upgrade myos --bundle /bundle-v2
# 自动执行: teardown → delete → create → prepare → start
# /data/home/user.txt 数据完整保留！
```

## Layer 管理

### 设计理念

```
原始镜像 (Base Image)
  │
  │  ← init-layer 快照
  │
  ├── 用户修改 (commit 1): 新增 myapp.conf
  ├── 用户修改 (commit 2): 更新 myapp.conf  
  │
  │  ← rebase: 替换底层 OS，重放用户层
  │
  ▼
新 OS + 用户修改 = 完整 rootfs
```

### 完整用例

```bash
# 1. 创建容器 + 初始化层跟踪
runb create myapp --bundle ./bundle
runb init-layer myapp

# 2. 用户修改容器内容
echo "config=production" > rootfs/etc/myapp.conf
mkdir -p rootfs/usr/local/bin
echo '#!/bin/sh' > rootfs/usr/local/bin/myapp

# 3. 提交为新层
runb commit myapp -m "添加 myapp 配置和脚本"
# 输出: Layer 001 committed: 0 changed, 2 added, 0 deleted

# 4. 用户继续修改
echo "config=staging" > rootfs/etc/myapp.conf
echo "debug=true" >> rootfs/etc/myapp.conf

# 5. 再次提交
runb commit myapp -m "更新配置为 staging"
# 输出: Layer 002 committed: 1 changed, 0 added, 0 deleted

# 6. 查看所有层
runb layers myapp
#   layer-001  2026-04-09 05:00  添加 myapp 配置和脚本
#   layer-002  2026-04-09 05:01  更新配置为 staging

# 7. 升级底层 OS（如 Alpine 3.20 → 3.21）
runb rebase myapp /new-alpine-rootfs
# 自动: 替换 base → 重新应用 layer-001 → layer-002
# myapp.conf 内容保留！os-release 已更新！

# 8. 验证
runb start myapp
cat /etc/os-release        # ← 新版本
cat /etc/myapp.conf        # ← 用户修改保留
cat /usr/local/bin/myapp   # ← 用户添加的文件保留
```

### 层存储结构

```
bundle/
  layers/
    base.sha256           # base 镜像文件 SHA256 清单
    layer-001/
      meta.json           # {created_at, description, layer_number}
      files/              # 变更的文件（相对于 rootfs）
        etc/myapp.conf
        usr/local/bin/myapp
    layer-002/
      meta.json
      files/
        etc/myapp.conf    # 覆盖 layer-001 的版本
      deleted.txt          # 被删除的文件列表（如有）
```

## 测试

```bash
# Alpine Docker 测试（需要 Docker）
docker build -t runb-test .
docker run --privileged runb-test
```

## 限制

| 限制 | 说明 |
|------|------|
| 无进程隔离 | 容器进程共享宿主 PID namespace |
| 无网络隔离 | 共享宿主网络 |
| 无资源限制 | 无 cgroup（CPU/内存不受限） |
| 需要 root | chroot() 和 mount() 需要 root 权限 |
| 仅 Linux | 依赖 Linux syscall |

## 适用场景

- 边缘设备/ IoT 上运行轻量容器
- OS 热升级（替换 rootfs，保留用户数据）
- 开发测试环境的快速沙箱
- 学习容器原理（代码简洁，无魔法）

## License

MIT
