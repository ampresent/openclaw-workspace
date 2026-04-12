# 第二卷：架构深度剖析

---

## 2.1 三层架构

```
┌─────────────────────────────────────────────┐
│  用户层 (User)                               │
│  "nginx 502 了"                              │
│  确认关键决策、查看风险摘要                   │
└──────────────────┬──────────────────────────┘
                   │
┌──────────────────▼──────────────────────────┐
│  决策层 (Skill)                              │
│  skills/UtopOS/SKILL.md                     │
│  - 工作流决策树                               │
│  - 用户交互协议                               │
│  - 风险评估规则                               │
│  - 信任白名单                                 │
│                                              │
│  skills/UtopOS-{nix,rpm,conda}/SKILL.md     │
│  - 后端专项知识                               │
│  - 排障指南                                   │
└──────────────────┬──────────────────────────┘
                   │
┌──────────────────▼──────────────────────────┐
│  执行层 (Scripts)                            │
│  scripts/evo-*                               │
│  - evo-detect: 检测后端                       │
│  - evo-fetch-source: 下载源码                 │
│  - evo-build: 构建                            │
│  - evo-verify: 验证                           │
│  - evo-install: 安装                          │
│  - evo-rollback: 回滚                         │
│  - evo-patch-*: 补丁管理                      │
│  - ...14 个脚本                               │
└──────────────────┬──────────────────────────┘
                   │
┌──────────────────▼──────────────────────────┐
│  系统层 (OS)                                 │
│  nixos-rebuild / rpmbuild / conda build      │
│  journalctl / systemctl / rpm / conda        │
└─────────────────────────────────────────────┘
```

---

## 2.2 为什么是这个架构

### 早期设计（v0.1-v0.4）

```
用户 → MCP Server (TypeScript) → UtopOS-agent (Rust, 39 endpoints) → NixOS
```

问题：
- 两个独立进程需要维护
- Rust Agent 有 39 个 endpoint，但核心操作都是 bash 命令
- MCP Server 做的事情 Agent 本身就能做
- 部署复杂度高

### 当前设计（v0.5+）

```
用户 → AI Agent (skill) → bash scripts → OS
```

简化逻辑：
- Agent 有 exec 能力，不需要中间层翻译
- Skill 是纯文本，更新不需部署
- 脚本是 bash，谁都能改、能审计
- 安全靠约束规则，不靠代码强制

### 什么时候恢复 Agent 层

当出现以下需求时，可以恢复 Rust Agent + MCP Server：
- **多机编排**：需要协调多台机器的变更
- **MCP 兼容**：给非 OpenClaw 的 MCP 客户端用
- **强制执行**：多租户场景，需要代码层强制安全策略
- **高频操作**：Agent 调用频率高，需要常驻进程减少启动开销

代码保留在 `evo/` 和 `mcp-server/`，随时可恢复。

---

## 2.3 Skill 的设计

### 主 Skill（决策层）

`skills/UtopOS/SKILL.md` 是 agent 的"操作手册"，包含：

| 部分 | 作用 |
|------|------|
| 核心铁律 | 什么不能做（反模式） |
| 决策树 | 什么时候调什么脚本 |
| 脚本速查表 | 每个脚本的输入输出 |
| 用户交互协议 | 哪些节点必须确认 |
| 风险摘要模板 | 统一的风险展示格式 |
| 信任白名单 | safe 级补丁的自动策略 |

### 子 Skill（后端专项）

| Skill | 深度内容 |
|-------|---------|
| UtopOS-nix | Nix 语言速查、overlay 机制、module 开发 |
| UtopOS-rpm | Spec 文件结构、rpmbuild 用法、发行版差异 |
| UtopOS-conda | Feedstock、meta.yaml、conda build |

**分层原因**：
- Agent 只需要读主 Skill 就能走通通用流程
- 遇到后端特有问题才读子 Skill
- 减少 token 消耗，提高响应速度

---

## 2.4 脚本的设计

### 设计原则

1. **JSON 输出** — 所有脚本输出 JSON，agent 可直接解析
2. **幂等性** — 重复执行不会出错（evo-init、evo-fetch-source 有缓存）
3. **单一职责** — 每个脚本只做一件事
4. **环境变量配置** — `EVO_HOME` 控制工作目录根路径

