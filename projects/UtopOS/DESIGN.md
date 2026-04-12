# UtopOS 设计文档

> 源码修复工作流 — AI Agent 通过分析和修补源代码来解决系统软件问题。
> 支持 Nix、RPM、Conda 三种包管理后端。

## 一、项目定位

UtopOS 是一个 OpenClaw Skill，指导 AI Agent 遵循 source-first 的修复工作流：不绕过问题，不直接修改操作系统，而是解压源码、分析根因、修补源码、重新打包、通过包管理器安装。

### 核心场景

用户服务器出了问题，AI Agent 通过 UtopOS skill 自动检测包管理器、下载源码、诊断问题、生成补丁、重新打包、安全安装。

### 目标用户

- 不懂包管理的运维人员
- 使用 AI Agent 管理服务器的开发者

## 二、架构

```
用户 ("nginx 502 了")
    │
    ▼
AI Agent (OpenClaw, 遵循 UtopOS skill)
    │
    │ bash (exec)
    │
系统命令 (systemctl / journalctl / rpm / nix-build / conda)
    │
    ▼
操作系统 (NixOS / Rocky / Fedora / Conda env)
```

**无独立进程。** Agent 直接执行系统命令，skill 定义工作流和安全约束。

## 三、工作流

### 3.1 检测 + 源码获取

Agent 收到问题后第一件事：

```bash
# 检测包管理器
detect_backend() → nix | rpm | conda

# 根据结果下载源码
fetch_source("nginx") → /tmp/evo-fix-nginx/
```

### 3.2 诊断

```bash
systemctl status nginx
journalctl -u nginx -n 50
```

### 3.3 分析源码

进入 `/tmp/evo-fix-nginx/`，分析源码中的根因。

### 3.4 生成补丁

根据后端生成对应格式的补丁。

### 3.5 验证 + 安装

先验证再安装，不直接改文件。

### 3.6 提交上游（可选）

生成 PR 反哺社区。

## 四、后端差异

### Nix

- 源码获取：`nix-build '<nixpkgs>' -A pkg.src`
- 补丁方式：overlay + `overrideAttrs`
- 安装：`nixos-rebuild switch`
- 回滚：generation 回滚

### RPM

- 源码获取：`yumdownloader --source` → `rpm -ivh *.src.rpm`
- 补丁方式：修改源码 + spec 文件注册补丁
- 安装：`rpmbuild -ba` → `yum localinstall`
- 回滚：`yum history undo`

### Conda

- 源码获取：clone conda-forge feedstock / `conda skeleton pypi`
- 补丁方式：`meta.yaml` patches 字段
- 安装：`conda build` → `conda install --local`
- 回滚：`conda install --revision`

## 五、安全模型

### 反模式（绝对禁止）

1. **不编辑运行时文件** — 任何 `/nix/store`、`/usr/lib`、`$CONDA_PREFIX` 下的文件
2. **不绕过问题** — 不重启糊弄、不 while-loop 自动重启
3. **不跳过验证** — 必须先 dry-run / test build 再安装
4. **不混用包管理器** — 在 conda 里不用 pip，在 NixOS 上不用 yum

### 风险评估

| 风险等级 | 条件 |
|---------|------|
| safe | 只修改配置默认值，不删包，不改网络/磁盘/引导 |
| moderate | 重启核心服务，升级包版本 |
| dangerous | 删除包，改防火墙，改磁盘，改引导加载器 |

## 六、与早期设计的关系

早期设计（v0.1-v0.4）采用了双组件架构：UtopOS-agent (Rust HTTP) + UtopOS MCP Server (TypeScript)。代码保留在 `evo/` 和 `mcp-server/`。

v0.5 简化为纯 skill 模式，原因：
- 核心操作都是 bash 命令，不需要中间层
- Skill 更轻量，维护成本低
- 安全靠 skill 约束，不靠代码层强制
- 未来需要多机编排或 MCP 兼容时再恢复 agent 层
