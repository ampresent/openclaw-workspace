# 状态

- **阶段**: v0.6 — 脚本工具落地
- **创建**: 2026-04-12
- **上次更新**: 2026-04-12 16:12

## 架构决策

从「MCP Server + nix-evo-agent 双组件」简化为「纯 OpenClaw Skill」。

原因：
- 核心操作都是 bash 命令，AI agent 有 exec 能力，不需要中间层翻译
- Skill 更轻量，不用维护独立进程
- 安全靠 skill 约束保证，不靠代码层强制
- 多机编排和 MCP 兼容性留到后面有需求再加

## 当前模块

| 模块 | 状态 | 备注 |
|------|------|------|
| Skill 文档 | ✅ 活跃 | `skills/nix-evo/SKILL.md` |
| 架构图 | ✅ 完成 | `docs/architecture.svg` |
| 流程对比图 | ✅ 完成 | `docs/workflow-comparison.svg` |
| Nix 后端工作流 | ✅ 完成 | 源码获取 + overlay 补丁 + generation 回滚 |
| RPM 后端工作流 | ✅ 完成 | SRPM + rpmbuild + yum history 回滚 |
| Conda 后端工作流 | ✅ 完成 | feedstock + conda build + revision 回滚 |
| 自动检测 + 源码获取 | ✅ 完成 | detect_backend() + fetch_source() |
| **脚本工具 (Phase 1)** | ✅ **14 个脚本** | 见下方详情 |

### Phase 1 脚本工具（全部完成并验证）

| 脚本 | 功能 | 状态 |
|------|------|------|
| `evo-init` | 初始化 ~/.evo/ 目录结构 + config 模板 | ✅ |
| `evo-detect` | 自动检测 nix/rpm/conda，输出 JSON | ✅ |
| `evo-fetch-source` | 三后端源码下载 + 缓存 | ✅ |
| `evo-get-info` | 包信息查询（统一 JSON 输出） | ✅ |
| `evo-workspace` | create/list/archive/status/cleanup | ✅ |
| `evo-cleanup` | 临时文件清理 + 磁盘水位检查 + dry-run | ✅ |
| `evo-build` | 统一构建入口（nix overlay / rpmbuild / conda build） | ✅ |
| `evo-verify` | dry-run 验证 + 风险评估 | ✅ |
| `evo-install` | 通过包管理器安装 + 记录事务 ID | ✅ |
| `evo-rollback` | 回滚到指定状态（generation/txn/revision） | ✅ |
| `evo-patch-create` | 从工作目录 diff 生成补丁 + 元数据 | ✅ |
| `evo-patch-list` | 列出补丁 + 元数据 | ✅ |
| `evo-patch-check` | 补丁兼容性检查（git apply --check） | ✅ |
| `evo-patch-series` | 管理补丁应用顺序 | ✅ |

## 已暂停的模块（保留代码）

> 以下模块来自早期多组件架构，代码保留在 `evo/` 和 `mcp-server/`，
> 当前不使用，未来有需要（多机编排、给非 OpenClaw 的 MCP 客户端用）可复用。

| 模块 | 状态 | 备注 |
|------|------|------|
| nix-evo-agent (Rust HTTP) | ⏸ 暂停 | 39 个端点，axum 框架 |
| MCP Server (TypeScript) | ⏸ 暂停 | 13 tools + hosts.toml + SSH 隧道 |
| NixOS modules | ⏸ 暂停 | nextcloud, jellyfin, monitoring-stack |
| Docker 集成 | ⏸ 暂停 | 容器发现 + compose 验证 |
| CI/CD | ⏸ 暂停 | Webhook + 预览部署 |
| 可观测性 | ⏸ 暂停 | Prometheus 指标 + 告警 |
| 开发模式 | ⏸ 暂停 | 模拟系统 + mock 端点 |
| API 版本管理 | ⏸ 暂停 | v1/v2 |
| 智能顾问 | ⏸ 暂停 | 回滚评分 + 容量规划 |
| TLS | ⏸ 暂停 | 未实现 |

## 关键决策

- ~~从 GitOps pull 模型改为 AI 诊断 + 安全执行模型~~
- **从双组件 (MCP+Agent) 简化为纯 skill 模式**
- 支持 Nix / RPM / Conda 三后端
- 核心原则：source-first，修源码不修运行时
- 自动检测包管理器 + 自动下载源码
