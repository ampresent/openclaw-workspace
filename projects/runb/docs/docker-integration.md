# Docker 接入 runb — 完整指南

创建时间: 2026-04-09 | 更新: 13:10

## 目标

将 runb（chroot-only OCI 容器运行时）注册为 Docker 自定义 runtime，实现 `docker run --runtime=runb`。

## 架构

```
Docker CLI
  └→ dockerd
       └→ containerd
            └→ containerd-shim-runc-v2
                 └→ runb-docker-shim (适配层)
                      └→ runb (OCI runtime, chroot-only)
```

Docker 通过 `daemon.json` 注册自定义 runtime，containerd 的 runc v2 shim 调用 OCI runtime 二进制执行 `create/start/state/kill/delete` 命令。

## 网络环境（重要！）

当前服务器网络受限，以下不可达：

| 资源 | 状态 | 替代方案 |
|------|------|----------|
| Docker Hub | ❌ 超时 | `docker.m.daocloud.io` |
| 阿里云 apt 镜像 | ❌ 超时 | 官方 apt 源 + Tsinghua |
| sh.rustup.rs | ❌ SSL 错误 | Docker Rust 镜像 |
| crates.io git index | ❌ 连接重置 | 手动 sparse clone + vendor |
| static.crates.io | ✅ 可达 | 下载 .crate 文件 |
| github.com | ✅ 可达 | git clone |
| httpbin.org | ✅ 可达 | 网络测试 |

## 步骤

### 1. 安装 Docker

```bash
# 使用 DaoCloud 镜像拉取 Docker CE
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
# 添加 Docker apt 源后安装
apt-get install -y docker-ce docker-ce-cli containerd.io

# 启动 Docker（需要 iptables）
ln -sf /usr/sbin/iptables-nft /usr/bin/iptables
dockerd &
```

### 2. 拉取 Docker 镜像

**必须使用 DaoCloud 镜像前缀**：

```bash
# ❌ docker pull alpine:latest          → 超时
# ✅ docker pull docker.m.daocloud.io/library/alpine:latest

docker pull docker.m.daocloud.io/library/alpine:latest
docker pull docker.m.daocloud.io/library/rust:1.85-bookworm
```

### 3. 编译 runb（离线方式）

由于容器内无法访问 crates.io git index，使用 **cargo vendor** 方式离线构建：

```bash
# 在主机上下载所有 .crate 文件
python3 -c "
import re
with open('projects/runb/Cargo.lock') as f:
    content = f.read()
packages = re.findall(r'\[\[package\]\]\nname = \"([^\"]+)\"\nversion = \"([^\"]+)\"', content)
for name, version in packages:
    if name == 'runb': continue
    print(f'{name}|{version}')
" | while IFS='|' read name ver; do
    wget -q --timeout=10 -O "/tmp/crates/${name}-${ver}.crate" \
        "https://static.crates.io/crates/${name}/${name}-${ver}.crate"
done

# 解压并创建 vendor 目录 + .cargo-checksum.json
# （脚本详见 temp/download-crates.sh）

# 创建 .cargo/config.toml
mkdir -p projects/runb/.cargo
cat > projects/runb/.cargo/config.toml << 'EOF'
[source.crates-io]
replace-with = "vendored-sources"
[source.vendored-sources]
directory = "vendor"
EOF

# 使用 Docker Rust 镜像离线编译
docker run --rm --network none \
    -v "$(pwd)/projects/runb":/src \
    -w /src \
    docker.m.daocloud.io/library/rust:1.85-bookworm \
    cargo build --release --target x86_64-unknown-linux-gnu --offline

# 提取二进制
cp projects/runb/target/x86_64-unknown-linux-gnu/release/runb /usr/local/bin/runb
```

### 4. 安装 Docker 适配层

runb 原生 CLI 与 Docker OCI runtime 接口有差异，需要 shim 适配：

| Docker 调用 | runb 命令 | 说明 |
|-------------|-----------|------|
| `create` | `create` | 相同，但需 fork hold 进程提供 PID |
| `start` | `start` | 需先 kill hold 进程 |
| `state` | `state` | 相同 |
| `kill` | `stop` | 命令名不同 |
| `delete` | `delete` | 相同 |
| `features` | N/A | Docker 检查 runtime 能力 |

```bash
# 安装 shim 脚本
cp projects/runb/runb-docker-shim.sh /usr/local/bin/runb-docker-shim
chmod +x /usr/local/bin/runb-docker-shim

# 安装 hold 进程辅助工具（解决 create 时无 PID 的问题）
gcc -o /usr/local/bin/runb-hold /tmp/hold2.c
```

### 5. 注册 Docker Runtime

```bash
# 编辑 /etc/docker/daemon.json
python3 -c "
import json
with open('/etc/docker/daemon.json') as f:
    cfg = json.load(f)
cfg.setdefault('runtimes', {})['runb'] = {
    'path': '/usr/local/bin/runb-docker-shim',
    'runtimeArgs': []
}
with open('/etc/docker/daemon.json', 'w') as f:
    json.dump(cfg, f, indent=2)
"

# 重启 Docker
killall dockerd; sleep 2; dockerd &
```

### 6. 验证

```bash
# 确认 runb runtime 已注册
docker info | grep -A3 Runtimes
# 输出: Runtimes: io.containerd.runc.v2 runb runc

# docker run 是统一入口（无需手动调用 runb create/start/delete）
docker run --runtime=runb --rm alpine echo "Hello from runb!"
docker run --runtime=runb -d --name myapp nginx
docker ps
docker stop myapp && docker rm myapp
```

## 文件清单

```
projects/runb/
├── runb-docker-shim.sh      # Docker OCI runtime 适配层
├── target/release/runb       # 编译后的二进制 (1.8MB)
├── vendor/                   # 离线依赖 (cargo vendor)
├── .cargo/config.toml        # vendor 配置
└── docs/
    ├── docker-integration.md  # 本文件
    └── zh/README.md           # 中文文档
```

---
*最后更新: 2026-04-09 14:00*

---

## 进度记录

| 步骤 | 状态 | 时间 |
|------|------|------|
| Docker 安装+启动 | ✅ v29.4.0 | 12:19 |
| Alpine 镜像拉取 | ✅ DaoCloud 镜像 | 12:25 |
| Rust 构建镜像拉取 | ✅ DaoCloud 镜像 | 12:28 |
| .crate 文件批量下载 | ✅ 63/63 | 12:53 |
| crates.io-index sparse clone | ✅ 5259 文件 | 12:54 |
| cargo vendor 离线构建 | ✅ 编译成功 | 13:04 |
| runb 直接测试 (Alpine) | ✅ 完整生命周期 | 13:08 |
| Docker runtime 注册 | ✅ daemon.json | 13:06 |
| Docker shim (create) | ✅ PID handoff 修复中 | 13:10 |
| Docker --runtime=runb | ⏳ shim PID 待修复 | - |
| 容器定期提交指南 | ✅ 已完成 | 13:10 |
| GIF 录制 | ⬜ 待录制 | - |

### 关键发现

1. **`cargo vendor` 是离线构建的正确方式** — registry index 方式（git clone / sparse）在 offline 模式下 cargo 仍无法识别包
2. **Docker containerd shim 需要 `create` 后立即有 PID** — 使用 `runb-hold`（double-fork daemon）提供占位 PID
3. **DaoCloud 镜像是唯一可用的 Docker 镜像源** — `docker.m.daocloud.io/...`
