# 模型蒸馏设计文档 — CPU 密集型长期训练任务

**创建时间**: 2026-03-15  
**状态**: 已批准  
**作者**: 我的 AI 助理 · 严谨专业版

---

## 1. 项目目标

通过知识蒸馏 (Knowledge Distillation) 训练一个小型、低延迟的对话模型，能够：
- 在纯 CPU 环境下实时推理（目标：1-3 秒/回复）
- 充分利用服务器 CPU 资源（4 核，7GB 内存）
- 产出可独立部署的 GGUF 格式模型

---

## 2. 架构设计

### 2.1 整体架构

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   教师模型       │     │   蒸馏训练        │     │   学生模型       │
│  Qwen-7B-Chat   │────▶│  (Knowledge      │────▶│  TinyLlama-1.1B │
│  (GGUF 4bit)    │     │   Distillation)  │     │  (GGUF 4-6bit)  │
└─────────────────┘     └──────────────────┘     └─────────────────┘
        │                       │                        │
        ▼                       ▼                        ▼
   生成训练标签            最小化 KL 散度            CPU 实时推理
   (软标签 + 硬标签)        + 交叉熵损失
```

### 2.2 组件说明

| 组件 | 选型 | 理由 |
|------|------|------|
| **教师模型** | Qwen-7B-Chat (GGUF 4bit) | 中文能力强，4bit 量化后可在 7GB 内存运行 |
| **学生模型** | TinyLlama-1.1B | 生态成熟，有完整工具链，1.1B 是质量/速度平衡点 |
| **训练框架** | PyTorch + TRL | HuggingFace 生态，支持知识蒸馏 |
| **量化格式** | GGUF (llama.cpp) | CPU 推理优化，4-6bit 量化 |
| **训练数据** | OpenAssistant + ShareGPT | 公开对话数据集，多轮对话质量高 |

---

## 3. 数据流

### 3.1 训练数据准备

```
原始数据 (OASST1/ShareGPT)
        │
        ▼
┌─────────────────┐
│  数据预处理      │ — 过滤、格式化、去重
│  (Python 脚本)   │
└─────────────────┘
        │
        ▼
┌─────────────────┐
│  教师模型推理    │ — 对每个输入生成回复
│  (批量处理)      │ — 生成软标签 (logits)
└─────────────────┘
        │
        ▼
┌─────────────────┐
│  蒸馏数据集      │ — (prompt, student_output, teacher_logits)
│  (Parquet 格式)  │
└─────────────────┘
```

### 3.2 训练流程

```
for epoch in 1..N:
    for batch in dataloader:
        # 前向传播
        student_logits = student_model(batch.prompts)
        teacher_logits = cached_teacher_logits[batch.idx]
        
        # 损失计算
        distillation_loss = KL(student_logits, teacher_logits) * temperature
        task_loss = CrossEntropy(student_logits, batch.labels)
        total_loss = α * distillation_loss + (1-α) * task_loss
        
        # 反向传播 (梯度检查点降低内存)
        total_loss.backward()
        optimizer.step()
```

---

## 4. 资源需求

### 4.1 训练阶段

| 资源 | 需求 | 说明 |
|------|------|------|
| CPU | 4 核 100% | 训练期间持续满载 |
| 内存 | 6-8GB | 使用梯度检查点可降至 5GB |
| 磁盘 | 50GB | 模型 + 数据 + 检查点 |
| 时间 | 12-48 小时 | 取决于数据集大小 |

### 4.2 推理阶段

| 资源 | 需求 | 说明 |
|------|------|------|
| CPU | 1-2 核 | llama.cpp 优化 |
| 内存 | 1-2GB | 4bit 量化后 |
| 延迟 | 1-3 秒/回复 | 取决于输入长度 |

---

## 5. 错误处理

### 5.1 训练中断恢复

- 每 1000 step 保存检查点
- 支持从任意检查点恢复
- 训练日志记录到 `logs/training.log`

### 5.2 内存溢出处理

- 自动降低 batch size
- 启用梯度检查点
- 可选：增加 swap 分区

---

## 6. 测试策略

### 6.1 单元测试

- 数据加载器测试
- 损失函数计算测试
- 模型前向/反向传播测试

### 6.2 集成测试

- 完整训练流程（小数据集）
- 模型导出 GGUF 格式
- CPU 推理延迟测试

### 6.3 质量评估

- 人工评估回复质量（抽样 100 条）
- 与教师模型对比（BLEU/ROUGE）
- 延迟基准测试

---

## 7. 交付物

1. **训练脚本** — `scripts/train.py`
2. **蒸馏数据** — `data/distillation_dataset.parquet`
3. **学生模型** — `models/student-1.1B-distilled/`
4. **量化模型** — `models/student-1.1B-Q4_K_M.gguf`
5. **推理脚本** — `scripts/infer.py`
6. **训练日志** — `logs/training.log`

---

## 8. 长期运行说明

本项目设计为**一次性训练任务**，训练完成后：
- 模型可导出并在任何 CPU 上运行
- 训练脚本可重复使用（用新数据重新蒸馏）
- 建议保留训练日志和检查点用于复现

---

## 9. 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| 内存不足 | 中 | 高 | 梯度检查点、小 batch、swap |
| 训练时间过长 | 高 | 中 | 支持断点续训、进度监控 |
| 模型质量不佳 | 中 | 高 | 调整温度参数、增加数据 |
| CPU 过热 | 低 | 中 | 监控温度、限流 |

---

## 10. 下一步

调用 `writing-plans` 技能创建详细实现计划。
