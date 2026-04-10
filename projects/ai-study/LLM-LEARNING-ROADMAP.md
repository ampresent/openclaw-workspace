# LLM 散点式学习地图

> 核心思路：每个节点是一个独立可切入的知识单元，节点之间的连线表示「如果卡住了，可能需要先了解 X」，而不是「必须先学完 X」。

## 知识图谱

```
                              ┌─────────────┐
                              │  🧮 数学工具  │
                              │ 线代·概率·优化 │
                              └──────┬──────┘
                                     │ (按需查阅)
                    ┌────────────────┼────────────────┐
                    ▼                ▼                ▼
             ┌──────────┐    ┌──────────┐    ┌──────────┐
             │ 反向传播  │    │ Attention │    │ 概率图模型│
             │ 与优化器  │    │  机制原理  │    │  (可选)   │
             └────┬─────┘    └────┬─────┘    └──────────┘
                  │               │
                  ▼               ▼
             ┌──────────┐   ┌──────────────────────────┐
             │  RNN/LSTM │   │     ⭐ Transformer        │
             │  (了解演进) │   │  可直接从这里开始！        │
             └──────────┘   └─────────┬───────────────┘
                                       │
                  ┌────────────┬───────┴───────┬────────────┐
                  ▼            ▼               ▼            ▼
           ┌──────────┐ ┌──────────┐  ┌──────────────┐ ┌──────────┐
           │  BERT    │ │   GPT    │  │  位置编码     │ │  分词器   │
           │ 双向编码  │ │ 自回归   │  │ RoPE/ALiBi  │ │  BPE    │
           └────┬─────┘ └────┬─────┘  └──────────────┘ └──────────┘
                │            │
                │            ▼
                │     ┌──────────────┐
                │     │  预训练 & SFT │
                │     │  Scaling Laws│
                │     └──────┬───────┘
                │            │
                │     ┌──────┴──────────────────────┐
                ▼     ▼                              ▼
         ┌──────────┐ ┌──────────────┐      ┌──────────────┐
         │  RAG     │ │  对齐         │      │  推理优化     │
         │ 检索增强  │ │ RLHF / DPO   │      │ KV Cache     │
         └──────────┘ └──────────────┘      │ Flash Attn   │
                                            │ 量化         │
                  ┌─────────────┐           └──────────────┘
                  │  🤖 Agent    │
                  │ ReAct/工具调用│ ◄──── 以上所有都可能是前置
                  │ 规划/记忆    │
                  └─────────────┘
```

## 散点入口（从哪里开始都行）

### 入口 A：从 Transformer 直接切入
- 精读 **Attention Is All You Need**，遇到不懂的数学再回头查
- 跟着 **The Annotated Transformer** 手写一遍
- 优势：直击核心，最快建立 LLM 全局视野

### 入口 B：从一个 LLM 实现切入
- 直接跑 **nanoGPT**（Karpathy），边跑边理解每个模块
- 或者跑 **LLaMA** 的开源实现（如 llama.cpp）
- 优势：动手优先，从代码反推理论

### 入口 C：从一个具体问题切入
- 「为什么 GPT 能生成连贯文本？」→ 追到自回归语言模型
- 「为什么长文本效果变差？」→ 追到位置编码 + 注意力机制
- 「为什么推理这么慢？」→ 追到 KV Cache + 量化
- 优势：带着问题学，目标感强

### 入口 D：从一篇论文切入
- 随便挑一篇感兴趣的核心论文（见下方清单）
- 读不懂的部分就是你需要补充的节点
- 优势：研究驱动，最接近真实科研

## 节点索引（按主题分组，非学习顺序）

