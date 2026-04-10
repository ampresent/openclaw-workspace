# 模型蒸馏实现计划

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 通过知识蒸馏训练一个小型、低延迟的对话模型，可在纯 CPU 环境下实时推理。

**Architecture:** 使用 Qwen-7B-Chat 作为教师模型，TinyLlama-1.1B 作为学生模型，通过 TRL 框架进行知识蒸馏训练，最终导出 GGUF 格式用于 llama.cpp CPU 推理。

**Tech Stack:** Python 3.10+, PyTorch, HuggingFace Transformers, TRL, llama.cpp, GGUF

---

## 文件结构

### 创建的文件

| 文件 | 职责 |
|------|------|
| `projects/model-distillation/requirements.txt` | Python 依赖 |
| `projects/model-distillation/config.py` | 配置（超参数、路径） |
| `projects/model-distillation/data/prepare_data.py` | 数据下载与预处理 |
| `projects/model-distillation/data/generate_teacher_logits.py` | 教师模型推理生成软标签 |
| `projects/model-distillation/train/distill_trainer.py` | 蒸馏训练器 |
| `projects/model-distillation/train/train.py` | 训练主脚本 |
| `projects/model-distillation/export/export_to_gguf.py` | 模型导出脚本 |
| `projects/model-distillation/infer/infer.py` | CPU 推理测试脚本 |
| `projects/model-distillation/tests/test_data_loader.py` | 数据加载测试 |
| `projects/model-distillation/tests/test_trainer.py` | 训练器测试 |
| `projects/model-distillation/logs/` | 训练日志目录 |
| `projects/model-distillation/checkpoints/` | 模型检查点目录 |
| `projects/model-distillation/models/` | 模型输出目录 |

---

## Task 1: 项目初始化与环境准备

**Files:**
- Create: `projects/model-distillation/requirements.txt`
- Create: `projects/model-distillation/config.py`

- [ ] **Step 1: 创建项目目录**

```bash
mkdir -p projects/model-distillation/{data,train,export,infer,tests,logs,checkpoints,models}
cd projects/model-distillation
```

- [ ] **Step 2: 创建 requirements.txt**

```txt
# Core
torch>=2.0.0
transformers>=4.35.0
datasets>=2.14.0
accelerate>=0.24.0

# Distillation
trl>=0.7.0

# Data processing
pandas>=2.0.0
pyarrow>=14.0.0

# GGUF export
llama-cpp-python>=0.2.0

# Utilities
tqdm>=4.66.0
wandb>=0.16.0
pytest>=7.4.0
```

- [ ] **Step 3: 创建 config.py**

```python
from dataclasses import dataclass
from pathlib import Path

@dataclass
class ModelConfig:
    # 教师模型
    teacher_name: str = "Qwen/Qwen-7B-Chat"
    teacher_bits: int = 4  # 4bit 量化
    
    # 学生模型
    student_name: str = "TinyLlama/TinyLlama-1.1B-Chat-v1.0"
    student_hidden_size: int = 2048
    student_num_layers: int = 22
    student_num_heads: int = 32
    
    # 蒸馏参数
    temperature: float = 2.0
    alpha: float = 0.5  # distillation_loss 权重

@dataclass
class TrainConfig:
    batch_size: int = 4  # 小 batch 适配 7GB 内存
    gradient_accumulation_steps: int = 4
    learning_rate: float = 2e-5
    num_epochs: int = 3
    max_seq_length: int = 512
    gradient_checkpointing: bool = True
    warmup_ratio: float = 0.1
    
@dataclass
class PathConfig:
    base_dir: Path = Path(__file__).parent
    data_dir: Path = base_dir / "data"
    output_dir: Path = base_dir / "models"
    checkpoint_dir: Path = base_dir / "checkpoints"
    log_dir: Path = base_dir / "logs"

config = ModelConfig()
train_config = TrainConfig()
paths = PathConfig()
```

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "feat: 初始化项目结构和配置"
```

---

## Task 2: 数据下载与预处理

**Files:**
- Create: `projects/model-distillation/data/prepare_data.py`
- Create: `projects/model-distillation/tests/test_data_loader.py`

- [ ] **Step 1: 编写数据预处理脚本**

```python
# data/prepare_data.py
import argparse
from datasets import load_dataset
from pathlib import Path

def load_oasst1():
    """加载 OpenAssistant 对话数据集"""
    dataset = load_dataset("OpenAssistant/oasst1")
    return dataset

