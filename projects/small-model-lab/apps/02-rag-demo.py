#!/usr/bin/env python3
"""
场景 2：本地 RAG（检索增强生成）
把本地文档切片 → 用简单 TF-IDF 匹配相关段落 → 注入 prompt 让模型基于文档回答。
适用于：项目文档问答、知识库查询。
"""
import os
import sys
import re
import json
from llama_cpp import Llama

MODEL_PATH = "/opt/llm-models/gemma3-1b/gemma-3-1b-it-Q4_K_M.gguf"
CHUNK_SIZE = 300  # 每段大约 300 字符
CHUNK_OVERLAP = 50


def chunk_text(text, chunk_size=CHUNK_SIZE, overlap=CHUNK_OVERLAP):
    """按段落和句子切分文本"""
    paragraphs = text.split("\n\n")
    chunks = []
    current = ""
    for para in paragraphs:
        para = para.strip()
        if not para:
            continue
        if len(current) + len(para) > chunk_size and current:
            chunks.append(current.strip())
            current = current[-overlap:] + "\n" + para
        else:
            current += "\n" + para if current else para
    if current.strip():
        chunks.append(current.strip())
    return chunks


def score_chunk(query, chunk):
    """简单的关键词重叠打分"""
    query_words = set(re.findall(r'[\u4e00-\u9fff]+|[a-zA-Z]+', query.lower()))
    chunk_words = set(re.findall(r'[\u4e00-\u9fff]+|[a-zA-Z]+', chunk.lower()))
    if not query_words:
        return 0
    overlap = query_words & chunk_words
    return len(overlap) / len(query_words)


def build_rag_prompt(query, chunks, top_k=3):
    """构建 RAG prompt"""
    scored = [(score_chunk(query, c), c) for c in chunks]
    scored.sort(key=lambda x: -x[0])
    relevant = [c for s, c in scored[:top_k] if s > 0]

    if not relevant:
        return None

    context = "\n---\n".join(relevant)
    prompt = f"""根据以下参考资料回答用户问题。如果参考资料中没有相关信息，请说明。

参考资料：
{context}

用户问题：{query}

回答："""
    return prompt


def main():
    if len(sys.argv) < 2:
        print("用法: python3 02-rag-demo.py <文档文件路径> [问题]")
        print("示例: python3 02-rag-demo.py /path/to/readme.md '这个项目是做什么的？'")
        sys.exit(1)

    doc_path = sys.argv[1]
    query = sys.argv[2] if len(sys.argv) > 2 else None

    # 读取文档
    with open(doc_path, "r", encoding="utf-8") as f:
        text = f.read()

    chunks = chunk_text(text)
    print(f"文档已加载: {doc_path}")
    print(f"切分为 {len(chunks)} 个片段")

    # 加载模型
    print("加载模型...")
    llm = Llama(model_path=MODEL_PATH, n_ctx=4096, n_threads=2, verbose=False)

    if query:
        # 单次问答
        prompt = build_rag_prompt(query, chunks)
        if not prompt:
            print("未找到相关内容")
            return
        print(f"\n[问题] {query}")
        result = llm(prompt, max_tokens=256, temperature=0.5)
        print(f"\n[回答] {result['choices'][0]['text'].strip()}")
    else:
        # 交互模式
        print("\n进入交互模式（输入 q 退出）")
        while True:
            q = input("\n你的问题: ").strip()
            if q.lower() in ("q", "quit", "exit"):
                break
            prompt = build_rag_prompt(q, chunks)
            if not prompt:
                print("未找到相关内容")
                continue
            result = llm(prompt, max_tokens=256, temperature=0.5)
            print(f"\n{result['choices'][0]['text'].strip()}")


if __name__ == "__main__":
    main()
