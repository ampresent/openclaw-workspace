## 2026-04-12 (续7) — Rounds 6-12: Integration tests, observability, AI config, backups, community, security, integrations
- Round 6: MCP tool routing tests (tools.test.ts) — host resolution, request construction, validation, formatting
- Round 7: Enhanced health handler with uptime, version info; command timeout support
- Round 8: AI config generation (ai_config.rs) — template-based NixOS config from natural language, 9 patterns (nginx, docker, ssh, firewall, postgresql, redis, node, python, backup)
- Round 9: Backup system (backup.rs) — snapshot /etc/nixos before apply, rotation policy (20 auto, 50 manual), restore with safety backup
- Round 10: CONTRIBUTING.md, NixOS module README, nixpkgs PR template, example scripts (nginx, docker)
- Round 11: config_test.rs (test-before-switch with auto-switch after delay), tls.rs (TLS config structure), SECURITY.md (threat model + hardening plan)
- Round 12: INTEGRATIONS.md (multi-host, Docker, systemd-nspawn, Kubernetes, monitoring, CI/CD roadmap)


## 2026-04-12 (续6) — Round 5: v0.2 features
- SSH 隧道自动建立：MCP 端 ssh-tunnel.ts，自动端口发现，超时处理，退出清理
- DESIGN-V0.2.md：test-before-switch + secrets 管理设计
- 集成 SSH 隧道到 MCP server 的 host 解析流程

## 2026-04-12 (续5) — Round 2: MCP server completeness
- 实现 hosts.toml 配置文件解析（~/.config/nix-evo/hosts.toml）
- 添加主机选择逻辑：显式指定 > default > 单主机自动
- 添加人类可读格式化：system_snapshot、config_validate、generations、rollback_list
- 结构化输出：格式化摘要 + 原始 JSON
- 移除 smol-toml 外部依赖，使用内置解析器
- 更新工具描述，添加工作流指引

## 2026-04-12 (续4) — Round 1: Agent robustness
- 新增 `error.rs`：AppError 枚举，包含 CommandFailed/IoError/Validation/NotFound/Unauthorized/Internal
- 新增 `auth.rs`：Bearer token 认证中间件，--api-token CLI 参数或 NIX_EVO_TOKEN 环境变量
- 改进 dry-build 解析：多策略尝试（flake → no-flake → basic → impure）
- 改进包名提取：从 /nix/store/ 路径中解析哈希前的包名
- 添加 generation 描述读取：从 nix-evo-description 文件读取
- 添加配置读取路径校验：防止路径穿越
- 所有错误消息改为中文
- 添加单元测试：风险评估 + 包解析
- main.rs 路由重构：/api 嵌套路由，auth 仅保护 API 端点

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

## 2026-04-12 (续8) — Rounds 13-19: Docker, CI/CD, Observability, Modules, Dev Mode, API Versioning, Advisor

- Round 13: Docker & Container Integration (docker.rs) — container discovery, compose validation, 13 NixOS alternatives
- Round 14: CI/CD Pipeline (cicd.rs) — Git webhook receiver, preview deployments, deployment tracking
- Round 15: Observability Stack (observability.rs) — journald→structured JSON, Prometheus metrics, alert rules
- Round 16: NixOS Module Ecosystem — nextcloud.nix, jellyfin.nix, monitoring-stack.nix with full options
- Round 17: Developer Experience (dev.rs) — mock system for testing, HOW-TO-CONTRIBUTE.md
- Round 18: API Versioning (api_version.rs) — v1/v2 support, deprecation headers, API-STABILITY.md
- Round 19: Smart Advisor (advisor.rs) — rollback scoring + capacity planning (cherry-picked from feature/experimental)
