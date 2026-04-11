# 状态

- **阶段**: 设计完成，待实现
- **创建**: 2026-04-12
- **上次更新**: 2026-04-12

## 进度

| 模块 | 状态 | 备注 |
|------|------|------|
| 设计文档 | ✅ 完成 | pull 模型 (GitOps) |
| README | ✅ 完成 | |
| evo CLI 骨架 | ⬜ 待做 | Rust |
| agent (init/watch/apply/rollback) | ⬜ 待做 | |
| MCP server | ⬜ 待做 | stdio, 6 tools |
| NixOS module | ⬜ 待做 | |

## 决策记录

- 从 evolution-os (Rocky/RPM/push) 转向 nix-evo (NixOS/GitOps/pull)
- AI 不内置，通过 MCP 接入外部 agent
- v0.1 只做单机 + 轮询，不搞多机和 webhook
