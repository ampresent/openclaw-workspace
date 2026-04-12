# 第三卷：工作流详解

---

## 3.1 六步工作流

### 第零步：初始化

```bash
scripts/evo-init
```

幂等操作。检测 `~/.evo/` 是否存在，不存在则创建目录结构和默认配置。
Agent 在第一次使用或 `~/.evo/` 被删除后自动调用。

### 第一步：诊断 + 获取源码

**并行执行**三个操作：

```bash
# 1. 检测后端
scripts/evo-detect → { backend, version, tools }

# 2. 下载源码（依赖 detect 结果）
scripts/evo-fetch-source <pkg> → { src_dir, spec?, recipe? }

# 3. 系统诊断（不依赖脚本，直接 exec）
journalctl -u <service> -n 50
systemctl status <service>
```

**为什么并行**：诊断和源码下载互不依赖，同时执行节省时间。
Agent 可以在等待源码下载的同时分析日志。

**关键点**：源码下载是整个工作流的基础，不能跳过。
没有源码就无法分析根因，后续所有步骤都无法执行。

### 第二步：分析源码

**这是核心步骤，也是最需要 AI 能力的步骤。**

```
进入 src_dir/
├── 配置模板（*.conf.in, *.service）
├── 源码（*.c, *.py, *.rs）
├── 构建配置（Makefile, CMakeLists.txt, Cargo.toml）
└── 文档（README, CHANGELOG）
```

分析策略：
1. **先看配置模板** — 很多问题出在默认配置值
2. **再看源码** — 如果配置没问题，可能是代码逻辑 bug
3. **对照日志** — 日志中的错误信息指向源码中的位置
4. **检查编译参数** — 有时候问题出在构建选项

Agent 不需要"读懂"所有源码。它需要：
- 定位到和问题相关的文件
- 理解相关的代码段
- 生成正确的修改

### 第三步：生成补丁

```bash
# 1. 在工作目录中修改源码（Agent 直接 exec edit/write）
cd /tmp/evo-fix-<pkg>/src/
vim/编辑相关文件

# 2. 生成补丁
scripts/evo-patch-create <pkg> --desc "修复描述" [--ticket ISSUE-123] [--risk safe|moderate|dangerous]
→ { patch_file, risk, files_changed, insertions, deletions }
```

**补丁命名**：自动从描述生成安全文件名
```
"修复 upstream timeout 默认值" → fix-upstream-timeout-default-value.patch
```

**元数据**：每个补丁附带 `.meta.json`
```json
{
  "pkg": "nginx",
  "desc": "修复 upstream timeout 默认值",
  "ticket": "",
  "risk": "moderate",
  "created": "2026-04-12T16:00:00+08:00",
  "patch_file": "~/.evo/patches/nginx/fix-upstream-timeout.patch",
  "size_bytes": 1024,
  "files_changed": 1,
  "insertions": 5,
  "deletions": 2
}
```

### 第四步：验证 + 应用

```bash
# 1. 构建
scripts/evo-build <pkg> --patch <patch_file>
→ { result, log }

# 2. 验证（必须在安装前）
scripts/evo-verify <pkg>
→ { risk, changes, missing_deps, test_install_ok }
```

验证内容因后端而异：
- **Nix**：`nixos-rebuild dry-build`（构建但不激活）
- **RPM**：`rpm -Uvh --test`（测试安装，不实际执行）+ 依赖检查
- **Conda**：`conda install --dry-run --use-local`（解析依赖，不安装）

### 第五步：安装

```bash
scripts/evo-install <pkg>
→ { txn_id, rollback_cmd, generation?, revision? }
```

安装后必须验证服务状态：
```bash
systemctl status <service>
# 确认服务正常运行
```

### 第六步：提交上游（可选）

如果修复的是上游 bug：
1. 基于补丁生成 PR
2. 使用项目的 `NIXPKGS-PR-TEMPLATE.md`
3. 提交到对应上游仓库

---

## 3.2 分支逻辑

### 正常流程

```
detect → fetch → diagnose → analyze → patch → build → verify → install
```

### 异常分支

#### 检测失败（未识别包管理器）

```
detect → { backend: "unknown" }
→ 提示用户手动指定 --backend
→ 用户指定后继续
```

#### 源码下载失败

```
fetch → { status: "error" }
→ 检查网络/权限
→ 尝试其他下载方式（feedstock → skeleton）
→ 仍然失败则告知用户，建议手动提供源码
```

#### 分析找不到根因

```
analyze → 无法定位问题
→ 扩大搜索范围（更多日志、更多源码文件）
→ 如果仍然找不到，建议用户补充信息
```

#### 构建失败

```
build → { status: "error", log: "..." }
→ 分析构建日志
→ 常见原因：依赖缺失、patch 格式错误、编译参数不对
→ 修改后重试
```

#### 验证失败

```
verify → { risk: "dangerous", missing_deps: [...] }
→ 展示风险详情
→ 用户决定：修改补丁 / 强制安装 / 放弃
```

#### 回滚

```
install → 服务异常
→ evo-rollback <pkg>
→ 确认服务恢复正常
→ 分析失败原因，从第三步重新开始
```

---

## 3.3 用户交互节点

### 必须确认的节点

| 节点 | 展示内容 | 确认方式 |
|------|---------|---------|
| 补丁生成后 | 风险摘要卡片 | y/n |
| 构建验证后 | 验证结果 + 风险 | y/n |
| 构建失败后 | 日志摘要 + 建议 | y/n / 修改 |

### 自动跳过确认的条件

```toml
# ~/.evo/trust.toml
[trust.nginx]
safe_auto = true
risk_levels = ["safe"]
```

当 `patch.risk` 在 `risk_levels` 中时，跳过确认。

### 永远需要确认的情况

- risk == "dangerous"（即使在白名单中）
- 验证失败后
- 回滚操作

---

## 3.4 多补丁场景

当一个包有多个补丁时：

```bash
# 查看所有补丁
scripts/evo-patch-list <pkg>

# 检查兼容性
scripts/evo-patch-check <pkg>
→ { results: [{ patch, status: "compatible"|"conflict" }] }

# 管理应用顺序
scripts/evo-patch-series show <pkg>
scripts/evo-patch-series add <pkg> <patch> [--after <other>]
```

构建时自动按 series 顺序应用所有补丁。

---

## 3.5 工作目录生命周期

```
创建 (evo-workspace create)
  → /tmp/evo-fix-<pkg>/ 创建
  → .evo-meta.json 写入

工作中
  → 源码、补丁、构建日志都在里面
  → .last-build 记录构建时间

归档 (evo-workspace archive)
  → 打包为 tar.gz 存入 ~/.evo/archive/
  → 状态标记为 archived

清理 (evo-cleanup)
  → 超过 N 天的工作目录自动清理
  → .evo-keep 文件保护的目录不清理
```
