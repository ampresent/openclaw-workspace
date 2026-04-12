# nix-evo Skill — 决策层

> AI Agent 的源码修复工作流。支持 Nix / RPM / Conda 三种包管理后端。
> **脚本做机械事，Skill 做决策，Agent 做连接，用户做确认。**

## 前置条件

所有脚本路径：相对于本 skill 目录下的 `scripts/`。
如果脚本不存在，先运行 `evo-init` 初始化环境。

```bash
SCRIPTS="$(dirname "$0")/scripts"    # skill 目录下的 scripts/
# 或项目路径:
# SCRIPTS="projects/nix-evo/scripts/"
```

---

## 核心铁律（不可违反）

1. **❌ 不要**用 workaround 绕过问题（重启服务、改运行时配置糊弄）
2. **❌ 不要**直接替换二进制、编辑运行时文件、打热补丁
3. **❌ 不要**跳过验证直接安装（必须先 dry-build / test install）
4. **❌ 不要**混用包管理器（conda 环境里用 pip、RPM 系统上用 apt）
5. **✅ 要**走完整流程：检测 → 源码 → 分析 → 补丁 → 验证 → 安装

---

## 工作流决策树

收到用户问题后，按以下流程走：

```
┌─ 0. 初始化 ──────────────────────────────────────────┐
│  evo-init                                           │
│  (幂等，已有 ~/.evo/ 就跳过)                          │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─ 1. 诊断 + 获取源码 ─────────────────────────────────┐
│  evo-detect           → { backend, version, tools } │
│  evo-fetch-source pkg → { src_dir, spec?, recipe? } │
│  evo-get-info pkg     → { version, desc, deps }     │
│  (并行) system diagnosis: journalctl / systemctl     │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─ 2. 分析源码 ────────────────────────────────────────┐
│  进入 src_dir，找到根因                                │
│  关注：配置模板默认值、编译参数、源码 bug              │
│  ⚠️ 不看运行时文件，只看源码                          │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─ 3. 生成补丁 ────────────────────────────────────────┐
│  修改源码 → evo-patch-create pkg --desc "..."        │
│  → { patch_file, risk, files_changed }               │
│  ▶ 向用户展示风险摘要（见下方模板）                   │
│  ▶ 等待用户确认                                      │
└─────────────────────────────────────────────────────┘
                         │
                         ▼ 用户确认
┌─ 4. 构建 ────────────────────────────────────────────┐
│  evo-build pkg --patch <patch_file>                  │
│  → { result, log }                                   │
│  evo-verify pkg       → { risk, changes }            │
│  ▶ 再次向用户确认风险等级                             │
└─────────────────────────────────────────────────────┘
                         │
                         ▼ 用户确认
┌─ 5. 安装 ────────────────────────────────────────────┐
│  evo-install pkg      → { txn_id, rollback_cmd }    │
│  验证服务是否正常                                    │
└─────────────────────────────────────────────────────┘
                         │
                         ▼ (可选)
┌─ 6. 提交上游 ────────────────────────────────────────┐
│  如果是上游 bug，生成 PR                              │
└─────────────────────────────────────────────────────┘
```

---

## 脚本速查

### 基础设施

| 脚本 | 什么时候调 | 输出关键字段 |
|------|-----------|-------------|
| `evo-init` | 首次使用、`~/.evo/` 不存在时 | `dirs` |
| `evo-workspace create <pkg>` | 开始处理一个新包 | `work_dir`, `evo_work` |
| `evo-workspace list` | 查看有哪些进行中的修复 | `workspaces[]` |
| `evo-workspace status <pkg>` | 查看某包的工作目录状态 | `patches`, `size`, `last_build` |
| `evo-workspace archive <pkg>` | 修复完成，归档工作目录 | `archive` |
| `evo-cleanup` | 定期清理、磁盘紧张时 | `cleaned`, `freed_bytes`, `disk_warning` |

### 检测 + 获取

| 脚本 | 什么时候调 | 输出关键字段 |
|------|-----------|-------------|
| `evo-detect` | 第一步，确定后端 | `backend`, `version`, `tools` |
| `evo-fetch-source <pkg>` | 检测后立即调 | `src_dir`, `spec`, `recipe` |
| `evo-get-info <pkg>` | 需要包详细信息时 | `version`, `description`, `homepage` |

### 构建 + 安装

| 脚本 | 什么时候调 | 输出关键字段 |
|------|-----------|-------------|
| `evo-build <pkg> --patch <f>` | 分析完成、用户确认后 | `result`, `log` |
| `evo-verify <pkg>` | 安装前必须调 | `risk`, `changes`, `missing_deps` |
| `evo-install <pkg>` | verify 通过、用户确认后 | `txn_id`, `rollback_cmd` |
| `evo-rollback <pkg>` | 出问题时 | `rolled_to`, `log` |

### Patch 管理

| 脚本 | 什么时候调 | 输出关键字段 |
|------|-----------|-------------|
| `evo-patch-create <pkg> --desc "..."` | 源码修改完成后 | `patch`, `risk`, `files_changed` |
| `evo-patch-list <pkg>` | 查看已有补丁 | `patches[]` |
| `evo-patch-check <pkg>` | 构建前检查兼容性 | `results[].status` (compatible/conflict) |
| `evo-patch-series show <pkg>` | 多个补丁需要排序时 | `series[]` |

