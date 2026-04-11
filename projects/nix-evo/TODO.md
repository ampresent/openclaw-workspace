# TODO

## v0.1 — 最小可用

- [x] nix-evo-agent Rust 项目骨架 (axum HTTP server)
- [x] `system_snapshot` — systemctl + 磁盘/内存 + 最近失败服务
- [x] `service_logs` — journalctl 封装
- [x] `config_read` — 读 /etc/nixos/ 配置文件
- [x] `package_info` — nix-store / nix 查询封装
- [x] `generation_diff` — nixos-rebuild generations + diff
- [x] `config_validate` — nixos-rebuild dry-build + 摘要解析 + 风险评估
- [x] `config_apply` — nixos-rebuild switch + generation 记录
- [x] `rollback_list` / `rollback_apply` — generation 管理
- [x] MCP server — stdio transport, JSON-RPC 2.0, 9 tools
- [ ] hosts.toml — 多主机连接配置 (MCP 侧，目前用环境变量)
- [ ] SSH 隧道 — 远程访问 agent API
- [ ] NixOS 集成测试 — 需要 NixOS 机器编译 + 运行
- [ ] Cargo.lock — 需要 cargo generate-lockfile

## v0.2 — 增强

- [ ] nixpkgs 源码级修改工具
- [ ] nixos-rebuild test 后再 switch（自动测试）
- [ ] secrets 管理集成 (agenix/sops-nix)
- [ ] TUI 看板

## v0.3 — 扩展

- [ ] 多机编排
- [ ] webhook 触发（GitHub/Gitea）
- [ ] 配置模板/共享模块
