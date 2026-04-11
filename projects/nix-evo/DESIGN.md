# nix-evo 设计文档

> NixOS 的人话接口 — 让不懂 Nix 的人通过 AI 安全地管理 NixOS 服务器。

## 一、项目定位

nix-evo 是 Claude Code 等 AI Agent 与 NixOS 服务器之间的桥梁层。它不自己写 Nix 代码，也不强加 GitOps 工作流，而是让 AI agent 能安全地**诊断问题、预览变更、执行修复、一键回滚**。

### 核心场景

用户服务器出了问题，在本地打开 Claude Code，用自然语言描述问题（如"nginx 502 了"），Claude Code 通过 nix-evo 读取服务器状态、定位问题、生成修复方案、dry-run 验证、安全执行。

### 目标用户

- **不懂 Nix 的服务器运维**：知道服务器有问题，但不想学 Nix 语言
- **AI 研究者**：需要 AI 能程序化地理解和修改远程系统环境

### 核心价值

> nix-evo 不是 GitOps 工具，是 NixOS 的"人话接口"。

## 二、架构

```
用户 ("nginx 502 了")
    │
    ▼
Claude Code (AI Agent, 本地运行)
    │
    │ MCP stdio (JSON-RPC 2.0)
    │
nix-evo MCP Server (本地运行, 作为桥)
    │
    │ HTTP / SSH
    │
nix-evo-agent (NixOS 服务器上)
    │
    ▼
NixOS 系统 (configuration.nix + nixpkgs)
```

### 组件

| 组件 | 运行位置 | 职责 |
|------|----------|------|
| nix-evo-agent | NixOS 服务器 | 执行诊断命令、apply/rollback 操作 |
| nix-evo MCP Server | 用户本地 | 翻译 MCP 协议 → agent API，生成摘要和风险评估 |

nix-evo **不包含 AI**。AI 能力由 Claude Code 等外部 agent 提供。

## 三、MCP Tools（给 Claude Code 的接口）

### 3.1 诊断阶段

#### `system_snapshot`

第一步必须调的工具，获取服务器全局状态。

```json
{
  "name": "system_snapshot",
  "description": "获取 NixOS 服务器的全局状态快照",
  "parameters": {
    "host": "服务器标识 (在 hosts.toml 中定义)"
  },
  "returns": {
    "hostname": "string",
    "nixos_version": "string",
    "kernel": "string",
    "uptime": "string",
    "services": [
      {"name": "nginx.service", "active": "active", "sub": "running", "description": "A high performance web server"}
    ],
    "disk": [
      {"mount": "/", "used_pct": 45}
    ],
    "memory": {"total": "16GB", "used": "8GB", "available": "8GB"},
    "recent_failures": [
      {"unit": "phpfpm.service", "since": "2h ago", "log_excerpt": "Failed to start PHP FastCGI Process Manager"}
    ]
  }
}
```

#### `service_logs`

定向深挖某个服务的日志。

```json
{
  "name": "service_logs",
  "parameters": {
    "host": "string",
    "unit": "服务名，如 nginx.service",
    "lines": 50
  },
  "returns": {
    "logs": ["log line 1", "log line 2", "..."]
  }
}
```

#### `config_read`

读取当前 NixOS 配置源码。

```json
{
  "name": "config_read",
  "parameters": {
    "host": "string",
    "path": "可选，指定文件路径，默认 configuration.nix"
  },
  "returns": {
    "content": "Nix 源码字符串",
    "path": "文件绝对路径"
  }
}
```

#### `package_info`

查询已安装包的详细信息。

```json
{
  "name": "package_info",
  "parameters": {
    "host": "string",
    "name": "包名，如 nginx"
  },
  "returns": {
    "name": "nginx",
    "version": "1.24.2",
    "description": "...",
    "dependencies": ["openssl", "pcre2", "zlib"],
    "service_unit": "nginx.service"
  }
}
```

#### `generation_diff`

对比两个 NixOS generation 的差异。

```json
{
  "name": "generation_diff",
  "parameters": {
    "host": "string",
    "from": "可选，generation 编号，默认上一个",
    "to": "可选，generation 编号，默认当前"
  },
  "returns": {
    "packages_added": ["new-pkg-1.0"],
    "packages_removed": ["old-pkg-0.9"],
    "services_changed": ["nginx.service"],
    "config_diff": "unified diff 格式"
  }
}
```

### 3.2 修复阶段

#### `config_validate`

预览变更，不执行，只验证。Claude Code 生成修复方案后调用此工具。

```json
{
  "name": "config_validate",
  "parameters": {
    "host": "string",
    "config": "新的 NixOS 配置内容（完整或增量）"
  },
  "returns": {
    "valid": true,
    "dry_run_output": "nixos-rebuild dry-build 原始输出",
    "summary": {
      "packages_added": ["php83-1.0"],
      "packages_removed": [],
      "services_restart": ["nginx.service"],
      "services_stop": [],
      "risk_level": "safe | moderate | dangerous",
      "risk_reasons": ["涉及防火墙规则变更", "将删除 3 个包"]
    }
  }
}
```

**risk_level 判定规则**：
- **safe**：只添加/修改配置项，不删除包，不改防火墙/磁盘/引导
- **moderate**：重启核心服务、升级包版本
- **dangerous**：删除包、改防火墙、改磁盘分区、改引导加载器、改网络配置