def format_conversation(example):
    """将对话格式化为 (prompt, response) 对"""
    messages = example["messages"]
    
    # 提取最后一轮问答
    prompts = []
    responses = []
    
    for i, msg in enumerate(messages):
        if msg["role"] == "prompter":
            prompt = msg["content"]
            # 找下一个 assistant 回复
            if i + 1 < len(messages) and messages[i + 1]["role"] == "assistant":
                response = messages[i + 1]["content"]
                prompts.append(prompt)
                responses.append(response)
    
    return {"prompt": prompts, "response": responses}

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output_dir", type=str, default="data/")
    parser.add_argument("--max_samples", type=int, default=10000)
    args = parser.parse_args()
    
    print("加载 OASST1 数据集...")
    dataset = load_oasst1()
    
    # 只使用训练集
    train = dataset["train"]
    
    # 格式化
    formatted = train.map(format_conversation, batched=True, remove_columns=train.column_names)
    
    # 过滤空值
    formatted = formatted.filter(lambda x: len(x["prompt"]) > 0 and len(x["response"]) > 0)
    
    # 限制样本数
    if args.max_samples:
        formatted = formatted.select(range(min(args.max_samples, len(formatted))))
    
    # 保存到 parquet
    output_path = Path(args.output_dir) / "oasst1_formatted.parquet"
    formatted.to_parquet(output_path)
    
    print(f"保存 {len(formatted)} 条样本到 {output_path}")

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: 编写数据加载测试**

```python
# tests/test_data_loader.py
import pytest
import pandas as pd
from pathlib import Path

def test_parquet_file_exists():
    """测试预处理后的数据文件存在"""
    data_path = Path("data/oasst1_formatted.parquet")
    assert data_path.exists(), f"数据文件不存在：{data_path}"

def test_parquet_columns():
    """测试数据格式正确"""
    data_path = Path("data/oasst1_formatted.parquet")
    df = pd.read_parquet(data_path)
    
    assert "prompt" in df.columns, "缺少 prompt 列"
    assert "response" in df.columns, "缺少 response 列"
    assert len(df) > 0, "数据为空"

def test_sample_content():
    """测试样本内容有效性"""
    data_path = Path("data/oasst1_formatted.parquet")
    df = pd.read_parquet(data_path)
    
    # 检查前 10 条
    for idx in range(min(10, len(df))):
        prompt = df.iloc[idx]["prompt"]
        response = df.iloc[idx]["response"]
        
        assert isinstance(prompt, str) and len(prompt) > 0
        assert isinstance(response, str) and len(response) > 0
```

- [ ] **Step 3: 运行数据预处理**

```bash
cd projects/model-distillation
pip install -r requirements.txt
python data/prepare_data.py --max_samples 5000
```

Expected: 生成 `data/oasst1_formatted.parquet` 文件

- [ ] **Step 4: 运行测试**

```bash
pytest tests/test_data_loader.py -v
```

Expected: 所有测试通过

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat: 数据下载与预处理"
```

---

## Task 3: 教师模型推理生成软标签

**Files:**
- Create: `projects/model-distillation/data/generate_teacher_logits.py`

- [ ] **Step 1: 编写教师模型推理脚本**

```python
# data/generate_teacher_logits.py
import torch
import pandas as pd
import argparse
from pathlib import Path
from transformers import AutoTokenizer, AutoModelForCausalLM
from tqdm import tqdm

def load_teacher_model(model_name, bits=4):
    """加载量化的教师模型"""
    from transformers import BitsAndBytesConfig
    
    # 4bit 量化配置（降低内存占用）
    bnb_config = BitsAndBytesConfig(
        load_in_4bit=(bits == 4),
        bnb_4bit_use_double_quant=True,
        bnb_4bit_quant_type="nf4",
        bnb_4bit_compute_dtype=torch.float16
    )
    
    tokenizer = AutoTokenizer.from_pretrained(model_name, trust_remote_code=True)
    model = AutoModelForCausalLM.from_pretrained(
        model_name,
        quantization_config=bnb_config if bits == 4 else None,
        device_map="auto",
        trust_remote_code=True,
        torch_dtype=torch.float16
    )
    
    return model, tokenizer

