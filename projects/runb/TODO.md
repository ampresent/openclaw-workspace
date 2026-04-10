# TODO.md

## Phase 1: 调研 & 设计

- [x] 调研 runc 架构（namespace 隔离、cgroup 管理、rootfs 挂载）
- [x] 调研 crun 架构（C 实现、性能优化手段）
- [x] 调研 youki 架构（Rust 实现、安全模型）
- [x] 确定语言选型 → Rust，记录到 DECISIONS.md
- [x] 输出调研对比文档 (`docs/research-comparison.md`)
- [x] 输出架构设计文档（README.md 中）

## Phase 2: 核心实现

- [x] 实现 OCI bundle config.json 解析
- [x] 实现 `create` — 创建容器（chroot 模式）
- [x] 实现 `start` — chroot + exec 启动进程
- [x] 实现 `kill/stop` — 发送信号
- [x] 实现 `delete` — 清理资源
- [x] 实现 `state` — 查询容器状态（自动检测进程退出）
- [x] 实现 `list` — 列出所有容器
- [x] 实现 CLI 入口
- [x] 环境变量清理（clearenv）
- [x] 本地测试通过
- [x] Alpine Dockerfile 编写

## Phase 3: 验证 & 优化

- [ ] 通过 Alpine Docker 测试
- [ ] 性能基准测试（启动延迟、内存占用）
- [ ] 体积优化（strip、LTO 已完成，当前 1.3MB）
- [ ] 文档完善
