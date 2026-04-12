# nix-evo Skill

> NixOS 源码修复工作流 — AI Agent 通过分析和修补源代码来解决系统问题。

## 核心原则

**永远不要绕过问题，永远不要直接修改操作系统。**

当你（AI Agent）在 NixOS 上遇到系统软件问题时：

1. **❌ 不要**尝试用 workaround 绕过（比如重启服务、改配置文件糊弄过去）
2. **❌ 不要**直接修改 `/nix/store`、手动替换二进制、或在运行时打热补丁
3. **✅ 要**解压源代码，分析源码中的根因
4. **✅ 要**修改源代码，生成补丁，重新打包
5. **✅ 要**通过 nixos-rebuild 应用新包，而不是直接动系统

## 为什么

NixOS 的哲学是**声明式**和**可复现**。直接修改操作系统：

- 重启后丢失（Nix store 是只读的）
- 无法追溯（谁改了什么、为什么改）
- 无法回滚（没有 generation 记录）
- 破坏复现性（同一配置不再生成同一系统）

修补源代码 + 重新打包：

- 变更持久化（写入 Nix 配置，生成新 generation）
- 变更可追溯（补丁文件 + commit 记录）
- 变更可回滚（`nixos-rebuild switch --rollback`）
- 变更可复现（任何人在任何机器上复现）

## 工作流

### 第一步：诊断（不要急着修）

使用 `system_snapshot` 和 `service_logs` 了解问题。

```
用户: nginx 502 了

→ 调用 system_snapshot
→ 发现 php-fpm.service failed
→ 调用 service_logs("phpfpm.service")
→ 发现错误信息
```

**关键**：此时不要动手修。先理解根因。

### 第二步：定位源码包

找到出问题的软件对应的 Nix 包。

```
→ 调用 package_info("php-fpm")
→ 获取包名、版本、源码路径
```

源码通常在 `/nix/store/<hash>-<pkg>-<version>/` 或可以从 nixpkgs 源码树中找到对应的 derivation。

### 第三步：解压并分析源代码

**这是核心步骤。** 不要看运行时文件，要看源码。

```bash
# 找到源码包
nix-store --realise $(nix-instantiate '<nixpkgs>' -A php83)

# 解压源码
cd /tmp && mkdir fix-workspace && cd fix-workspace
nix-build '<nixpkgs>' -A php83.src --no-out-link
# 或直接从 /nix/store 解压源码 tarball

# 分析代码
grep -r "listen" --include="*.conf" .
cat php-fpm.conf.in
```

分析时关注：

- 配置文件模板（`*.conf.in`、`*.service` 模板）
- 默认值是否有问题
- 编译参数是否导致了错误行为
- 源码中的 bug

### 第四步：生成补丁

基于源码分析结果，生成 Nix 补丁。

```nix
# overlay 或 package override 示例
final: prev: {
  php83 = prev.php83.overrideAttrs (old: {
    patches = (old.patches or []) ++ [
      ./patches/fix-fpm-listen-port.patch
    ];
  });
}
```

或者通过修改 NixOS 模块配置来覆盖默认值：

```nix
# configuration.nix 中
services.phpfpm.pools.main.settings = {
  "listen" = "/run/phpfpm.sock";  # 修复监听地址
};
```

### 第五步：验证 + 应用

```bash
# 先 dry-build
nixos-rebuild dry-build

# 确认无误后 apply
nixos-rebuild switch
```

通过 nix-evo：

```
→ 调用 config_validate（dry-run + 风险评估）
→ 用户确认
→ 调用 config_apply（nixos-rebuild switch）
→ 生成新 generation
```

### 第六步：提交到上游（可选但推荐）

如果修复的是上游 bug，生成 nixpkgs PR：

```bash
# 参考 NIXPKGS-PR-TEMPLATE.md
```

## MCP Tools 速查

| 工具 | 用途 | 对应阶段 |
|------|------|---------|
| `system_snapshot` | 全局状态 | 诊断 |
| `service_logs` | 服务日志 | 诊断 |
| `package_info` | 包信息 | 定位源码 |
| `config_read` | 读 NixOS 配置 | 分析 |
| `generation_diff` | generation 对比 | 分析 |
| `config_validate` | dry-run 验证 | 验证 |
| `config_apply` | 执行变更 | 应用 |
| `rollback_list` | 列出可回滚版本 | 兜底 |
| `rollback_apply` | 回滚 | 兜底 |

## 反模式（绝对不要做）

### ❌ 直接编辑运行时文件

```bash
# 禁止
vim /etc/php-fpm.d/www.conf    # 这不是 NixOS 的方式
systemctl restart php-fpm       # 改了也没用，nixos-rebuild 会覆盖
```

### ❌ 绕过问题

```bash
# 禁止
systemctl restart nginx          # 重启不解决根因
while true; do systemctl restart nginx; sleep 1; done  # 更不要这样
```

### ❌ 手动修改 Nix Store

```bash
# 禁止（而且会失败，因为 /nix/store 是只读的）
vim /nix/store/abc123-php-8.3/etc/php-fpm.conf
```

### ❌ 不经过验证直接 apply

```bash
# 禁止（跳过 dry-run = 踩雷）
nixos-rebuild switch  # 不先 dry-build
```

## 安全约束

- 所有变更通过 `config_validate` dry-run 后才允许 `config_apply`
- 风险等级评估：safe / moderate / dangerous
- 每次 apply 自动创建 generation，支持回滚
- API 默认只监听 127.0.0.1
- 补丁文件保存在版本控制中，可追溯
