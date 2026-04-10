# runb vs runc — 核心差异对比

## 统一入口：`docker run`

runc 和 runb **都通过 `docker run` 作为唯一入口**，用户无需手动调用 OCI runtime 命令，也无需手动导出 rootfs。

```bash
# runc（Docker 默认 runtime）
docker run --rm alpine echo "hello from runc"

# runb（自定义 runtime，只需加 --runtime=runb）
docker run --runtime=runb --rm alpine echo "hello from runb"
```

Docker 自动完成：拉取镜像 → 创建 rootfs → 生成 OCI bundle → 调用 runtime → 清理。用户完全无感。

### 切换 runtime

| 场景 | 命令 |
|------|------|
| 临时使用 runb | `docker run --runtime=runb ...` |
| 全局默认切换 | 修改 `/etc/docker/daemon.json` 的 `default-runtime` |
| 单容器指定 | `docker create --runtime=runb ...` |

## 架构对比

| 维度 | runc | runb |
|------|------|------|
| **隔离方式** | Linux namespaces (PID/NET/UTS/IPC/MNT) | chroot only |
| **资源限制** | cgroups v1/v2 | 无 |
| **rootfs** | pivot_root + mount | chroot |
| **网络隔离** | network namespace + veth | 无（共享宿主网络） |
| **安全机制** | seccomp + capabilities + AppArmor | 无 |
| **二进制大小** | ~9MB (Go) | ~1.8MB (Rust, strip+LTO) |
| **依赖** | Go runtime + libseccomp + libapparmor | libc only |
| **rootless** | ✅ 支持 | ❌ 需要 root |
| **热升级** | ❌ 不支持 | ✅ overlay bind mount |
| **Layer 管理** | ❌ 需 containerd | ✅ 内建 (diff/tar/hardlink/git) |
| **启动方式** | 直接 exec（一步到位） | create → start（两步） |
| **OCI 兼容** | 完整 OCI Runtime Spec | 子集（无 namespace/cgroup） |

## Docker 内部流程对比

`docker run --runtime=runb` 背后发生的流程：

```
docker run --runtime=runb alpine echo hello
  ↓
dockerd → containerd → containerd-shim-runc-v2
  ↓
runb-docker-shim（适配层）
  ├── pull image → 解压 rootfs（Docker 自动完成，用户无需手动 export）
  ├── create → fork hold 进程（满足 shim PID 要求）
  ├── start → kill hold → chroot + clearenv + execvp
  ├── state → 读 state.json
  ├── kill → 发信号给容器进程
  └── delete → 清理 state 目录
  ↓
容器进程在宿主 PID namespace 中执行
```

对比 runc：

```
docker run alpine echo hello
  ↓
dockerd → containerd → containerd-shim-runc-v2
  ├── create → namespace 隔离 + cgroup 设置 + pivot_root
  ├── start → unblock init 进程
  └── ...
  ↓
容器进程在隔离的 namespace 中执行
```

## 性能对比

| 指标 | runc | runb | 说明 |
|------|------|------|------|
| 创建延迟 | ~50ms | <5ms | runb 无 namespace 开销 |
| 启动延迟 | ~30ms | <10ms | runb 无 pivot_root |
| 内存占用 | ~15MB (Go runtime) | <1MB | runb 静态链接无 GC |
| 二进制大小 | ~9MB | ~1.8MB | Rust strip + LTO |

> ⚠️ 以上数据为估算值，实际取决于工作负载。runb 的低开销以牺牲隔离性为代价。

## 安全模型对比

```
runc:
  宿主内核
  ├── namespace 隔离 → 容器看不到宿主进程/网络/文件系统
  ├── cgroup 限制 → 容器无法耗尽宿主资源
  ├── seccomp → 过滤危险 syscall
  └── capabilities → 最小权限原则

runb:
  宿主内核
  └── chroot → 容器只能看到指定 rootfs（但能看到宿主所有进程/网络）
```

**关键区别**：runb 的容器进程在宿主的 PID namespace 中可见，可以访问宿主网络，没有资源限制。适合受信任环境，不适合多租户。

## 适用场景

### runc 适用
- 生产环境容器编排（Kubernetes、Docker）
- 需要严格安全隔离（多租户）
- 需要网络隔离、资源限制
- 完整 OCI Runtime Spec 要求

### runb 适用
- 嵌入式/IoT 设备（极小二进制）
- 单租户环境（个人服务器、开发机）
- 需要 rootfs 热升级（替换 rootfs 不丢数据）
- 不需要网络隔离的批处理任务
- 快速原型/测试环境

---
*创建: 2026-04-09 13:15 | 更新: 2026-04-09 14:00*
