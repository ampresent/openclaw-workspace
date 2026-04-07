"""
拦截层 — 统一的设备访问拦截抽象。

支持两种拦截模式：
1. GDB 模式：通过 GDB/MI 在 MMIO/PIO 地址设置断点
2. ptrace 模式（后续）：直接 ptrace 拦截 syscall

核心概念：
- InterceptRule: 一条拦截规则（地址范围 + 回调）
- InterceptEvent: 一次拦截事件（地址、操作、当前值）
- InterceptAction: 拦截后的动作（返回 mock 值、修改内存、继续等）
"""

import enum
import logging
from dataclasses import dataclass, field
from typing import Optional, Callable, Dict, List, Any
from .gdb_backend import GDBMIBackend, BreakpointHit

logger = logging.getLogger(__name__)


class AccessType(enum.Enum):
    """设备访问类型"""
    MMIO_READ = "mmio_read"
    MMIO_WRITE = "mmio_write"
    PIO_READ = "pio_read"
    PIO_WRITE = "pio_write"
    DMA_READ = "dma_read"
    DMA_WRITE = "dma_write"
    CONFIG_READ = "config_read"
    CONFIG_WRITE = "config_write"


@dataclass
class InterceptEvent:
    """一次拦截事件"""
    access_type: AccessType
    address: int           # 访问的地址
    size: int              # 访问大小（字节）
    value: Optional[int] = None       # 写操作的值（读操作时为 None）
    register_name: str = ""           # 相关寄存器名
    instruction: str = ""             # 触发的指令
    timestamp: float = 0.0
    extra: Dict[str, Any] = field(default_factory=dict)


@dataclass
class InterceptAction:
    """拦截后的动作"""
    # 返回 mock 值（读操作）
    return_value: Optional[int] = None
    # 修改内存（写操作）
    write_to_memory: Optional[Dict[int, int]] = None  # {address: value}
    # 执行自定义回调
    callback: Optional[Callable] = None
    # 是否继续执行
    should_continue: bool = True
    # 日志消息
    log_message: str = ""


@dataclass
class InterceptRule:
    """一条拦截规则"""
    name: str
    address: int
    size: int                    # 拦截的地址范围 [address, address+size)
    access_types: List[AccessType]
    handler: Callable[[InterceptEvent], InterceptAction]
    enabled: bool = True
    hit_count: int = 0
    description: str = ""


