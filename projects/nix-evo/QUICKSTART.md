# nix-evo Quick Start

> 让 AI Agent 用源码修复工作流解决系统软件问题。
>
> 详细用户指南：[docs/guide/GETTING-STARTED.md](docs/guide/GETTING-STARTED.md)
> 深度系列文档：[docs/series/INDEX.md](docs/series/INDEX.md)

## 前提

- OpenClaw 已安装
- 目标机器有对应的包管理器（nixos-rebuild / rpm / conda）

## 1. 确认 skill 已安装

```bash
ls ~/.openclaw/workspace/skills/nix-evo/SKILL.md
```

skill 会在 AGENTS.md 的 `available_skills` 中自动注册，AI Agent 会识别并遵循其工作流。

## 2. 直接用自然语言描述问题

在对话中说：

> "nginx 502 了"

AI Agent 会自动：

1. **检测包管理器** — `which nixos-rebuild / rpm / conda`
2. **下载源码** — 根据检测结果执行对应的源码获取命令
3. **诊断问题** — `systemctl status`、`journalctl` 等
4. **分析源码** — 进入下载的源码目录，定位根因
5. **生成补丁** — 修改源码，生成 patch 文件
6. **重新打包** — `rpmbuild -ba` / `conda build` / `nix-build`
7. **验证安装** — 先 dry-run 再安装
8. **提交上游**（可选）— 生成 PR

## 3. 典型交互

```
用户: php-fpm 启动失败了

Agent: [检测到 rpm 系]
        [下载 php 源码到 /tmp/evo-fix-php/]
        [查看 journalctl 日志]
        [分析源码中的 php-fpm 默认配置]
        发现问题：php-fpm 默认监听 127.0.0.1:9000，
        但 nginx upstream 配置的是 unix socket。
        这是上游默认值的问题。

        生成补丁修复监听地址...
        重新打包...
        验证：dry-build 通过，风险等级 safe。

        要我安装吗？

用户: 确认

Agent: [安装新包]
        已完成！php-fpm 已修复。
        如果有问题可以回滚：yum history undo <ID>
```

## 安全

- 变更前必须验证（dry-run / test build）
- 风险分级：safe / moderate / dangerous
- 不直接修改运行时文件
- 补丁文件可追溯

## 故障排除

### 未识别包管理器

确保目标机器安装了 `nixos-rebuild`、`rpm` 或 `conda` 中至少一个。

### 源码下载失败

Agent 会尝试多种方式：
- Nix：`nix-build -A pkg.src`
- RPM：`yumdownloader --source` → `dnf download --source`
- Conda：`conda-forge feedstock` → `conda skeleton pypi`

### 权限不足

打包和安装通常需要 root 权限。Agent 会提示需要 `sudo`。
