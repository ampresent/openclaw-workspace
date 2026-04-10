"""
Mock Backend 2.0 — 薄框架 + OpenClaw 决策。

架构：
┌──────────────────────────────────────────────┐
│  mock framework (本进程)                      │
│  ┌──────────┐  ┌───────────┐  ┌───────────┐ │
│  │ GDB 后端  │→│ 事件队列   │→│ HTTP API   │ │
│  └──────────┘  └───────────┘  └─────┬─────┘ │
└──────────────────────────────────────┼───────┘
                                       │ 事件上报 / 指令下发
                                       ▼
                              ┌──────────────────┐
                              │ OpenClaw (AI)     │
                              │ • grep 源码       │
                              │ • 搜索手册        │
                              │ • 决策返回值      │
                              │ • push 响应       │
                              └──────────────────┘

框架职责（不可变）：
1. 连接 GDB，管理目标进程
2. 设置/清除断点
3. 断点命中时，提取上下文（地址、寄存器、反汇编）
4. 通过 HTTP API 暴露这些信息
5. 等待外部 push 响应值，注入到目标继续执行

框架不做的事：
- 不知道什么是 "e1000"
- 不知道寄存器含义
- 不做任何默认响应
- 一切决策由 OpenClaw 通过 HTTP API 下达
"""

import json
import logging
import threading
import queue
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from dataclasses import dataclass, asdict
from typing import Optional, Dict, Any, List, Callable
from pathlib import Path

logger = logging.getLogger(__name__)


# ============================================================
# GDB 后端（简化版，只做底层通信）
# ============================================================

import subprocess
import re


@dataclass
class BreakpointEvent:
    """断点命中事件 — 通过 HTTP API 暴露给外部"""
    event_id: str
    address: int           # 命中地址
    bp_id: int
    registers: Dict[str, int]  # 当前寄存器快照
    instruction: str = ""  # 当前指令（反汇编）
    stack_trace: List[str] = None

    def to_dict(self):
        d = asdict(self)
        d['address_hex'] = f"0x{self.address:x}"
        d['registers_hex'] = {k: f"0x{v:x}" for k, v in self.registers.items()}
        return d