class InterceptLayer:
    """
    统一拦截层。

    职责：
    1. 管理拦截规则（注册/注销/启用/禁用）
    2. 在 GDB 中设置对应的断点
    3. 断点命中时提取上下文、调用 handler、执行 action
    """

    def __init__(self, gdb: GDBMIBackend):
        self.gdb = gdb
        self._rules: Dict[int, InterceptRule] = {}   # bp_id -> rule
        self._addr_rules: Dict[int, InterceptRule] = {}  # address -> rule
        self._bp_counter = 0
        self._global_handlers: List[Callable[[InterceptEvent], Optional[InterceptAction]]] = []
        self._event_log: List[InterceptEvent] = []

    def register(self, rule: InterceptRule) -> int:
        """
        注册一条拦截规则，返回 GDB 断点 ID。
        """
        bp_location = f"*0x{rule.address:x}"
        bp_id = self.gdb.set_breakpoint(bp_location)

        self._rules[bp_id] = rule
        self._addr_rules[rule.address] = rule

        logger.info(
            f"注册拦截规则 '{rule.name}' @ 0x{rule.address:x} "
            f"(size={rule.size}, bp=#{bp_id})"
        )
        return bp_id

    def register_address(
        self,
        name: str,
        address: int,
        size: int,
        handler: Callable[[InterceptEvent], InterceptAction],
        access_types: List[AccessType] = None,
        description: str = "",
    ) -> int:
        """便捷方法：注册地址拦截"""
        if access_types is None:
            access_types = [AccessType.MMIO_READ, AccessType.MMIO_WRITE]

        rule = InterceptRule(
            name=name,
            address=address,
            size=size,
            access_types=access_types,
            handler=handler,
            description=description,
        )
        return self.register(rule)

    def register_range(
        self,
        name: str,
        base_address: int,
        total_size: int,
        step: int,
        handler: Callable[[InterceptEvent], InterceptAction],
        access_types: List[AccessType] = None,
    ) -> List[int]:
        """
        注册地址范围拦截（每 step 字节一个断点）。

        用于拦截 MMIO BAR 区域的所有寄存器访问。
        """
        bp_ids = []
        for offset in range(0, total_size, step):
            addr = base_address + offset
            bp_id = self.register_address(
                name=f"{name}+0x{offset:x}",
                address=addr,
                size=step,
                handler=handler,
                access_types=access_types,
            )
            bp_ids.append(bp_id)
        logger.info(f"注册地址范围: 0x{base_address:x} + {total_size} bytes, {len(bp_ids)} 个断点")
        return bp_ids

    def unregister(self, bp_id: int):
        """注销拦截规则"""
        rule = self._rules.pop(bp_id, None)
        if rule:
            self._addr_rules.pop(rule.address, None)
            self.gdb.delete_breakpoint(bp_id)
            logger.info(f"注销拦截规则 #{bp_id}: {rule.name}")

    def add_global_handler(self, handler: Callable[[InterceptEvent], Optional[InterceptAction]]):
        """添加全局拦截处理器（所有断点都会调用）"""
        self._global_handlers.append(handler)

    def handle_breakpoint_hit(self, hit: BreakpointHit) -> InterceptAction:
        """
        处理断点命中。由事件循环调用。

        1. 根据地址找到规则
        2. 提取上下文（读寄存器/内存）
        3. 调用 handler
        4. 执行 action
        """
        rule = self._rules.get(hit.bp_id)
        if not rule:
            logger.warning(f"未知断点 #{hit.bp_id} @ 0x{hit.address:x}")
            return InterceptAction(should_continue=True)

        rule.hit_count += 1

        # 构造拦截事件
        event = InterceptEvent(
            access_type=AccessType.MMIO_READ,  # TODO: 通过反汇编判断读/写
            address=hit.address,
            size=4,
            instruction=f"{hit.frame_func}:{hit.frame_line}",
            timestamp=__import__('time').time(),
        )

        # 尝试读取当前指令判断读/写
        try:
            inst_bytes = self.gdb.read_memory(hit.address, 4)
            # TODO: 反汇编判断操作类型
        except Exception:
            pass

        # 读取当前寄存器上下文
        try:
            event.extra['rax'] = self.gdb.read_register('rax')
            event.extra['rdx'] = self.gdb.read_register('rdx')
            event.extra['rip'] = hit.address
            event.extra['function'] = hit.frame_func
        except Exception:
            pass

        self._event_log.append(event)

        # 调用全局 handler
        action = InterceptAction()
        for gh in self._global_handlers:
            try:
                ga = gh(event)
                if ga:
                    action = ga
                    break
            except Exception as e:
                logger.error(f"全局 handler 异常: {e}")

        # 调用规则 handler
        try:
            rule_action = rule.handler(event)
            if rule_action:
                action = rule_action
        except Exception as e:
            logger.error(f"规则 handler 异常 [{rule.name}]: {e}")

        return action

    def execute_action(self, action: InterceptAction):
        """执行拦截动作"""
        if action.log_message:
            logger.info(f"[MOCK] {action.log_message}")

        if action.return_value is not None:
            # 回写 mock 值到 rax/eax（x86 返回值寄存器）
            try:
                self.gdb.write_memory(
                    self.gdb.read_register('rip'), 4,
                    action.return_value
                )
                # 实际上需要修改即将执行的 mov 指令的目标寄存器
                # 这里简化处理，通过修改 rax 返回
                # TODO: 更精确的寄存器注入
            except Exception as e:
                logger.error(f"注入返回值失败: {e}")

        if action.callback:
            try:
                action.callback()
            except Exception as e:
                logger.error(f"执行回调失败: {e}")

    @property
    def event_log(self) -> List[InterceptEvent]:
        return self._event_log