### 🧮 数学工具箱（按需查阅，不用系统学）
| 节点 | 什么时候需要 | 推荐资源 |
|------|-------------|----------|
| 矩阵分解（SVD/特征值） | 理解 Attention、位置编码、PCA | 3Blue1Brown 线代系列 |
| 梯度与链式法则 | 理解反向传播 | 同上 |
| 贝叶斯定理 | 理解概率语言模型、DPO | MIT 6.041 |
| 交叉熵/KL 散度 | 理解损失函数 | 任何 ML 教材 |
| 凸优化基础 | 理解训练动态 | Boyd 的 Convex Optimization |
| 信息论基础 | 理解熵、困惑度（Perplexity） | Cover & Thomas |

### ⚡ 核心节点
| 节点 | 关键内容 | 直接资源 |
|------|----------|----------|
| **反向传播** | 链式法则、计算图、自动微分 | CS 231n Lecture 4 |
| **Attention 机制** | Q/K/V、Scaled Dot-Product、Multi-Head | Attention Is All You Need §3 |
| **Transformer 架构** | 完整架构、编码器/解码器 | 同上 + The Annotated Transformer |
| **分词器 (BPE)** | Byte Pair Encoding、词表构建 | CS 336 Lecture 1 + SentencePiece |
| **位置编码** | Sinusoidal → RoPE → ALiBi | Su et al. 2021 (RoPE 论文) |
| **BERT** | 双向编码、MLM + NSP、微调范式 | Devlin et al. 2018 |
| **GPT** | 自回归解码、Next Token Prediction | Radford et al. 2018/2019 |
| **预训练** | 无监督预训练 + 有监督微调 | CS 224n Lecture 8-10 |
| **Scaling Laws** | Loss ∝ N^(-α), D^(-β), C^(-γ) | Kaplan et al. 2020 |
| **RLHF/DPO** | 奖励模型 + PPO / 直接偏好优化 | Ouyang 2022, Rafailov 2023 |
| **KV Cache** | 推理时缓存 K/V 矩阵 | 任何 LLM 推理优化博客 |
| **Flash Attention** | IO-aware 注意力计算 | Dao et al. 2022 |
| **量化** | INT8/INT4、GGUF、GPTQ | 任何 LLM 量化教程 |
| **RAG** | 检索 + 生成 | Lewis et al. 2020 |
| **Agent** | ReAct、工具调用、规划 | Yao et al. 2022 |

### 🔬 扩展节点（可选）
| 节点 | 关键内容 |
|------|----------|
| MoE (Mixture of Experts) | Switch Transformer, Mixtral |
| Speculative Decoding | 小模型猜测 + 大模型验证 |
| Long Context | 长上下文技术（Ring Attention 等） |
| 多模态 | Vision-Language Models |
| Mamba / SSM | Transformer 之外的架构选择 |

## 推荐的散点学习策略

**每周做一件事**：挑一个你此刻最感兴趣的节点，深入搞清楚，然后自然扩散到相邻节点。

**记录你的探索路径**：每学完一个节点，记录你从哪个节点跳过来的、遇到了什么问题、又跳去了哪个节点。这个路径本身就是你独特的知识结构。

**定期回顾全局**：每两周看一次这个知识图谱，看看哪些节点已经覆盖了，哪些还是空白，按兴趣填补。

## 资源汇总

| 类型 | 资源 | 链接 |
|------|------|------|
| 论文精读 | Attention Is All You Need | https://arxiv.org/abs/1706.03762 |
| 代码实践 | nanoGPT | https://github.com/karpathy/nanoGPT |
| 代码实践 | llm.c | https://github.com/karpathy/llm.c |
| 课程 | CS 224n | https://web.stanford.edu/class/cs224n/ |
| 课程 | CS 336 | https://stanford-cs336.github.io |
| 课程 | MIT 6.S191 | http://introtodeeplearning.com |
| 课程 | CS 230 | https://www.coursera.org/specializations/deep-learning |
| 可视化 | The Illustrated Transformer | https://jalammar.github.io/illustrated-transformer/ |
| 可视化 | The Annotated Transformer | https://nlp.seas.harvard.edu/annotated-transformer/ |
