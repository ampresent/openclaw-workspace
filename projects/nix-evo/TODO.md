# TODO

## v0.3.1 — 已完成

- [x] 配置 diff 端点 — LCS-based unified diff
- [x] 请求计数器 — AtomicU64 for tracing
- [x] 命令超时 — 120s default, configurable
- [x] MCP 13 tools — 添加 config_diff, config_generate, backup_list, backup_create, backup_restore
- [x] NixOS module 增强 — systemd hardening, 更多配置选项
- [x] flake.nix 改进 — flake-utils, apps, devShells, formatter
- [x] 强化健康检查 — version + nixos detection + uptime

## v0.4 — 下一步

- [ ] MCP 的 config_diff/config_generate 与 agent 对接
- [ ] WebSocket 支持 — 流式日志
- [ ] 请求 ID header (X-Request-Id) 传递
- [ ] Rate limiting (tower-governor)
- [ ] 多文件配置管理 (imports)
- [ ] 配置模板系统 (常见场景的 snippet 库)
