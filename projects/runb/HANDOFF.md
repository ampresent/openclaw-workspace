# HANDOFF.md

## 项目快照

runb — 轻量 OCI 兼容容器运行时，刚启动，处于 Phase 1 调研阶段。

## 恢复方式

1. 读 `README.md` 了解目标
2. 读 `STATUS.md` 看当前进度
3. 读 `TODO.md` 找第一个可执行任务
4. 读 `DECISIONS.md` 了解已做决策

## 当前下一步

调研 runc / crun / youki 三个实现的架构差异，重点对比：
- namespace 隔离策略
- cgroup 管理方式
- rootfs 处理
- 二进制体积与启动性能

输出到 `docs/research-comparison.md`，然后做语言选型。

---

*checkpoint: 2026-04-09 03:11*
