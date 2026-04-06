# REFERENCES

## 关键文件
- 模型文件：`/opt/llm-models/gemma3-1b/gemma-3-1b-it-Q4_K_M.gguf` (768.7 MB)
- 推理代码：见下方「关键命令」

## 关键链接
- unsloth Gemma 3 GGUF：https://hf-mirror.com/unsloth/gemma-3-1b-it-GGUF
- llama-cpp-python 文档：https://llama-cpp-python.readthedocs.io/
- Gemma 3 HuggingFace 镜像搜索：`HF_ENDPOINT=https://hf-mirror.com` + `list_models(search='gemma-3-1b gguf')`

## 关键命令

### 环境安装
```bash
pip3 install --break-system-packages llama-cpp-python huggingface-hub
```

### 下载模型（通过 hf-mirror）
```bash
HF_ENDPOINT=https://hf-mirror.com python3 -c "
from huggingface_hub import hf_hub_download
path = hf_hub_download(
    repo_id='unsloth/gemma-3-1b-it-GGUF',
    filename='gemma-3-1b-it-Q4_K_M.gguf',
    local_dir='/opt/llm-models/gemma3-1b'
)
print(f'Downloaded: {path}')
"
```

### 推理测试
```bash
python3 -c "
from llama_cpp import Llama
llm = Llama(
    model_path='/opt/llm-models/gemma3-1b/gemma-3-1b-it-Q4_K_M.gguf',
    n_ctx=4096, n_threads=2, verbose=False
)
out = llm('你好，请用中文介绍一下你自己。', max_tokens=256, temperature=0.7, stop=['<end_of_turn>'])
print(out['choices'][0]['text'])
"
```

## 关键输出 / 数据位置
- 模型存储：`/opt/llm-models/`
- 首次推理结果：约 256 tokens，幻觉明显（编造"李明"身份）

## 备注
- 所有 HuggingFace 下载必须加 `HF_ENDPOINT=https://hf-mirror.com`
- Google 官方 Gemma GGUF 有 gated access 限制，优先用 unsloth 社区版
- Ollama 安装目前不可用，等网络问题修复后可重试
