# runb Layer Backend 对比分析

## 四种版本管理引擎

| 后端 | 原理 | init | commit | apply | 磁盘占用 |
|------|------|------|--------|-------|---------|
| **diff** | SHA256 manifest 对比，只存变更文件 | 4ms | 4ms | <1ms | 极小 (3.5KB) |
| **tar** | 变更文件打包为 tar.gz，类似 Docker | 7ms | 7ms | 2ms | 中等 (含 base.tar) |
| **hardlink** | 快照目录，未变更文件用硬链接 | 5ms | 13ms | 12ms | 较大(每层全量快照) |
| **git** | git 仓库跟踪变更 | ~100ms | ~50ms | ~30ms | 中等(增量压缩) |

> 测试环境: 35 文件 rootfs, 2 个 commit (2 新增 + 1 变更)

## Benchmark 结果 (本地测试)

```
=== diff ===
  init:     4ms    commit: 4ms    commit2: 4ms
  disk:     3.5 KB (仅变更文件)

=== tar ===
  init:     7ms    commit: 7ms    commit2: 6ms
  disk:     2423.8 KB (含 base.tar 基线)

=== hardlink ===
  init:     5ms    commit: 13ms   commit2: 13ms
  disk:     7143.0 KB (每层全量快照)
```

## 优劣分析

### diff (当前默认)

**优势:**
- 最快、最小 — 仅存储变更文件，开销极低
- 代码简单，无外部依赖
- 适合资源受限的边缘设备

**劣势:**
- 无跨层去重 — 相同文件在不同层各存一份
- 无压缩 — 文本文件浪费空间
- 不可移植 — 需要 runb 工具解析

**适用场景:** 边缘 IoT 设备、资源受限环境、快速原型

### tar (Docker 兼容)

**优势:**
- 单文件层 — 便于传输、分发
- gzip 压缩 — 文本文件压缩率高
- Docker 格式兼容 — 可与 Docker 生态互通
- base.tar 可作为分发格式

**劣势:**
- 需要解压 — apply 时有 I/O 开销
- base.tar 体积大 — 包含完整根文件系统
- 依赖外部 tar 命令

**适用场景:** 需要分发镜像、与 Docker 互通、网络传输

### hardlink (空间共享)

**优势:**
- 随机访问快 — 直接读文件，无需解压
- 层间共享 — 未变更文件零额外空间 (硬链接)
- 类似 overlayfs 思路 — commit 越多越省空间

**劣势:**
- 初始快照大 — 每层看起来是完整目录
- 硬链接限制 — 跨文件系统不支持，部分备份工具不兼容
- commit 较慢 — 需要遍历整个目录

**适用场景:** 频繁 commit、需要快速回滚、本地开发环境

### git (全功能版本控制)

**优势:**
- 完整历史 — log、diff、blame、branch 全支持
- 成熟生态 — 可用 git 工具链管理
- 增量压缩 — packfile 自动去重
- 分支支持 — 可并行开发多个版本

**劣势:**
- 最慢 — git 初始化和 commit 开销大
- 需要 git 二进制
- .git 目录会随历史增长
- 不适合大量二进制文件

**适用场景:** 需要完整版本历史、分支管理、团队协作

## 选择建议

| 场景 | 推荐后端 |
|------|---------|
| 边缘 IoT | `diff` — 最轻量 |
| 生产部署 / 分发 | `tar` — 便携可压缩 |
| 频繁迭代开发 | `hardlink` — 快速 commit/回滚 |
| 需要历史追踪 | `git` — 完整版本管理 |
| 磁盘敏感 | `diff` (少层) 或 `hardlink` (多层) |
| 网络传输 | `tar` — 压缩单文件 |

## 用法

```bash
# 指定后端初始化
runb init-layer myapp --backend diff      # 默认
runb init-layer myapp --backend tar
runb init-layer myapp --backend hardlink
runb init-layer myapp --backend git

# commit 自动检测后端
runb commit myapp -m "my changes"

# benchmark 对比所有后端
runb bench /path/to/rootfs
```

---

*测试时间: 2026-04-09*
