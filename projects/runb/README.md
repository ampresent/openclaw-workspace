# runb — Run on Baremetal

A lightweight OCI-compatible container runtime using **chroot only** — no namespaces, no cgroups.

## Quick Start

**一切通过 Docker，不需要手动调用 runb。**

```bash
# 直接用，和 docker run 完全一样，加个 --runtime=runb
docker run --runtime=runb --rm alpine echo "Hello from runb!"
```

Docker 自动处理：镜像拉取 → rootfs 创建 → OCI bundle 生成 → 调用 runb → 清理。

## Features

- **OCI Runtime Spec compatible** — 通过 `docker run --runtime=runb` 统一入口
- **Chroot-only isolation** — no Linux namespaces, no cgroups
- **Overlay hot upgrade** — bind-mount persistent host data dirs, swap rootfs without losing data
- **Clean environment** — inherited env vars are cleared before exec
- **Minimal binary** — ~1.3MB (stripped, LTO optimized)
- **No daemon** — single binary, direct process management
- **State auto-detection** — container state auto-transitions when process exits

## Usage (Docker 入口)

```bash
# 运行容器
docker run --runtime=runb --rm alpine echo "hello"

# 后台运行
docker run --runtime=runb -d --name myapp nginx

# 查看容器
docker ps

# 停止/删除
docker stop myapp && docker rm myapp
```

## Overlay / Hot Upgrade

runb 支持 rootfs 热升级：替换 rootfs 不丢持久化数据。

```bash
# 创建持久化卷
docker volume create app-data

# 部署 v1
docker run --runtime=runb -d -v app-data:/home myapp:v1

# 写入数据
docker exec myapp sh -c 'echo "important" > /home/user.txt'

# 热升级到 v2（通过 runb CLI）
runb upgrade myapp --bundle /bundle-v2
# /home/user.txt 数据完整保留！
```

## Layer 管理（高级）

```bash
runb init-layer app --backend diff    # 初始化 layer tracking
runb commit app -m "installed nginx"  # 提交变更
runb layers app                        # 查看历史
```

## What's NOT included

- **No namespaces** — processes share the host's PID/network/UTS/IPC namespace
- **No cgroups** — no resource limits (CPU, memory, I/O)
- **No pivot_root** — uses `chroot` directly
- **No seccomp** — no syscall filtering
- **No capabilities management** — inherits parent capabilities
- **No rootless mode** — requires root for `chroot()` and `mount()`

## License

MIT
