# 状态

- **阶段**: 实现中
- **创建**: 2026-04-12
- **上次更新**: 2026-04-12 06:05

## 进度

| 模块 | 状态 | 备注 |
|------|------|------|
| 设计文档 | ✅ 完成 | v0.1 + v0.2 设计 |
| README | ✅ 完成 | |
| QUICKSTART | ✅ 完成 | 5 步快速入门指南 |
| agent (axum HTTP) | ✅ v0.3.1 | 10 端点 + 超时 + 请求追踪 |
| MCP Server | ✅ v0.3.1 | 13 tools + 格式化输出 + SSH 隧道 |
| NixOS module | ✅ v0.3.1 | systemd hardening + 新配置选项 |
| flake.nix | ✅ v0.3.1 | flake-utils + apps + devShells |
| 单元测试 | ✅ 完成 | 风险评估 + 包解析 + diff |
| 配置 diff | ✅ 完成 | LCS-based unified diff |
| 配置生成 | ✅ 模板模式 | 10 个常见模式 + 置信度评分 |

## 关键决策

- 从 GitOps pull 模型改为 AI 诊断 + 安全执行模型
- 核心价值：NixOS 的人话接口，不是配置同步工具
- AI 不内置，通过 MCP 接入 Claude Code 等外部 agent
- risk_level 分级帮助非技术用户理解变更影响
- nix-evo-agent 暴露 HTTP API，MCP Server 做翻译层
- Rust agent 无法在非 NixOS 机器上编译，需推送到 NixOS 机器测试
- 命令超时 120s 防止 nixos-rebuild hang 住
