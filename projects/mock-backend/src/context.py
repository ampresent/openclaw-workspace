"""
上下文引擎 — 加载设备文档和源码，为 AI 提供可检索的知识库。

支持：
- 文本/Markdown 文档
- C 头文件（寄存器定义）
- C 源码（驱动实现参考）
- PDF（需要额外依赖）

用法：
    ctx = ContextEngine()
    ctx.load_file("e1000_regs.h")
    ctx.load_file("e1000_hw_manual.txt")
    snippet = ctx.search("TCTL register")
"""

import os
import re
import logging
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Tuple
from pathlib import Path

logger = logging.getLogger(__name__)


@dataclass
class Document:
    """加载的文档"""
    name: str
    path: str
    content: str
    doc_type: str  # "header", "source", "manual", "spec"
    indexed_chunks: List[str] = field(default_factory=list)


@dataclass
class SearchResult:
    """搜索结果"""
    document: str
    chunk: str
    score: float
    line_start: int = 0
    line_end: int = 0


class ContextEngine:
    """
    上下文引擎。

    将设备文档和源码加载为可搜索的知识库，
    在拦截事件发生时快速检索相关上下文。
    """

    def __init__(self):
        self.documents: Dict[str, Document] = {}
        self._register_defs: Dict[str, Dict] = {}  # 寄存器名 -> 定义
        self._defines: Dict[str, str] = {}  # #define 名 -> 值

    def load_file(self, path: str, doc_type: str = None) -> Document:
        """
        加载文件到知识库。

        doc_type 可选：header, source, manual, spec
        如果不指定，根据文件扩展名自动判断。
        """
        p = Path(path)
        if not p.exists():
            raise FileNotFoundError(f"文件不存在: {path}")

        if doc_type is None:
            doc_type = self._guess_doc_type(p.suffix)

        content = p.read_text(encoding='utf-8', errors='replace')
        doc = Document(
            name=p.name,
            path=str(p.absolute()),
            content=content,
            doc_type=doc_type,
        )

        # 按段落/函数分块
        doc.indexed_chunks = self._chunk_document(content, doc_type)
        self.documents[doc.name] = doc

        # 解析头文件中的 #define
        if doc_type == "header":
            self._parse_defines(content, doc.name)
            self._parse_register_defs(content, doc.name)

        logger.info(f"加载文档: {doc.name} ({doc_type}, {len(content)} chars, {len(doc.indexed_chunks)} chunks)")
        return doc

    def load_text(self, name: str, text: str, doc_type: str = "manual") -> Document:
        """直接加载文本"""
        doc = Document(
            name=name,
            path="<inline>",
            content=text,
            doc_type=doc_type,
            indexed_chunks=self._chunk_document(text, doc_type),
        )
        self.documents[name] = doc
        return doc

    def search(self, query: str, max_results: int = 5) -> List[SearchResult]:
        """
        搜索相关文档片段。

        使用关键词匹配（简单但实用），后续可升级为向量检索。
        """
        query_lower = query.lower()
        query_terms = set(re.findall(r'\w+', query_lower))

        results = []
        for doc in self.documents.values():
            for i, chunk in enumerate(doc.indexed_chunks):
                chunk_lower = chunk.lower()
                # 计算相关度
                term_hits = sum(1 for t in query_terms if t in chunk_lower)
                if term_hits == 0:
                    continue
                score = term_hits / len(query_terms) if query_terms else 0

                # 精确匹配加权
                if query_lower in chunk_lower:
                    score += 2.0

                if score > 0.3:
                    results.append(SearchResult(
                        document=doc.name,
                        chunk=chunk[:2000],  # 限制长度
                        score=score,
                    ))

        results.sort(key=lambda r: r.score, reverse=True)
        return results[:max_results]

    def get_register_definition(self, reg_name: str) -> Optional[Dict]:
        """获取寄存器定义"""
        return self._register_defs.get(reg_name.upper())

    def get_define(self, name: str) -> Optional[str]:
        """获取 #define 值"""
        return self._defines.get(name)

    def get_all_defines(self, prefix: str = "") -> Dict[str, str]:
        """获取所有 #define（可按前缀过滤）"""
        if not prefix:
            return dict(self._defines)
        return {k: v for k, v in self._defines.items() if k.startswith(prefix)}

    def dump_context_for_query(self, query: str, max_chunks: int = 3) -> str:
        """
        为 AI 查询生成格式化的上下文。

        返回：搜索结果 + 相关寄存器定义，拼成一个文本块。
        """
        parts = []

        # 搜索相关文档
        results = self.search(query, max_results=max_chunks)
        for r in results:
            parts.append(f"--- From {r.document} (score={r.score:.1f}) ---")
            parts.append(r.chunk)

        # 添加相关 #define
        query_terms = set(re.findall(r'\w+', query.lower()))
        related_defines = {}
        for name, val in self._defines.items():
            name_lower = name.lower()
            if any(t in name_lower for t in query_terms):
                related_defines[name] = val

        if related_defines:
            parts.append("--- Related #defines ---")
            for name, val in sorted(related_defines.items()):
                parts.append(f"#define {name} {val}")

        return "\n\n".join(parts)

    # ---- 内部方法 ----

    def _guess_doc_type(self, suffix: str) -> str:
        mapping = {
            ".h": "header",
            ".c": "source",
            ".py": "source",
            ".txt": "manual",
            ".md": "manual",
            ".pdf": "spec",
            ".rst": "manual",
        }
        return mapping.get(suffix.lower(), "manual")

    def _chunk_document(self, content: str, doc_type: str) -> List[str]:
        """将文档分块"""
        if doc_type == "header":
            return self._chunk_c_header(content)
        elif doc_type == "source":
            return self._chunk_c_source(content)
        else:
            return self._chunk_text(content)

    def _chunk_c_header(self, content: str) -> List[str]:
        """按 #define 群组或结构体分块"""
        chunks = []
        current_chunk = []
        for line in content.split('\n'):
            current_chunk.append(line)
            # 每 50 行一个块，或者遇到空行+注释分隔
            if len(current_chunk) >= 50:
                chunks.append('\n'.join(current_chunk))
                current_chunk = []
        if current_chunk:
            chunks.append('\n'.join(current_chunk))
        return chunks

    def _chunk_c_source(self, content: str) -> List[str]:
        """按函数分块"""
        # 简单的函数分割：以行首 } 结束
        chunks = []
        current_func = []
        brace_depth = 0

        for line in content.split('\n'):
            current_func.append(line)
            brace_depth += line.count('{') - line.count('}')

            if brace_depth == 0 and current_func and '}' in line:
                chunk = '\n'.join(current_func)
                if len(chunk.strip()) > 10:
                    chunks.append(chunk)
                current_func = []

        if current_func:
            chunks.append('\n'.join(current_func))

        return chunks

    def _chunk_text(self, content: str) -> List[str]:
        """按段落分块"""
        paragraphs = re.split(r'\n\s*\n', content)
        chunks = []
        current = []
        char_count = 0

        for para in paragraphs:
            current.append(para)
            char_count += len(para)
            if char_count >= 1000:
                chunks.append('\n\n'.join(current))
                current = []
                char_count = 0

        if current:
            chunks.append('\n\n'.join(current))

        return chunks

    def _parse_defines(self, content: str, source: str):
        """解析 #define"""
        for m in re.finditer(r'#define\s+(\w+)\s+(.+?)(?:\s*/[/*].*)?$', content, re.MULTILINE):
            name = m.group(1)
            value = m.group(2).strip()
            self._defines[name] = value

    def _parse_register_defs(self, content: str, source: str):
        """尝试解析寄存器定义结构"""
        # 查找 struct 定义中的寄存器字段
        for m in re.finditer(r'struct\s+(\w+)\s*\{([^}]+)\}', content, re.DOTALL):
            struct_name = m.group(1)
            body = m.group(2)
            fields = re.findall(r'(?:u\w+|__le\w+|__be\w+|unsigned\s+\w+|\w+_t)\s+(\w+);', body)
            if fields:
                self._register_defs[struct_name] = {
                    "fields": fields,
                    "source": source,
                }
