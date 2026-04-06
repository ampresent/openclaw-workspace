#!/usr/bin/env python3
"""
场景 1：HTTP API 服务
把 Gemma 3 1B 封装成 REST API，其他工具可以通过 HTTP 调用。
类似轻量版 Ollama。
"""
import json
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from llama_cpp import Llama

MODEL_PATH = "/opt/llm-models/gemma3-1b/gemma-3-1b-it-Q4_K_M.gguf"

print("正在加载模型...")
llm = Llama(model_path=MODEL_PATH, n_ctx=4096, n_threads=2, verbose=False)
print("模型加载完成，启动 API 服务...")


class LLMHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path == "/v1/chat/completions":
            content_len = int(self.headers.get("Content-Length", 0))
            body = json.loads(self.rfile.read(content_len))
            messages = body.get("messages", [])
            max_tokens = body.get("max_tokens", 256)
            temperature = body.get("temperature", 0.7)

            # 构建 prompt（简单 chat template）
            prompt = ""
            for msg in messages:
                role = msg["role"]
                content = msg["content"]
                if role == "system":
                    prompt += f"<start_of_turn>system\n{content}<end_of_turn>\n"
                elif role == "user":
                    prompt += f"<start_of_turn>user\n{content}<end_of_turn>\n"
                elif role == "assistant":
                    prompt += f"<start_of_turn>model\n{content}<end_of_turn>\n"
            prompt += "<start_of_turn>model\n"

            t0 = time.time()
            result = llm(prompt, max_tokens=max_tokens, temperature=temperature,
                         stop=["<end_of_turn>"])
            dt = time.time() - t0

            response = {
                "model": "gemma-3-1b-q4",
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": result["choices"][0]["text"].strip()
                    }
                }],
                "usage": result["usage"],
                "timing": {"total_seconds": round(dt, 2)}
            }

            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps(response, ensure_ascii=False).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def do_GET(self):
        if self.path == "/health":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"status": "ok", "model": "gemma-3-1b-q4"}).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format, *args):
        print(f"[API] {args[0]}")


if __name__ == "__main__":
    server = HTTPServer(("0.0.0.0", 8091), LLMHandler)
    print("服务已启动: http://0.0.0.0:8091")
    print("  POST /v1/chat/completions  — 聊天补全")
    print("  GET  /health              — 健康检查")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n服务已停止")
