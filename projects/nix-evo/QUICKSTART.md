# nix-evo Quick Start

> **Agent-native OS Management** — 让 AI Agent 管理你的操作系统。
>
> 详细用户指南：[docs/guide/GETTING-STARTED.md](docs/guide/GETTING-STARTED.md)
> 竞品全景：[docs/series/vol-00-landscape.md](docs/series/vol-00-landscape.md)
> 深度系列文档：[docs/series/INDEX.md](docs/series/INDEX.md)

## 三步开始

```bash
# 1. 初始化
scripts/evo-init

# 2. 对 AI 说
"nginx 502 了"

# 3. AI 自动走完：检测 → 源码 → 分析 → 补丁 → 构建 → 验证 → 安装
```

## 前提

- Claude Code / OpenClaw 已安装
- 目标机器有 NixOS / RPM 系 / Conda 其中之一

## 这是什么

nix-evo 是一个 AI 驱动的系统软件修复工具。
当你的服务器出问题时，AI Agent 自动：
1. 检测包管理器
2. 下载源码
3. 诊断问题
4. 分析源码找到根因
5. 生成补丁
6. 重新打包
7. 验证 + 安装

全程不直接碰运行时文件。变更可追溯、可回滚。

## 安全

- 变更前必须验证（dry-run / test build）
- 风险分级：safe / moderate / dangerous
- 所有安装支持原生回滚
- 补丁文件保存在版本控制中
