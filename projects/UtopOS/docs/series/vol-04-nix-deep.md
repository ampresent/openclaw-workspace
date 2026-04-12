# 第四卷：Nix 后端完全指南

---

## 4.1 Nix 语言基础

### 表达式类型

```nix
# 字符串
"hello"

# 数字
42

# 布尔
true / false

# 列表
[ 1 2 3 ]

# 属性集（字典）
{ name = "nginx"; version = "1.24.0"; }

# 函数
x: x + 1

# 带解构的函数
{ name, version }: name + "-" + version
```

### 函数调用

```nix
# f x → 把 x 传给函数 f
(import <nixpkgs> {}).nginx

# 等价于：
let pkgs = import <nixpkgs> {}; in pkgs.nginx
```

### let ... in

```nix
let
  x = 1;
  y = 2;
in
  x + y  # → 3
```

### 取属性

```nix
pkgs.nginx.version          # "1.24.0"
pkgs.nginx.meta.description # "A reverse proxy..."
pkgs.nginx.src              # /nix/store/...-nginx-1.24.0.tar.gz
```

---

## 4.2 Derivation（构建指令）

Derivation 是 Nix 的核心概念。一个 derivation 描述了如何构建一个包：

```nix
# 简化的 derivation
derivation {
  name = "hello-2.10";
  system = "x86_64-linux";
  builder = /bin/sh;
  args = [ ./build.sh ];
  src = ./hello-2.10.tar.gz;
}
```

实际使用中很少直接写 derivation，而是用 stdenv.mkDerivation：

```nix
stdenv.mkDerivation {
  pname = "nginx";
  version = "1.24.0";

  src = fetchurl {
    url = "https://nginx.org/download/nginx-1.24.0.tar.gz";
    hash = "sha256-...";
  };

  patches = [ ./fix-timeout.patch ];  # ← evo 在这里加补丁

  configureFlags = [ "--with-http_ssl_module" ];

  buildInputs = [ openssl pcre2 zlib ];
}
```

---

## 4.3 Overlay 机制

Overlay 是修改 nixpkgs 中包定义的机制。

### 基本结构

```nix
# overlay 是一个函数
final: prev: {
  # final: 最终的 nixpkgs（包含所有 overlay 的结果）
  # prev: 上一层的 nixpkgs（应用本 overlay 之前的）

  nginx = prev.nginx.overrideAttrs (old: {
    # old: 原始的 nginx derivation
    patches = (old.patches or []) ++ [ ./fix.patch ];
  });
}
```

### 使用 overlay

```nix
# 方式一：在 nix-build 命令行中
nix-build -E "
  with import <nixpkgs> {
    overlays = [ (import ./overlay/default.nix) ];
  };
  nginx
"

# 方式二：在 configuration.nix 中
nixpkgs.overlays = [ (import ./overlay/default.nix) ];
```

### overlay 链

多个 overlay 按顺序应用：

```
原始 nixpkgs
  → overlay1 → nixpkgs'
  → overlay2 → nixpkgs''
  → final nixpkgs
```

`final` 引用最终结果，`prev` 引用上一步结果。

---

## 4.4 overrideAttrs 详解

`overrideAttrs` 修改一个已存在的 derivation：

```nix
nginx.overrideAttrs (old: {
  # 可以修改任何 stdenv.mkDerivation 的参数

  # 补丁
  patches = (old.patches or []) ++ [ ./fix.patch ];

  # configure 参数
  configureFlags = old.configureFlags ++ [ "--with-threads" ];

  # 构建依赖
  buildInputs = old.buildInputs ++ [ openssl ];

  # 编译前执行
  prePatch = ''
    substituteInPlace src/core/nginx.h \
      --replace 'NGINX_VERSION "1.24.0"' 'NGINX_VERSION "1.24.0-patched"'
  '';

  # 编译后执行
  postInstall = ''
    mkdir -p $out/share/nginx
    cp -r conf/* $out/share/nginx/
  '';

  # 元数据
  meta = old.meta // {
    description = "Patched nginx with custom timeout defaults";
  };
})
```

### 常用 overrideAttrs 操作

| 操作 | 代码 |
|------|------|
| 加补丁 | `patches = (old.patches or []) ++ [ ./fix.patch ];` |
| 加 configure 参数 | `configureFlags = old.configureFlags ++ [ "--flag" ];` |
| 加依赖 | `buildInputs = old.buildInputs ++ [ pkg ];` |
| 替换字符串 | `postPatch = "sed -i 's/old/new/g' file.c";` |
| 加环境变量 | `env.CFLAGS = "-O2";` |
| 改版本号 | 改 `src` 即可，`version` 从 src 推断 |

---

## 4.5 NixOS Module 系统

### 模块结构

```nix
{ config, lib, pkgs, ... }:
with lib;
let
  cfg = config.services.myapp;
in {
  # 选项声明
  options.services.myapp = {
    enable = mkEnableOption "myapp";
    port = mkOption {
      type = types.port;
      default = 8080;
      description = "Port to listen on";
    };
    dataDir = mkOption {
      type = types.path;
      default = "/var/lib/myapp";
      description = "Data directory";
    };
  };

  # 配置实现
  config = mkIf cfg.enable {
    systemd.services.myapp = {
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${pkgs.myapp}/bin/myapp --port ${toString cfg.port}";
        StateDirectory = baseNameOf cfg.dataDir;
      };
    };
  };
}
```

### 常用 NixOS 选项

| 选项 | 说明 |
|------|------|
| `mkEnableOption` | 布尔开关 |
| `mkOption` | 通用选项（type + default + description） |
| `mkIf` | 条件配置 |
| `mkMerge` | 合并多个配置块 |
| `mkOverride` | 优先级覆盖 |

---

## 4.6 Generation 管理

### 列出 generations

```bash
nixos-rebuild list-generations
# 42  2026-04-12  16:00:00  (current)
# 41  2026-04-11  14:30:00
# 40  2026-04-10  10:00:00
```

### 切换 generation

```bash
nixos-rebuild switch --rollback    # 上一个
nixos-rebuild switch --to 41       # 指定
```

### 清理旧 generation

```bash
# 保留最近 5 天
nix-collect-garbage --delete-older-than 5d

# 保留最近 3 个
sudo nix-collect-garbage --delete-older-than 3d

# 清理所有（危险！）
sudo nix-collect-garbage -d
```

---

## 4.7 常见 Nix 排障

### "attribute 'X' not found"

```bash
# 查找正确的 attribute path
nix-env -qaP | grep nginx
# nginxPackages.nginx  nginx-1.24.0

# 或搜索 nixpkgs 源码
grep -r "nginx" $(nix-instantiate --eval -E '<nixpkgs>')/pkgs/top-level/all-packages.nix
```

### "infinite recursion encountered"

通常是因为 overlay 中引用了自己：
```nix
# 错误：用 final 引用了自己
final: prev: {
  nginx = final.nginx.overrideAttrs (...);  # 无限递归！
}
# 正确：用 prev 引用原始版本
final: prev: {
  nginx = prev.nginx.overrideAttrs (...);
}
```

### "hash mismatch"

源码 hash 变了（通常是版本更新）：
```bash
# 获取新 hash
nix-prefetch-url https://nginx.org/download/nginx-1.25.0.tar.gz
# 或用 nix hash
nix store prefetch-file --json https://nginx.org/download/nginx-1.25.0.tar.gz
```
