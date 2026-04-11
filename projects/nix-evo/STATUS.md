# 状态

- **阶段**: 实现中
- **创建**: 2026-04-12
- **上次更新**: 2026-04-12 05:45

## 进度

| 模块 | 状态 | 备注 |
|------|------|------|
| 设计文档 | ✅ 完成 | v0.1 + v0.2 设计 |
| README | ✅ 完成 | |
| QUICKSTART | ✅ 完成 | 5 步快速入门指南 |
| agent (axum HTTP) | ✅ v0.1 完成 | 8 个端点 + 错误类型 + 认证 + 改进解析 |
| MCP Server | ✅ v0.1 完成 | 9 tools + hosts.toml + SSH 隧道 + 格式化输出 |
| NixOS module | ✅ 完成 | module.nix + package.nix + flake.nix |
| 单元测试 | ✅ 完成 | 风险评估 + 包解析 + hosts.toml 解析 |
| SSH 隧道 | ✅ MCP 端完成 | 自动建立 SSH 隧道 |
| v0.2 设计 | ✅ 完成 | test-before-switch + secrets 管理 |

## 关键决策

- 从 GitOps pull 模型改为 AI 诊断 + 安全执行模型
- 核心价值：NixOS 的人话接口，不是配置同步工具
- AI 不内置，通过 MCP 接入 Claude Code 等外部 agent
- risk_level 分级帮助非技术用户理解变更影响
- nix-evo-agent 暴露 HTTP API，MCP Server 做翻译层
- Rust agent 无法在非 NixOS 机器上编译，需推送到 NixOS 机器测试