class GDBSession:
    """GDB 会话 — 只做底层通信"""

    def __init__(self, gdb_path: str = "gdb"):
        self.gdb_path = gdb_path
        self._proc = None
        self._token = 0
        self._pending: Dict[int, queue.Queue] = {}
        self._reader = None
        self._stopped = threading.Event()
        self._stop_info = ""

    def launch(self, program: str, args: List[str] = None):
        cmd = [self.gdb_path, "--interpreter=mi2", "--quiet", program]
        self._proc = subprocess.Popen(
            cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT, text=True, bufsize=1
        )
        self._reader = threading.Thread(target=self._read_loop, daemon=True)
        self._reader.start()
        self._cmd("-gdb-set mi-async on")
        if args:
            self._cmd(f"-exec-arguments {' '.join(args)}")
        logger.info(f"GDB 已启动: {program}")

    def stop(self):
        if self._proc:
            try:
                self._cmd("-gdb-exit", timeout=3)
            except:
                pass
            self._proc.terminate()

    def set_breakpoint(self, location: str) -> int:
        result = self._cmd(f"-break-insert {location}")
        m = re.search(r'number="(\d+)"', result)
        if not m:
            raise RuntimeError(f"设置断点失败: {result}")
        return int(m.group(1))

    def delete_breakpoint(self, bp_id: int):
        self._cmd(f"-break-delete {bp_id}")

    def continue_execution(self):
        self._stopped.clear()
        self._cmd("-exec-continue --all")

    def wait_for_stop(self, timeout: float = None) -> bool:
        return self._stopped.wait(timeout)

    def read_register(self, name: str) -> int:
        result = self._cmd(f'-data-evaluate-expression "${name}"')
        m = re.search(r'value="([^"]*)"', result)
        if not m:
            return 0
        v = m.group(1)
        return int(v, 16) if v.startswith("0x") else int(v)

    def read_registers(self, names: List[str]) -> Dict[str, int]:
        return {n: self.read_register(n) for n in names}

    def read_memory(self, addr: int, size: int) -> bytes:
        result = self._cmd(f"-data-read-memory-bytes 0x{addr:x} {size}")
        m = re.search(r'contents="([^"]*)"', result)
        if not m:
            return b'\x00' * size
        return bytes.fromhex(m.group(1).replace(" ", "").replace("\n", ""))

    def write_register(self, name: str, value: int):
        self._cmd(f'-gdb-set ${name} = {value}')

    def get_disassembly(self, addr: int, count: int = 1) -> str:
        result = self._cmd(f"-data-disassemble -s 0x{addr:x} -e 0x{addr+count*15:x} -- 0")
        return result

    def snapshot(self, addr: int) -> BreakpointEvent:
        """在断点处快照当前状态"""
        regs = self.read_registers([
            'rax', 'rbx', 'rcx', 'rdx', 'rsi', 'rdi',
            'rbp', 'rsp', 'rip', 'r8', 'r9', 'r10', 'r11', 'r12', 'r13', 'r14', 'r15',
            'eflags', 'cs', 'ds', 'es', 'fs', 'gs', 'ss'
        ])
        # 尝试获取当前指令
        try:
            dis = self.get_disassembly(addr, 1)
            inst_match = re.search(r'"([^"]+)"\s*$', dis.split('\n')[1] if '\n' in dis else dis)
            instruction = inst_match.group(1) if inst_match else ""
        except:
            instruction = ""

        # 获取栈回溯
        try:
            bt = self._cmd("-stack-list-frames 0 5")
            frames = re.findall(r'func="([^"]*)"', bt)
        except:
            frames = []

        return BreakpointEvent(
            event_id=f"bp-{int(time.time()*1000)}",
            address=addr,
            bp_id=-1,
            registers=regs,
            instruction=instruction,
            stack_trace=frames,
        )

    # --- 内部 ---

    def _next_token(self):
        self._token += 1
        return self._token

    def _cmd(self, command: str, timeout: float = 10) -> str:
        token = self._next_token()
        q = queue.Queue()
        self._pending[token] = q
        self._proc.stdin.write(f"{token}{command}\n")
        self._proc.stdin.flush()
        try:
            return q.get(timeout=timeout)
        except queue.Empty:
            del self._pending[token]
            raise TimeoutError(f"GDB 命令超时: {command}")

    def _read_loop(self):
        while self._proc and self._proc.poll() is None:
            try:
                line = self._proc.stdout.readline().strip()
                if not line:
                    continue
                self._process(line)
            except:
                break

    def _process(self, line: str):
        # 命令结果
        m = re.match(r'^(\d+)\^(done|running|error|exit)(,.*)?$', line)
        if m:
            token = int(m.group(1))
            status = m.group(2)
            payload = m.group(3) or ""
            if token in self._pending:
                self._pending[token].put(payload if status != "error" else f"ERROR{payload}")
            return

        # *stopped 事件
        if line.startswith("*stopped"):
            self._stop_info = line
            self._stopped.set()

    @property
    def stop_address(self) -> int:
        m = re.search(r'addr="0x([0-9a-fA-F]+)"', self._stop_info)
        return int(m.group(1), 16) if m else 0

    @property
    def stop_bp_id(self) -> int:
        m = re.search(r'bkptno="(\d+)"', self._stop_info)
        return int(m.group(1)) if m else -1


# ============================================================
# HTTP API — 暴露给 OpenClaw
# ============================================================

