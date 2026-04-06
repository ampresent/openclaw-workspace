# 入口 A：从 Transformer 论文切入 — 详细扩展方案

> 核心路径：精读 Attention Is All You Need → 遇到什么补什么 → 自然扩散到整个 LLM 知识体系

## 阅读策略

**不是从头读到尾**，而是按模块拆解，每个模块独立深入：

```
论文 §1-2 背景         → 快速扫过
论文 §3.1 Encoder-Decoder → 核心，反复读
论文 §3.2 Attention      → 核心中的核心，必须手推
论文 §3.3 Position-wise FFN → 简单，理解即可
论文 §3.4 Embeddings     → 追到分词器
论文 §3.5 Positional Enc → 追到 RoPE/ALiBi
论文 §4 为什么用 Attention → 理解动机
论文 §5 Training         → 追到优化器、学习率调度
论文 §6 Results          → 看实验设计思路
论文 §7 Conclusion       → 快速扫过
```

---

## 模块 1：论文动机与背景（§1-2）

### 目标
理解「为什么需要 Transformer」——在它之前，序列建模被 RNN/LSTM 主导，存在两大问题：
1. 无法并行计算（逐步依赖）
2. 长距离依赖衰减

### 学习资料

| 类型 | 资源 | 说明 |
|------|------|------|
| 📄 论文 | **Sequence to Sequence Learning with Neural Networks** (Sutskever 2014) | Seq2Seq 的起点 |
| 📄 论文 | **Neural Machine Translation by Jointly Learning to Align and Translate** (Bahdanau 2014) | Attention 的起源 |
| 📄 论文 | **Long Short-Term Memory** (Hochreiter 1997) | LSTM 原始论文，了解 RNN 的核心问题 |
| 🎥 视频 | CS 224n Lecture 5: Recurrent Neural Networks | RNN/LSTM 的讲解 |
| 📝 博客 | [Understanding LSTM Networks](https://colah.github.io/posts/2015-08-Understanding-LSTMs/) — colah's blog | 经典 LSTM 图解 |
| 📝 博客 | [The Unreasonable Effectiveness of RNNs](https://karpathy.github.io/2015/05/21/rnn-effectiveness/) — Karpathy | RNN 能做什么、有什么局限 |

### 可跳过条件
如果你已经了解 RNN/LSTM 的原理和局限性，这一部分可以直接跳过。

---

## 模块 2：Encoder-Decoder 架构（§3.1）

### 目标
理解 Transformer 的整体结构——编码器堆叠 + 解码器堆叠。

### 学习资料

| 类型 | 资源 | 说明 |
|------|------|------|
| 📝 博客 | [The Illustrated Transformer](https://jalammar.github.io/illustrated-transformer/) — Jay Alammar | ⭐ 最经典的 Transformer 图解，必看 |
| 📝 博客 | [The Annotated Transformer](https://nlp.seas.harvard.edu/annotated-transformer/) — Harvard NLP | ⭐ 逐行代码解读，必看 |
| 🎥 视频 | CS 224n Lecture 8: Self-Attention and Transformers | Stanford NLP 课程讲解 |
| 🎥 视频 | [Attention is All You Need (Paper Explained)](https://www.youtube.com/watch?v=iDulhoQ2pro) — Yannic Kilcher | 论文逐段讲解 |
| 🎥 视频 | [Transformer Neural Network Explained](https://www.youtube.com/watch?v=4Bdc55j80l8) — Muneeb | 简洁直觉讲解 |
| 💻 代码 | [nanoGPT](https://github.com/karpathy/nanoGPT) — Karpathy | 最简 GPT 实现，约 300 行核心代码 |

### 延伸阅读
| 资源 | 说明 |
|------|------|
| [The Illustrated Transformer](https://jalammar.github.io/illustrated-transformer/) 中的 Encoder-Decoder 图 | 跟着画一遍结构图 |
| CS 230 Lecture on Sequence Models | 如果对 Encoder-Decoder 不熟 |

### 动手任务
- [ ] 用纸笔画出 Encoder Block 和 Decoder Block 的完整结构
- [ ] 标注每一层的输入输出 shape

---

## 模块 3：Self-Attention 机制（§3.2）⭐⭐⭐ 核心中的核心

### 目标
彻底理解 Scaled Dot-Product Attention 和 Multi-Head Attention，能手推公式。

### 关键公式
```
Attention(Q, K, V) = softmax(QK^T / √d_k) V
```

### 学习资料

| 类型 | 资源 | 说明 |
|------|------|------|
| 📄 论文 | Attention Is All You Need §3.2 | 原始定义 |
| 📝 博客 | [The Illustrated Transformer](https://jalammar.github.io/illustrated-transformer/) 中的 Attention 章节 | ⭐ 图解 Q/K/V |
| 📝 博客 | [Attention? Attention!](https://lilianweng.github.io/posts/2018-06-24-attention/) — Lilian Weng | 全面梳理各种 Attention 变体 |
| 📝 博客 | [The Transformer Family](https://lilianweng.github.io/posts/2020-04-07-the-transformer-family/) — Lilian Weng | Transformer 变体综述 |
| 🎥 视频 | [Attention in transformers, step-by-step](https://www.youtube.com/watch?v=g90LqPg2TgI) — 3Blue1Brown | ⭐ 数学直觉最佳 |
| 🎥 视频 | [Attention is all you need (Transformer) - Model explained](https://www.youtube.com/watch?v=KJtZARuO3JY) — AI Coffee Break | 详细可视化 |
| 📝 博客 | [Transformer Explainer](https://transformer-explainer.com/) | 交互式可视化 |
| 📄 论文 | **Neural Machine Translation by Jointly Learning to Align and Translate** (Bahdanau 2014) | Attention 的起源 |
| 📄 论文 | **Effective Approaches to Attention-based Neural Machine Translation** (Luong 2015) | Luong Attention |

### 延伸阅读
| 资源 | 说明 |
|------|------|
| [The Annotated Transformer](https://nlp.seas.harvard.edu/annotated-transformer/) 中的 Multi-Head Attention 代码 | 逐行读，理解 PyTorch 实现 |
| CS 224n Lecture 8 中的 Q/K/V 示例 | 跟着做数值计算 |

### 动手任务
- [ ] 手动计算一个 3-token 序列的 Attention 矩阵（用 NumPy）
- [ ] 理解为什么要除以 √d_k（梯度消失问题）
- [ ] 在 The Annotated Transformer 中找到 Multi-Head Attention 代码，逐行理解

---

## 模块 4：分词器 / Tokenization（§3.4 追溯）

### 目标
理解文本如何变成 token 序列——这是 LLM 的输入层。

### 核心概念
- Token ≠ 字符 ≠ 单词
- BPE (Byte Pair Encoding)：最常用的分词算法
- Token Embedding + Positional Encoding = 输入表示

### 学习资料

| 类型 | 资源 | 说明 |
|------|------|------|
| 🎥 视频 | CS 336 Lecture 1: Overview and Tokenization | Stanford 最新课程，从零讲分词 |
| 📝 博告 | [Byte Pair Encoding](https://huggingface.co/learn/nlp-course/chapter6/5) — Hugging Face NLP Course | BPE 实战教程 |
| 📝 博客 | [SentencePiece](https://github.com/google/sentencepiece) GitHub | Google 的分词库 |
| 📄 论文 | **Neural Machine Translation of Rare Words with Subword Units** (Sennrich 2016) | BPE 原始论文 |
| 💻 代码 | [minBPE](https://github.com/karpathy/minbpe) — Karpathy | 最简 BPE 实现，约 300 行 |
| 📝 博客 | [Let's build the GPT Tokenizer](https://www.youtube.com/watch?v=zduSFxRajkE) — Karpathy 视频 | ⭐ 2 小时手把手实现 BPE |

### 延伸阅读
| 资源 | 说明 |
|------|------|
| Tiktoken (OpenAI 的分词库) | 理解 GPT 系列实际用的分词方案 |
| Hugging Face Tokenizers 库 | 工业级分词实现 |

### 动手任务
- [ ] 用 minBPE 实现一个简单的分词器
- [ ] 对比同一句话在 GPT-2 和 LLaMA 下的不同分词结果

---

## 模块 5：位置编码 / Positional Encoding（§3.5 追溯）

### 目标
理解 Transformer 如何注入位置信息——Self-Attention 本身是位置无关的。

### 演进路线
```
Sinusoidal (原始论文) → Learned Positional → RoPE (旋转位置编码) → ALiBi (线性偏置)
```

### 学习资料

| 类型 | 资源 | 说明 |
|------|------|------|
| 📄 论文 | Attention Is All You Need §3.5 | Sinusoidal 位置编码 |
| 📄 论文 | **RoFormer: Enhanced Transformer with Rotary Position Embedding** (Su 2021) | ⭐ RoPE 原始论文 |
| 📄 论文 | **Train Short, Test Long: Attention with Linear Biases** (Press 2022) | ALiBi 论文 |
| 📝 博客 | [Rotary Position Embedding](https://blog.eleuther.ai/rotary-embedding/) — EleutherAI | RoPE 图解 |
| 📝 博客 | [Understanding Positional Encoding in Transformers](https://medium.com/@bavalpreet-singh/understanding-positional-encoding-in-transformers) | 通俗讲解 |
| 🎥 视频 | CS 224n 中的位置编码章节 | 课程讲解 |

### 延伸阅读
| 资源 | 说明 |
|------|------|
| YaRN、NTK-aware Scaling 等长上下文扩展方法 | 如果对「为什么长文本效果变差」感兴趣 |

### 动手任务
- [ ] 用 NumPy 实现 Sinusoidal 位置编码，可视化不同位置的编码向量
- [ ] 理解 RoPE 的旋转矩阵直觉（复数表示）

---

## 模块 6：前馈网络与残差连接（§3.3）

### 目标
理解 FFN（Position-wise Feed-Forward Network）和残差连接 + LayerNorm 的作用。

### 学习资料

| 类型 | 资源 | 说明 |
|------|------|------|
| 📝 博客 | [The Illustrated Transformer](https://jalammar.github.io/illustrated-transformer/) 中的 FFN 章节 | 图解 |
| 📄 论文 | **On Layer Normalization in the Transformer Architecture** (Xiong 2020) | Pre-Norm vs Post-Norm |
| 📄 论文 | **Flash-LLaMA** 中关于 SwiGLU 的讨论 | 现代 LLM 中 FFN 的变体（SwiGLU） |
| 📝 博客 | [Cramming: Training a Language Model on a Single GPU in One Day](https://arxiv.org/abs/2212.14034) | FFN 设计选择的实验 |

### 动手任务
- [ ] 在 The Annotated Transformer 中找到 FFN 层代码
- [ ] 理解 SwiGLU 为什么比 ReLU 更好

---

## 模块 7：训练策略（§5 追溯）

### 目标
理解 Transformer 怎么训练——优化器、学习率调度、正则化。

### 学习资料

| 类型 | 资源 | 说明 |
|------|------|------|
| 📄 论文 | Attention Is All You Need §5 | 原始训练策略 |
| 📄 论文 | **Adam: A Method for Stochastic Optimization** (Kingma 2014) | Adam 优化器 |
| 📄 论文 | **Decoupled Weight Decay Regularization** (Loshchilov 2017) | AdamW |
| 📄 论文 | **Deep Networks with Stochastic Depth** (Huang 2016) | Dropout 相关 |
| 📝 博客 | [The Annotated Transformer](https://nlp.seas.harvard.edu/annotated-transformer/) 中的训练代码 | Noam Learning Rate Schedule |
| 🎥 视频 | CS 230 Lecture: Optimization | 优化器讲解 |

### 延伸阅读
| 资源 | 说明 |
|------|------|
| Mixed Precision Training (FP16/BF16) | 大模型训练必备 |
| Gradient Accumulation | 显存不够时的技巧 |
| Distributed Training (DDP, FSDP, DeepSpeed) | 多卡训练 |

### 动手任务
- [ ] 理解 Noam Learning Rate Schedule 的 warmup + decay
- [ ] 在 nanoGPT 中找到学习率调度代码

---

## 模块 8：从 Transformer 到 GPT / BERT（论文延伸）

### 目标
理解 Transformer 如何演变成两大预训练范式。

### 学习资料

| 类型 | 资源 | 说明 |
|------|------|------|
| 📄 论文 | **Improving Language Understanding by Generative Pre-Training** (Radford 2018) | GPT-1 |
| 📄 论文 | **Language Models are Unsupervised Multitask Learners** (Radford 2019) | GPT-2 |
| 📄 论文 | **BERT: Pre-training of Deep Bidirectional Transformers** (Devlin 2018) | BERT |
| 📄 论文 | **Language Models are Few-Shot Learners** (Brown 2020) | GPT-3 |
| 🎥 视频 | CS 224n Lecture 9-10 | 预训练模型讲解 |
| 📝 博客 | [The Illustrated BERT](https://jalammar.github.io/illustrated-bert/) — Jay Alammar | BERT 图解 |
| 📝 博客 | [The Illustrated GPT-2](https://jalammar.github.io/illustrated-gpt2/) — Jay Alammar | ⭐ GPT-2 图解，必看 |

### 延伸阅读
| 资源 | 说明 |
|------|------|
| [nanoGPT](https://github.com/karpathy/nanoGPT) 代码 | 对应 GPT-2 架构 |
| [Let's build GPT from scratch](https://www.youtube.com/watch?v=kCc8FmEb1nY) — Karpathy 视频 | ⭐ 2 小时从零实现 GPT |

---

## 模块 9：Scaling Laws（后续扩散）

### 目标
理解「模型越大越好」背后的规律。

### 学习资料

| 类型 | 资源 | 说明 |
|------|------|------|
| 📄 论文 | **Scaling Laws for Neural Language Models** (Kaplan 2020) | OpenAI 的 Scaling Laws |
| 📄 论文 | **Training Compute-Optimal Large Language Models** (Hoffmann 2022) | Chinchilla，修正 Scaling Laws |
| 📝 博客 | [Chinchilla's Wild Implications](https://www.lesswrong.com/posts/6Fpvch8RR29qLEWNH/chinchilla-s-wild-implications) — LessWrong | 通俗解读 |
| 🎥 视频 | CS 336 Lecture on Scaling Laws | Stanford 最新讲解 |

---

## 模块 10：推理优化（后续扩散）

### 目标
理解为什么 LLM 推理慢、怎么优化。

### 学习资料

| 类型 | 资源 | 说明 |
|------|------|------|
| 📄 论文 | **FlashAttention: Fast and Memory-Efficient Attention** (Dao 2022) | ⭐ |
| 📄 论文 | **FlashAttention-2** (Dao 2023) | 改进版 |
| 📄 论文 | **GPTQ: Accurate Post-Training Quantization** (Frantar 2022) | 量化 |
| 📝 博客 | [KV Cache Explained](https://medium.com/@joaolages/kv-cache-explained-2f7ed07c0c41) | KV Cache 图解 |
| 📝 博客 | [PagedAttention / vLLM](https://blog.vllm.ai/) | 推理引擎 |
| 💻 代码 | [llm.c](https://github.com/karpathy/llm.c) — Karpathy | C 语言实现 GPT-2 训练，理解底层 |

---

## 模块 11：对齐 — RLHF / DPO（后续扩散）

### 学习资料

| 类型 | 资源 | 说明 |
|------|------|------|
| 📄 论文 | **Training Language Models to Follow Instructions with Human Feedback** (Ouyang 2022) | InstructGPT / RLHF |
| 📄 论文 | **Direct Preference Optimization** (Rafailov 2023) | DPO，简化版 RLHF |
| 🎥 视频 | CS 224n Lecture on RLHF | Stanford 讲解 |
| 📝 博客 | [Illustrating RLHF](https://huggingface.co/blog/rlhf) — Hugging Face | ⭐ RLHF 图解 |

---

## 模块 12：Agent（后续扩散）

### 学习资料

| 类型 | 资源 | 说明 |
|------|------|------|
| 📄 论文 | **ReAct: Synergizing Reasoning and Acting** (Yao 2022) | Agent 核心框架 |
| 📄 论文 | **Toolformer** (Schick 2023) | 工具使用 |
| 📄 论文 | **Chain-of-Thought Prompting** (Wei 2022) | 推理链 |
| 📄 论文 | **Tree of Thoughts** (Yao 2023) | 树状推理 |
| 📝 博客 | [LLM Powered Autonomous Agents](https://lilianweng.github.io/posts/2023-06-23-agent/) — Lilian Weng | ⭐ Agent 综述 |
| 💻 代码 | LangChain / LlamaIndex | 实践框架 |

---

## 扩展延伸总览

```
Transformer 论文精读
├── 模块 1: 动机与背景 ──→ RNN/LSTM (可跳过)
├── 模块 2: Encoder-Decoder ──→ Illustrated Transformer
├── 模块 3: Attention ⭐ ──→ 手推公式 + 代码实现
├── 模块 4: 分词器 ──→ BPE + minBPE 实践
├── 模块 5: 位置编码 ──→ RoPE → ALiBi → 长上下文
├── 模块 6: FFN + 残差 ──→ SwiGLU + Pre/Post-Norm
├── 模块 7: 训练策略 ──→ AdamW + 学习率调度 + 分布式训练
├── 模块 8: GPT/BERT ──→ 预训练范式 → nanoGPT 实践
├── 模块 9: Scaling Laws ──→ Chinchilla → 模型设计决策
├── 模块 10: 推理优化 ──→ Flash Attention + KV Cache + 量化
├── 模块 11: 对齐 ──→ RLHF → DPO
└── 模块 12: Agent ──→ ReAct → 工具调用 → 规划
```

## 推荐阅读顺序（散点式，按兴趣跳转）

```
Day 1-3:  模块 2 + 3（Encoder-Decoder + Attention）← 核心
Day 4-5:  模块 4（分词器）← 输入层
Day 6-7:  模块 5（位置编码）← 与 Attention 紧密相关
Day 8-9:  模块 6 + 7（FFN + 训练）← 快速扫过
Day 10+:  模块 8（GPT/BERT）← 自然过渡到实践
Day 15+:  按兴趣扩散到模块 9-12
```
