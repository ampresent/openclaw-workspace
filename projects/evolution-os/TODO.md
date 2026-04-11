# TODO

## P0 - 项目启动

- [ ] Rocky Linux src.rpm 提取脚本 (`build/extract-srpm.sh`)
- [ ] Patch 管理工具原型 (`patches/patch-manager.sh`)
- [ ] `evo` CLI 项目骨架 (`evo/` Cargo.toml + 基础命令框架)
- [ ] `evo status` 最小实现 (显示当前 patch 栈和构建状态)

## P1 - 核心功能

- [ ] 构建调度器 (`evo-daemon`)
- [ ] `evo build` - 触发单包构建
- [ ] `evo rebase` - 上游同步 + 冲突检测
- [ ] `evo patch create/drop/list` - Patch 栈操作
- [ ] `evo tag` - 稳定标记

## P2 - AI 集成

- [ ] 本地小模型集成 (意图感知 / 错误截获)
- [ ] Claude Code 安全接口 (用户权限沙箱)
- [ ] "命令错误即开发" 工作流
- [ ] `evo freeze` - 进化暂停

## P3 - 救援与回滚

- [ ] 救援系统集成
- [ ] `evo rollback` - 回滚到指定 Tag
- [ ] 快照管理 (Btrfs/ZFS)

## 激进愿景

- [ ] `/etc` 修改拦截 → 自动转 Patch
- [ ] 原子级源码映射
- [ ] 自举验证流程
