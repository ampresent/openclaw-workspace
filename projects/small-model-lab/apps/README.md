# Apps — 小模型实际应用场景

三个独立的应用场景，均基于 Gemma 3 1B Q4_K_M。

## 01. HTTP API 服务

把模型封装成 REST API，其他工具可通过 HTTP 调用。

```bash
python3 apps/01-http-api.py
# 服务启动在 http://0.0.0.0:8091

# 调用示例
curl -X POST http://localhost:8091/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"messages": [{"role": "user", "content": "你好"}]}'
```

## 02. RAG 文档问答

把本地文档切片，用关键词匹配相关内容，让模型基于文档回答问题。

```bash
# 单次问答
python3 apps/02-rag-demo.py <文档路径> "你的问题"

# 交互模式
python3 apps/02-rag-demo.py <文档路径>
```

## 03. 多轮对话

支持 system prompt、历史记忆、预设场景。

```bash
# 自由对话
python3 apps/03-multi-turn-chat.py

# 预设场景（翻译官、代码审查、情绪日记等）
python3 apps/03-multi-turn-chat.py --scenario
```

## 依赖

```bash
pip3 install --break-system-packages llama-cpp-python huggingface-hub
```
