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

- **阶段**：设计 → 骨架搭建
- **语言**：Rust (evo CLI), Shell (构建脚本)
- **基础**：Rocky Linux src.rpm
- **AI 层**：Claude Code (云端) + 本地小模型 (意图感知)

## 关键命令 (规划)

```bash
evo rebase          # 同步上游，交互式冲突解决
evo build           # 触发构建
evo status          # TUI 看板
evo tag --create    # 稳定标记
evo freeze          # 暂停进化，退化为只读 Linux
```

详细设计见 [DESIGN.md](./DESIGN.md)