def generate_logits(model, tokenizer, prompts, batch_size=4):
    """为所有提示生成教师模型的 logits"""
    all_logits = []
    all_input_ids = []
    
    for i in tqdm(range(0, len(prompts), batch_size), desc="生成教师 logits"):
        batch_prompts = prompts[i:i + batch_size]
        
        #  tokenize
        inputs = tokenizer(
            batch_prompts,
            return_tensors="pt",
            padding=True,
            truncation=True,
            max_length=512
        ).to(model.device)
        
        # 前向传播获取 logits
        with torch.no_grad():
            outputs = model(**inputs)
            logits = outputs.logits.cpu()
        
        all_logits.append(logits)
        all_input_ids.append(inputs["input_ids"].cpu())
    
    return torch.cat(all_logits, dim=0), torch.cat(all_input_ids, dim=0)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--teacher", type=str, default="Qwen/Qwen-7B-Chat")
    parser.add_argument("--data_path", type=str, default="data/oasst1_formatted.parquet")
    parser.add_argument("--output_path", type=str, default="data/teacher_logits.pt")
    parser.add_argument("--batch_size", type=int, default=4)
    parser.add_argument("--max_samples", type=int, default=1000)
    args = parser.parse_args()
    
    # 加载数据
    print(f"加载数据：{args.data_path}")
    df = pd.read_parquet(args.data_path)
    prompts = df["prompt"].tolist()[:args.max_samples]
    
    # 加载模型
    print(f"加载教师模型：{args.teacher}")
    model, tokenizer = load_teacher_model(args.teacher)
    
    # 生成 logits
    print("生成教师 logits...")
    logits, input_ids = generate_logits(model, tokenizer, prompts, args.batch_size)
    
    # 保存
    print(f"保存到：{args.output_path}")
    torch.save({"logits": logits, "input_ids": input_ids}, args.output_path)
    
    print(f"完成！生成 {len(logits)} 条 logits")

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: 运行教师模型推理**

```bash
cd projects/model-distillation
python data/generate_teacher_logits.py --max_samples 1000
```

Expected: 生成 `data/teacher_logits.pt` 文件（约 2-4GB）

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "feat: 教师模型推理生成软标签"
```

---

## Task 4: 蒸馏训练器实现

**Files:**
- Create: `projects/model-distillation/train/distill_trainer.py`

- [ ] **Step 1: 编写蒸馏训练器**

```python
# train/distill_trainer.py
import torch
import torch.nn as nn
import torch.nn.functional as F
from transformers import Trainer
from typing import Dict, Optional, Tuple

class DistillationTrainer(Trainer):
    """知识蒸馏训练器"""
    
    def __init__(
        self,
        teacher_logits: torch.Tensor,
        temperature: float = 2.0,
        alpha: float = 0.5,
        **kwargs
    ):
        super().__init__(**kwargs)
        self.teacher_logits = teacher_logits
        self.temperature = temperature
        self.alpha = alpha
        
    def compute_loss(
        self,
        model: nn.Module,
        inputs: Dict[str, torch.Tensor],
        return_outputs: bool = False
    ) -> Tuple[torch.Tensor, Optional[Dict]]:
        labels = inputs.pop("labels")
        batch_indices = inputs.pop("batch_indices")
        
        # 学生模型前向传播
        outputs = model(**inputs)
        student_logits = outputs.logits
        
        # 获取对应的教师 logits
        teacher_logits = self.teacher_logits[batch_indices].to(student_logits.device)
        
        # 截断到相同长度
        min_len = min(student_logits.shape[1], teacher_logits.shape[1])
        student_logits = student_logits[:, :min_len, :]
        teacher_logits = teacher_logits[:, :min_len, :]
        
        # 蒸馏损失 (KL 散度)
        distillation_loss = F.kl_div(
            F.log_softmax(student_logits / self.temperature, dim=-1),
            F.softmax(teacher_logits / self.temperature, dim=-1),
            reduction="batchmean"
        ) * (self.temperature ** 2)
        
        # 任务损失 (交叉熵)
        task_loss = F.cross_entropy(
            student_logits.view(-1, student_logits.shape[-1]),
            labels.view(-1),
            ignore_index=-100
        )
        
        # 总损失
        total_loss = self.alpha * distillation_loss + (1 - self.alpha) * task_loss
        
        return (total_loss, outputs) if return_outputs else total_loss
