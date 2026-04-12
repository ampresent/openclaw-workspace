# UtopOS-nix — Nix / NixOS 后端专项

> 本 skill 是 `UtopOS` 的子 skill，专注 Nix 后端。
> 通用工作流（检测、诊断、补丁、构建、验证、安装）见父 skill：`UtopOS/SKILL.md`

## 前置

- 系统必须是 NixOS（有 `nixos-rebuild`）
- 需要 `nix-build`、`nix-instantiate` 等工具
- 检测命令：`evo-detect`，确认 `backend == "nix"`

## Nix 语言速查（给 Agent 看的）

### 基本概念

```nix
# 属性集（attribute set）= 字典
{ name = "nginx"; version = "1.24.0"; }

# 函数 = lambda
x: x + 1
{ name, version }: name + "-" + version

# let 绑定
let prefix = "/usr"; in prefix + "/bin"

# 取属性
pkg.version
pkg.meta.description
```

### overlay 是什么

overlay 是一个函数，接收 `final`（整个 nixpkgs 最终结果）和 `prev`（修改前的 nixpkgs），返回一个属性集覆盖原有定义：

```nix
# overlay = "我要改 nixpkgs 里的某个包"
final: prev: {
  nginx = prev.nginx.overrideAttrs (old: {
    # old 是原来的 derivation
    patches = (old.patches or []) ++ [ ./my-fix.patch ];
    # 或改 configureFlags
    configureFlags = old.configureFlags ++ [ "--with-threads" ];
  });
}
```

### overrideAttrs 常用字段

| 字段 | 说明 | 示例 |
|------|------|------|
| `patches` | 补丁列表 | `(old.patches or []) ++ [ ./fix.patch ]` |
| `configureFlags` | configure 参数 | `old.configureFlags ++ [ "--enable-feature" ]` |
| `buildInputs` | 构建依赖 | `old.buildInputs ++ [ openssl ]` |
| `prePatch` | patch 前执行 | `"sed -i 's/old/new/g' src/main.c"` |
| `postPatch` | patch 后执行 | `"patch -p1 < my-fix.patch"` |
| `preBuild` | 构建前执行 | `""` |
| `meta` | 包元数据 | `{ description = "..."; }` |

### NixOS 模块配置

```nix
# /etc/nixos/configuration.nix 中
services.nginx = {
  enable = true;
  virtualHosts."example.com" = {
    root = "/var/www";
    locations."/api" = { proxyPass = "http://localhost:3000"; };
  };
};

# 用 overlay 改 nginx 包本身
nixpkgs.overlays = [
  (final: prev: {
    nginx = prev.nginx.overrideAttrs (old: { ... });
  })
];
```

## 源码获取细节

```bash
# evo-fetch-source 内部做的事
nix-build '<nixpkgs>' -A nginx.src --no-out-link
# → /nix/store/abc123-nginx-1.24.0.tar.gz
# 复制到 /tmp/evo-fix-nginx/src/ 并解压
```

**nixpkgs 路径**：
- `<nixpkgs>` 是 NIX_PATH 中的 nixpkgs
- 查看版本：`nix-instantiate --eval -E '(import <nixpkgs> {}).lib.version'`
- 如果要改指定版本的 nixpkgs：`nix-build '<nixos-unstable>' -A ...`

**获取包信息**：
```bash
# 版本
nix-instantiate --eval -E '(import <nixpkgs> {}).nginx.version'

# 描述
nix-instantiate --eval -E '(import <nixpkgs> {}).nginx.meta.description'

# 所有输出
nix show-derivation $(nix-build '<nixpkgs>' -A nginx --no-out-link)
```

## 补丁工作流

### 方式一：overlay + overrideAttrs（推荐）

由 `evo-build` 自动生成 overlay 文件：

```bash
# evo-build 内部做的事:
# 1. 生成 overlay
cat > /tmp/evo-fix-nginx/overlay/default.nix <<'EOF'
final: prev: {
  nginx = prev.nginx.overrideAttrs (old: {
    patches = (old.patches or []) ++ [
      /root/.evo/patches/nginx/fix-timeout.patch
    ];
  });
}
EOF

# 2. 用 overlay 构建
nix-build -E "with import <nixpkgs> { overlays = [ (import /tmp/evo-fix-nginx/overlay/default.nix) ]; }; nginx"
```

### 方式二：NixOS 模块覆盖（配置层）

当问题是配置默认值不合理时，不改包本身，改模块配置：

```nix
# 在 configuration.nix 中
services.phpfpm.pools.main.settings = {
  "listen.backlog" = 1024;  # 覆盖默认的 511
};
```

这种情况下不需要 `evo-build`，直接 `nixos-rebuild switch`。

### 方式三：overrideAttrs + 其他修改

不只是补丁，还可以改：

