# nix-evo 设计文档

> GitOps for NixOS — AI agent 改 Git 仓库，机器自动 pull + rebuild。

## 一、项目定位

nix-evo 是 AI Agent 的 NixOS 操作层。它不自己写 Nix 代码，而是让会写 Nix 的 AI（Claude Code、Cursor、Codex 等）能安全地管理 NixOS 系统。

核心理念：**配置在 Git 里，机器来拉取。** 开发者或 AI 修改 Git 仓库中的 NixOS 配置，NixOS 主机上的 agent 检测变更后自动 pull + rebuild。

### 目标用户

- **服务器运维**：需要可控的更新、审计变更、一键回滚
- **AI 研究者**：需要 AI 能程序化地理解和修改系统环境

### 与 NixOS 的关系

nix-evo 不替代 NixOS，而是补全 NixOS 缺失的"运维交互层"。NixOS 解决了"系统是什么"（声明式配置、原子回滚），nix-evo 解决"谁来管理配置变更的生命周期"。

## 二、架构

```
AI Agent / 开发者
    │
    │ 修改 Git 仓库
    ▼
┌──────────────────────┐
│  Git 仓库 (唯一真相源) │
│  configuration.nix    │
│  flake.nix / flake.lock│
│  modules/             │
└──────────┬───────────┘
           │
           │ git pull (检测变更)
           │
┌──────────▼───────────┐
│  nix-evo-agent        │  ← 跑在 NixOS 主机上
│                       │
│  1. git pull          │
│  2. nixos-rebuild     │
│     dry-run           │
│  3. 通知用户变更内容   │
│  4. 用户确认 → switch │
│  5. generation 快照   │
└──────────────────────┘
```

### 关键设计原则

1. **Pull, not Push**：主机主动拉取，不接受远程推送。减少攻击面，符合最小权限原则。
2. **Git 是唯一真相源**：系统状态由 Git 仓库的 commit hash 唯一确定。
3. **变更需确认**：agent 检测到变更后先 dry-run 展示 diff，用户确认后才 apply（可配置为自动 apply）。
4. **原子回滚**：每次 apply 生成 NixOS generation，回滚 = `nixos-rebuild switch --rollback` 或 `git revert`。

## 三、组件设计

### 3.1 nix-evo-agent（NixOS 主机端）

跑在 NixOS 主机上的后台服务，负责拉取和应用配置变更。

#### 命令接口

```bash
# 初始化：关联 Git 仓库
nix-evo-agent init --repo <git-url> [--branch main] [--auto-apply]

# 开始监听（前台/后台）
nix-evo-agent watch [--interval 60]   # 轮询间隔，默认 60 秒

# 手动触发
nix-evo-agent apply                   # pull + dry-run + switch
nix-evo-agent apply --dry-run         # 只 dry-run，不实际执行
nix-evo-agent apply --auto            # 跳过确认，直接 switch

# 回滚
nix-evo-agent rollback                # 回滚到上一个 generation
nix-evo-agent rollback --to <gen>     # 回滚到指定 generation
nix-evo-agent rollback --git          # git revert + apply

# 状态
nix-evo-agent status                  # 当前 generation、上次同步、待应用变更
```

#### 工作流程

```
watch 循环 (每 N 秒):
  1. git fetch origin
  2. 比较本地 HEAD vs origin/<branch>
  3. 有差异？
     ├─ 否 → 跳过
     └─ 是 → git pull
              nixos-rebuild dry-build
              输出变更 diff（包变化、服务变化、配置变化）
              写入待应用队列
  4. 如果 auto-apply: 直接 nixos-rebuild switch
     否则: 等待用户手动 apply
```

#### 状态存储

```
/etc/nixos/                    # Git 仓库工作目录
/var/lib/nix-evo/
  state.json                   # agent 状态（上次同步时间、当前 generation 等）
  queue/                       # 待应用变更队列
```

### 3.2 nix-evo CLI（管理端，可选）

给运维人员和 AI agent 的管理接口。可以远程操作（通过 SSH 查询 agent 状态），也可以本机直接操作。

