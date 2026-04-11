# nix-evo

> GitOps for NixOS — AI agent 改 Git 仓库，机器自动 pull + rebuild。

## 快速开始

```bash
# 在 NixOS 主机上
nix-evo-agent init --repo git@github.com:you/nixos-config.git
nix-evo-agent watch

# 修改配置（手动或通过 AI agent）
# 编辑 configuration.nix → git commit → git push

# 主机自动检测变更并提示 apply
nix-evo-agent apply
```

## 架构

```
AI Agent ──修改──→ Git 仓库 (configuration.nix)
                        │
                   git pull (检测变更)
                        │
               nix-evo-agent (NixOS 主机上)
                   │          │
              dry-run     nixos-rebuild switch
```

## 组件

- **nix-evo-agent** — 跑在 NixOS 主机上的 agent，负责 pull + rebuild
- **nix-evo CLI** — 管理端工具（可选）
- **MCP Server** — 给 AI agent 的工具接口

## 命令

```bash
nix-evo-agent init --repo <url>    # 关联 Git 仓库
nix-evo-agent watch [--interval 60] # 监听变更
nix-evo-agent apply [--dry-run]    # 应用变更
nix-evo-agent rollback [--to <gen>] # 回滚
nix-evo-agent status               # 查看状态
```

## MCP Tools

| Tool | 作用 |
|------|------|
| system_status | 系统状态快照 |
| config_get | 读取配置文件 |
| config_diff | 查看待应用变更 |
| config_apply | 提交并触发 apply |
| rollback_list | generation 历史 |
| rollback_apply | 回滚到指定 generation |

详细设计见 [DESIGN.md](./DESIGN.md)
