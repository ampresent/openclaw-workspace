# 日志

## 2026-04-12 (续3) — 实现 v0.1 骨架
- nix-evo-agent Rust 项目：axum HTTP server + 6 个端点全部实现
  - system_snapshot, service_logs, config_read, package_info
  - generation_diff, config_validate, config_apply, rollback
- MCP Server TypeScript：9 个 MCP tool，stdio transport
  - 集成 risk assessment 层（在 MCP 侧做风险标注）
  - 支持环境变量配置 agent URL 和 token
- NixOS 模块：module.nix (systemd service) + package.nix + flake.nix
- hosts.toml 示例配置
- 待做：需要 NixOS 机器上编译测试、Cargo.lock 生成

## 2026-04-12 (续2) — 设计重构：从 GitOps 到 AI 诊断
- 重新定位核心场景：不懂 Nix 的用户通过 Claude Code 管理 NixOS 服务器
- 从 "配置同步" 转向 "诊断 + 预览 + 安全执行 + 回滚"
- 核心价值重定义：nix-evo = NixOS 的人话接口
- MCP Tools 设计：9 个工具覆盖诊断→修复→回滚全流程
- 引入 risk_level 分级（safe/moderate/dangerous）帮助非技术用户
- agent 从 CLI 改为 HTTP API (axum)，MCP Server 做翻译层

## 2026-04-12 (续) — 初始设计
- 从 evolution-os (Rocky/RPM/push) 转向 nix-evo (NixOS/GitOps/pull)
- 确认技术栈：Rust, SSH, Git (git2-rs)
- 设计文档 v0.1 完成（GitOps pull 模型）

## 2026-04-12
- 项目创建
- 从 evolution-os 分离，独立项目目录
