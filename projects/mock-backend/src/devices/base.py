"""
设备基类 — 所有 mock 设备的抽象接口。

扩展新设备只需：
1. 继承 MockDevice
2. 定义设备元信息（名称、PCI ID、BAR）
3. 实现 handle_access 或注册规则
4. 提供参考文档
"""

import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Optional, Dict, List, Any, Callable
from pathlib import Path

from src.context import ContextEngine
from src.intercept import InterceptEvent, InterceptAction, AccessType, InterceptRule
from src.response import AIResponseEngine, DeviceState

logger = logging.getLogger(__name__)


@dataclass
class PCIDeviceConfig:
    """PCI 设备配置空间"""
    vendor_id: int = 0xFFFF
    device_id: int = 0xFFFF
    command: int = 0x0000
    status: int = 0x0000
    revision: int = 0x00
    class_code: int = 0x000000  # 24-bit
    header_type: int = 0x00
    bar0: int = 0x00000000
    bar1: int = 0x00000000
    bar2: int = 0x00000000
    subsystem_vendor: int = 0x0000
    subsystem_id: int = 0x0000
    interrupt_line: int = 0x00
    interrupt_pin: int = 0x00


@dataclass
class MMIORegion:
    """MMIO 映射区域"""
    bar_index: int       # BAR0, BAR1, ...
    base_offset: int     # 在 BAR 中的偏移
    size: int            # 区域大小
    name: str = ""
    description: str = ""


class MockDevice(ABC):
    """
    Mock 设备基类。

    每个 mock 设备代表一个可被驱动访问的虚拟硬件设备。
    """

    def __init__(self, name: str):
        self.name = name
        self.state = DeviceState()
        self.context = ContextEngine()
        self.ai_engine: Optional[AIResponseEngine] = None
        self._rules: List[Callable] = []
        self._mmio_regions: List[MMIORegion] = []

    @abstractmethod
    def get_pci_config(self) -> PCIDeviceConfig:
        """返回设备的 PCI 配置空间"""
        ...

    @abstractmethod
    def get_mmio_regions(self) -> List[MMIORegion]:
        """返回设备的 MMIO 区域"""
        ...

    @abstractmethod
    def init_state(self):
        """初始化设备默认状态（上电状态）"""
        ...

    def load_reference(self, path: str, doc_type: str = None):
        """加载参考文档到上下文引擎"""
        self.context.load_file(path, doc_type)

    def set_ai_engine(self, engine: AIResponseEngine):
        """设置 AI 响应引擎"""
        self.ai_engine = engine

    def add_register_rule(
        self,
        reg_offset: int,
        handler: Callable[[InterceptEvent, DeviceState], InterceptAction],
    ):
        """添加寄存器级手动规则"""
        self._rules.append((reg_offset, handler))

    def handle_access(self, event: InterceptEvent) -> InterceptAction:
        """
        处理设备访问。优先级：
        1. 手动规则
        2. AI 引擎
        3. 默认响应（返回当前状态值）
        """
        # 手动规则
        for reg_offset, handler in self._rules:
            if event.address == reg_offset:
                action = handler(event, self.state)
                if action:
                    return action

        # AI 引擎
        if self.ai_engine:
            return self.ai_engine.handle_event(event, self.state)

        # 默认：返回当前寄存器值
        val = self.state.get_reg(event.address)
        return InterceptAction(
            return_value=val,
            log_message=f"[{self.name}] 默认响应: 0x{event.address:x} -> 0x{val:x}",
        )


class DeviceRegistry:
    """设备注册表 — 管理所有 mock 设备实例"""

    def __init__(self):
        self._devices: Dict[str, MockDevice] = {}

    def register(self, device: MockDevice):
        self._devices[device.name] = device
        logger.info(f"注册设备: {device.name}")

    def get(self, name: str) -> Optional[MockDevice]:
        return self._devices.get(name)

    def list_devices(self) -> List[str]:
        return list(self._devices.keys())
