# 上下文交接

## 项目概况

Evolution OS — 一个基于 Rocky Linux 的元操作系统，源码驱动 + AI 辅助演进。

**阶段**: AI 集成完成 → 测试迭代  
**语言**: Rust (evo CLI), Shell (构建脚本)  
**基础**: Rocky Linux src.rpm  
**AI 层**: MiMo V2 Pro (通过 OpenClaw)

## 已完成

- ✅ 设计白皮书 (DESIGN.md) v0.3
- ✅ 技术架构 (ARCHITECTURE.md)
- ✅ evo CLI 骨架 — 8 个命令，Rust 实现
- ✅ Patch 管理 — create/list/drop/show/apply
- ✅ 构建系统 — rpmbuild + make fallback
- ✅ 上游 Rebase — 冲突检测, dry-run, `--ai` 自动解决
- ✅ 稳定标记 — 快照 + JSON 持久化
- ✅ 冻结/解冻 — 锁文件机制
- ✅ 状态看板 — 静态 ASCII TUI + `--live` ratatui 交互式看板
- ✅ AI 集成 — analyze/patch/resolve/config
- ✅ GIF/视频录屏 — Unicode fallback 机制（见下方）

## GIF 录屏 Unicode 修复

**问题**: DejaVu Sans Mono 不支持 emoji (⛔🟢🧪🎮) 和带圈数字 (①②③)，PIL 渲染时降级为 □ 方框。

**方案**: `gif_utils.py` 字符级 fallback 引擎：
- ⛔ → ● (红色)
- 🟢 → ● (绿色)
- ①②③ → [1][2][3]
- 🧪🎮 → ● (彩色)
- Rust 侧 CLI 输出同步移除 emoji，改用 colored text

**涉及文件**: `gif_utils.py`(新增), `test_recorder.py`, `test_tui_video.py`, `status.rs`, `freeze.rs`

## 录制方式

```bash
# GIF 录屏（交互式命令序列）
python3 test_recorder.py

# MP4 视频（TUI 交互录屏）
python3 test_tui_video.py
```

## 已实现命令

```bash
evo init <pkg>           # Rocky src.rpm → 源码树
evo status [--live]      # TUI 看板（静态/交互）
evo build [pkg...]       # rpmbuild / make
evo patch create/list/drop/show/apply
evo rebase [pkg...]      # 上游同步（--ai 冲突解决）
evo tag --create/list/show/delete
evo freeze [--unfreeze]  # 暂停/恢复进化
evo ai analyze/patch/resolve/config  # AI 驱动
```

## 下一步

- [ ] `evo status --live` 真实容器环境端到端测试
- [ ] 更多 Rocky 包的 init 测试（gcc, openssl 等大包）
- [ ] 救援系统集成
- [ ] `evo rollback` — 回滚到指定 Tag
- [ ] 快照管理 (Btrfs/ZFS)

## 关键参考

- small-model-lab: 本地小模型集成参考
- Rocky Linux src.rpm: 基础源码来源
- 设计决策详见 DECISIONS.md
