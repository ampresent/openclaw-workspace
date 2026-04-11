# TODO

## v0.3.1 — 已完成

- [x] 配置 diff 端点 — LCS-based unified diff
- [x] 请求计数器 — AtomicU64 for tracing
- [x] 命令超时 — 120s default, configurable
- [x] MCP 13 tools — 添加 config_diff, config_generate, backup_list, backup_create, backup_restore
- [x] NixOS module 增强 — systemd hardening, 更多配置选项
- [x] flake.nix 改进 — flake-utils, apps, devShells, formatter
- [x] 强化健康检查 — version + nixos detection + uptime

## v0.4 — 已完成 (Rounds 13-19)

- [x] Docker 集成 — 容器发现 + compose 验证 + NixOS 替代建议
- [x] CI/CD pipeline — Webhook 接收 + 预览部署 + 部署追踪
- [x] 可观测性 — 结构化日志 + Prometheus 指标 + 告警规则
- [x] NixOS 模块 — nextcloud, jellyfin, monitoring-stack
- [x] 开发模式 — 模拟系统环境 + mock 端点 + HOW-TO-CONTRIBUTE
- [x] API 版本管理 — v1/v2 + 弃用头 + 稳定性保证文档
- [x] 智能顾问 — 回滚评分 + 容量规划

## v0.5 — 下一步

- [ ] TLS 实现 (rustls 集成)
- [ ] MCP 新工具对接 (docker_status, advisor_rollback, capacity)
- [ ] WebSocket 支持 — 流式日志
- [ ] 请求 ID header (X-Request-Id) 传递
- [ ] Rate limiting (tower-governor)
- [ ] 多文件配置管理 (imports)
- [ ] 配置模板系统 (常见场景的 snippet 库)
- [ ] 多主机编排 (v2 API)
- [ ] Loki 日志推送集成
- [ ] Grafana dashboard 自动配置