---

## 用户交互协议

### 必须确认的节点

以下 3 个节点**必须**向用户展示信息并等待确认，不能自动跳过：

#### ① 补丁生成后

```
📋 风险摘要
─────────
包名: nginx
修改: src/http/ngx_http_core_module.c
影响: upstream timeout 默认值从 60s → 120s
风险: moderate
回滚: evo-rollback nginx

要继续构建吗？[y/n]
```

#### ② 验证通过后（安装前）

```
🔍 验证结果
─────────
风险等级: safe
缺失依赖: 0
测试安装: 通过

要安装吗？[y/n]
```

#### ③ 异常情况

- 构建失败 → 展示日志摘要，让用户决定是否重试或修改补丁
- 验证发现冲突 → 展示冲突详情，让用户决定是否解决
- 磁盘空间不足 → 询问是否清理

### 风险摘要卡片模板

```
┌─────────────────────────────────────┐
│ 📦 包名: {pkg}                       │
│ 🔧 修改: {文件列表}                  │
│ 📝 描述: {补丁描述}                  │
│ ⚠️  风险: {safe|moderate|dangerous}  │
│ 💾 回滚: {rollback_cmd}             │
│ 🎫 关联: {ticket 或 none}           │
└─────────────────────────────────────┘
```

风险等级判定规则：
- **safe**: 配置值变更、不影响二进制、可无损回滚
- **moderate**: 源码逻辑修改、影响单一功能、有回滚方案
- **dangerous**: 核心模块修改、影响面广、回滚代价高

---

## 信任白名单

文件位置：`~/.evo/trust.toml`

格式：
```toml
[trust.nginx]
safe_auto = true          # safe 级补丁自动 apply，不再询问
risk_levels = ["safe"]

[trust.php]
safe_auto = true
risk_levels = ["safe", "moderate"]   # moderate 也自动
```

**决策逻辑**：

```
1. 读 ~/.evo/trust.toml
2. 找到 [trust.<pkg>] 配置
3. 如果 patch.risk 在 risk_levels 内 → 跳过确认，直接构建
4. 否则 → 走正常确认流程
```

**安全限制**：
- `dangerous` 级补丁永远不自动 apply，即使在白名单中
- 白名单变更需要用户手动编辑 trust.toml，agent 不自动修改

---

## 后端专项参考

### Nix / NixOS

补丁方式：overlay + `overrideAttrs`

```nix
# evo-build 自动生成的 overlay（~/.evo/builds/<pkg>/overlay-*.nix）
final: prev: {
  <pkg> = prev.<pkg>.overrideAttrs (old: {
    patches = (old.patches or []) ++ [ ./patch.patch ];
  });
}
```

回滚：`nixos-rebuild switch --rollback` 或 `--to <generation>`

### RPM（Rocky / RHEL / Fedora）

补丁方式：SRPM + spec 文件注册 Patch

- 源码在 `~/rpmbuild/SOURCES/`
- Spec 在 `~/rpmbuild/SPECS/`
- `evo-build` 自动注册 patch 到 spec

回滚：`yum history undo <txn_id>`

### Conda

补丁方式：feedstock recipe + meta.yaml patches 列表

- Source 在 `/tmp/evo-fix-<pkg>/src/`
- Recipe 在 `src/recipe/` 或 `src/<pkg>/`

回滚：`conda install --revision <N>`

---

## 常见场景快速入口

### "服务挂了 / 502 / 启动失败"

```
1. evo-detect
2. evo-fetch-source <pkg>
3. 诊断（journalctl / systemctl）
4. 分析源码 → evo-patch-create
5. evo-build → evo-verify → evo-install
```

### "默认配置不合理，想改默认值"

```
1. evo-detect
2. evo-fetch-source <pkg>
3. 找到配置模板/默认值 → evo-patch-create --risk safe
4. 检查 trust.toml → 如果 safe_auto → 直接构建
5. evo-build → evo-verify → evo-install
```

### "补丁冲突了（上游更新后）"

```
1. evo-fetch-source <pkg> --force   # 重新拉取新版本
2. evo-patch-check <pkg>            # 检查哪些 patch 冲突
3. 逐个解决冲突 → evo-patch-create
4. evo-build → evo-verify → evo-install
```

### "想回滚"

```
1. evo-rollback <pkg>                    # 回滚到上一个
2. evo-rollback <pkg> --to <id>          # 回滚到指定版本
```

---

## 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `EVO_HOME` | `~/.evo` | 工作目录根路径 |
| `EVO_SCRIPTS` | skill 目录下的 `scripts/` | 脚本搜索路径 |

---

## 反模式速查

| ❌ 反模式 | ✅ 正确做法 |
|----------|-----------|
| `vim /etc/nginx/nginx.conf` | 分析源码 → patch → build → install |
| `systemctl restart nginx` | 找根因，修源码 |
| 跳过 `evo-verify` | 永远先 verify 再 install |
| `pip install` in conda | 只用对应包管理器 |
| 不记录就修改 | `evo-patch-create` + git commit |
