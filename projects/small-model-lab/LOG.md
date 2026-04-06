# LOG

## 2026-04-06

### 做了什么
- 安装 llama-cpp-python (v0.3.20, pip --break-system-packages)
- 安装 huggingface-hub (v1.9.0)
- 通过 hf-mirror.com 下载 unsloth/gemma-3-1b-it-GGUF (Q4_K_M)
- 首次推理测试：中文自我介绍 prompt

### 发现了什么
- HuggingFace 主站不可达，hf-mirror.com 可用
- Google 官方 Gemma GGUF 需要 gated access，unsloth 版本无限制
- Ollama 安装脚本下载失败（GnuTLS recv error -110），可能与 GitHub TLS 有关
- Gemma 3 1B Q4_K_M 加载约 3 秒，推理约 15-20 tok/s
- 中文回答有幻觉倾向（无中生有"李明"的身份）
- llama_cpp 提示 n_ctx 2048 < n_ctx_train 32768，建议调大

### 改了什么
- 系统新增依赖：llama-cpp-python、huggingface-hub、numpy、diskcache

### 结论
- 1B Q4 模型可以在当前硬件上流畅运行
- 质量适合体验和技术验证，不适合生产使用

### 下一步
- 编写一键初始化脚本
- 测试更多模型和量化级别
