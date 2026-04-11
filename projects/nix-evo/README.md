# nix-evo

> NixOS 的人话接口 — 让不懂 Nix 的人通过 AI 安全地管理 NixOS 服务器。

## 核心场景

用户服务器出了问题，在本地打开 Claude Code，用自然语言描述问题。Claude Code 通过 nix-evo 读取服务器状态、定位问题、生成修复方案、dry-run 验证、安全执行。

## 架构

```
用户 ("nginx 502")
    │
    ▼
Claude Code (AI Agent, 本地)
    │
    │ MCP stdio
    │
nix-evo MCP Server (本地, 桥梁)
    │
    │ HTTP / SSH
    │
nix-evo-agent (NixOS 服务器)
    │
    ▼
NixOS 系统
```

## MCP Tools

| Tool | 作用 |
|------|------|
| system_snapshot | 服务器全局状态（服务、磁盘、内存、最近失败） |
| service_logs | 指定服务的 journalctl 日志 |
| config_read | 读取 configuration.nix 源码 |
| package_info | 已安装包的详细信息 |
| generation_diff | 对比两个 generation 的差异 |
| config_validate | dry-run 验证变更 + 摘要 + 风险评估 |
| config_apply | 确认执行 nixos-rebuild switch |
| rollback_list | 列出可用 generation |
| rollback_apply | 回滚到指定 generation |

## 两个组件

- **nix-evo-agent** — 跑在 NixOS 服务器上的 HTTP API 服务
- **nix-evo MCP Server** — 跑在用户本地，翻译 MCP → agent API

## 安全

- 变更前必须 dry-run + 展示影响摘要
- 风险分级：safe / moderate / dangerous
- 所有 apply 生成 generation，支持一键回滚
- API 默认只监听 127.0.0.1

详细设计见 [DESIGN.md](./DESIGN.md)