```

- [ ] **Step 2: 提交**

```bash
git add train/distill_trainer.py
git commit -m "feat: 实现知识蒸馏训练器"
```

---

## Task 5: 训练主脚本

**Files:**
- Create: `projects/model-distillation/train/train.py`

- [ ] **Step 1: 编写训练主脚本**

```python
# train/train.py
import torch
import pandas as pd
import argparse
from pathlib import Path
from transformers import (
    AutoTokenizer,
    AutoModelForCausalLM,
    TrainingArguments,
    DataCollatorForLanguageModeling
)
from datasets import Dataset
from distill_trainer import DistillationTrainer
from config import config, train_config, paths

def load_data():
    """加载预处理数据"""
    df = pd.read_parquet(paths.data_dir / "oasst1_formatted.parquet")
    return Dataset.from_pandas(df)

def tokenize_function(examples, tokenizer, max_length):
    """tokenize 函数"""
    return tokenizer(
        examples["prompt"],
        examples["response"],
        truncation=True,
        max_length=max_length,
        padding="max_length"
    )

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--teacher_logits", type=str, default="data/teacher_logits.pt")
    parser.add_argument("--output_dir", type=str, default="models/student-distilled")
    args = parser.parse_args()
    
    # 加载教师 logits
    print("加载教师 logits...")
    teacher_data = torch.load(args.teacher_logits)
    teacher_logits = teacher_data["logits"]
    
    # 加载学生模型
    print(f"加载学生模型：{config.student_name}")
    tokenizer = AutoTokenizer.from_pretrained(config.student_name)
    student_model = AutoModelForCausalLM.from_pretrained(
        config.student_name,
        torch_dtype=torch.float16
    )
    
    # 启用梯度检查点（降低内存）
    if train_config.gradient_checkpointing:
        student_model.gradient_checkpointing_enable()
    
    # 加载数据
    print("加载训练数据...")
    dataset = load_data()
    
    # tokenize
    tokenized = dataset.map(
        lambda x: tokenize_function(x, tokenizer, train_config.max_seq_length),
        batched=True
    )
    
    # 添加 batch 索引（用于获取对应的教师 logits）
    tokenized = tokenized.add_column("batch_indices", list(range(len(tokenized))))
    tokenized = tokenized.add_column("labels", tokenized["input_ids"])
    
    # 训练参数
    training_args = TrainingArguments(
        output_dir=args.output_dir,
        per_device_train_batch_size=train_config.batch_size,
        gradient_accumulation_steps=train_config.gradient_accumulation_steps,
        learning_rate=train_config.learning_rate,
        num_train_epochs=train_config.num_epochs,
        warmup_ratio=train_config.warmup_ratio,
        logging_dir=paths.log_dir,
        logging_steps=10,
        save_steps=500,
        save_total_limit=3,
        fp16=True,
        gradient_checkpointing=train_config.gradient_checkpointing,
    )
    
    # 数据 collator
    data_collator = DataCollatorForLanguageModeling(
        tokenizer=tokenizer,
        mlm=False
    )
    
    # 创建训练器
    trainer = DistillationTrainer(
        model=student_model,
        args=training_args,
        train_dataset=tokenized,
        data_collator=data_collator,
        teacher_logits=teacher_logits,
        temperature=config.temperature,
        alpha=config.alpha,
    )
    
    # 开始训练
    print("开始训练...")
    trainer.train()
    
    # 保存模型
    print(f"保存模型到：{args.output_dir}")
    trainer.save_model(args.output_dir)
    tokenizer.save_pretrained(args.output_dir)
    
    print("训练完成！")

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: 运行训练（小规模测试）**

```bash
cd projects/model-distillation
python train/train.py --teacher_logits data/teacher_logits.pt --output_dir models/test-run
```

Expected: 训练开始，看到 loss 下降

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "feat: 训练主脚本"
```

---

## Task 6: 模型导出为 GGUF 格式

**Files:**
- Create: `projects/model-distillation/export/export_to_gguf.py`

- [ ] **Step 1: 编写导出脚本**

```python
# export/export_to_gguf.py
import argparse
import subprocess
from pathlib import Path