### 脚本分类

```
基础设施:  evo-init, evo-workspace, evo-cleanup
检测获取:  evo-detect, evo-fetch-source, evo-get-info
构建管理:  evo-build, evo-verify, evo-install, evo-rollback
补丁管理:  evo-patch-create, evo-patch-list, evo-patch-check, evo-patch-series
```

### 数据流

```
evo-detect
  → { backend: "rpm", version: "4.18.0", tools: {...} }

evo-fetch-source nginx
  → { src_dir: "/tmp/evo-fix-nginx/src", spec: "~/rpmbuild/SPECS/nginx.spec" }

evo-patch-create nginx --desc "fix timeout"
  → { patch: "~/.evo/patches/nginx/fix-timeout.patch", risk: "moderate" }

evo-build nginx --patch ~/.evo/patches/nginx/fix-timeout.patch
  → { result: "~/rpmbuild/RPMS/x86_64/nginx-*.rpm", log: "~/.evo/logs/..." }

evo-verify nginx
  → { risk: "safe", missing_deps: [], test_install_ok: true }

evo-install nginx
  → { txn_id: "15", rollback_cmd: "yum history undo 15" }
```

Agent 解析 JSON → 决定下一步 → 调下一个脚本 → 循环。

---

## 2.5 目录结构

```
~/.evo/                          # EVO_HOME
├── config                       # 后端、策略、缓存天数
├── trust.toml                   # 信任白名单
├── inventory.toml               # 多机清单
├── patches/                     # 补丁文件
│   └── nginx/
│       ├── fix-timeout.patch
│       └── fix-timeout.patch.meta.json
├── builds/                      # 构建产物
│   └── nginx/
│       └── overlay-20260412.nix
├── cache/                       # 源码缓存
│   └── nginx/
│       └── src/
├── logs/                        # 构建/安装日志
│   └── nginx-20260412-160000.log
├── archive/                     # 归档的工作目录
│   └── nginx-20260412.tar.gz
├── work/                        # 持久化工作区
│   └── nginx/
├── upstream/                    # 上游仓库注册
│   └── nginx.toml
└── history/                     # 变更历史 (JSONL)
    └── nginx.jsonl

/tmp/evo-fix-nginx/              # 临时工作目录
├── src/                         # 解压的源码
├── patches/                     # 工作中的补丁
├── overlay/                     # nix overlay (nix 后端)
├── .evo-meta.json               # 工作目录元数据
├── .last-build                  # 上次构建时间
└── .evo-keep                    # 存在则不被 cleanup 清理
```

---

## 2.6 技术决策记录 (ADR)

### ADR-001: 从双组件简化为纯 skill

**状态**：已采纳
**日期**：2026-04-12
**背景**：v0.4 有 Rust Agent (39 endpoints) + TypeScript MCP Server (13 tools)
**决策**：简化为 bash scripts + skill 文档
**原因**：Agent 有 exec 能力，不需要中间层；维护两个进程成本高
**后果**：失去多机编排和 MCP 兼容性（暂不需要时可接受）

### ADR-002: JSON 统一输出

**状态**：已采纳
**背景**：脚本输出格式不一致
**决策**：所有脚本输出 JSON，stderr 用于进度信息
**原因**：Agent 解析 JSON 最方便；结构化输出利于自动化

### ADR-003: 后端 skill 拆分

**状态**：已采纳
**背景**：单一 SKILL.md 太长，包含大量后端专项内容
**决策**：拆为主 Skill（通用决策）+ 3 个子 Skill（后端专项）
**原因**：减少 token 消耗；agent 按需加载后端知识

### ADR-004: 信任白名单

**状态**：已采纳
**背景**：每次 safe 级补丁都要用户确认，体验差
**决策**：trust.toml 允许用户配置哪些包的 safe 级补丁自动 apply
**限制**：dangerous 级永远不自动，白名单变更需要手动编辑

### ADR-005: 保留早期代码

**状态**：已采纳
**背景**：v0.4 的 Rust Agent 和 MCP Server 代码量大
**决策**：保留在 `evo/` 和 `mcp-server/`，不删除
**原因**：未来有需要（多机编排、MCP 兼容）可复用
