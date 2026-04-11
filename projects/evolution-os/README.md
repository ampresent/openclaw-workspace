# Evolution OS (evo)

> 一个处于"持续编译与自我进化"状态的元操作系统。

## 核心理念

1. **源码即系统**：系统状态由本地 Git 仓库树和 Patch 栈定义
2. **AI 辅助演进**：用户发起意图，Claude Code 修改源码，本地小模型感知分流
3. **自举与自治**：基于 Rocky Linux，能编译自身，大部分时间静默进化

## 项目结构

```
evolution-os/
├── evo/              # evo CLI 工具 (Rust)
├── patches/          # Patch 管理
├── build/            # 构建系统集成
├── ai-integration/   # Claude Code + 本地小模型接口
├── docs/             # 详细文档
├── DESIGN.md         # 设计白皮书
├── ARCHITECTURE.md   # 技术架构
└── ...
```

## 快速状态

- **阶段**：AI 集成完成 → 测试迭代
- **语言**：Rust (evo CLI), Shell (构建脚本)
- **基础**：Rocky Linux src.rpm
- **AI 层**：MiMo V2 Pro（云端）

---

## 环境初始化

在新机器上从零搭建 evo 开发/测试环境：

### 1. 安装 Docker

```bash
curl -fsSL https://get.docker.com | sh
# 启动守护进程
dockerd &>/var/log/dockerd.log &
sleep 3 && docker version
```

### 2. 安装 Rust 工具链

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

### 3. 安装系统依赖

```bash
# Ubuntu/Debian
apt-get install -y rpm cpio git curl

# Rocky/RHEL
dnf install -y rpm-build cpio git curl
```

### 4. 编译 evo

```bash
cd evo/
cargo build --release
# 产物: target/release/evo
```

### 5. 准备测试 src.rpm

```bash
# 方式 A: 从 Rocky 源下载（需要 dnf）
evo init curl

# 方式 B: 本地 src.rpm（离线/跨平台）
evo init --srpm /path/to/package.src.rpm curl
```

### 6. 运行测试

```bash
# 自动化测试 + GIF 录制
python3 test_recorder.py
# 输出: /tmp/evo-test.gif
```

### 7. 上传图床（可选）

```bash
# litterbox（临时 24h）
curl -sF "reqtype=fileupload" -F "time=24h" \
  -F "fileToUpload=@/tmp/evo-test.gif" \
  https://litterbox.catbox.moe/resources/internals/api.php
```

---

## 关键命令

```bash
evo init <pkg>          # 下载 src.rpm → 源码树 + git
evo status              # ASCII TUI 看板 (--live 交互式)
evo patch create/list/drop/show/apply
evo build [pkg...]      # rpmbuild / make fallback
evo rebase [pkg...]     # 上游同步，冲突检测 + dry-run
evo tag --create/list/show/delete
evo freeze [--unfreeze] # 锁文件机制
evo ai analyze/patch/resolve/config  # MiMo 驱动
```

详细设计见 [DESIGN.md](./DESIGN.md)
