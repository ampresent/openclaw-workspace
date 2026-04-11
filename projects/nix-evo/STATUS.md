# 状态

- **阶段**: 实现中
- **创建**: 2026-04-12
- **上次更新**: 2026-04-12 05:45

## 进度

| 模块 | 状态 | 备注 |
|------|------|------|
| 设计文档 | ✅ 完成 | v0.1 — AI 诊断 + 安全执行 |
| README | ✅ 完成 | |
| agent (axum HTTP) | 🔨 骨架完成 | 6 个诊断/执行端点，待编译测试 |
| MCP Server | 🔨 骨架完成 | stdio, 9 tools, TypeScript |
| NixOS module | ✅ 完成 | module.nix + package.nix + flake.nix |
| SSH 隧道 | ⬜ 待做 | 远程访问 |

## 关键决策

- 从 GitOps pull 模型改为 AI 诊断 + 安全执行模型
- 核心价值：NixOS 的人话接口，不是配置同步工具
- AI 不内置，通过 MCP 接入 Claude Code 等外部 agent
- risk_level 分级帮助非技术用户理解变更影响
- nix-evo-agent 暴露 HTTP API，MCP Server 做翻译层
- Rust agent 无法在非 NixOS 机器上编译，需推送到 NixOS 机器测试
