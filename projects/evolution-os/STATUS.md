# 状态

- **阶段**: AI 集成完成 → 测试迭代
- **开始**: 2026-04-12
- **上次更新**: 2026-04-12

## 当前进度

| 模块 | 状态 | 备注 |
|------|------|------|
| 设计白皮书 | ✅ 完成 | v0.3，远程模型为主 |
| 架构文档 | ✅ 完成 | |
| evo CLI 骨架 | ✅ 完成 | Rust, 8 命令 |
| Patch 管理 | ✅ 完成 | create/list/drop/show/apply |
| 构建系统 | ✅ 完成 | rpmbuild + make fallback |
| 上游 Rebase | ✅ 完成 | 冲突检测, dry-run |
| 稳定标记 | ✅ 完成 | 快照 + JSON 持久化 |
| 冻结/解冻 | ✅ 完成 | |
| 状态看板 | ✅ 完成 | ASCII TUI + JSON 输出 |
| AI 集成 | ✅ 完成 | MiMo V2 Pro，已测试通过 |
| 主页 | ✅ 完成 | index.html |

## Rocky 容器测试

- ✅ `init curl` — 下载 src.rpm → 47 文件 + spec + git commit
- ✅ `patch create/list` — 生成 0001-add-test-file.patch
- ✅ `status` — 正确显示 1 包 / 2 patches / 48 files
- ✅ `tag` — 快照记录 git HEAD + patch 栈
- ✅ `freeze/unfreeze` — 锁文件机制正常
- ✅ `ai config` — 识别 MiMo V2 Pro 配置
- ✅ `ai analyze` — 成功分析 curl 源码结构
- ✅ `ai patch` — 生成 --silent-progress patch

## 已实现命令

```bash
evo init <pkg>           # Rocky src.rpm → 源码树
evo status               # TUI 看板
evo build [pkg...]       # rpmbuild / make
evo patch create/list/drop/show/apply
evo rebase [pkg...]      # 上游同步
evo tag --create/list/show/delete
evo freeze [--unfreeze]  # 暂停/恢复进化
evo ai analyze/patch/resolve/config  # AI 驱动
```

## 待办

- [ ] build: patch apply 改用 `git apply` 支持新增文件
- [ ] `.evo/config.toml` 自动从 OpenClaw 复制 AI 配置
- [ ] `evo ai resolve` — rebase 冲突时自动调用
- [ ] TUI 看板 (ratatui)
