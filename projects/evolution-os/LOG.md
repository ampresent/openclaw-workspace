# 日志

## 2026-04-12 (续)
- `patch apply` / `util::apply_patches` / `rebase` 全部切换到 `git apply`，支持新增文件
- rebase 冲突加 `git apply --3way` fallback
- `.evo/config.toml` 自动从 OpenClaw 配置检测 AI 模型（model + base_url），API key 仍由用户设环境变量
- `evo rebase --ai` — 冲突时自动调 AI resolver（检查 .rej + git merge conflicts + diff stat）
- `evo status --live` — ratatui 交互式 TUI 看板（q/r/↑↓/jk/Enter/f 快捷键，5s 自动刷新）

## 2026-04-12
- 项目创建
- 设计白皮书 v0.2 完成（基于用户提案）
- 技术架构文档完成
- 5 个关键设计决策记录
- 确认技术栈：Rust (evo CLI), Rocky Linux (base), 用户权限隔离 (安全模型)
