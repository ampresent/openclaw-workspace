# 第八卷：脚本工具参考手册

> 14 个 evo-* 脚本的完整 API 参考。
> 所有脚本输出 JSON，错误信息和进度写入 stderr。

---

## 8.1 通用约定

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `EVO_HOME` | `$HOME/.evo` | 工作目录根路径 |

### 输出格式

```json
// 成功
{"status":"ok","action":"...","data":"..."}

// 错误
{"status":"error","error":"reason","details":"..."}

// 需要输入
{"status":"needs_input","prompt":"...","options":["..."]}
```

### 退出码

| 码 | 含义 |
|----|------|
| 0 | 成功 |
| 1 | 通用错误 |
| 2 | 参数错误 |

---

## 8.2 evo-init

初始化 `~/.evo/` 目录结构。

```bash
evo-init [--force]
```

| 参数 | 说明 |
|------|------|
| `--force` | 强制重新初始化（覆盖已有配置） |

**输出**：`{ status, action:"init", home, dirs:[...] }`

**创建的目录**：`patches/`, `builds/`, `cache/`, `logs/`, `archive/`, `work/`, `upstream/`, `history/`

**创建的文件**：`config`, `trust.toml`, `inventory.toml`（模板）

**幂等性**：有。已有 `~/.evo/` 且无 `--force` 时直接返回 `already_exists`。

---

## 8.3 evo-detect

检测当前系统的包管理后端。

```bash
evo-detect [--backend nix|rpm|conda]
```

| 参数 | 说明 |
|------|------|
| `--backend` | 手动指定，跳过自动检测 |

**输出**：`{ status, backend, version, tools:{...}, auto_detected }`

**检测优先级**：nix > rpm > conda

**示例**：
```json
{"status":"ok","auto_detected":true,"backend":"rpm","version":"4.18.0","pkg_mgr":"dnf","tools":{"rpm":true,"yumdownloader":true,"dnf":true,"rpmbuild":true}}
```

---

## 8.4 evo-fetch-source

下载指定包的源码。

```bash
evo-fetch-source <pkg> [--backend nix|rpm|conda] [--force]
```

| 参数 | 必填 | 说明 |
|------|------|------|
| `<pkg>` | ✅ | 包名 |
| `--backend` | | 后端（默认自动检测） |
| `--force` | | 强制重新下载（忽略缓存） |

**输出**：`{ status, backend, pkg, src_dir, spec?, recipe?, cache_dir }`

**缓存**：源码缓存到 `~/.evo/cache/<pkg>/`，存在时跳过下载。

**行为差异**：
- **nix**：`nix-build '<nixpkgs>' -A pkg.src` → 复制到 src_dir
- **rpm**：`dnf download --source` → `rpm -ivh` → 解压 tarball
- **conda**：`git clone feedstock` 或 `conda skeleton pypi`

---

## 8.5 evo-get-info

获取包的详细信息。

```bash
evo-get-info <pkg> [--backend nix|rpm|conda]
```

**输出**：`{ status, backend, pkg, version, description, ... }`

**后端差异输出字段**：

| 字段 | nix | rpm | conda |
|------|-----|-----|-------|
| version | ✅ | ✅ | ✅ |
| description | ✅ | ✅ | |
| homepage | ✅ | | |
| license | ✅ | | |
| store_path | ✅ | | |
| arch | | ✅ | |
| vendor | | ✅ | |
| channel | | | ✅ |

---

## 8.6 evo-workspace

工作目录生命周期管理。

```bash
evo-workspace <create|list|archive|status|cleanup> [pkg]
```

### create

```bash
evo-workspace create <pkg>
```
创建 `/tmp/evo-fix-<pkg>/` + `~/.evo/work/<pkg>/`。

### list

```bash
evo-workspace list
```
列出所有 `/tmp/evo-fix-*` 工作目录。

### archive

```bash
evo-workspace archive <pkg>
```
打包为 tar.gz 存入 `~/.evo/archive/`。

### status

```bash
evo-workspace status <pkg>
```
显示：补丁数量、补丁列表、大小、上次构建时间。

### cleanup

```bash
evo-workspace cleanup <pkg>
```
删除 `/tmp/evo-fix-<pkg>/`。

---

## 8.7 evo-cleanup

清理临时文件、缓存、构建产物。

