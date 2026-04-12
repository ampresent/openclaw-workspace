# nix-evo 系列文档

> 从理念到实践，系统性理解 nix-evo。

## 目录

| 卷 | 标题 | 内容 |
|----|------|------|
| [第一卷](vol-01-overview.md) | 项目概览与设计理念 | 为什么存在、解决什么问题、核心哲学 |
| [第二卷](vol-02-architecture.md) | 架构深度剖析 | Skill / Script / Agent 三层架构、设计决策记录 |
| [第三卷](vol-03-workflow.md) | 工作流详解 | 六步工作流每一步的细节、分支逻辑、异常处理 |
| [第四卷](vol-04-nix-deep.md) | Nix 后端完全指南 | Nix 语言、overlay 机制、NixOS module、generation |
| [第五卷](vol-05-rpm-deep.md) | RPM 后端完全指南 | Spec 文件、rpmbuild、SRPM、发行版差异 |
| [第六卷](vol-06-conda-deep.md) | Conda 后端完全指南 | Feedstock、meta.yaml、conda build、环境管理 |
| [第七卷](vol-07-security.md) | 安全与信任模型 | 风险分级、白名单、反模式、审计日志 |
| [第八卷](vol-08-script-ref.md) | 脚本工具参考手册 | 14 个脚本的完整 API、输入输出、示例 |

## 阅读建议

**快速上手**：第一卷 → 用户指南

**日常使用**：用户指南 + 对应后端的卷（四/五/六）

**深度理解**：全卷通读

**开发贡献**：第二卷 + 第七卷 + 第八卷
