# REFERENCES — AI 技术学习资源

## 经典 AI 课程体系

### 第一层：基础课（无需先修 AI 知识）

| 课程 | 学校 | 主题 | 公开资料 |
|------|------|------|----------|
| CS 61A | UC Berkeley | 计算机程序的构造与解释 | https://cs61a.org |
| CS 229 | Stanford | 机器学习（Andrew Ng） | https://cs229.stanford.edu |
| CS 229a | Stanford/DeepLearning.AI | 机器学习入门（Coursera 版） | https://www.coursera.org/specializations/machine-learning-introduction |
| MIT 6.036 | MIT | 机器学习导论 | https://openlearninglibrary.mit.edu |
| CMU 10-601 | CMU | 机器学习入门 | https://www.cs.cmu.edu/~nisar/courses/601sp15/ |
| MIT 18.06 | MIT | 线性代数（Gilbert Strang） | https://ocw.mit.edu/courses/18-06-linear-algebra-spring-2010/ |

### 第二层：方向进阶课

| 课程 | 学校 | 主题 | 先修要求 | 公开资料 |
|------|------|------|----------|----------|
| CS 221 | Stanford | AI 原理与技术 | 概率论、算法、线性代数 | https://web.stanford.edu/class/cs221/ |
| CS 231n | Stanford | 计算机视觉与 CNN | Python、线性代数、概率论、CS 229 或同等 ML 基础 | https://cs231n.stanford.edu |
| CS 224n | Stanford | NLP 与深度学习 | Python、概率论、线性代数、CS 229 或同等 ML 基础 | https://web.stanford.edu/class/cs224n/ |
| CS 230 | Stanford | 深度学习 | Python、线性代数、概率论、CS 229 或同等 ML 基础 | https://www.coursera.org/specializations/deep-learning |
| MIT 6.S191 | MIT | 深度学习导论 | Python、线性代数 | http://introtodeeplearning.com |
| CMU 11-785 | CM | 深度学习导论 | Python、线性代数、概率论、ML 基础 | https://deeplearning.cs.cmu.edu |
| CMU 10-701 | CMU | 机器学习（PhD 级） | 概率论、统计、线性代数、优化 | https://www.cs.cmu.edu/~epxing/Class/10701/ |
| MIT 6.034 | MIT | 人工智能 | 编程基础 | https://ocw.mit.edu/courses/6-034-artificial-intelligence-fall-2010/ |

### 第三层：高级专题课

| 课程 | 学校 | 主题 | 先修要求 | 公开资料 |
|------|------|------|----------|----------|
| CS 336 | Stanford | Language Modeling from Scratch | CS 224n、深度学习基础 | https://stanford-cs336.github.io |
| CS 234 | Stanford | 强化学习 | CS 229、概率论 | https://web.stanford.edu/class/cs234/ |
| CS 228 | Stanford | 概率图模型 | CS 229、概率论 | https://ermongroup.github.io/cs228-notes/ |
| CS 237ab | Stanford | 概率论与随机方法 | 微积分 | https://stats385.github.io |
| CS 229T | Stanford | 统计学习理论 | CS 229、统计学 | - |
| CS 236 | Stanford | 深度生成模型 | CS 231n 或 CS 224n、概率论 | - |
| CS 285 | UC Berkeley | 深度强化学习 | CS 229 或 CS 189、强化学习基础 | https://rail.eecs.berkeley.edu/deeprlcourse/ |
| CMU 11-777 | CMU | 多模态机器学习 | CS 224n 或同等 NLP、CV 基础 | https://cmu-multicourse.github.io |
| MIT 6.S094 | MIT | 深度学习与自动驾驶 | 深度学习基础 | http://deeplearning.mit.edu |

## 先修关系图

```
                    ┌─────────────────────────────────┐
                    │         基础数学 & 编程          │
                    │  线性代数 | 概率论 | Python | 算法 │
                    └──────────┬──────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
       ┌──────────┐    ┌──────────┐    ┌──────────┐
       │ CS 229   │    │ MIT 6.034│    │ CS 61A   │
       │ 机器学习  │    │    AI    │    │  编程基础 │
       └────┬─────┘    └────┬─────┘    └──────────┘
            │               │
     ┌──────┼──────┐        │
     ▼      ▼      ▼        ▼
┌───────┐┌───────┐┌───────┐┌──────────┐
│CS231n ││CS224n ││CS 230 ││ CS 221   │
│  CV   ││ NLP   ││DeepLrn││ AI原理   │
└───┬───┘└───┬───┘└───────┘└──────────┘
    │        │
    ▼        ▼
┌───────┐┌───────┐┌───────┐┌───────┐
│CS 236 ││CS 336 ││CS 234 ││CMU    │
│生成模型││LM from││强化学习││11-785 │
└───────┘│Scratch│└───────┘└───────┘
         └───────┘
```

## 推荐学习路径

### 路径 A：偏 CV 方向
MIT 18.06 线性代数 → CS 229 机器学习 → CS 231n 计算机视觉 → CS 236 深度生成模型

### 路径 B：偏 NLP / LLM 方向
MIT 18.06 线性代数 → CS 229 机器学习 → CS 224n NLP → CS 336 Language Modeling from Scratch

### 路径 C：偏 Agent / RL 方向
MIT 18.06 线性代数 → CS 229 机器学习 → CS 221 AI 原理 → CS 234 强化学习 → CS 285 深度强化学习

### 路径 D：偏理论方向
CS 229 机器学习 → CMU 10-701 PhD 级 ML → CS 229T 统计学习理论

## 论文
- Attention Is All You Need (2017) — https://arxiv.org/abs/1706.03762

## 待补充
- 书籍、课程、博客等资源逐步添加
