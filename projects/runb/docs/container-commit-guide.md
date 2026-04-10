# runb 容器定期提交指南

创建时间: 2026-04-09

## 概述

runb 支持将容器 rootfs 的当前状态保存为 layer（类似 Docker commit），配合 cron 实现每日自动快照。

## 前提条件

- runb 已编译并安装到 `/usr/local/bin/runb`
- 容器已通过 `runb init-layer` 初始化 layer tracking
- 至少有一个 layer backend（diff / tar / hardlink / git）

## 手动提交

```bash
# 1. 初始化 layer tracking（只需一次）
runb init-layer myapp --backend diff

# 2. 容器运行中，做些修改...
#    比如安装软件、修改配置等

# 3. 提交当前状态
runb commit myapp --message "安装了 nginx + 配置 SSL"

# 4. 查看所有 layer
runb layers myapp
# 输出:
#   Layers (backend: diff):
#   layer-001  +12 -0 ~3  45056 bytes  安装了 nginx + 配置 SSL
#   layer-002  +2 -1 ~5   8192  bytes  更新了 nginx.conf
#   Total layer disk: 53248 bytes (52.0 KB)
```

## 每日自动提交脚本

创建定时提交脚本：

```bash
cat > /usr/local/bin/runb-daily-commit << 'SCRIPT'
#!/bin/bash
# runb-daily-commit: 每日自动提交容器状态
# 用法: runb-daily-commit <container-id> [message]

set -e

CONTAINER_ID="${1:?用法: runb-daily-commit <container-id> [message]}"
MESSAGE="${2:-daily snapshot $(date +%Y-%m-%d)}"
LOG="/var/log/runb-commits.log"

# 检查容器存在
if ! runb state "$CONTAINER_ID" &>/dev/null; then
    echo "[$(date)] ERROR: 容器 $CONTAINER_ID 不存在" >> "$LOG"
    exit 1
fi

# 检查 layer 是否已初始化
BUNDLE=$(runb state "$CONTAINER_ID" | python3 -c "import sys,json; print(json.load(sys.stdin)['bundle'])")
if [ ! -d "$BUNDLE/layers" ]; then
    echo "[$(date)] INFO: 初始化 layer tracking (diff backend)"
    runb init-layer "$CONTAINER_ID" --backend diff
fi

# 提交
echo "[$(date)] 提交容器 $CONTAINER_ID: $MESSAGE" >> "$LOG"
runb commit "$CONTAINER_ID" --message "$MESSAGE" >> "$LOG" 2>&1

# 输出当前 layer 列表
echo "[$(date)] 当前 layers:" >> "$LOG"
runb layers "$CONTAINER_ID" >> "$LOG" 2>&1

echo "[$(date)] 提交完成" >> "$LOG"
SCRIPT
chmod +x /usr/local/bin/runb-daily-commit
```

## 配置 Cron 定时任务

### 方式一：系统 crontab

```bash
# 每天凌晨 3 点提交
crontab -e
# 添加：
0 3 * * * /usr/local/bin/runb-daily-commit myapp "daily $(date +\%Y-\%m-\%d)"
```

### 方式二：systemd timer（推荐）

```bash
# 创建 service
cat > /etc/systemd/system/runb-daily-commit.service << 'EOF'
[Unit]
Description=runb daily container commit
After=network.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/runb-daily-commit myapp
User=root

[Install]
WantedBy=multi-user.target
EOF

# 创建 timer
cat > /etc/systemd/system/runb-daily-commit.timer << 'EOF'
[Unit]
Description=Daily runb container commit

[Timer]
OnCalendar=*-*-* 03:00:00
Persistent=true

[Install]
WantedBy=timers.target
EOF

# 启用
systemctl enable --now runb-daily-commit.timer
```

### 方式三：OpenClaw Cron（适合 AI 管理）

通过 OpenClaw 的 cron 功能管理，支持智能判断是否需要提交：

```
# 在 HEARTBEAT.md 或 cron job 中定期执行
runb-daily-commit myapp "$(date +%Y-%m-%d) 快照"
```

## 热升级 + Layer 恢复

定期提交的 layer 可以用于热升级后恢复用户数据：

```bash
# 1. 当前容器正在运行
runb state myapp

# 2. 热升级到新 rootfs
runb upgrade myapp --bundle /bundle-v2

# 3. 如果需要回滚，使用 rebase 切换回旧 rootfs
runb rebase myapp /old-rootfs

# 4. 查看 layer 历史，确认数据完整性
runb layers myapp
```

## Layer Backend 对比

| Backend | 特点 | 适用场景 |
|---------|------|----------|
| **diff** | SHA256 文件级 diff，小体积 | 日常快照（推荐） |
| **tar** | tar 归档，Docker 兼容 | 需要跨机器迁移 |
| **hardlink** | 硬链接快照，空间高效 | 大 rootfs，频繁提交 |
| **git** | Git 版本控制，可 diff/merge | 需要查看文件级变更 |

## 清理旧 Layer

```bash
# 查看所有 layer 及磁盘占用
runb layers myapp

# 删除特定 container（会清除所有 layer）
runb delete myapp

# 重新创建并初始化
runb create myapp --bundle /path/to/bundle
runb init-layer myapp --backend diff
```

## 完整示例

```bash
# 创建容器
runb create webserver --bundle /bundles/webserver-v1
runb start webserver

# 第一天：安装 nginx
# ... 用户操作 ...
runb commit webserver --message "Day 1: 安装 nginx"

# 第二天：修改配置
runb commit webserver --message "Day 2: 配置反向代理"

# 第三天：添加 SSL
runb commit webserver --message "Day 3: 添加 Let's Encrypt SSL"

# 查看历史
runb layers webserver
# Layers (backend: diff):
#   layer-001  +15 -0 ~2  61440 bytes  Day 1: 安装 nginx
#   layer-002  +1 -0 ~3   4096  bytes  Day 2: 配置反向代理
#   layer-003  +3 -0 ~1   8192  bytes  Day 3: 添加 Let's Encrypt SSL
#   Total layer disk: 73728 bytes (72.0 KB)

# 热升级，数据保留
runb upgrade webserver --bundle /bundles/webserver-v2
runb layers webserver  # layer 历史保留！
```

---
*创建: 2026-04-09 13:10*