#### `config_apply`

确认执行。用户确认后 Claude Code 调用。

```json
{
  "name": "config_apply",
  "parameters": {
    "host": "string",
    "config": "NixOS 配置内容",
    "message": "可选，变更说明，记录到 generation 注释"
  },
  "returns": {
    "success": true,
    "generation": 43,
    "summary": "php-fpm 已启用，nginx 已重启，配置已生效",
    "rollback_command": "nix-evo rollback --to 42"
  }
}
```

### 3.3 兜底阶段

#### `rollback_list`

列出可用的 generation。

```json
{
  "name": "rollback_list",
  "parameters": {"host": "string"},
  "returns": {
    "current": 43,
    "generations": [
      {"number": 43, "date": "2026-04-12 05:30", "description": "启用 php-fpm"},
      {"number": 42, "date": "2026-04-11 22:00", "description": "初始配置"}
    ]
  }
}
```

#### `rollback_apply`

回滚到指定 generation。

```json
{
  "name": "rollback_apply",
  "parameters": {
    "host": "string",
    "target": "generation 编号，不指定则回滚到上一个"
  },
  "returns": {
    "success": true,
    "reverted_to": 42,
    "summary": "已回滚到 generation 42 (2小时前的状态)"
  }
}
```

## 四、nix-evo-agent（服务器端）

跑在 NixOS 服务器上的轻量服务，暴露 HTTP API 供 MCP Server 调用。

### API 端点

| 端点 | 方法 | 对应 MCP Tool |
|------|------|---------------|
| `/api/snapshot` | GET | system_snapshot |
| `/api/logs` | GET | service_logs |
| `/api/config` | GET | config_read |
| `/api/package` | GET | package_info |
| `/api/generations` | GET | generation_diff, rollback_list |
| `/api/config/validate` | POST | config_validate |
| `/api/config/apply` | POST | config_apply |
| `/api/rollback` | POST | rollback_apply |

### 安全

- API 默认只监听 `127.0.0.1`，远程访问需通过 SSH 隧道或 VPN
- 可选 API token 认证
- 所有写操作（apply/rollback）需二次确认（agent 端也可配置 auto-approve）

### 实现

- Rust（复用已有经验）
- HTTP 框架：axum 或 actix-web
- 系统操作：调用 systemctl、nixos-rebuild、nix-store 等子进程

## 五、典型交互流程

```
用户: nginx 502 了

Claude Code: [调用 system_snapshot]
  → 发现 php-fpm.service 状态为 failed
  → [调用 service_logs("phpfpm.service")]
  → 发现 php-fpm 配置错误：监听端口与 nginx upstream 不匹配

Claude Code: 我查了一下，php-fpm 服务启动失败了，原因是监听端口配置错误。
             nginx upstream 指向 9000 端口，但 php-fpm 配置监听的是 9001。
             我来修复配置。

  [调用 config_read("/etc/nixos/configuration.nix")]
  [Claude Code 生成修复后的 Nix 配置]
  [调用 config_validate]

Claude Code: 这个修改会：
  ✅ 将 php-fpm 监听端口从 9001 改为 9000
  ✅ 重启 php-fpm 和 nginx 服务
  风险等级：低（只修改服务配置端口）

要我应用吗？

用户: 确认

Claude Code: [调用 config_apply]
  → 成功，generation 43

Claude Code: 已完成！php-fpm 端口已修正，两个服务都已重启。
             nginx 502 应该修复了。如果还有问题，可以回滚到之前的状态。
```

## 六、与 evolution-os 的关系

nix-evo 是 evolution-os 思路的重新定位。核心变化：

| | evolution-os | nix-evo |
|---|---|---|
| 底层 | Rocky Linux (RPM) | NixOS (Nix) |
| 变更模型 | Patch 栈叠加源码 | NixOS 配置声明式 |
| AI 接入 | 内置 AI (调 API) | 外部 Agent (MCP) |
| 部署模式 | CLI push | Agent API pull |
| 回滚 | Tag 快照 | NixOS generation |
| 核心价值 | 源码驱动的元 OS | NixOS 的人话接口 |
| 目标用户 | 开发者/Hacker | 不懂 Nix 的运维 |

evolution-os 的文档保留在 `projects/evolution-os/` 作为参考。

## 七、v0.1 范围

### 包含

- [ ] nix-evo-agent：HTTP API 服务 + 6 个诊断/执行端点
- [ ] nix-evo MCP Server：stdio transport，将 MCP tool 调用翻译为 agent API
- [ ] system_snapshot：systemctl 状态 + 磁盘/内存 + 最近失败服务
- [ ] service_logs：journalctl 封装
- [ ] config_read：读 /etc/nixos/ 配置文件
- [ ] config_validate：nixos-rebuild dry-build + 摘要解析 + 风险评估
- [ ] config_apply：nixos-rebuild switch + generation 记录
- [ ] rollback_list / rollback_apply：generation 管理
- [ ] hosts.toml：多主机连接配置
- [ ] SSH 隧道：远程访问 agent API

### 不包含（v0.2+）

- nixpkgs 源码级修改
- GUI/TUI 看板
- webhook 触发
- 多机编排
- 自动化测试（nixos-rebuild test 后再 switch）
- secrets 管理（agenix/sops-nix 集成）
