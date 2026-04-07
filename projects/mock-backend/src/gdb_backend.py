"""
GDB/MI 后端 — 通过 GDB Machine Interface 协议与 GDB 通信。

功能：
- 启动/连接 GDB
- 设置/清除断点
- 读写寄存器和内存
- 断点事件回调
"""

import subprocess
import threading
import queue
import re
import time
import logging
from dataclasses import dataclass, field
from typing import Optional, Callable, Dict, Any, List

logger = logging.getLogger(__name__)


@dataclass
class BreakpointHit:
    """断点命中事件"""
    bp_id: int
    address: int  # 命中地址
    thread_id: int
    frame_func: str = ""
    frame_file: str = ""
    frame_line: int = 0


@dataclass
class Register:
    """寄存器值"""
    name: str
    value: int


class GDBMIError(Exception):
    """GDB/MI 通信错误"""
    pass


class GDBMIBackend:
    """
    GDB/MI 协议后端。

    用法：
        gdb = GDBMIBackend()
        gdb.start("my_program", ["--arg1"])
        bp_id = gdb.set_breakpoint("*0xf8000000")
        gdb.continue_execution()
        event = gdb.wait_for_breakpoint()
        val = gdb.read_memory(0xf8000000, 4)
        gdb.write_memory(0xf8000000, 4, 0x12345678)
        gdb.continue_execution()
    """

    # GDB/MI 输出模式
    _RE_RESULT = re.compile(r'^(\d+)\^(done|running|error|exit)(,.*)?$')
    _RE_EXEC = re.compile(r'^\*?(stopped|running)(,.*)?$', re.IGNORECASE)
    _RE_BREAKPOINT = re.compile(r'bkptno="(\d+)"')
    _RE_ADDRESS = re.compile(r'addr="0x([0-9a-fA-F]+)"')
    _RE_THREAD_ID = re.compile(r'thread-id="(\d+)"')
    _RE_FRAME_FUNC = re.compile(r'func="([^"]*)"')
    _RE_FRAME_FILE = re.compile(r'file="([^"]*)"')
    _RE_FRAME_LINE = re.compile(r'line="(\d+)"')

    def __init__(self, gdb_path: str = "gdb"):
        self.gdb_path = gdb_path
        self._proc: Optional[subprocess.Popen] = None
        self._token_counter = 0
        self._pending: Dict[int, queue.Queue] = {}
        self._breakpoint_callbacks: List[Callable[[BreakpointHit], None]] = []
        self._reader_thread: Optional[threading.Thread] = None
        self._running = False
        self._stopped_event = threading.Event()
        self._last_stop_reason: Optional[BreakpointHit] = None

    def start(self, program: str, args: List[str] = None, env: Dict[str, str] = None):
        """启动 GDB 并加载目标程序"""
        cmd = [self.gdb_path, "--interpreter=mi2", "--quiet"]
        if program:
            cmd.append(program)

        logger.info(f"启动 GDB: {' '.join(cmd)}")
        self._proc = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
            env=env,
        )
        self._running = True
        self._reader_thread = threading.Thread(target=self._read_loop, daemon=True)
        self._reader_thread.start()

        # 等待 GDB 就绪
        self._send_command("-gdb-set mi-async on")
        if args:
            self._send_command(f"-exec-arguments {' '.join(args)}")

        logger.info("GDB/MI 后端就绪")

    def stop(self):
        """关闭 GDB"""
        self._running = False
        if self._proc:
            try:
                self._send_command("-gdb-exit", timeout=3)
            except Exception:
                pass
            self._proc.terminate()
            self._proc.wait(timeout=5)
            self._proc = None

    def set_breakpoint(self, location: str, condition: str = None) -> int:
        """
        设置断点。

        location 可以是：
        - "*0xf8000000" — 绝对地址
        - "function_name" — 函数名
        - "file:line" — 文件行号
        """
        cmd = f"-break-insert {location}"
        if condition:
            cmd += f" -c \"{condition}\""
        result = self._send_command(cmd)
        # 解析 breakpoint number
        m = re.search(r'number="(\d+)"', result)
        if not m:
            raise GDBMIError(f"无法解析断点 ID: {result}")
        bp_id = int(m.group(1))
        logger.info(f"设置断点 #{bp_id} @ {location}")
        return bp_id

    def delete_breakpoint(self, bp_id: int):
        """删除断点"""
        self._send_command(f"-break-delete {bp_id}")
        logger.info(f"删除断点 #{bp_id}")

    def continue_execution(self) -> Optional[BreakpointHit]:
        """继续执行，返回 None 表示正常运行，返回 BreakpointHit 表示命中断点"""
        self._stopped_event.clear()
        self._send_command("-exec-continue --all")
        return None

    def wait_for_stop(self, timeout: float = None) -> Optional[BreakpointHit]:
        """等待目标停止（断点命中或信号）"""
        if self._stopped_event.wait(timeout):
            return self._last_stop_reason
        return None

    def step_instruction(self) -> Optional[BreakpointHit]:
        """单步执行一条指令"""
        self._send_command("-exec-step-instruction")
        return self._last_stop_reason

    def read_register(self, name: str) -> int:
        """读取单个寄存器"""
        result = self._send_command(f"-data-evaluate-expression \"${name}\"")
        m = re.search(r'value="([^"]*)"', result)
        if not m:
            raise GDBMIError(f"无法读取寄存器 {name}: {result}")
        val_str = m.group(1)
        if val_str.startswith("0x") or val_str.startswith("0X"):
            return int(val_str, 16)
        return int(val_str)

    def read_registers(self) -> Dict[str, int]:
        """读取所有寄存器"""
        result = self._send_command("-data-list-register-values x")
        registers = {}
        # 解析 [number,value] 对
        pairs = re.findall(r'\{number="([^"]+)",value="([^"]+)"\}', result)
        for num, val in pairs:
            try:
                registers[num] = int(val, 16) if val.startswith("0x") else int(val)
            except ValueError:
                pass
        return registers

    def read_memory(self, address: int, size: int) -> bytes:
        """读取内存"""
        result = self._send_command(
            f"-data-read-memory-bytes 0x{address:x} {size}"
        )
        # 解析 memory 内容
        m = re.search(r'contents="([^"]*)"', result)
        if not m:
            raise GDBMIError(f"无法读取内存 @ 0x{address:x}: {result}")
        hex_str = m.group(1).replace(" ", "").replace("\n", "")
        return bytes.fromhex(hex_str)

    def read_memory_u32(self, address: int) -> int:
        """读取 32 位内存值"""
        data = self.read_memory(address, 4)
        return int.from_bytes(data, byteorder='little')

    def write_memory(self, address: int, size: int, value: int):
        """写入内存"""
        data = value.to_bytes(size, byteorder='little').hex()
        self._send_command(
            f"-data-write-memory-bytes 0x{address:x} \"{data}\""
        )

    def call_function(self, expr: str) -> str:
        """调用目标进程中的函数"""
        result = self._send_command(f"-data-evaluate-expression \"{expr}\"")
        m = re.search(r'value="([^"]*)"', result)
        return m.group(1) if m else result

    def on_breakpoint(self, callback: Callable[[BreakpointHit], None]):
        """注册断点回调"""
        self._breakpoint_callbacks.append(callback)

    # ---- 内部方法 ----

    def _next_token(self) -> int:
        self._token_counter += 1
        return self._token_counter

    def _send_command(self, command: str, timeout: float = 10) -> str:
        """发送命令并等待响应"""
        token = self._next_token()
        q = queue.Queue()
        self._pending[token] = q

        full_cmd = f"{token}{command}\n"
        logger.debug(f">>> {full_cmd.strip()}")

        if not self._proc or self._proc.poll() is not None:
            raise GDBMIError("GDB 进程未运行")

        self._proc.stdin.write(full_cmd)
        self._proc.stdin.flush()

        try:
            result = q.get(timeout=timeout)
            logger.debug(f"<<< {result[:200]}")
            return result
        except queue.Empty:
            del self._pending[token]
            raise GDBMIError(f"命令超时: {command}")
        finally:
            self._pending.pop(token, None)

    def _read_loop(self):
        """后台读取 GDB 输出的线程"""
        while self._running and self._proc and self._proc.poll() is None:
            try:
                line = self._proc.stdout.readline()
                if not line:
                    break
                line = line.strip()
                if not line:
                    continue
                self._process_line(line)
            except Exception as e:
                logger.error(f"读取 GDB 输出异常: {e}")
                break

    def _process_line(self, line: str):
        """处理 GDB/MI 输出行"""
        logger.debug(f"GDB<<< {line}")

        # 检查是否是命令结果 (token^done/running/error)
        m = self._RE_RESULT.match(line)
        if m:
            token = int(m.group(1))
            status = m.group(2)
            payload = m.group(3) or ""
            if token in self._pending:
                if status == "error":
                    err_msg = re.search(r'msg="([^"]*)"', payload)
                    self._pending[token].put(
                        f"ERROR: {err_msg.group(1) if err_msg else payload}"
                    )
                else:
                    self._pending[token].put(payload)
            return

        # 检查是否是 *stopped 事件
        if line.startswith("*stopped"):
            self._handle_stopped(line)

    def _handle_stopped(self, line: str):
        """处理 *stopped 事件"""
        # 解析断点信息
        bp_match = self._RE_BREAKPOINT.search(line)
        addr_match = self._RE_ADDRESS.search(line)
        thread_match = self._RE_THREAD_ID.search(line)
        func_match = self._RE_FRAME_FUNC.search(line)
        file_match = self._RE_FRAME_FILE.search(line)
        line_match = self._RE_FRAME_LINE.search(line)

        hit = BreakpointHit(
            bp_id=int(bp_match.group(1)) if bp_match else -1,
            address=int(addr_match.group(1), 16) if addr_match else 0,
            thread_id=int(thread_match.group(1)) if thread_match else 1,
            frame_func=func_match.group(1) if func_match else "",
            frame_file=file_match.group(1) if file_match else "",
            frame_line=int(line_match.group(1)) if line_match else 0,
        )

        self._last_stop_reason = hit
        self._stopped_event.set()

        # 调用已注册的回调
        for cb in self._breakpoint_callbacks:
            try:
                cb(hit)
            except Exception as e:
                logger.error(f"断点回调异常: {e}")
