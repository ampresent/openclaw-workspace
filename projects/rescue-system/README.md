# 救援系统 (Rescue System)

基于小模型的 Linux 系统故障诊断与修复救援系统。

## 快速开始

```bash
# 1. 搭建开发环境 (编译 llama.cpp + 下载模型)
bash scripts/setup-dev.sh

# 2. 启动模型服务
bash src/model-server/start.sh

# 3. 启动救援 Shell
python3 src/rescue-shell/shell.py
```

## 核心用法

```bash
# 完整扫描：诊断 → 分析 → 修复
python3 src/repair-engine/repair.py --target /mnt/rescue-target

# 只诊断 + 分析，不修复
python3 src/repair-engine/repair.py --target /mnt/rescue-target --no-repair

# 模拟执行修复
python3 src/repair-engine/repair.py --target /mnt/rescue-target --dry-run

# 自动修复 critical/high 级别问题
python3 src/repair-engine/repair.py --target /mnt/rescue-target -y --filter-severity high

# 单独运行诊断
bash src/diagnostics/rescue-diag all /mnt/rescue-target
bash src/diagnostics/rescue-diag disk /mnt/rescue-target
```

## 项目结构

```
rescue-system/
├── config/
│   └── rescue.toml                  # 全局配置
├── scripts/
│   └── setup-dev.sh                 # 开发环境搭建
├── skills/                          # 诊断知识库
│   ├── sysadmin-toolbox/            # 运维工具参考 (the-book-of-secret-knowledge)
│   │   ├── references/
│   │   │   ├── cli-tools.md         # CLI 工具推荐
│   │   │   ├── shell-oneliners.md   # Shell 单行命令
│   │   │   ├── security-tools.md    # 安全工具
│   │   │   ├── web-tools.md         # Web 工具
│   │   │   └── shell-tricks.md      # Shell 技巧
│   │   └── scripts/refresh.sh       # 知识库更新
│   └── system-info/                 # 快速系统诊断
├── src/
│   ├── model-server/                # 模型服务 (llama.cpp)
│   │   ├── start.sh                 # 启动
│   │   └── healthcheck.sh           # 健康检查
│   ├── diagnostics/                 # Bash 诊断模块
│   │   ├── rescue-diag              # 主入口
│   │   └── modules/
│   │       ├── disk.sh              # P0 磁盘
│   │       ├── boot.sh              # P0 启动
│   │       ├── services.sh          # P1 服务
│   │       ├── memory.sh            # P1 内存
│   │       ├── network.sh           # P2 网络
│   │       ├── packages.sh          # P2 包管理
│   │       └── kernel.sh            # P3 内核
│   ├── repair-engine/               # Python 分析修复引擎
│   │   ├── repair.py                # 主入口 (完整流程)
│   │   ├── analyzer.py              # 模型分析器
│   │   └── executor.py              # 修复执行器
│   └── rescue-shell/                # 交互 Shell
│       └── shell.py                 # 自然语言入口
├── tests/
└── README.md
```

## 架构

```
用户问题 → rescue-shell → rescue-diag all → JSON报告
    → analyzer.py (调小模型分析) → 修复方案
    → confirm.py (用户确认) → executor.py (执行修复)
```

## 诊断模块优先级

| 优先级 | 模块 | 诊断内容 |
|--------|------|---------|
| P0 | disk | 磁盘空间、文件系统损坏、SMART 状态 |
| P0 | boot | GRUB/EFI、内核文件、fstab |
| P1 | services | 失败服务、关键服务状态、崩溃记录 |
| P1 | memory | 内存使用、OOM 历史、Swap |
| P2 | network | 接口、路由、DNS、防火墙 |
| P2 | packages | 包管理器状态、锁定文件、损坏包 |
| P3 | kernel | 内核 panic、taint、kdump |

## 配置

编辑 `config/rescue.toml`：

```toml
[model]
model_path = "models/qwen2.5-7b-instruct-q4_k_m.gguf"

[model.server]
port = 8081
threads = 0  # 自动

[rescue]
target_path = "/mnt/rescue-target"
```

## 开发阶段

- [x] Phase 1: 工具开发
  - [x] 诊断模块 (7 个模块)
  - [x] 模型分析器
  - [x] 修复执行器
  - [x] 交互 Shell
  - [ ] 模型服务搭建 (需要 `setup-dev.sh`)
- [ ] Phase 2: 集成测试 (虚拟机故障模拟)
- [ ] Phase 3: ISO 制作 (Alpine/Debian live)

## 硬件要求

- 内存 ≥512GB（开发机配置）
- CPU ≥100 核心
- 推荐模型: Qwen2.5-7B Q4 (约 4GB，16GB 内存即可)
