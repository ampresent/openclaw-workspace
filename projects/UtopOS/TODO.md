# UtopOS TODO

> v0.6 规划：工具化 + 脚本化 + 多后端 skill

## 架构原则

- **脚本做机械事** — 队列、压缩、构建、清理、git 操作等 → `scripts/` 下的可执行脚本
- **Skill 做决策** — 什么时候调什么脚本、怎么分析结果、怎么问用户 → SKILL.md 指导
- **Agent 做连接** — AI Agent 读 skill，调脚本，处理异常情况

```
SKILL.md (决策层)  →  "先跑 evo-detect，拿到后端后跑 evo-fetch-source"
scripts/ (执行层)  →  evo-detect, evo-fetch-source, evo-build, evo-cleanup ...
用户 (确认层)      →  "风险等级 moderate，要继续吗？"
```

---

## Phase 1: 核心脚本工具（P0）

### 1.1 基础设施脚本

- [x] `scripts/evo-init` — 初始化 evo 工作目录结构
  - 创建 `~/.evo/` (patches/, builds/, cache/, logs/, archive/, work/, upstream/, history/)
  - 生成默认 config、trust.toml、inventory.toml 模板
  - 支持 --force 重新初始化

- [x] `scripts/evo-cleanup` — 临时文件清理
  - 清理 `/tmp/evo-fix-*` 中超过 N 天的工作目录
  - 清理 `~/.evo/cache/` 中超过 N 天的源码缓存
  - 清理 `~/.evo/builds/` 中超过 N 天的构建产物
  - 磁盘水位检查，空间不足时激进清理
  - 保留标记为 `keep` 的工作目录
  - 支持 --dry-run 预览

- [x] `scripts/evo-workspace` — 工作目录生命周期管理
  - `evo-workspace create <pkg>` — 创建 `/tmp/evo-fix-<pkg>/` + `~/.evo/work/<pkg>/`
  - `evo-workspace list` — 列出活跃工作目录
  - `evo-workspace archive <pkg>` — 打包归档（patch + 日志 + 元数据 → tar.gz）
  - `evo-workspace status <pkg>` — 显示工作目录状态（patch 数、上次构建、磁盘占用）

### 1.2 包管理器检测 + 源码获取

- [x] `scripts/evo-detect` — 检测当前系统包管理器
  - 输出: `nix` | `rpm` | `conda` | `unknown`
  - 检测版本号（nixos-rebuild 版本、rpm 版本、conda 版本）
  - 写入 `~/.evo/config` 缓存

- [x] `scripts/evo-fetch-source` — 下载源码
  - `evo-fetch-source <pkg> [--backend nix|rpm|conda]`
  - 自动调用对应后端的下载逻辑
  - 源码缓存到 `~/.evo/cache/<pkg>-<version>/`
  - 已有缓存则跳过下载
  - 输出源码路径供 agent 使用

- [x] `scripts/evo-get-info` — 获取包信息
  - `evo-get-info <pkg> [--backend nix|rpm|conda]`
  - 输出: 包名、版本、源码路径、构建依赖、service unit
  - 统一 JSON 格式输出

### 1.3 构建管理

- [x] `scripts/evo-build` — 统一构建入口
  - `evo-build <pkg> [--backend nix|rpm|conda] [--patch <patch-file>]`
  - 应用 patch → 调用后端构建命令 → 产出包文件
  - 构建日志写入 `~/.evo/logs/<pkg>-<timestamp>.log`
  - 构建产物复制到 `~/.evo/builds/<pkg>-<version>/`
  - 返回构建结果 (success/failure) + 日志路径

- [x] `scripts/evo-build-queue` — 构建队列管理
  - `evo-build-queue add <pkg> [--priority high|normal|low]`
  - `evo-build-queue run [--max-parallel N]` — 顺序/并行执行队列
  - `evo-build-queue status` — 查看队列状态
  - `evo-build-queue clear` — 清空队列
  - 并发控制：大包限制并发，避免 OOM

- [x] `scripts/evo-verify` — 安装验证
  - `evo-verify <pkg> [--backend nix|rpm|conda]`
  - dry-run / test build
  - 输出风险评估摘要（JSON）
  - 不实际安装，只验证

