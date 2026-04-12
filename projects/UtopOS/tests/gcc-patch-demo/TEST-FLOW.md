# 测试流程 — GCC 编译修复

> 每一步都是有效操作，无"尝试"步骤。

## 前置条件

- UtopOS-agent 在运行（`curl http://127.0.0.1:7890/health` 返回 `{"status":"ok"}`）
- `legacy-network-build.service` 状态为 `failed`

## 流程

### 步骤 0 — 启动 Agent

```bash
# 在 NixOS 服务器上
UtopOS-agent --host 127.0.0.1 --port 7890

# 或在模拟环境中
python3 mock_agent.py 7890
```

验证：
```bash
curl -s http://127.0.0.1:7890/health | jq .
# 预期：{"status":"ok","version":"0.3.1",...}
```

---

### 步骤 1 — 获取系统快照

```bash
curl -s http://127.0.0.1:7890/api/snapshot | jq .
```

**目的**: 查看服务器全局状态，发现 failed 服务。

**预期结果**:
- `legacy-network-build.service` 在 `services` 中 `active: "failed"`
- `recent_failures` 包含该服务的错误日志摘要
- 日志中出现 `error: implicit declaration of function 'memcpy'`

**判定**: 确认问题存在 → 进入下一步。

---

### 步骤 2 — 查看服务日志

```bash
curl -s "http://127.0.0.1:7890/api/logs?unit=legacy-network-build.service&lines=20" | jq .
```

**目的**: 深入查看构建失败的完整日志。

**预期结果**:
- 日志包含 `legacy_network.c:42:5: error: implicit declaration of function 'memcpy'`
- 编译命令为 `gcc -O2 -Wall -Werror=implicit-function-declaration`
- GCC 版本为 14+

**判定**: 确认编译错误类型为 implicit-function-declaration → 进入下一步。

---

### 步骤 3 — 读取配置

```bash
curl -s http://127.0.0.1:7890/api/config | jq .
```

**目的**: 查看当前 NixOS 配置，了解 legacy-network 是如何构建的。

**预期结果**:
- `configuration.nix` 中包含 `systemd.services.legacy-network-build`
- 构建脚本路径：`/opt/legacy-network/build.sh`
- 使用系统默认 GCC 编译

**判定**: 确认构建方式 → 进入下一步。

---

### 步骤 4 — 定位问题源码

```bash
# 在服务器上查看源码
cat /opt/legacy-network/legacy_network.c | grep -n memcpy
```

**目的**: 确认根因。

**预期结果**:
- 第 42 行 `memcpy(dst, src, len)` 调用
- 文件头部没有 `#include <string.h>`

**根因**: `memcpy` 定义在 `<string.h>` 中，代码遗漏了这个头文件。

**修复策略选择**:
- ❌ 修改源码（可能影响多个项目，或源码不可改）
- ✅ 修改 GCC 编译行为（通过 NixOS overlay，只影响这个包）

---

### 步骤 5 — 验证当前配置（修复前）

```bash
curl -s -X POST http://127.0.0.1:7890/api/config/validate \
  -H "Content-Type: application/json" \
  -d '{"config": "{ config, pkgs, ... }: { services.nginx.enable = true; }"}' | jq .
```

**目的**: 用当前配置做 dry-build，确认问题可复现。

**预期结果**:
- `valid: false`
- `dry_run_output` 包含 `error: implicit declaration of function 'memcpy'`
- `risk_level: "unknown"`（因为 dry-build 失败，无法评估）

**判定**: 问题确认 → 准备修复方案。

---

### 步骤 6 — 应用 GCC overlay 并验证

准备修复配置（NixOS overlay），将 `-Werror=implicit-function-declaration` 降级为 warning：

```nix
# nixos-gcc-tolerance-overlay.nix
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

验证修复后的配置：

```bash
curl -s -X POST http://127.0.0.1:7890/api/config/validate \
  -H "Content-Type: application/json" \
  -d '{"config": "<修复后的 Nix 配置内容>"}' | jq .
```

**预期结果**:
- `valid: true`
- `dry_run_output` 包含 `warning: implicit declaration of function 'memcpy'`（warning 而非 error）
- `summary.risk_level: "safe"`
- `summary.packages_added: ["legacy-network-0.1.0-fixed"]`
- `summary.services_restart: ["legacy-network-build.service"]`

**判定**: dry-build 通过，风险评估 safe → 执行。

---

### 步骤 7 — 应用配置变更

```bash
curl -s -X POST http://127.0.0.1:7890/api/config/apply \
  -H "Content-Type: application/json" \
  -d '{"message": "GCC overlay: 容忍 legacy-network implicit-function-declaration"}' | jq .
```

**预期结果**:
- `success: true`
- `generation: 43`
- `summary` 包含 "配置已生效"
- `rollback_command: "nixos-rebuild switch --rollback"`

**判定**: 配置已生效。

---

### 步骤 8 — 验证结果

```bash
curl -s http://127.0.0.1:7890/api/generations | jq .
```

**预期结果**:
- `current: 43`
- generation 43 的 description 为 "GCC overlay: 容忍 legacy-network implicit-function-declaration"

**验证服务状态**:
```bash
curl -s http://127.0.0.1:7890/api/snapshot | jq '.services[] | select(.name == "legacy-network-build.service")'
# 预期：active: "active", sub: "running"（在真实环境中构建完成后）
```

---

## 流程总结

| 步骤 | API 调用 | 目的 | 判定依据 |
|------|----------|------|----------|
| 0 | `/health` | 确认 agent 运行 | `status: "ok"` |
| 1 | `GET /api/snapshot` | 发现 failed 服务 | `active: "failed"` |
| 2 | `GET /api/logs` | 定位编译错误 | `implicit declaration of function` |
| 3 | `GET /api/config` | 读取 NixOS 配置 | 包含 legacy-network 配置 |
| 4 | 源码检查 | 确认根因 | 缺少 `#include <string.h>` |
| 5 | `POST /api/config/validate` | dry-build（修复前）| `valid: false` |
| 6 | `POST /api/config/validate` | dry-build（修复后）| `valid: true`, `risk: safe` |
| 7 | `POST /api/config/apply` | 执行配置变更 | `success: true`, gen 43 |
| 8 | `GET /api/generations` | 确认变更生效 | `current: 43` |

## 回滚

如果修复后出现问题：

```bash
nixos-rebuild switch --rollback
# 或通过 API
curl -s -X POST http://127.0.0.1:7890/api/rollback | jq .
```

回滚到 generation 42（修复前的状态）。
