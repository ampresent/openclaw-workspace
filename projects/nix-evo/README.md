# nix-evo

> 源码修复工作流 — AI Agent 通过分析和修补源代码来解决系统软件问题。
> 支持 **Nix**、**RPM**、**Conda** 三种包管理后端。

## 核心理念

**永远不要绕过问题，永远不要直接修改操作系统。**

当 AI Agent 遇到系统软件问题时：解压源码 → 分析根因 → 修源码 → 重新打包 → 通过包管理器安装。不碰运行时文件。

## 工作流

```
用户: nginx 502 了

1. 识别包管理器 → rpm
2. 下载源码     → yumdownloader --source nginx → ~/rpmbuild/
3. 诊断问题     → systemctl status / journalctl
4. 分析源码     → 进入源码目录，定位根因
5. 生成补丁     → 修改源码，生成 .patch
6. 重新打包     → rpmbuild -ba / conda build / nix-build
7. 验证安装     → 先验证再安装，不直接改文件
8. 提交上游     → 可选，生成 PR 反哺社区
```

## 支持的后端

| 后端 | 下载源码 | 打包 | 安装 | 回滚 |
|------|---------|------|------|------|
| **Nix** | `nix-build -A pkg.src` | overlay + overrideAttrs | `nixos-rebuild switch` | generation 回滚 |
| **RPM** | `yumdownloader --source` | `rpmbuild -ba` | `yum localinstall` | `yum history undo` |
| **Conda** | feedstock / conda skeleton | `conda build` | `conda install --local` | `conda install --revision` |

## 使用方式

nix-evo 以 **OpenClaw Skill** 的形式使用，不依赖独立的 MCP Server 或 agent 进程。

安装 skill 后，AI Agent 自动遵循源码修复工作流：检测包管理器 → 下载源码 → 分析 → 补丁 → 打包 → 安装。

详见 [skills/nix-evo/SKILL.md](../../skills/nix-evo/SKILL.md)

## 安全约束

- 所有变更先验证（dry-run / test build）再安装
- 风险等级评估：safe / moderate / dangerous
- 补丁文件保存在版本控制中，可追溯
- 反模式：不编辑运行时文件、不跳过验证、不混用包管理器

## 架构（历史）

> 以下为早期多组件架构设计，现已简化为纯 skill 模式。代码保留在 `evo/` 和 `mcp-server/` 以备未来多机编排场景复用。

```
早期设计（已暂停）:
用户 → MCP Server → nix-evo-agent → NixOS

当前设计（活跃）:
用户 → AI Agent (skill) → 系统 (bash 直接执行)
```
