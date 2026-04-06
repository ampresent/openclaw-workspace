# DECISIONS

## 2026-04-06 使用 llama-cpp-python 而非 Ollama
- 决策：采用 llama-cpp-python 作为推理引擎
- 原因：Ollama 安装脚本因 GnuTLS 问题下载失败
- 备选方案：Ollama、transformers + quantized model
- 为什么没选：Ollama 网络不通；transformers 内存开销更大
- 影响范围：后续所有模型加载都通过 llama_cpp.Llama 接口
- 后续需要观察什么：是否有 Ollama 的国内安装源

## 2026-04-06 使用 hf-mirror.com 下载模型
- 决策：使用 hf-mirror.com 替代 huggingface.co
- 原因：HuggingFace 主站在当前服务器不可达（被墙）
- 备选方案：ModelScope、手动 curl 下载
- 为什么没选：ModelScope 上 GGUF 格式模型较少；手动下载不可维护
- 影响范围：所有模型下载命令需加 `HF_ENDPOINT=https://hf-mirror.com`
- 后续需要观察什么：hf-mirror 长期稳定性

## 2026-04-06 选择 Q4_K_M 量化级别
- 决策：主力使用 Q4_K_M 量化
- 原因：质量与体积的最佳平衡点
- 备选方案：Q2_K（更小更快）、Q8_0（更好质量）
- 为什么没选：Q2_K 质量损失明显；Q8_0 在 3.4GB RAM 下空间紧张
- 影响范围：后续对比测试以 Q4_K_M 为基线
- 后续需要观察什么：中文场景下 Q4_K_M 是否有明显质量缺陷
