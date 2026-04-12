# GCC 编译修复 — nix-evo 测试用例

## 场景描述

NixOS 服务器上的 `legacy-network` 模块因 GCC 14+ 将
`-Wimplicit-function-declaration` 默认升级为 error 而编译失败。

代码中遗漏了 `#include <string.h>`，导致 `memcpy()` 被当作隐式声明的函数。

通过 nix-evo 的完整工作流修复：**诊断 → 定位 → 应用 GCC overlay → 验证 → 执行**。

## 文件清单

| 文件 | 用途 |
|------|------|
| `legacy_network.c` | 问题源码 — 缺少 `<string.h>` 导致编译失败 |
| `legacy-network.nix` | 失败的 Nix 表达式 |
| `legacy-network-fixed.nix` | 修复后的 Nix 表达式（使用容错 GCC） |
| `nixos-gcc-tolerance-overlay.nix` | NixOS 配置片段（注入到 configuration.nix） |
| `mock_agent.py` | nix-evo-agent 模拟服务器 |
| `demo.sh` | 完整演示脚本 |
| `record.sh` | asciinema 录制脚本 |
| `TEST-FLOW.md` | 测试流程文档 |

## 快速运行

```bash
# 交互式演示
bash demo.sh

# 录制 asciinema
bash record.sh
```

## 测试流程（8 步）

```
步骤 0: 启动 nix-evo-agent（模拟模式）
步骤 1: system_snapshot  → 发现构建失败
步骤 2: service_logs     → 定位编译错误
步骤 3: config_read      → 读取 NixOS 配置
步骤 4: 查看问题源码     → 确认根因
步骤 5: config_validate  → dry-build 失败（修复前）
步骤 6: config_validate  → dry-build 通过（GCC overlay 后）
步骤 7: config_apply     → 执行配置变更
步骤 8: 验证结果         → 确认 generation 43 生效
```

## GCC 修复策略

### Per-package overlay（推荐）

只为 `legacy-network` 包使用定制 GCC，不影响系统全局 GCC：

```nix
nixpkgs.overlays = [
  (final: prev: {
    legacy-network = prev.legacy-network.override {
      stdenv = prev.stdenvAdapters.overrideCC prev.stdenv
        (prev.gcc.overrideAttrs (old: {
          NIX_CFLAGS_COMPILE = (old.NIX_CFLAGS_COMPILE or "") +
            " -Wno-error=implicit-function-declaration";
        }));
    };
  })
];
```

### 风险评估

- **风险等级**: safe
- **影响范围**: 仅 legacy-network 包
- **副作用**: 无 — 不影响系统 GCC 或其他包的编译
- **回滚**: `nixos-rebuild switch --rollback`