def export_to_gguf(model_path, output_path, quantization="Q4_K_M"):
    """将模型导出为 GGUF 格式"""
    
    # 使用 llama.cpp 的 convert 脚本
    convert_script = Path("llama.cpp/convert-hf-to-gguf.py")
    
    if not convert_script.exists():
        print("克隆 llama.cpp...")
        subprocess.run(["git", "clone", "https://github.com/ggerganov/llama.cpp"])
        convert_script = Path("llama.cpp/convert-hf-to-gguf.py")
    
    # 转换为 GGUF
    print(f"转换为 GGUF 格式：{quantization}")
    subprocess.run([
        "python", str(convert_script),
        str(model_path),
        "--outfile", str(output_path),
        "--outtype", "f16"
    ])
    
    # 量化
    quantized_path = str(output_path).replace(".gguf", f"-{quantization}.gguf")
    print(f"量化：{quantization}")
    subprocess.run([
        "./llama.cpp/quantize",
        str(output_path),
        quantized_path,
        quantization
    ])
    
    print(f"导出完成：{quantized_path}")
    return quantized_path

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model_path", type=str, default="models/student-distilled")
    parser.add_argument("--output", type=str, default="models/student-Q4_K_M.gguf")
    parser.add_argument("--quantization", type=str, default="Q4_K_M")
    args = parser.parse_args()
    
    export_to_gguf(args.model_path, args.output, args.quantization)

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: 运行导出**

```bash
cd projects/model-distillation
python export/export_to_gguf.py --model_path models/student-distilled
```

Expected: 生成 `models/student-Q4_K_M.gguf` 文件（约 600MB-1GB）

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "feat: GGUF 导出脚本"
```

---

## Task 7: CPU 推理测试

**Files:**
- Create: `projects/model-distillation/infer/infer.py`

- [ ] **Step 1: 编写推理脚本**

```python
# infer/infer.py
import argparse
import time
from llama_cpp import Llama

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", type=str, default="models/student-Q4_K_M.gguf")
    parser.add_argument("--prompt", type=str, default="你好，请介绍一下自己。")
    parser.add_argument("--max_tokens", type=int, default=256)
    parser.add_argument("--temperature", type=float, default=0.7)
    args = parser.parse_args()
    
    # 加载模型
    print(f"加载模型：{args.model}")
    llm = Llama(
        model_path=args.model,
        n_ctx=512,
        n_threads=4,  # 使用 4 个 CPU 线程
        verbose=False
    )
    
    # 推理
    print(f"提示词：{args.prompt}")
    start = time.time()
    
    output = llm(
        args.prompt,
        max_tokens=args.max_tokens,
        temperature=args.temperature,
        stop=["\n\n", "用户："]
    )
    
    elapsed = time.time() - start
    response = output["choices"][0]["text"]
    
    print(f"\n回复：{response}")
    print(f"\n推理时间：{elapsed:.2f}秒")
    print(f"生成速度：{len(response) / elapsed:.1f} 字/秒")

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: 运行推理测试**

```bash
cd projects/model-distillation
pip install llama-cpp-python
python infer/infer.py --model models/student-Q4_K_M.gguf
```

Expected: 看到模型回复，推理时间 1-3 秒

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "feat: CPU 推理测试脚本"
```

---

## Task 8: 长期运行配置（systemd 服务）

**Files:**
- Create: `projects/model-distillation/systemd/distillation.service`

- [ ] **Step 1: 创建 systemd 服务配置**

```ini
# systemd/distillation.service
[Unit]
Description=Model Distillation Training Service
After=network.target

[Service]
Type=oneshot
WorkingDirectory=/home/admin/openclaw/workspace/projects/model-distillation
Environment=PATH=/home/admin/.nvm/versions/node/v24.14.0/bin:/usr/local/bin:/usr/bin:/bin
ExecStart=/home/admin/.nvm/versions/node/v24.14.0/bin/python train/train.py
RemainAfterExit=yes

# 资源限制
CPUQuota=100%
MemoryLimit=6G

# 日志
StandardOutput=journal
StandardError=journal
SyslogIdentifier=model-distillation

[Install]
WantedBy=multi-user.target
```

- [ ] **Step 2: 安装服务（可选）**

```bash
sudo cp systemd/distillation.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable model-distillation
```

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "feat: systemd 服务配置"
```

---

## 完成检查

- [ ] 所有任务完成
- [ ] 所有测试通过
- [ ] 模型可正常推理
- [ ] 文档完整

---

## 预计总时间

| 阶段 | 时间 |
|------|------|
| 环境准备 | 30 分钟 |
| 数据预处理 | 1 小时 |
| 教师模型推理 | 2-4 小时 |
| 蒸馏训练 | 12-48 小时 |
| 导出与测试 | 1 小时 |
| **总计** | **16-55 小时** |
