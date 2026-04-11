# TODO

## v0.1 — 最小可用

- [ ] nix-evo-agent Rust 项目骨架 (axum HTTP server)
- [ ] `system_snapshot` — systemctl + 磁盘/内存 + 最近失败服务
- [ ] `service_logs` — journalctl 封装
- [ ] `config_read` — 读 /etc/nixos/ 配置文件
- [ ] `package_info` — nix-store / nix 查询封装
- [ ] `generation_diff` — nixos-rebuild generations + diff
- [ ] `config_validate` — nixos-rebuild dry-build + 摘要解析 + 风险评估
- [ ] `config_apply` — nixos-rebuild switch + generation 记录
- [ ] `rollback_list` / `rollback_apply` — generation 管理
- [ ] MCP server — stdio transport, JSON-RPC 2.0
- [ ] hosts.toml — 多主机连接配置
- [ ] SSH 隧道 — 远程访问 agent API

## v0.2 — 增强

- [ ] nixpkgs 源码级修改工具
- [ ] nixos-rebuild test 后再 switch（自动测试）
- [ ] secrets 管理集成 (agenix/sops-nix)
- [ ] TUI 看板

## v0.3 — 扩展

- [ ] 多机编排
- [ ] webhook 触发（GitHub/Gitea）
- [ ] 配置模板/共享模块