- [x] `scripts/evo-install` — 通过包管理器安装构建产物
  - `evo-install <built-pkg-path> [--backend nix|rpm|conda]`
  - 调用后端安装命令
  - 记录安装事务 ID（用于回滚）

- [x] `scripts/evo-rollback` — 回滚
  - `evo-rollback <pkg> [--backend nix|rpm|conda] [--to <id>]`
  - 调用后端回滚命令

### 1.4 Diff / Patch 管理

- [x] `scripts/evo-patch-create` — 创建补丁
  - `evo-patch-create <pkg> --desc "修复说明" [--ticket ISSUE-123]`
  - 在源码目录中生成 diff
  - 写入 `~/.evo/patches/<pkg>/<patch-file>`
  - 生成元数据 JSON（原因、作者、时间、关联 issue、风险等级）

- [x] `scripts/evo-patch-list` — 列出某包的所有补丁
  - `evo-patch-list <pkg>`
  - 输出: patch 文件名、描述、状态（applied/pending/expired）、风险等级

- [x] `scripts/evo-patch-check` — 补丁兼容性检查
  - `evo-patch-check <pkg>` — 检查所有 patch 能否干净 apply
  - `evo-patch-check <pkg> --against <version>` — 检查对指定上游版本的兼容性
  - 输出: 哪些 patch 可以 apply，哪些冲突

- [x] `scripts/evo-patch-series` — 管理补丁应用顺序
  - `~/.evo/patches/<pkg>/series` 文件定义 apply 顺序
  - `evo-patch-series add <pkg> <patch> [--after <other-patch>]`
  - `evo-patch-series reorder <pkg>`

---

## Phase 2: 上游管理（P0）

- [x] `scripts/evo-upstream-add` — 注册上游仓库
  - `evo-upstream-add <pkg> --url <repo-url> [--branch main] [--type community|internal]`
  - 写入 `~/.evo/upstream/<pkg>.toml`

- [x] `scripts/evo-upstream-check` — 检查上游更新
  - `evo-upstream-check <pkg>` — fetch 上游，对比当前跟踪版本
  - 输出: 有无新版本、新版本号、变更摘要
  - 支持 `--all` 批量检查所有已注册包

- [x] `scripts/evo-upstream-fetch` — 拉取上游变更
  - `evo-upstream-fetch <pkg>` — fetch + 更新本地上游副本
  - 首次自动 clone，后续增量 fetch

- [x] `scripts/evo-rebase` — 自动 rebase 本地 patch 到新上游
  - `evo-rebase <pkg> [--to <version>] [--auto-resolve]`
  - 拉取上游 → 尝试 apply 所有 patch → 冲突时暂停并输出冲突详情
  - 成功：更新 patch 系列，记录 rebase 结果
  - 失败：输出冲突 patch，等待 agent/用户处理
  - `--auto-resolve` 尝试 3-way merge

- [x] `scripts/evo-upstream-prompt` — 交互式上游选择
  - `evo-upstream-prompt <pkg>` — 检测到多个上游时提示用户选择
  - 支持：社区上游 / 公司内部 fork / 两者都跟踪
  - 自动检测 nixpkgs homepage、RPM URL、已知 GitHub 仓库
  - 写入用户选择到 `~/.evo/upstream/<pkg>.toml`

---

## Phase 3: 用户交互增强（P1）

以下在 SKILL.md 中指导，不写脚本：

- [ ] SKILL.md: 每个关键节点自动询问用户确认
  - 下载源码前：确认包名是否正确
  - 分析完成后：展示自然语言摘要，确认根因判断
  - 生成 patch 后：展示风险摘要（不给 raw diff）
  - 构建前：确认 patch 内容
  - 安装前：确认风险等级
  - 上游有更新时：询问是否要 rebase

- [ ] SKILL.md: 风险摘要卡片模板
  - 用自然语言描述变更，不用 diff 语法
  - 包含：改了什么、影响范围、风险等级、回滚方式

- [ ] SKILL.md: 信任白名单
  - "以后这个包的 safe 级 patch 自动 apply，不再询问"
  - 写入 `~/.evo/trust.toml`