class MockAPI:
    """
    HTTP API 服务器。

    OpenClaw 通过这个 API 与 mock 框架交互：

    GET  /state                    — 获取当前状态（等待中/运行中）
    GET  /event                    — 获取当前断点事件详情（阻塞等待）
    POST /respond                  — 推送响应值，注入后继续执行
    POST /breakpoint               — 设置断点
    DELETE /breakpoint/{id}        — 删除断点
    GET  /memory?addr=X&size=Y     — 读取目标内存
    POST /memory                   — 写入目标内存
    GET  /registers                — 读取寄存器
    POST /register                 — 写入寄存器
    POST /continue                 — 继续执行（无需注入）
    """

    def __init__(self, gdb: GDBSession, host: str = "127.0.0.1", port: int = 19876):
        self.gdb = gdb
        self.host = host
        self.port = port
        self._server = None
        self._thread = None
        self._event_queue: queue.Queue = queue.Queue()
        self._response_queue: queue.Queue = queue.Queue()
        self._state = "init"  # init / running / waiting / done
        self._bp_counter = 0
        self._breakpoints: Dict[int, Dict] = {}

    @property
    def state(self):
        return self._state

    def start(self):
        api = self

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                if self.path == "/state":
                    self._json(200, {
                        "state": api._state,
                        "breakpoints": api._breakpoints,
                    })
                elif self.path == "/event":
                    # 阻塞等待事件（最长 30s）
                    try:
                        event = api._event_queue.get(timeout=30)
                        self._json(200, event.to_dict())
                    except queue.Empty:
                        self._json(204, {"error": "timeout"})
                elif self.path.startswith("/memory"):
                    params = self._query_params()
                    addr = int(params.get("addr", ["0"])[0], 16)
                    size = int(params.get("size", ["4"])[0])
                    data = api.gdb.read_memory(addr, size)
                    self._json(200, {
                        "address": f"0x{addr:x}",
                        "size": size,
                        "data": data.hex(),
                    })
                elif self.path == "/registers":
                    names = ['rax','rbx','rcx','rdx','rsi','rdi','rbp','rsp','rip']
                    regs = api.gdb.read_registers(names)
                    self._json(200, {k: f"0x{v:x}" for k, v in regs.items()})
                else:
                    self._json(404, {"error": "not found"})

            def do_POST(self):
                body = self._read_body()
                if self.path == "/respond":
                    value = body.get("value")
                    reg = body.get("register", "rax")
                    cont = body.get("continue", True)
                    if value is not None:
                        val = int(str(value), 16) if isinstance(value, str) and value.startswith("0x") else int(value)
                        api.gdb.write_register(reg, val)
                        logger.info(f"[API] 注入 ${reg} = 0x{val:x}")
                    if cont:
                        api._state = "running"
                        api.gdb.continue_execution()
                    self._json(200, {"ok": True})
                elif self.path == "/breakpoint":
                    loc = body.get("location", "")
                    if loc:
                        bp_id = api.gdb.set_breakpoint(loc)
                        api._breakpoints[bp_id] = {"location": loc, "id": bp_id}
                        self._json(200, {"id": bp_id, "location": loc})
                    else:
                        self._json(400, {"error": "missing location"})
                elif self.path == "/continue":
                    api._state = "running"
                    api.gdb.continue_execution()
                    self._json(200, {"ok": True})
                elif self.path == "/memory":
                    addr = int(body.get("address", "0"), 16)
                    data = bytes.fromhex(body.get("data", ""))
                    for i, b in enumerate(data):
                        api.gdb._cmd(f"-data-write-memory-bytes 0x{addr+i:x} \"{b:02x}\"")
                    self._json(200, {"ok": True})
                elif self.path == "/register":
                    name = body.get("name", "")
                    value = int(str(body.get("value", 0)), 16)
                    api.gdb.write_register(name, value)
                    self._json(200, {"ok": True})
                else:
                    self._json(404, {"error": "not found"})

            def do_DELETE(self):
                if self.path.startswith("/breakpoint/"):
                    bp_id = int(self.path.split("/")[-1])
                    api.gdb.delete_breakpoint(bp_id)
                    api._breakpoints.pop(bp_id, None)
                    self._json(200, {"ok": True})
                else:
                    self._json(404, {"error": "not found"})

            def _json(self, code, data):
                body = json.dumps(data, ensure_ascii=False).encode()
                self.send_response(code)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def _read_body(self):
                length = int(self.headers.get("Content-Length", 0))
                if length == 0:
                    return {}
                return json.loads(self.rfile.read(length))

            def _query_params(self):
                from urllib.parse import urlparse, parse_qs
                return parse_qs(urlparse(self.path).query)

            def log_message(self, fmt, *args):
                logger.debug(f"HTTP {args[0]}")

        self._server = HTTPServer((self.host, self.port), Handler)
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)
        self._thread.start()
        logger.info(f"Mock API 启动: http://{self.host}:{self.port}")

    def stop(self):
        if self._server:
            self._server.shutdown()

    def push_event(self, event: BreakpointEvent):
        """向 API 推送断点事件（由事件循环调用）"""
        self._state = "waiting"
        self._event_queue.put(event)


