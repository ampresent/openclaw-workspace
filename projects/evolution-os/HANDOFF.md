# 上下文交接

## 项目概况

Evolution OS - 一个基于 Rocky Linux 的元操作系统，源码驱动 + AI 辅助演进。

## 已完成

- 设计白皮书 (DESIGN.md) - 完整的架构和交互设计
- 技术架构 (ARCHITECTURE.md) - 分层设计、安全模型、源码组织
- 决策记录 (DECISIONS.md) - 5 个关键决策已记录

## 下一步

1. **最优先**: 搭建 `evo` CLI Rust 项目骨架
2. 研究 Rocky Linux src.rpm 结构，编写提取脚本
3. 设计 Patch 栈的文件格式和操作接口
4. 参考 small-model-lab 设计本地模型集成方案

## 关键参考

- small-model-lab: 本地小模型集成参考
- Rocky Linux src.rpm: 基础源码来源
- 设计决策详见 DECISIONS.md
