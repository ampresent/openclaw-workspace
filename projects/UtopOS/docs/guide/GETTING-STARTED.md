# UtopOS 用户指南

> 用 Claude Code 写业务代码的同时，参与操作系统开发。
> 不需要懂 Nix 语言，不需要会写 spec 文件。AI 帮你做。

---

## 这是什么

UtopOS 是一个 AI 驱动的系统软件修复工具。

你在用 Claude Code 写 React、写 Go、写 Python。服务器出问题了——nginx 502、php-fpm 起不来、某个库有安全漏洞。以前你要么重启糊弄，要么手动改配置文件然后祈祷下次 update 不覆盖。

UtopOS 给你第三条路：**告诉 AI 你的问题，它帮你定位根因、修补源码、重新打包、安全安装。**

全程不直接碰运行时文件。变更可追溯、可回滚。

## 为什么需要这个

### 传统运维的困境

```
问题出现 → vim /etc/nginx/nginx.conf → 改完 → 问题解决
         → 三个月后 yum update → 配置被覆盖 → 问题复现
         → "上次谁改的？改了什么？" → 没人知道
```

### UtopOS 的方式

```
问题出现 → AI 分析源码 → 生成补丁 → 重新打包 → 包管理器安装
         → yum update → 补丁在，配置在，问题不复现
         → "谁改的？" → git log / patch 文件 / 变更历史
```

### 核心价值

| 传统方式 | UtopOS |
|---------|---------|
| 直接编辑运行时文件 | 修补源码，重新打包 |
| 下次更新覆盖你的修改 | 你的修改持久化 |
| 不知道谁改了什么 | 补丁文件 + 版本控制 |
| 回滚靠记忆 | 包管理器原生回滚 |
| 需要懂包管理器 | AI 帮你处理 |

## 谁适合用

- **使用 Claude Code 的开发者** — 你已经习惯用 AI 写代码，现在用它管系统
- **小团队运维** — 没有专职 SRE，但服务器需要维护
- **NixOS 学习者** — 通过实际修复案例学习 Nix，边做边学
- **开源贡献者** — 发现上游 bug，AI 帮你生成补丁和 PR

## 快速上手

### 前提

- Claude Code 已安装并能执行 bash 命令
- 目标机器有 NixOS / RPM 系 / Conda 其中之一
- UtopOS skill 已安装（见下方）

### 安装 Skill

把 `skills/UtopOS/` 目录放到你的 Claude Code workspace 的 skills 下：

```
~/.openclaw/workspace/skills/
├── UtopOS/SKILL.md          # 通用决策层
├── UtopOS-nix/SKILL.md      # Nix 后端（可选）
├── UtopOS-rpm/SKILL.md      # RPM 后端（可选）
└── UtopOS-conda/SKILL.md    # Conda 后端（可选）
```

### 初始化

跟 Claude Code 说：

> "帮我初始化 UtopOS"

它会自动运行 `evo-init`，创建 `~/.evo/` 目录结构。

### 开始使用

直接用自然语言描述问题，不需要任何特殊语法：

```
你: nginx 502 了

Claude: 我来诊断。
        [检测到 rpm 后端]
        [下载 nginx 源码到 /tmp/evo-fix-nginx/]
        [查看 journalctl 日志]
        [分析源码...]
        发现问题：upstream timeout 默认值太短，导致长请求超时。
        生成补丁，将默认超时从 60s 改为 120s。
        风险等级：moderate。要继续吗？

你: 继续

Claude: [构建中...]
        [验证通过]
        [安装完成]
        事务 ID: 15。回滚命令: yum history undo 15
```

就这么简单。

## 工作流总览

```
你说问题 → AI 检测包管理器 → 下载源码 → 诊断
        → 分析源码 → 生成补丁 → 向你展示风险摘要
        → 你确认 → 构建 → 验证 → 安装
        → 完成（可随时回滚）
```

