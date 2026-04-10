# DECISIONS.md

## D-001: 语言选型 ✅ Rust

- **决定时间**: 2026-04-09
- **选择**: Rust
- **详细对比**: `docs/research-comparison.md`

**决策依据**：

| 因素 | Go | C | Rust ✅ |
|------|----|---|--------|
| 二进制体积 | ~14MB ❌ | ~2MB ✅ | ~6MB ✅ |
| 启动性能 | ~225ms ❌ | ~47ms ✅ | ~112ms ✅ |
| 内存安全 | 有 GC ✅ | 无 ❌ | 编译期保证 ✅ |
| 可维护性 | 好 ✅ | 差 ❌ | 好 ✅ |
| syscall 处理 | 需 C workaround ❌ | 直接 ✅ | nix crate ✅ |
| 生态 | 成熟 | 基础 | 成长中 |

**关键考量**：
- 目标 < 5MB → Go 排除
- 安全是运行时核心 → C 排除
- Rust 有 `oci-spec-rs`、`nix` 等成熟 crate 可复用
- 差异化方向：比 youki 更激进的体积优化 + 边缘场景优化

## D-002: 目标平台 — Linux amd64 优先

- **决定时间**: 2026-04-09
- **第一阶段**: Linux amd64 only
- **后续**: arm64 交叉编译支持

## D-003: 隔离策略 — 纯 chroot，无 namespace / 无 cgroup ✅

- **决定时间**: 2026-04-09
- **选择**: chroot only

**决策依据**：
- 用户明确要求不要 namespace、不要 cgroup
- chroot 提供文件系统隔离的基础能力
- 无 namespace 意味着进程共享宿主 PID/network/UTS/IPC 空间
- 无 cgroup 意味着无资源限制
- 适用场景：简单沙箱、开发测试、IoT 边缘设备、学习容器原理
- 环境变量通过 `clearenv()` 清理后重新注入，防止信息泄漏