# ============================================================
# 事件循环 — 串联 GDB 和 API
# ============================================================

class MockFramework:
    """
    Mock 主框架。

    职责极简：
    1. 启动 GDB
    2. 启动 HTTP API
    3. 事件循环：断点命中 → 快照 → push 到 API → 等外部响应
    """

    def __init__(self, gdb_path: str = "gdb", api_host: str = "127.0.0.1", api_port: int = 19876):
        self.gdb = GDBSession(gdb_path)
        self.api = MockAPI(self.gdb, api_host, api_port)
        self._running = False

    def launch(self, program: str, args: List[str] = None):
        """启动 mock 框架"""
        self.gdb.launch(program, args)
        self.api.start()
        self._running = True
        logger.info(f"""
╔══════════════════════════════════════════════╗
║  Mock Backend 就绪                           ║
║  API: http://{self.api.host}:{self.api.port}              ║
║                                              ║
║  OpenClaw 交互方式:                          ║
║  POST /breakpoint  {"location":"*0x..."}     ║
║  GET  /event       获取断点事件              ║
║  POST /respond     {"value":"0x...",        ║
║                     "register":"rax"}        ║
║  GET  /memory?addr=X&size=Y                  ║
║  GET  /registers                             ║
╚══════════════════════════════════════════════╝
""")

    def run(self):
        """主事件循环"""
        while self._running:
            # 等待 GDB 停止
            if not self.gdb.wait_for_stop(timeout=1.0):
                # 检查进程是否还活着
                if self.gdb._proc and self.gdb._proc.poll() is not None:
                    logger.info("目标进程已退出")
                    break
                continue

            # 断点命中 — 快照并推送到 API
            addr = self.gdb.stop_address
            bp_id = self.gdb.stop_bp_id
            event = self.gdb.snapshot(addr)
            event.bp_id = bp_id
            logger.info(f"断点命中: #{bp_id} @ 0x{addr:x}")
            self.api.push_event(event)

            # 等待外部通过 API 发来响应
            # 注意：响应通过 POST /respond 处理，会触发 continue
            # 这里只需等待状态从 waiting 变回 running
            while self.api.state == "waiting" and self._running:
                time.sleep(0.1)

    def stop(self):
        self._running = False
        self.api.stop()
        self.gdb.stop()


# ============================================================
# CLI 入口
# ============================================================

def main():
    import argparse
    parser = argparse.ArgumentParser(description="Mock Backend — 薄框架 + OpenClaw 决策")
    parser.add_argument("program", help="目标程序")
    parser.add_argument("--gdb", default="gdb", help="GDB 路径")
    parser.add_argument("--host", default="127.0.0.1", help="API 监听地址")
    parser.add_argument("--port", type=int, default=19876, help="API 监听端口")
    parser.add_argument("--breakpoint", "-b", action="append", default=[], help="初始断点 (可多次)")
    parser.add_argument("--args", nargs="*", default=[], help="目标程序参数")
    parser.add_argument("-v", "--verbose", action="store_true")
    args = parser.parse_args()

    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format='%(asctime)s [%(levelname)s] %(message)s',
        datefmt='%H:%M:%S',
    )

    fw = MockFramework(args.gdb, args.host, args.port)
    fw.launch(args.program, args.args)

    # 设置初始断点
    for bp_loc in args.breakpoint:
        bp_id = fw.gdb.set_breakpoint(bp_loc)
        fw.api._breakpoints[bp_id] = {"location": bp_loc, "id": bp_id}
        logger.info(f"初始断点: #{bp_id} @ {bp_loc}")

    import signal
    def on_sigint(sig, frame):
        fw.stop()
    signal.signal(signal.SIGINT, on_sigint)

    fw.gdb.continue_execution()
    fw.run()


if __name__ == "__main__":
    main()
