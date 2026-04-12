# 第一卷：项目概览与设计理念

---

## 1.1 问题的起源

### 运维的日常

2026 年，一个典型的服务器运维场景：

```
凌晨 3 点，告警响了。nginx 502。
你 ssh 进去，vim /etc/nginx/nginx.conf，改了个参数。
重启 nginx，问题解决。睡觉。

三个月后，yum update。
nginx 升级了，配置文件被覆盖。
凌晨 3 点，告警又响了。
```

这个循环每天都在发生。无数运维工程师在重复：
- 直接改运行时文件
- 忘记自己改了什么
- 下次更新覆盖一切
- 问题反复出现

### 根本矛盾

操作系统软件的管理存在一个根本矛盾：

> **包管理器维护的"干净状态"** vs **实际运维需要的"定制状态"**

包管理器假设：所有文件都由它管理，用户不应该手动修改。
运维现实：默认配置永远不够用，总得改。

传统的解决方式：
1. **配置管理工具**（Ansible/Puppet）— 管理配置文件，但不管理包本身
2. **容器化**（Docker）— 把问题推到镜像层，但镜像内依然有同样的问题
3. **祈祷** — 希望默认配置永远够用

nix-evo 提出了第四条路：

> **修改源码，重新打包，通过包管理器安装。**

不绕过问题，不和包管理器对抗，而是顺着它的逻辑走。

---

## 1.2 核心哲学

### Source-First：修源码，不修运行时

```
❌  vim /etc/nginx/nginx.conf     → 运行时文件，下次更新覆盖
✅  修改 nginx 源码中的默认值     → 通过包管理器安装，持久化
```

**为什么 source-first？**

| 维度 | 修运行时 | 修源码 |
|------|---------|--------|
| 持久化 | ❌ 被更新覆盖 | ✅ 包管理器维护 |
| 可追溯 | ❌ "谁改的？" | ✅ 补丁文件 + git log |
| 可回滚 | ❌ 靠记忆 | ✅ 包管理器原生支持 |
| 可复现 | ❌ 环境依赖 | ✅ 同一构建流程 |

### 不绕过问题

```
❌  systemctl restart nginx       → 重启不解决根因
❌  while true; do restart; done  → 更不要这样
✅  找到根因，修源码，根治
```

### 先验证再安装

```
❌  rpmbuild -ba && rpm -Uvh      → 没验证就装
✅  rpmbuild -ba && rpm -Uvh --test → 验证通过再装
```

---

## 1.3 设计原则

### 原则一：无独立进程

nix-evo 不运行守护进程、不维护常驻服务。它是一组 bash 脚本 + 一份 skill 文档。AI Agent 读 skill，调脚本，执行命令。

为什么？
- 守护进程需要维护、需要资源、需要安全审计
- 核心操作都是 bash 命令，AI Agent 有 exec 能力
- Skill 更轻量，更新不需重启

### 原则二：脚本做机械事，Skill 做决策

```
脚本（scripts/）  →  执行：下载、构建、安装、回滚
Skill（SKILL.md） →  决策：什么时候调什么脚本、怎么分析结果
Agent             →  连接：读 skill，调脚本，处理异常
用户              →  确认：关键节点批准继续
```

每一层各司其职，不越界。

### 原则三：三后端统一

Nix、RPM、Conda 是三种完全不同的包管理系统。nix-evo 用统一的脚本接口（`evo-*`）屏蔽差异，agent 只需要知道 `evo-build pkg --patch f`，不需要关心底层是 `nix-build` 还是 `rpmbuild`。

### 原则四：安全靠约束，不靠代码

早期设计用 Rust 代码强制执行安全策略（白名单、RBAC、审计日志）。现在简化为 skill 文档中的约束规则。

为什么敢这样做？
- AI Agent 读了 skill 后会遵循其约束
- 用户在关键节点有确认权
- 补丁文件有风险分级
- 所有安装支持回滚

如果有需要强制执行的场景（比如多租户），可以恢复 Rust Agent 层。

---

## 1.4 项目演进

| 版本 | 阶段 | 关键变化 |
|------|------|---------|
| v0.1 | 初始 | 5 个 MCP tool，Nix 后端 |
| v0.2 | 扩展 | SSH 隧道、13 个 MCP tool |
| v0.3 | 转向 | GitOps pull → AI 诊断 + 安全执行 |
| v0.4 | 迭代 | 19 轮 subagent，39 个 Rust endpoint |
| v0.5 | 精简 | 双组件 → 纯 skill 模式 |
| **v0.6** | **落地** | **14 个脚本工具 + 决策层文档 + 后端 skill 拆分** |

代码保留策略：早期的 Rust Agent 和 MCP Server 保留在 `evo/` 和 `mcp-server/`，未来有需要（多机编排、MCP 兼容）可复用。

---

## 1.5 与其他工具的关系

### vs Ansible/Puppet

Ansible 管配置文件，nix-evo 管包本身。互补，不竞争。

```yaml
# Ansible：管理运行时配置
- template: src=nginx.conf.j2 dest=/etc/nginx/nginx.conf
```
```bash
# nix-evo：修改源码默认值
# nginx 源码中 listen 的默认值从 80 改为 8080
# → 重新打包 → 通过包管理器安装
```

### vs Docker

Docker 把问题推到镜像层。nix-evo 在系统层解决。

```
Docker 思路：应用问题 → 改 Dockerfile → rebuild image → redeploy
nix-evo 思路：系统问题 → 改源码 → 重新打包 → 包管理器安装
```

适用场景不同：Docker 适合应用部署，nix-evo 适合系统软件。

### vs NixOS Configuration

NixOS 本身通过 `configuration.nix` 管理一切。nix-evo 在此基础上提供 AI 辅助的源码级修复能力。

```nix
# NixOS configuration.nix：管理配置
services.nginx.virtualHosts."example.com".root = "/var/www";
```
```bash
# nix-evo：当 configuration.nix 覆盖不了时
# 源码中的默认值有问题 → 修改源码 → overlay → nixos-rebuild
```

nix-evo 是 NixOS 的补丁层，不是替代品。

---

## 1.6 适用场景

### ✅ 适合用 nix-evo

- 上游 bug 影响你的系统，但上游还没修复
- 默认配置不合理，configuration.nix 覆盖不了（NixOS）或改配置文件会被覆盖（RPM）
- 安全漏洞补丁，发行版还没跟上
- 需要给系统软件加自定义功能
- 想学习包管理和系统软件构建

### ❌ 不适合用 nix-evo

- 只改应用配置（用 Ansible 或手动改就行）
- 应用部署问题（用 Docker/K8s）
- 内核编译（流程不同，不在 nix-evo 范围）
- 纯粹的运维任务（重启、扩容、监控）
