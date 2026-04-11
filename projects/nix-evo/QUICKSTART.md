# nix-evo Quick Start

> 5 分钟让你的 NixOS 服务器接入 Claude Code。

## 前提

- NixOS 服务器（有 `nixos-rebuild` 命令）
- 本地有 Claude Code（或兼容 MCP 的 AI agent）

## 1. 安装 nix-evo-agent（服务器端）

### 方式 A: Flake input（推荐）

```nix
# flake.nix
{
  inputs.nix-evo.url = "github:your-org/nix-evo";

  outputs = { self, nixpkgs, nix-evo, ... }: {
    nixosConfigurations.myserver = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        nix-evo.nixosModules.nix-evo-agent
        ./configuration.nix
      ];
    };
  };
}
```

```nix
# configuration.nix
services.nix-evo-agent = {
  enable = true;
  # 可选：API token 文件（每台服务器一个）
  # tokenFile = "/etc/nix-evo-token";
};
```

### 方式 B: 手动构建

```bash
# 在 NixOS 机器上
cd evo/
nix build
./result/bin/nix-evo-agent --host 127.0.0.1 --port 7890
```

## 2. 安装 MCP Server（本地）

```bash
cd mcp-server/
npm install
npm run build
```

## 3. 配置 hosts

创建 `~/.config/nix-evo/hosts.toml`：

```toml
[hosts.default]
url = "http://127.0.0.1:7890"
```

如果 agent 在远程服务器上，通过 SSH 隧道连接：

```bash
# 本地终端
ssh -L 7890:127.0.0.1:7890 user@your-server
```

多服务器配置：

```toml
[hosts.default]
url = "http://127.0.0.1:7890"
token = "local-token"

[hosts.production]
url = "http://127.0.0.1:7890"
token = "prod-token"
ssh_tunnel = "admin@prod-server:7890"
description = "生产服务器"
```

## 4. 接入 Claude Code

在 Claude Code 的 MCP 配置中添加：

```json
{
  "mcpServers": {
    "nix-evo": {
      "command": "node",
      "args": ["/path/to/nix-evo/mcp-server/dist/index.js"]
    }
  }
}
```

## 5. 开始使用

在 Claude Code 中说：

> "检查一下我的服务器状态"

Claude Code 会调用 `system_snapshot`，看到服务、磁盘、内存等信息。

> "nginx 502 了，帮我看看"

Claude Code 会自动诊断 → 读取配置 → 生成修复 → dry-run → 等你确认。

## 典型工作流

```
1. system_snapshot    → 看全局状态
2. service_logs       → 定位问题服务
3. config_read        → 读配置文件
4. Claude Code 生成修复方案
5. config_validate    → dry-run 验证 + 风险评估
6. config_apply       → 确认执行（等用户确认）
7. rollback_list      → 查看可用回滚点
8. rollback_apply     → 如需回滚
```

## 安全建议

1. **默认只监听 127.0.0.1** — 远程访问必须走 SSH 隧道
2. **配置 API token** — 生产环境务必启用
3. **永远先 dry-run** — `config_validate` 在 `config_apply` 之前
4. **保留 generation** — 每次 apply 自动记录，随时可回滚

## 故障排除

### MCP Server 连不上 agent

```bash
# 检查 agent 是否运行
curl http://127.0.0.1:7890/health

# 检查 hosts.toml 配置
cat ~/.config/nix-evo/hosts.toml
```

### dry-build 失败

通常意味着 NixOS 配置有语法错误。agent 会尝试多种 dry-build 策略（flake、no-flake、impure），如果都失败了，配置本身可能有问题。

### 权限不足

agent 需要权限执行 `nixos-rebuild`、`systemctl`、`journalctl` 等命令。NixOS module 使用 `DynamicUser` + `SupplementaryGroups` 自动处理。
