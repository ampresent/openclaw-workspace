# TODO

## v0.1 — 最小可用

- [ ] `nix-evo-agent` Rust 项目骨架 (Cargo.toml + CLI 框架)
- [ ] `init` — clone 仓库 + 写 agent 配置
- [ ] `watch` — 轮询 Git remote 检测变更
- [ ] `apply` — pull + nixos-rebuild dry-run + switch
- [ ] `rollback` — 列出 generation + 回滚
- [ ] `status` — 当前 generation、上次同步时间、待应用变更
- [ ] MCP server — stdio transport + 6 个 tool
- [ ] NixOS module — `services.nix-evo-agent` 声明式配置
- [ ] 文档 + README

## v0.2 — 增强

- [ ] webhook 触发（GitHub/Gitea webhook）
- [ ] nixpkgs 源码级修改工具
- [ ] 自动测试（nixos-rebuild test 后再 switch）
- [ ] TUI 看板

## v0.3 — 多机

- [ ] 多主机编排
- [ ] Monorepo profiles 支持
- [ ] 配置模板/共享模块管理
