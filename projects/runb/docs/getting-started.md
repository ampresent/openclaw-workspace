# runb 快速上手

## 核心理念

**不需要手动调用 runb，不需要手动导出 rootfs。**

一切通过 `docker run --runtime=runb` 完成，Docker 自动处理镜像拉取、rootfs 创建、OCI bundle 生成和清理。

## 前置条件

1. Docker 已安装并运行
2. runb 二进制和 shim 已注册为 Docker runtime（见 [docker-integration.md](docker-integration.md)）

```bash
# 验证 runb runtime 已注册
docker info | grep -A3 Runtimes
# Runtimes: io.containerd.runc.v2 runb runc
```

## 基本用法

```bash
# 运行容器（与 docker run 完全一致，只需加 --runtime=runb）
docker run --runtime=runb --rm alpine echo "Hello from runb!"

# 后台运行
docker run --runtime=runb -d --name myapp nginx

# 查看容器
docker ps

# 停止/删除
docker stop myapp
docker rm myapp
```

## Overlay 热升级

runb 的核心优势：rootfs 可热替换，持久化数据不丢失。

```bash
# 创建持久化数据卷
docker volume create app-data

# 部署 v1
docker run --runtime=runb -d \
    --name myapp \
    -v app-data:/home \
    myapp:v1

# 写入数据
docker exec myapp sh -c 'echo "important" > /home/user.txt'

# 热升级到 v2（通过 runb CLI 直接操作 overlay）
runb upgrade myapp --bundle /bundle-v2
# /home/user.txt 数据完整保留
```

## Layer 管理（高级）

如果需要 runb 独有的 layer 追踪功能，可直接调用 runb CLI：

```bash
# 初始化 layer tracking
runb init-layer myapp --backend diff

# 提交变更
runb commit myapp -m "installed nginx"

# 查看历史
runb layers myapp
```

## 命令速查

### Docker 入口（日常使用）

| 命令 | 说明 |
|------|------|
| `docker run --runtime=runb ...` | 运行容器 |
| `docker create --runtime=runb ...` | 创建容器 |
| `docker start/stop/restart ...` | 生命周期管理 |
| `docker ps / logs / exec ...` | 查看和调试 |

### runb CLI（高级/热升级/layer）

| 命令 | 说明 |
|------|------|
| `runb upgrade <id> --bundle <dir>` | 热升级 rootfs |
| `runb prepare <id>` | 手动挂载 overlay |
| `runb teardown <id>` | 手动卸载 overlay |
| `runb init-layer <id> --backend <b>` | 初始化 layer |
| `runb commit <id> -m "msg"` | 提交 layer |
| `runb layers <id>` | 查看 layer 历史 |
| `runb rebase <id> <rootfs>` | 替换 base rootfs |

---
*创建: 2026-04-09 13:15 | 更新: 2026-04-09 14:00*