每一步 AI 都会告诉你它在做什么，关键决策点会等你确认。

## 风险分级

AI 生成补丁后会给出风险评估：

| 等级 | 含义 | 需要确认 |
|------|------|---------|
| 🟢 safe | 只改配置默认值，不影响核心功能 | 可配置为自动 |
| 🟡 moderate | 修改源码逻辑，影响单一功能 | 必须确认 |
| 🔴 dangerous | 核心模块修改，影响面广 | 必须确认 + 二次确认 |

你可以在 `~/.evo/trust.toml` 中配置白名单，让 safe 级别的补丁自动构建，不再每次询问。

## 回滚

任何安装都有回滚方案：

```bash
# NixOS — 切换回上一个 generation
nixos-rebuild switch --rollback

# RPM — undo 上一次事务
yum history undo 15

# Conda — 回退到上一个 revision
conda install --revision 14
```

AI 安装完成后会告诉你具体的回滚命令。

## 真实场景

### 场景一：线上服务故障

```
你: php-fpm 启动失败，日志报 "failed to create socket"

Claude: [下载 php 源码... 分析 fpm 配置模板...]
        根因：php-fpm 默认监听 unix socket，
        但你的 nginx upstream 配的是 TCP 127.0.0.1:9000。
        生成补丁修改 fpm 默认监听方式为 TCP。
        风险：safe。
        [构建... 验证... 安装...]
        完成。php-fpm 现在默认监听 TCP 9000。
```

### 场景二：安全漏洞修复

```
你: openssl 有个 CVE，上游补丁已经出了，但我们的发行版还没更新

Claude: [下载 openssl 源码... 获取上游补丁...]
        已应用上游安全补丁 CVE-2026-XXXX。
        风险：moderate（影响 TLS 握手逻辑）。
        要继续吗？

你: 继续

Claude: [构建... 验证... 安装...]
        已完成。openssl 已更新。
        回滚：yum history undo 22
```

### 场景三：默认配置不合理

```
你: nginx 的 worker_connections 默认才 512，太小了

Claude: [下载 nginx 源码... 分析配置模板...]
        生成 safe 级补丁，将默认值改为 4096。
        由于你在 trust.toml 中开启了 nginx 的 safe_auto，
        直接构建。
        [构建... 验证... 安装...]
        done。
```

### 场景四：学习 NixOS

```
你: 我想了解 NixOS 的 overlay 机制是怎么工作的

Claude: 读 UtopOS-nix skill，结合实际案例讲解。
        我们来改一个包的默认值，你会看到 overlay 的完整工作流程。
        [下载 nginx 源码... 创建 overlay...]
        这就是 overlay：一个函数，接收 prev（原始 nixpkgs），
        返回修改后的版本。
```

## 常见问题

**Q: 我需要懂包管理器吗？**
A: 不需要。AI 会处理所有技术细节。你只需要描述问题和确认关键决策。

**Q: 如果 AI 生成的补丁不对怎么办？**
A: 构建会失败，AI 会分析错误日志并建议修改。你可以随时让它调整补丁。

**Q: 这个工具适合生产环境吗？**
A: UtopOS 的设计就是面向生产环境的。所有变更先验证再安装，支持回滚，补丁可追溯。

**Q: 和直接 vim 改配置有什么区别？**
A: vim 改的是运行时文件，下次包更新会覆盖。UtopOS 改的是源码，通过包管理器安装，变更持久化、可追溯、可回滚。

**Q: 支持哪些系统？**
A: NixOS / RHEL / Rocky / Fedora / CentOS（RPM 系）/ 任何 Conda 环境。

## 下一步

- [设计文档](../DESIGN.md) — 了解架构决策
- [安全模型](../SECURITY.md) — 详细的安全约束
- [贡献指南](../CONTRIBUTING.md) — 参与开发
- [系列文档](../series/) — 深入理解各个主题
