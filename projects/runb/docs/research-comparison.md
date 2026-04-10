# OCI 容器运行时架构调研对比

## 1. 概览

| 维度 | runc | crun | youki |
|------|------|------|-------|
| **语言** | Go | C | Rust |
| **作者** | Docker → OCI 标准 | Giuseppe Scrivano (Red Hat) | utam0k 等社区 |
| **star** | ~12k | ~3.4k | ~6k |
| **成熟度** | 最成熟，生产主力 | 生产可用，Podman 默认 | 生产可用，containerd e2e 通过 |
| **设计哲学** | 功能完整、兼容性优先 | 极致性能、可作嵌入库 | 安全 + 性能平衡 |
| **二进制大小** | ~14MB | ~2MB | ~6MB |

## 2. 性能基准

（来源：youki 官方 benchmark，create → start → delete 100 次 `/bin/true`）

| Runtime | 耗时 | 相对 youki |
|---------|------|-----------|
| crun | 47.3ms ± 2.8ms | 42% |
| youki | 111.5ms ± 11.6ms | 100% |
| runc | 224.6ms ± 12.0ms | 200% |

**关键发现**：
- crun 最快，但用了 C（手动内存管理）
- youki 比 runc 快一倍，且有内存安全保证
- crun 可在 512KB 内存限制下运行，runc 最低需要 4MB

## 3. 架构对比

### 3.1 runc（Go）

**架构**：
```
runc binary
  ├─ Go runtime (GC, goroutine scheduler)
  ├─ libcontainer (Go namespace/cgroup 封装)
  └─ nsexec.c (C 模块，处理 fork/setns)
```

**核心特点**：
- 使用 CGO 调用 C 模块 (`nsexec.c`) 处理 namespace 操作，因为 Go runtime 和 clone() 冲突
- runc 会 re-exec 自己进入 namespace，用 `nsexec.c` 做 pre-exec 设置
- 功能最全：checkpoint/restore (CRIU)、seccomp、AppArmor、SELinux 全支持
- OCI 参考实现，兼容性最好

**劣势**：
- Go runtime 开销大（GC、goroutine 栈），启动慢
- 二进制大 (~14MB)
- 内存占用高（Go runtime ~10-20MB 基础开销）
- Go + C 混合模型复杂，曾出过多个安全漏洞

### 3.2 crun（C）

**架构**：
```
crun binary
  ├─ libcrun (C 库，可嵌入)
  │   ├─ namespace/cgroup 直接 syscall
  │   ├─ yajl (JSON 解析，可选嵌入)
  │   └─ libocispec (OCI spec 解析)
  └─ CLI 入口
```

**核心特点**：
- 纯 C 实现，零 runtime 开销
- `libcrun` 是独立 C 库，可被其他程序嵌入（无需 fork 新进程管理容器）
- 直接 syscall，无中间层
- 二进制极小 (~2MB)
- Podman / CRI-O 的默认运行时

**劣势**：
- C 的手动内存管理，安全风险
- 功能相对 runc 略少（但覆盖 OCI 核心）
- 扩展性不如 Go/Rust，代码维护成本高

### 3.3 youki（Rust）

**架构**：
```
youki binary
  ├─ liboci-spec-rs (OCI spec 类型定义)
  ├─ libcontainer (Rust namespace/cgroup 实现)
  │   ├─ 直接调用 nix crate (Rust syscall 封装)
  │   ├─ cgroups 模块 (v1 & v2)
  │   └─ rootfs / mount 管理
  └─ youki CLI (clap)
```

**核心特点**：
- 内存安全 + 无 GC + 零成本抽象
- 使用 `nix` crate 做 Linux syscall，避免 C 绑定
- 模块化设计好（libcontainer 独立 crate 可复用）
- 二进制大小适中 (~6MB)
- 有专门的 OCI spec Rust 实现 (`oci-spec-rs`)
- Rust edition 2024，活跃开发

**劣势**：
- 比 crun 慢（但仍比 runc 快一倍）
- 生态不如 Go 成熟，遇到问题资料少
- 学习曲线陡

## 4. 关键模块对比

### 4.1 Namespace 隔离

| 实现 | 方法 |
|------|------|
| runc | CGO 调用 `nsexec.c`，Go 进程 re-exec 进入 namespace |
| crun | C 直接 `clone()`/`unshare()`/`setns()` |
| youki | Rust 通过 `nix` crate 调 `clone()`/`unshare()`/`setns()` |

**关键问题**：Go runtime 与 `clone()` 冲突（goroutine 会穿越 namespace），所以 runc 必须用 C 模块。Rust 和 C 没有这个问题。

### 4.2 Cgroup 管理

| 实现 | v1 | v2 | 特点 |
|------|----|----|------|
| runc | ✅ | ✅ | 最完整，复杂场景覆盖好 |
| crun | ✅ | ✅ | systemd 集成好 |
| youki | ✅ | ✅ | cgroupfs + systemd 双模式 |

### 4.3 Rootfs 处理

| 实现 | pivot_root | chroot | OverlayFS |
|------|-----------|--------|-----------|
| runc | ✅ | ✅ (fallback) | ✅ |
| crun | ✅ | ✅ | ✅ |
| youki | ✅ | ✅ | ✅ |

三者都用 `pivot_root` 作为首选，`chroot` 作为 fallback。

### 4.4 安全特性

| 特性 | runc | crun | youki |
|------|------|------|-------|
| seccomp | ✅ (libseccomp) | ✅ (libseccomp) | ✅ (libseccomp) |
| AppArmor | ✅ | ✅ | ✅ |
| SELinux | ✅ | ✅ | ✅ |
| capabilities | ✅ | ✅ | ✅ |
| rootless | ✅ | ✅ | ✅ |

## 5. runb 选型建议

### 推荐：Rust

**理由**：

1. **二进制体积**：< 5MB 的目标，Rust 最容易达成（crun ~2MB 验证了 C 可行，但 Rust 也能做到 ~3-5MB）
2. **性能**：无 GC、零成本抽象，预期在 crun 和 youki 之间
3. **安全性**：内存安全是容器运行时的核心要求（runc 因 Go+C 混合出过多个 CVE）
4. **可复用组件**：`oci-spec-rs`、`nix` crate 已经成熟，不需要从零写
5. **差异化**：相比 youki，可以在以下方向做差异化：
   - 更激进的体积优化（strip、LTO、去除不需要的特性）
   - 针对边缘/IoT 场景的 cgroup 策略
   - 更简单的架构（youki 的模块化也带来了复杂度）

### 不选 Go 的理由

- Go runtime 的 GC 和 goroutine 开销无法避免，二进制 ~14MB
- clone() 和 Go runtime 冲突，需要 C 模块 workaround
- 目标 < 5MB 几乎不可能

### 不选纯 C 的理由

- 手动内存管理在安全敏感的运行时中是定时炸弹
- 扩展性差，后续添加功能成本高
- 代码维护性差

## 6. 下一步

- [ ] 确定语言选型 → 记录到 DECISIONS.md
- [ ] 输出架构设计文档 (`docs/architecture.md`)
- [ ] 搭建 Rust 项目骨架

---

*调研时间: 2026-04-09 03:13*