```nix
final: prev: {
  nginx = prev.nginx.overrideAttrs (old: {
    # 改 configure 参数
    configureFlags = old.configureFlags ++ [ "--with-http_v3_module" ];
    # 加依赖
    buildInputs = old.buildInputs ++ [ prev.openssl ];
    # 替换源码中的文件
    postPatch = old.postPatch + ''
      substituteInPlace src/http/ngx_http_core_module.c \
        --replace 'default_timeout 60' 'default_timeout 120'
    '';
  });
}
```

## 构建 + 验证

```bash
# evo-build 内部做的事（dry-build 是 evo-verify 做的）
nixos-rebuild dry-build    # 只构建，不激活 — evo-verify
nixos-rebuild test         # 构建 + 激活到临时 profile，不改 bootloader
nixos-rebuild switch       # 构建 + 激活 + 写 bootloader — evo-install
```

### 验证输出解读

```
building the system configuration...
these derivations will be built:
  /nix/store/xxx-nginx-1.24.0.drv
these paths will be fetched (23.45 MiB download, 89.12 MiB unpacked):
  /nix/store/yyy-dependency-1.0
```

- `will be built` = 需要从源码构建（你的补丁生效了）
- `will be fetched` = 从 binary cache 拉取依赖
- 如果没有 `will be built`，说明补丁没生效，检查 overlay

### 用 UtopOS 包装

```bash
scripts/evo-build nginx --patch /root/.evo/patches/nginx/fix.patch
# → 自动生成 overlay → 调用 nix-build → 记录日志

scripts/evo-verify nginx
# → 调用 nixos-rebuild dry-build → 解析输出 → JSON { risk, changes }

scripts/evo-install nginx
# → 调用 nixos-rebuild switch → 记录 generation → JSON { generation, rollback_cmd }
```

## 回滚

```bash
# 查看所有 generation
nixos-rebuild list-generations
# → generation 42  2026-04-12  16:00:00  (current)
# → generation 41  2026-04-11  14:30:00
# → generation 40  2026-04-10  10:00:00

# 回滚到上一个
nixos-rebuild switch --rollback

# 回滚到指定 generation
nixos-rebuild switch --to 41

# 用 UtopOS
scripts/evo-rollback nginx              # → --rollback
scripts/evo-rollback nginx --to 41      # → --to 41
```

**回滚是 NixOS 的杀手锏**：不删除任何东西，只是切换 profile 指针。所有 generation 的 /nix/store 都保留，随时可以切回去。

## 常见 Nix 特有问题

### "nix-build 找不到包"

```bash
# 检查 nixpkgs 版本
nix-instantiate --eval -E '(import <nixpkgs> {}).lib.version'

# 包名在不同 channel 可能不同
nix-env -qaP | grep nginx    # 查找确切的 attribute path
```

### "overlay 没生效"

```bash
# 检查 overlay 语法
nix-instantiate --parse overlay/default.nix

# 确认构建用的是带 overlay 的 nixpkgs
nix-build -E "with import <nixpkgs> { overlays = [ (import ./overlay/default.nix) ]; }; nginx"
# 如果还是旧的，检查 overlay 函数签名是否正确
```

### "patch 无法 apply"

```bash
# 检查 patch 格式
head -5 /root/.evo/patches/nginx/fix.patch
# 必须是 diff -u 格式，有 --- 和 +++ 行

# 检查 patch 路径前缀层数
grep '^---' /root/.evo/patches/nginx/fix.patch
# 如果是 a/src/file.c → -p1
# 如果是 src/file.c → -p0
```

### "磁盘空间不够构建"

```bash
# 清理旧 generation（保留最近 5 个）
sudo nix-collect-garbage --delete-older-than 5d

# 清理所有旧 generation
sudo nix-collect-garbage -d

# 清理 UtopOS 缓存
scripts/evo-cleanup --cache-days 7
```

## NixOS Module 开发（进阶）

如果需要写 NixOS 模块（不只是改已有包），需要了解：

```nix
# module.nix 结构
{ config, lib, pkgs, ... }:
with lib;
let
  cfg = config.services.myapp;
in {
  options.services.myapp = {
    enable = mkEnableOption "myapp";
    port = mkOption { type = types.port; default = 8080; };
  };
  config = mkIf cfg.enable {
    systemd.services.myapp = {
      wantedBy = [ "multi-user.target" ];
      serviceConfig.ExecStart = "${pkgs.myapp}/bin/myapp --port ${toString cfg.port}";
    };
  };
}
```

项目中的 NixOS module 参考：
- `evo/nix/modules/nextcloud.nix` — 完整的 Nextcloud 模块
- `evo/nix/modules/jellyfin.nix` — Jellyfin + 硬件转码
- `evo/nix/modules/monitoring-stack.nix` — Prometheus + Loki + Grafana
