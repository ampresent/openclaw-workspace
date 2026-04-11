# 状态

- **阶段**: v0.3 实现中
- **创建**: 2026-04-12
- **上次更新**: 2026-04-12 06:07

## 进度

| 模块 | 状态 | 备注 |
|------|------|------|
| 设计文档 | ✅ 完成 | v0.1 + v0.2 + v0.3 设计 |
| README | ✅ 完成 | |
| QUICKSTART | ✅ 完成 | 5 步快速入门指南 |
| CONTRIBUTING | ✅ 完成 | 开发者指南 |
| SECURITY | ✅ 完成 | 威胁模型 + 加固计划 |
| INTEGRATIONS | ✅ 完成 | 多主机、Docker、K8s 路线图 |
| agent (axum HTTP) | ✅ v0.3 完成 | 14 个端点 + 错误类型 + 认证 + 健康检查 + 备份 |
| MCP Server | ✅ v0.3 完成 | 13 tools + hosts.toml + SSH 隧道 + 格式化输出 |
| NixOS module | ✅ 完成 | systemd 加固 + 新选项 + flake 改进 |
| 单元测试 | ✅ 完成 | 风险评估 + 包解析 + hosts.toml + MCP 工具路由 |
| AI 配置生成 | ✅ 模板模式 | 9 种 NixOS 模式匹配，LLM 集成待定 |
| 备份系统 | ✅ 完成 | 快照 + 轮转 + 恢复 |
| config_test | ✅ 完成 | test-before-switch + 自动切换 |
| TLS | 📋 设计完成 | tls.rs 配置结构，实现待 rustls 集成 |
| SSH 隧道 | ✅ MCP 端完成 | 自动建立 SSH 隧道 |

## API 端点清单 (14)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /health | 健康检查 |
| GET | /api/snapshot | 系统快照 |
| GET | /api/logs | 服务日志 |
| GET | /api/config | 读取配置 |
| GET | /api/package | 包信息 |
| GET | /api/generations | Generation 列表/对比 |
| POST | /api/config/validate | Dry-build 验证 |
| POST | /api/config/apply | 应用配置 |
| POST | /api/config/test | Test-before-switch |
| POST | /api/config/generate | AI 配置生成 |
| POST | /api/rollback | 回滚 |
| GET | /api/backups | 备份列表 |
| POST | /api/backup/create | 创建备份 |
| POST | /api/backup/restore | 恢复备份 |

## MCP 工具清单 (13)

system_snapshot, service_logs, config_read, package_info, generation_diff, config_validate, config_apply, config_test, config_generate, rollback_list, rollback_apply, backup_list, backup_create, backup_restore

## 关键决策

- 从 GitOps pull 模型改为 AI 诊断 + 安全执行模型
- 核心价值：NixOS 的人话接口，不是配置同步工具
- AI 不内置，通过 MCP 接入 Claude Code 等外部 agent
- risk_level 分级帮助非技术用户理解变更影响
- nix-evo-agent 暴露 HTTP API，MCP Server 做翻译层
- Rust agent 无法在非 NixOS 机器上编译，需推送到 NixOS 机器测试