```bash
evo-cleanup [--work-days N] [--cache-days N] [--builds-days N] [--dry-run]
```

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--work-days` | 7 | /tmp/evo-fix-* 保留天数 |
| `--cache-days` | 30 | ~/.evo/cache/ 保留天数 |
| `--builds-days` | 14 | ~/.evo/builds/ 保留天数 |
| `--dry-run` | | 只报告，不删除 |

**输出**：`{ cleaned:{workspaces,cache,builds}, freed_bytes, disk_usage_pct, disk_warning }`

**保护机制**：`.evo-keep` 文件存在的目录不清理。

---

## 8.8 evo-build

统一构建入口。

```bash
evo-build <pkg> [--backend nix|rpm|conda] [--patch <patch-file>]
```

| 参数 | 说明 |
|------|------|
| `<pkg>` | 包名 |
| `--backend` | 后端 |
| `--patch` | 补丁文件路径 |

**行为**：
- **nix**：生成 overlay → `nix-build` with overlay
- **rpm**：复制 patch → 注册到 spec → `rpmbuild -ba`
- **conda**：复制 patch → 更新 meta.yaml → 递增 build number → `conda build`

**输出**：`{ status, result, log }`

**日志**：`~/.evo/logs/<pkg>-<timestamp>.log`

---

## 8.9 evo-verify

安装前验证（dry-run）。

```bash
evo-verify <pkg> [--backend nix|rpm|conda]
```

**验证内容**：
- **nix**：`nixos-rebuild dry-build`
- **rpm**：`rpm -Uvh --test` + 依赖检查
- **conda**：`conda install --dry-run --use-local`

**输出**：`{ risk, changes, missing_deps?, test_install_ok?, ... }`

---

## 8.10 evo-install

通过包管理器安装构建产物。

```bash
evo-install <pkg> [--backend nix|rpm|conda]
```

**行为**：
- **nix**：`nixos-rebuild switch` → 记录 generation
- **rpm**：`dnf/yum localinstall` → 记录事务 ID
- **conda**：`conda install --use-local` → 记录 revision

**输出**：`{ txn_id/generation/revision, rollback_cmd, log }`

**副作用**：写入 `~/.evo/history/<pkg>.jsonl`

---

## 8.11 evo-rollback

回滚到之前的状态。

```bash
evo-rollback <pkg> [--backend nix|rpm|conda] [--to <id>]
```

| 参数 | 说明 |
|------|------|
| `--to` | 指定目标（generation/txn/revision）。不指定则回滚到上一个 |

**行为**：
- **nix**：`nixos-rebuild switch --rollback` 或 `--to <generation>`
- **rpm**：`yum history undo <txn>`
- **conda**：`conda install --revision <N>`

---

## 8.12 evo-patch-create

从工作目录的修改创建补丁。

```bash
evo-patch-create <pkg> --desc "描述" [--ticket ID] [--risk safe|moderate|dangerous]
```

| 参数 | 必填 | 说明 |
|------|------|------|
| `<pkg>` | ✅ | 包名 |
| `--desc` | ✅ | 补丁描述 |
| `--ticket` | | 关联的 issue ID |
| `--risk` | | 手动指定风险等级（覆盖自动分级） |

**输出**：`{ patch, meta, risk, files_changed, size }`

**副作用**：
- 写入 `~/.evo/patches/<pkg>/<name>.patch`
- 写入 `~/.evo/patches/<pkg>/<name>.patch.meta.json`
- 更新 `/tmp/evo-fix-<pkg>/.evo-meta.json`

---

## 8.13 evo-patch-list

列出某包的所有补丁。

```bash
evo-patch-list <pkg>
```

**输出**：`{ patches:[{ name, desc, risk, created, ticket, size_bytes, files_changed }] }`

---

## 8.14 evo-patch-check

检查补丁兼容性。

```bash
evo-patch-check <pkg> [--against <version>]
```

**行为**：对每个补丁执行 `git apply --check`。

**输出**：`{ results:[{ patch, status:"compatible"|"conflict", conflicts }] }`

---

## 8.15 evo-patch-series

管理补丁应用顺序。

```bash
evo-patch-series show <pkg>
evo-patch-series add <pkg> <patch> [--after <other>]
evo-patch-series remove <pkg> <patch>
evo-patch-series reorder <pkg>
```

**数据文件**：`~/.evo/patches/<pkg>/series`

**格式**：纯文本，每行一个补丁文件名，`#` 开头为注释。

---

## 8.16 脚本调用关系图

```
用户
 │
 ▼
Agent ──→ evo-init (首次)
 │
 ├─→ evo-detect ──→ evo-fetch-source ──→ evo-get-info
 │                    │
 │                    ▼ (源码在 /tmp/evo-fix-<pkg>/src/)
 │                  Agent 分析源码
 │                    │
 │                    ▼
 │                  evo-patch-create
 │                    │
 │                    ▼
 │                  evo-patch-check (可选)
 │                    │
 │                    ▼
 │                  evo-build ──→ evo-verify
 │                    │              │
 │                    ▼              ▼
 │                  evo-install ← 用户确认
 │                    │
 │                    ▼ (出问题时)
 │                  evo-rollback
 │
 ├─→ evo-workspace (list/status/archive/cleanup)
 │
 └─→ evo-cleanup (定期清理)
```
