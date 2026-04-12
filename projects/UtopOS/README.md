# UtopOS

> **Agent-native OS Management** — 让 AI Agent 成为操作系统的原生管理者。
> 不是写代码的 Copilot，是管系统的 Copilot。

## 一句话

UtopOS 是第一个开源的 **Agent-native 操作系统管理平台**：
AI Agent 通过修补源码、重新打包、包管理器安装来管理操作系统。
不碰运行时文件，所有变更可追溯、可回滚。

## 为什么叫 "evo"

**Evolution（进化）**。操作系统不应该是一次安装后逐渐腐化的静态产物，
而应该是一个**持续进化的有机体**：
- 遇到问题 → AI 分析根因 → 修补源码 → 系统进化
- 上游 bug → AI 应用补丁 → 系统比上游更好
- 安全漏洞 → AI 即时修复 → 不等发行版更新

系统在 AI 的辅助下**持续进化**，而不是**持续腐化**。

---

## 市场定位

### Agentic OS 格局（2025-2026）

```
┌─────────────────────────────────────────────────────────────────┐
│                    Agentic OS 赛道全景                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ AIOS         │  │ Devin/Copilot│  │ Anthropic Operator   │  │
│  │ (学术研究)    │  │ (代码生成)    │  │ (浏览器自动化)        │  │
│  │ LLM as OS    │  │ 写代码       │  │ 操作 GUI             │  │
│  │ Agent as App │  │ 不管系统     │  │ 不碰底层             │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ Ansible+AI   │  │ K8s+AI       │  │ UtopOS ★            │  │
│  │ (配置管理)    │  │ (容器编排)    │  │ (源码级系统管理)       │  │
│  │ 管配置文件    │  │ 管容器       │  │ 管源码+包             │  │
│  │ 运行时修改    │  │ 不碰宿主OS   │  │ 包管理器原生集成       │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 我们和别人的区别

| 项目 | 做什么 | 不做什么 | Agent 的角色 |
|------|--------|---------|-------------|
| **AIOS** | LLM 作为 OS 内核的抽象层 | 不管真实 Linux 系统 | 概念验证 |
| **Devin** | 自主写代码、提交 PR | 不管部署和运行 | 代码工人 |
| **Claude Code** | 辅助编程、执行命令 | 不提供系统管理工作流 | 万能助手（但没有 OS 专项知识） |
| **Ansible + AI** | AI 生成 playbook，Ansible 执行 | 不改源码，只管配置 | 前端 |
| **K8s + AI** | AI 管容器编排 | 不碰宿主 OS | 编排器 |
| **Operator** | 浏览器自动化 | 不碰 CLI 和系统 | GUI 操作员 |
| **UtopOS** | **修补源码 → 重打包 → 包管理器安装** | 不碰运行时文件 | **系统的外科医生** |

### 核心差异：Source-First

所有其他方案都在操作系统的**表面层**工作：
- Devin 写代码，但不管理运行代码的系统
- Ansible 管配置文件，但下次包更新可能覆盖
- K8s 管容器，但容器内的 OS 问题还是问题
- AIOS 是学术概念，没有落地到真实系统

UtopOS 工作在操作系统的**源码层**：
- 直接修改软件源码
- 通过包管理器重新打包
- 变更持久化、可追溯、可回滚
- 系统不是被"维护"，而是被"进化"

---

## 架构

```
用户: "nginx 502 了"
    │
    ▼
AI Agent (Claude Code / OpenClaw)
    │ 读 skill（决策层）
    │ 调脚本（执行层）
    │
    ├──→ scripts/evo-detect     → 检测: rpm
    ├──→ scripts/evo-fetch-src  → 源码: /tmp/evo-fix-nginx/src/
    ├──→ exec: journalctl       → 诊断: upstream timeout
    ├──→ 分析源码               → 根因: 默认值 60s 太短
    ├──→ scripts/evo-patch      → 补丁: fix-timeout.patch
    ├──→ scripts/evo-build      → 构建: nginx-1.24.0-1.el9.rpm
    ├──→ scripts/evo-verify     → 验证: risk=moderate
    │    [用户确认]
    ├──→ scripts/evo-install    → 安装: txn_id=15
    │
    ▼
操作系统 (NixOS / Rocky / Fedora / Conda)
```

**无守护进程。** 无独立进程。纯脚本 + Skill 文档 + AI Agent。

---

## 支持的后端

| 后端 | 系统 | 补丁方式 | 回滚 |
|------|------|---------|------|
| **Nix** | NixOS | overlay + overrideAttrs | generation 切换 |
| **RPM** | Rocky / RHEL / Fedora / CentOS | SRPM + spec patch | yum history undo |
| **Conda** | 任何 Conda 环境 | feedstock + meta.yaml | conda --revision |

---

## 快速开始

```bash
# 1. 初始化
scripts/evo-init

# 2. 对 AI 说
"nginx 502 了"

# 3. AI 自动走完流程
```

详见 [用户指南](docs/guide/GETTING-STARTED.md)

---

## 文档

| 文档 | 说明 |
|------|------|
| [用户指南](docs/guide/GETTING-STARTED.md) | 5 分钟上手 |
| [竞品分析](docs/series/vol-00-landscape.md) | Agentic OS 全景 + 我们的差异 |
| [系列文档](docs/series/INDEX.md) | 8 卷深度解析 |
| [Skill 决策层](skills/UtopOS/SKILL.md) | Agent 的操作手册 |
| [设计文档](DESIGN.md) | 架构决策 |
| [贡献指南](CONTRIBUTING.md) | 参与开发 |
| [文件监控系统](docs/FILE-MONITORING.md) | 变更跟踪、异常检测、回滚 |
| [回滚方案](docs/ROLLBACK.md) | 两层回滚：包管理器 + 文件系统 |

---

## 愿景

操作系统的历史：

```
1970s  手动编译     →  系统管理员手动编译安装一切
1990s  包管理器     →  apt/yum 自动处理依赖和安装
2010s  容器化       →  Docker 把问题推到镜像层
2020s  配置即代码   →  Ansible/Terraform 管理基础设施
2026   Agent-native →  AI Agent 修补源码、重打包、进化系统
```

UtopOS 是 **2026 的操作系统管理范式**：
不是管配置（Ansible），不是管容器（K8s），不是写代码（Devin），
而是**直接管操作系统的源码，让系统持续进化**。