```bash
nix-evo status [host]          # 查看主机状态
nix-evo diff [host]            # 本地仓库 vs 远程实际状态
nix-evo apply [host]           # 触发远程 agent 执行 apply
nix-evo rollback [host]        # 触发远程回滚
```

### 3.3 MCP Server（AI Agent 接口）

nix-evo 同时作为 MCP server 运行，给 AI agent 提供工具调用接口。

| Tool | 参数 | 返回 |
|------|------|------|
| `system_status` | host | 硬件、内核、服务、generation 信息 |
| `config_get` | host, path? | configuration.nix 或指定模块内容 |
| `config_diff` | host | 待应用变更的 diff |
| `config_apply` | host, message? | commit + push + 触发 apply，返回结果 |
| `rollback_list` | host | generation 历史列表 |
| `rollback_apply` | host, target | 回滚到指定 generation |

MCP transport: **stdio**（JSON-RPC 2.0）

## 四、Git 仓库结构

```
nixos-config/                  # 每台主机一个仓库（或 monorepo 多主机）
├── flake.nix                  # Nix flake 入口
├── flake.lock                 # 锁定 nixpkgs 版本
├── configuration.nix          # 主配置
├── hardware-configuration.nix # 硬件配置（自动生成，通常不改）
├── modules/                   # 自定义模块
│   ├── monitoring.nix
│   └── backup.nix
├── secrets/                   # 加密密钥（agenix/sops-nix）
│   └── .gitkeep
└── README.md
```

### 多主机管理

两种模式，用户自选：

1. **每主机一个仓库**：简单直接，隔离性好
2. **Monorepo + NixOS profiles**：一个仓库多个 `hosts/<hostname>/` 目录，共享 modules

nix-evo agent 通过 `--repo` 指定仓库，`--host` 指定在 monorepo 中的主机名。

## 五、安全模型

### 信任边界

```
可信:
  - Git 仓库内容（由开发者/AI 提交，经过 review）
  - nix-evo-agent 二进制（由 NixOS 构建系统产出）

不可信:
  - 任何远程推送（agent 只 pull，不接受 push）
  - MCP/CLI 输入（所有外部输入需校验）
```

### 关键安全措施

1. **只出不进**：agent 只做 git pull，不开放任何端口接收外部连接
2. **变更确认**：默认 dry-run + 人工确认，auto-apply 需显式开启
3. **签名验证**：Git commit 可选 GPG 签名验证（`--verify-signatures`）
4. **最小权限**：agent 以 root 运行（nixos-rebuild 需要），但不做任何超出 rebuild 范围的操作
5. **Secrets 管理**：不处理明文密钥，推荐 agenix/sops-nix 方案

## 六、v0.1 范围

### 包含

- [ ] `nix-evo-agent init` — clone 仓库 + 写初始配置
- [ ] `nix-evo-agent watch` — 轮询 Git remote，检测变更
- [ ] `nix-evo-agent apply` — pull + dry-run + switch
- [ ] `nix-evo-agent rollback` — 回滚到上一个 generation
- [ ] `nix-evo-agent status` — 基本状态展示
- [ ] MCP server — 6 个 tool 的 stdio 接口
- [ ] NixOS module — `services.nix-evo-agent` 声明式配置

### 不包含（v0.2+）

- webhook 触发（v0.1 只用轮询）
- nixpkgs 源码级修改
- 多机编排
- GUI/TUI 看板
- 自动测试（nixos-rebuild test 后再 switch）

## 七、与旧项目 evolution-os 的关系

nix-evo 是 evolution-os 思路的重新实现。核心变化：

| | evolution-os | nix-evo |
|---|---|---|
| 底层 | Rocky Linux (RPM) | NixOS (Nix) |
| 变更模型 | Patch 栈叠加源码 | Git 仓库声明式配置 |
| AI 接入 | 内置 AI (调 API) | 外部 Agent (MCP) |
| 部署模式 | CLI push | Agent pull (GitOps) |
| 回滚 | Tag 快照 | NixOS generation + git revert |

evolution-os 的设计文档和代码保留在 `projects/evolution-os/` 作为参考，不删除。
