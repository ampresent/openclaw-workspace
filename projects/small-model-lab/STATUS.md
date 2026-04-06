# STATUS

## 当前目标
- 验证 Gemma 3 1B 在纯 CPU 环境下的部署与推理能力

## 当前判断
- 1B 模型可以在 3.4GB RAM + 2 核 CPU 上正常运行，速度约 15-20 tok/s
- Q4_K_M 量化是目前性价比最高的选择（769MB，质量可接受）
- 中文能力尚可，但存在明显幻觉问题
- 不适合用于 agent 工作流（工具调用、多步推理能力不足）

## 当前进展
### 已完成
- [x] 安装 llama-cpp-python (v0.3.20)
- [x] 通过 hf-mirror 下载 Gemma 3 1B Q4_K_M (769MB)
- [x] 首次推理验证成功（中文问答）
- [x] 项目结构搭建

### 进行中
- 待补充更多模型的对比测试

## 卡点 / 风险
- HuggingFace 主站被墙，需使用 hf-mirror.com
- Ollama 安装脚本下载失败（GnuTLS 问题），暂用 llama-cpp-python 替代
- 内存紧张，最多同时加载一个 Q4 模型

## 下一步
1. 测试 Gemma 3 1B 英文推理能力
2. 下载测试 Qwen2.5 1.5B 作为对比基线
3. 整理初始化脚本（一键部署）

## 如果明天继续
- 先读这些文件：`STATUS.md`、`TODO.md`
- 先看这些目录：`/opt/llm-models/gemma3-1b/`
- 第一件事做什么：运行 `/opt/llm-models/gemma3-1b/gemma-3-1b-it-Q4_K_M.gguf` 测试

## 关键上下文
- 关键文件：`/opt/llm-models/gemma3-1b/gemma-3-1b-it-Q4_K_M.gguf`
- 关键命令：`python3` + `llama_cpp.Llama`
- 关键输出：首次推理成功，~15-20 tok/s

## 最近更新时间
- 2026-04-06 18:18 by Agent
