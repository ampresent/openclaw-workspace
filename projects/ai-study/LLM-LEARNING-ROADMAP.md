# LLM（大语言模型）学习路线

> 创建时间：2026-04-06

## 总览

```
阶段一          阶段二              阶段三                  阶段四
基础数学  →  深度学习  →  NLP + Transformer  →  LLM 专题
(2-3 周)    (3-4 周)       (5-6 周)            (持续深入)

已有传统 ML 基础，跳过 ML 入门阶段。
```

---

## 阶段一：数学基础（2-4 周）

| 主题 | 推荐资源 | 备注 |
|------|----------|------|
| 线性代数 | MIT 18.06（Gilbert Strang） | 矩阵运算、特征值分解、SVD |
| 概率论与统计 | MIT 6.041 或 Stanford CS 109 | 贝叶斯、MLE、常见分布 |
| 微积分与优化 | 3Blue1Brown 微积分系列 | 梯度、链式法则（反向传播必备） |

**目标**：能读懂 ML 论文里的数学符号，能手推梯度下降。

---

## 阶段二：机器学习基础（4-6 周）

| 课程 | 内容 | 资源 |
|------|------|------|
| **CS 229**（Stanford） | 监督学习、无监督学习、EM、PCA、SVM | https://cs229.stanford.edu |
| Coursera ML Specialization | Andrew Ng 的入门版，偏实操 | https://www.coursera.org/specializations/machine-learning-introduction |

**关键知识点**：
- 线性/逻辑回归
- 决策树、随机森林、XGBoost
- 聚类、降维（PCA）
- 偏差-方差权衡、正则化
- 梯度下降及其变种（SGD、Adam）

**目标**：理解「模型训练」的完整流程，能用 sklearn 跑通一个 ML pipeline。

---

## 阶段三：深度学习（4-6 周）

| 课程 | 内容 | 资源 |
|------|------|------|
| **CS 230**（Stanford） | 神经网络、CNN、RNN、调参 | https://www.coursera.org/specializations/deep-learning |
| **MIT 6.S191** | 快速入门深度学习（2 周集中课程） | http://introtodeeplearning.com |
| **CMU 11-785** | 深度学习，偏工程实现 | https://deeplearning.cs.cmu.edu |

**关键知识点**：
- 前向传播 & 反向传播
- 激活函数（ReLU、GELU、SiLU）
- BatchNorm、LayerNorm、Dropout
- CNN 基础（为理解 Vision Transformer 做铺垫）
- RNN / LSTM / GRU（理解序列建模的演进）
- PyTorch 基础操作

**目标**：能用 PyTorch 从零实现一个多层感知机和简单 CNN。

---

## 阶段四：NLP + Transformer（6-8 周）⭐ 核心阶段

| 课程 | 内容 | 资源 |
|------|------|------|
| **CS 224n**（Stanford） | NLP 基础、Word2Vec、RNN → Transformer、预训练模型 | https://web.stanford.edu/class/cs224n/ |

**关键知识点（按顺序）**：

1. **NLP 基础**
   - 词向量：Word2Vec、GloVe
   - 语言模型：N-gram → 神经语言模型
   - 序列到序列：Encoder-Decoder

2. **Attention 机制**
   - Bahdanau Attention（2014）
   - Luong Attention
   - Self-Attention

3. **Transformer 架构** ⭐⭐⭐
   - 论文精读：Attention Is All You Need (2017)
   - Multi-Head Attention
   - 位置编码（Sinusoidal → RoPE → ALiBi）
   - Layer Normalization（Pre-Norm vs Post-Norm）
   - Feed-Forward Network
   - 残差连接

4. **预训练语言模型**
   - BERT（双向编码器）
   - GPT 系列（自回归解码器）
   - T5（Encoder-Decoder）

**目标**：
- 能手写一个简化版 Transformer
- 能读懂 BERT 和 GPT-2 的源码
- 理解 tokenization（BPE、WordPiece、SentencePiece）

---

## 阶段五：LLM 专题（持续深入）⭐⭐ 最终目标

### 5.1 从零构建语言模型

