# 状态

- **阶段**: v0.5 — 纯 skill 模式
- **创建**: 2026-04-12
- **上次更新**: 2026-04-12 15:38

## 架构决策

从「MCP Server + nix-evo-agent 双组件」简化为「纯 OpenClaw Skill」。

原因：
- 核心操作都是 bash 命令，AI agent 有 exec 能力，不需要中间层翻译
- Skill 更轻量，不用维护独立进程
- 安全靠 skill 约束保证，不靠代码层强制
- 多机编排和 MCP 兼容性留到后面有需求再加

## 当前模块

| 模块 | 状态 | 备注 |
|------|------|------|
| Skill 文档 | ✅ 活跃 | `skills/nix-evo/SKILL.md` |
| 架构图 | ✅ 完成 | `docs/architecture.svg` |
| 流程对比图 | ✅ 完成 | `docs/workflow-comparison.svg` |
| Nix 后端工作流 | ✅ 完成 | 源码获取 + overlay 补丁 + generation 回滚 |
| RPM 后端工作流 | ✅ 完成 | SRPM + rpmbuild + yum history 回滚 |
| Conda 后端工作流 | ✅ 完成 | feedstock + conda build + revision 回滚 |
| 自动检测 + 源码获取 | ✅ 完成 | detect_backend() + fetch_source() |

## 已暂停的模块（保留代码）

> 以下模块来自早期多组件架构，代码保留在 `evo/` 和 `mcp-server/`，
> 当前不使用，未来有需要（多机编排、给非 OpenClaw 的 MCP 客户端用）可复用。

| 模块 | 状态 | 备注 |
|------|------|------|
| nix-evo-agent (Rust HTTP) | ⏸ 暂停 | 39 个端点，axum 框架 |
| MCP Server (TypeScript) | ⏸ 暂停 | 13 tools + hosts.toml + SSH 隧道 |
| NixOS modules | ⏸ 暂停 | nextcloud, jellyfin, monitoring-stack |
| Docker 集成 | ⏸ 暂停 | 容器发现 + compose 验证 |
| CI/CD | ⏸ 暂停 | Webhook + 预览部署 |
| 可观测性 | ⏸ 暂停 | Prometheus 指标 + 告警 |
| 开发模式 | ⏸ 暂停 | 模拟系统 + mock 端点 |
| API 版本管理 | ⏸ 暂停 | v1/v2 |
| 智能顾问 | ⏸ 暂停 | 回滚评分 + 容量规划 |
| TLS | ⏸ 暂停 | 未实现 |

## 关键决策

- ~~从 GitOps pull 模型改为 AI 诊断 + 安全执行模型~~
- **从双组件 (MCP+Agent) 简化为纯 skill 模式**
- 支持 Nix / RPM / Conda 三后端
- 核心原则：source-first，修源码不修运行时
- 自动检测包管理器 + 自动下载源码
