# REFERENCES.md

## 核心规范

- OCI Runtime Spec: https://github.com/opencontainers/runtime-spec
- OCI Image Spec: https://github.com/opencontainers/image-spec
- OCI Runtime Tool (validation): https://github.com/opencontainers/runtime-tools

## 参考实现

- **runc** (Go): https://github.com/opencontainers/runc — 原版参考实现，功能最全
- **crun** (C): https://github.com/containers/crun — 轻量高性能，字节跳动等在用
- **youki** (Rust): https://github.com/containers/youki — Rust 实现，活跃开发中
- **containerd-shim**: https://github.com/containerd/containerd — shim 接口参考

## 关键技术

- Linux namespaces (clone / unshare)
- cgroups v1 & v2
- pivot_root / chroot
- seccomp / capabilities
- bind mount / overlayfs