| 资源 | 内容 | 链接 |
|------|------|------|
| **CS 336**（Stanford, 2025 Spring） | 从零构建语言模型：分词、训练、Scaling Laws、推理优化 | https://stanford-cs336.github.io |
| **nanoGPT**（Andrej Karpathy） | 用最简代码实现 GPT 训练 | https://github.com/karpathy/nanoGPT |
| **llm.c**（Andrej Karpathy） | 用纯 C 实现 GPT-2 训练 | https://github.com/karpathy/llm.c |

**CS 336 课程大纲**（2025 Spring）：
1. 课程概述与分词（Tokenization）
2. Transformer 架构实现
3. 训练基础设施（分布式训练）
4. Scaling Laws
5. 数据处理与清洗
6. 推理优化（KV Cache、Flash Attention、量化）
7. 对齐（RLHF、DPO）

### 5.2 LLM 核心技术深入

| 主题 | 关键论文/资源 |
|------|--------------|
| **分词器** | BPE (Sennrich 2016)、SentencePiece、Tiktoken |
| **位置编码** | RoPE (Su 2021)、ALiBi (Press 2022) |
| **注意力优化** | Flash Attention (Dao 2022)、MQA、GQA |
| **训练优化** | Mixed Precision Training、ZeRO、FSDP、DeepSpeed |
| **Scaling Laws** | Kaplan et al. 2020、Chinchilla (Hoffmann 2022) |
| **MoE** | Switch Transformer (Fedus 2021)、Mixtral |
| **推理优化** | KV Cache、Speculative Decoding、vLLM、GGUF/GGML 量化 |
| **对齐** | InstructGPT/RLHF (Ouyang 2022)、DPO (Rafailov 2023)、RLAIF |

### 5.3 Agent（基于 LLM 的智能体）

| 主题 | 关键资源 |
|------|---------|
| **ReAct** | Yao et al. 2022 — 推理+行动框架 |
| **Tool Use** | Toolformer (Schick 2023)、Function Calling |
| **规划** | Chain-of-Thought、Tree-of-Thought |
| **记忆** | RAG（检索增强生成）、向量数据库 |
| **框架** | LangChain、LlamaIndex、AutoGPT |

---

## 推荐学习顺序（精简版）

```
Week 1-3:   数学基础补齐（线性代数重点：矩阵分解、SVD；概率论重点：贝叶斯、MLE）
Week 4-7:   深度学习（CS 230 或 MIT 6.S191 快速版 + PyTorch 实操）
Week 8-13:  CS 224n NLP（重点：Transformer、BERT、GPT）
Week 14-20: CS 336 从零构建 LLM（或跟 nanoGPT + llm.c 实践）
Week 21+:   深入专题（Scaling Laws、推理优化、Agent）
```

---

## 实践项目（建议边学边做）

1. **Week 4-7**：用 PyTorch 实现 MNIST / CIFAR-10 分类器
2. **Week 8-13**：从零实现一个简化版 Transformer（参考 The Annotated Transformer）
3. **Week 14-20**：用 nanoGPT 训练一个小型 GPT（在莎士比亚数据集上）
4. **Week 21+**：实现一个简单的 RAG 系统或 Agent

---

## 必读论文清单

| # | 论文 | 年份 | 核心贡献 |
|---|------|------|---------|
| 1 | Attention Is All You Need | 2017 | Transformer 架构 |
| 2 | BERT: Pre-training of Deep Bidirectional Transformers | 2018 | 预训练+微调范式 |
| 3 | Language Models are Few-Shot Learners (GPT-3) | 2020 | In-context Learning |
| 4 | Training Language Models to Follow Instructions (InstructGPT) | 2022 | RLHF 对齐 |
| 5 | Scaling Laws for Neural Language Models | 2020 | Scaling Laws |
| 6 | FlashAttention: Fast and Memory-Efficient Attention | 2022 | 注意力优化 |
| 7 | LLaMA: Open and Efficient Foundation LLMs | 2023 | 开源 LLM 标杆 |
| 8 | Direct Preference Optimization (DPO) | 2023 | RLHF 替代方案 |
| 9 | Retrieval-Augmented Generation (RAG) | 2020 | 检索增强 |
| 10 | ReAct: Synergizing Reasoning and Acting | 2022 | Agent 框架 |