---

## Phase 4: 变更追踪 / 审计（P1）

- [ ] `scripts/evo-log` — 变更记录
  - `evo-log record <pkg> --action <action> --desc <desc> [--ticket <id>]`
  - 写入 `~/.evo/history/<pkg>.jsonl`
  - 记录: 时间、操作者、操作、描述、关联 issue、patch hash

- [ ] `scripts/evo-log-query` — 查询变更历史
  - `evo-log-query <pkg> [--since <date>] [--action patch|build|install|rollback]`

- [ ] `scripts/evo-deploy-status` — 部署矩阵
  - `evo-deploy-status <pkg>` — 查看哪些机器装了哪个版本
  - 需要配合 inventory 文件（`~/.evo/inventory.toml`）

---

## Phase 5: 测试管理（P1）

- [ ] `scripts/evo-test` — 补丁后测试
  - `evo-test <pkg> [--in container|vm|host]`
  - 在隔离环境安装补丁后的包 → 跑 test suite → 报告结果
  - 默认在容器中测试，不污染主机

- [ ] SKILL.md: 测试失败时的处理策略
  - 自动回滚 patch → 重新构建 → 通知用户

---

## Phase 6: 依赖图（P2）

- [ ] `scripts/evo-deps` — 反向依赖查询
  - `evo-deps rdepends <pkg>` — 哪些包依赖这个包
  - `evo-deps rebuild-plan <pkg>` — 改了这个包需要重构建哪些

- [ ] `scripts/evo-deps-batch` — 批量重构建
  - `evo-deps-batch <pkg>` — 按依赖拓扑排序，依次重构建下游包

---

## Phase 7: 多机同步（P2）

- [ ] `scripts/evo-sync` — Patch 同步到其他机器
  - `evo-sync <pkg> --to <host1,host2,...>`
  - 推送 patch + 元数据 → 远程机器 → 远程构建安装

- [ ] `scripts/evo-inventory` — 机器清单管理
  - `~/.evo/inventory.toml` — 机器列表、角色、已安装包版本

---

## 多后端 Skill 拆分

将当前单一 SKILL.md 拆分为：

- [x] `skills/UtopOS/SKILL.md` — 通用工作流（检测、诊断、patch 管理、用户交互）
- [x] `skills/UtopOS-nix/SKILL.md` — Nix 后端专项
  - nix-build / overlay / overrideAttrs / nixos-rebuild
  - Nix 语言基础（给 agent 看的）
  - flake.nix 结构
- [x] `skills/UtopOS-rpm/SKILL.md` — RPM 后端专项
  - rpmbuild / spec 文件 / SRPM
  - yum/dnf 操作
  - RPM 宏和构建系统
- [x] `skills/UtopOS-conda/SKILL.md` — Conda 后端专项
  - conda build / meta.yaml / recipe
  - feedstock 结构
  - conda 环境管理

---

## 脚本目录结构（规划）

```
scripts/
├── evo-init              # 初始化
├── evo-detect            # 检测包管理器
├── evo-fetch-source      # 下载源码
├── evo-get-info          # 包信息
├── evo-workspace         # 工作目录管理
├── evo-cleanup           # 清理临时文件
├── evo-build             # 构建
├── evo-build-queue       # 构建队列
├── evo-verify            # 验证（dry-run）
├── evo-install           # 安装
├── evo-rollback          # 回滚
├── evo-patch-create      # 创建补丁
├── evo-patch-list        # 列出补丁
├── evo-patch-check       # 补丁兼容性
├── evo-patch-series      # 补丁顺序
├── evo-upstream-add      # 注册上游
├── evo-upstream-check    # 检查上游更新
├── evo-upstream-fetch    # 拉取上游
├── evo-rebase            # 自动 rebase
├── evo-upstream-prompt   # 上游选择交互
├── evo-log               # 变更记录
├── evo-log-query         # 查询历史
├── evo-deploy-status     # 部署矩阵
├── evo-test              # 测试
├── evo-deps              # 依赖查询
├── evo-deps-batch        # 批量重构建
├── evo-sync              # 多机同步
└── evo-inventory         # 机器清单
```
