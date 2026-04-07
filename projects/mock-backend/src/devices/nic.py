"""
e1000 网卡 Mock — 第一个目标设备。

基于 Intel 82540EP/EM 手册和 Linux e1000e 驱动源码。

关键功能：
- PCI 配空间（vendor=0x8086, device=0x100E）
- MMIO 寄存器（BAR0）
- 基本的收发包描述符环
- 中断状态寄存器
"""

import logging
from typing import List

from src.devices.base import MockDevice, PCIDeviceConfig, MMIORegion
from src.intercept import InterceptEvent, InterceptAction, AccessType
from src.response import DeviceState

logger = logging.getLogger(__name__)

# ---- e1000 寄存器偏移（来自 Intel 手册） ----

# 控制寄存器
E1000_CTRL     = 0x00000  # Device Control
E1000_STATUS   = 0x00008  # Device Status
E1000_EECD    = 0x00010  # EEPROM/Flash Control/Data
E1000_EERD    = 0x00014  # EEPROM Read

# 中断
E1000_ICR      = 0x000C0  # Interrupt Cause Read
E1000_ICS      = 0x000C8  # Interrupt Cause Set
E1000_IMS      = 0x000D0  # Interrupt Mask Set/Read
E1000_IMC      = 0x000D8  # Interrupt Mask Clear

# 接收
E1000_RCTL     = 0x00100  # Receive Control
E1000_RDBAL    = 0x02800  # Receive Descriptor Base Low
E1000_RDBAH    = 0x02804  # Receive Descriptor Base High
E1000_RDLEN    = 0x02808  # Receive Descriptor Length
E1000_RDH      = 0x02810  # Receive Descriptor Head
E1000_RDT      = 0x02818  # Receive Descriptor Tail

# 发送
E1000_TCTL     = 0x00400  # Transmit Control
E1000_TDBAL    = 0x03800  # Transmit Descriptor Base Low
E1000_TDBAH    = 0x03804  # Transmit Descriptor Base High
E1000_TDLEN    = 0x03808  # Transmit Descriptor Length
E1000_TDH      = 0x03810  # Transmit Descriptor Head
E1000_TDT      = 0x03818  # Transmit Descriptor Tail

# MAC 地址
E1000_RAL      = 0x05400  # Receive Address Low (RAL[0])
E1000_RAH      = 0x05404  # Receive Address High (RAH[0])

# 状态位
E1000_STATUS_FD = 0x0001       # Full Duplex
E1000_STATUS_LU = 0x0002       # Link Up
E1000_STATUS_SPEED_100 = 0x0010  # Speed 100Mbps
E1000_STATUS_SPEED_1000 = 0x0080  # Speed 1000Mbps
E1000_STATUS_ASDV = 0x0200     # Auto-Speed Detection Value

# 控制位
E1000_CTRL_SLU = 0x00000040    # Set Link Up
E1000_CTRL_ASDE = 0x00000020   # Auto-Speed Detection Enable
E1000_CTRL_FD  = 0x00000001    # Full Duplex

# 中断位
E1000_ICR_LSC  = 0x00000004    # Link Status Change
E1000_ICR_RXTO = 0x00000008    # Receiver Timer Interrupt
E1000_ICR_RXT  = 0x00000080    # Receiver Timer (delayed)
E1000_ICR_TXDW = 0x00000001    # Transmit Descriptor Written Back


class E1000Device(MockDevice):
    """
    Intel e1000/e1000e 网卡 Mock。

    模拟基本的 e1000 网卡行为：
    - PCI 配置空间
    - MMIO 寄存器读写
    - 链路状态（始终 Link Up）
    - 接收/发送描述符环
    """

    def __init__(self, mac_address: bytes = b'\x52\x54\x00\x12\x34\x56'):
        super().__init__("e1000")
        self.mac = mac_address[:6]

    def get_pci_config(self) -> PCIDeviceConfig:
        return PCIDeviceConfig(
            vendor_id=0x8086,
            device_id=0x100E,  # 82540EM (QEMU 默认 e1000)
            command=0x0007,    # IO + Memory + Bus Master
            status=0x0010,     # Capabilities List
            revision=0x03,
            class_code=0x020000,  # Network Controller / Ethernet
            header_type=0x00,
            bar0=0xFEBC0000,  # MMIO base (将被 OS 重映射)
            subsystem_vendor=0x8086,
            subsystem_id=0x0000,
            interrupt_line=0x0B,
            interrupt_pin=0x01,
        )

    def get_mmio_regions(self) -> List[MMIORegion]:
        return [
            MMIORegion(bar_index=0, base_offset=0, size=0x20000,
                       name="e1000_regs", description="e1000 MMIO 寄存器区域"),
        ]

    def init_state(self):
        """初始化上电状态"""
        s = self.state

        # Device Status: Link Up, Full Duplex, 1000Mbps
        s.set_reg(E1000_STATUS,
                  E1000_STATUS_LU | E1000_STATUS_FD | E1000_STATUS_SPEED_1000)

        # Device Control: Auto-Speed, Set Link Up
        s.set_reg(E1000_CTRL, E1000_CTRL_SLU | E1000_CTRL_ASDE)

        # MAC 地址
        mac_low = int.from_bytes(self.mac[:4], 'little')
        mac_high = int.from_bytes(self.mac[4:6], 'little')
        s.set_reg(E1000_RAL, mac_low)
        s.set_reg(E1000_RAH, mac_high | 0x80000000)  # AV bit

        # RCTL 默认值
        s.set_reg(E1000_RCTL, 0x00000000)

        # TCTL 默认值
        s.set_reg(E1000_TCTL, 0x000400FA)

        # 描述符环指针
        s.set_reg(E1000_RDH, 0)
        s.set_reg(E1000_RDT, 0)
        s.set_reg(E1000_TDH, 0)
        s.set_reg(E1000_TDT, 0)

        # EEPROM: 简单标记
        s.set_reg(E1000_EECD, 0x00000110)

        # 中断
        s.set_reg(E1000_ICR, 0)
        s.set_reg(E1000_IMS, 0)

        logger.info(f"e1000 初始化完成, MAC={self.mac.hex(':')}")

        # 注册关键寄存器的手动规则
        self._register_rules()

    def _register_rules(self):
        """注册关键寄存器的手动规则（不走 LLM，直接返回确定值）"""

        def handle_status(event, state):
            """STATUS 寄存器：始终 Link Up"""
            val = E1000_STATUS_LU | E1000_STATUS_FD | E1000_STATUS_SPEED_1000
            return InterceptAction(
                return_value=val,
                log_message="[e1000] STATUS -> Link Up, FD, 1000Mbps",
            )

        def handle_icr(event, state):
            """ICR 读取后清除（真实硬件行为）"""
            val = state.get_reg(E1000_ICR)
            state.set_reg(E1000_ICR, 0)  # 读后清零
            return InterceptAction(
                return_value=val,
                log_message=f"[e1000] ICR read -> 0x{val:x} (cleared)",
            )

        def handle_ral(event, state):
            """RAL: MAC 地址低 32 位"""
            mac_low = int.from_bytes(self.mac[:4], 'little')
            return InterceptAction(
                return_value=mac_low,
                log_message=f"[e1000] RAL -> MAC low 0x{mac_low:08x}",
            )

        def handle_rah(event, state):
            """RAH: MAC 地址高 16 位 + AV bit"""
            mac_high = int.from_bytes(self.mac[4:6], 'little')
            return InterceptAction(
                return_value=mac_high | 0x80000000,
                log_message=f"[e1000] RAH -> MAC high 0x{mac_high:04x} + AV",
            )

        # 注册规则
        self.add_register_rule(E1000_STATUS, handle_status)
        self.add_register_rule(E1000_ICR, handle_icr)
        self.add_register_rule(E1000_RAL, handle_ral)
        self.add_register_rule(E1000_RAH, handle_rah)

    def inject_rx_packet(self, data: bytes):
        """
        注入一个接收包。

        检查当前 RDH/RDT，如果有可用描述符，
        将数据写入描述符指向的 buffer，并更新 RDH。
        """
        rdh = self.state.get_reg(E1000_RDH)
        rdt = self.state.get_reg(E1000_RDT)
        rdlen = self.state.get_reg(E1000_RDLEN)
        if rdlen == 0:
            logger.warning("[e1000] RDLEN not configured, using default ring size")
            rdlen = 256 * 16  # 默认 256 个描述符

        # 检查是否有空间
        next_head = (rdh + 1) % (rdlen // 16)
        if next_head == rdt:
            logger.warning("[e1000] RX ring full, dropping packet")
            return False

        # 更新 RDH
        self.state.set_reg(E1000_RDH, next_head)

        # 触发接收中断
        self.state.set_reg(E1000_ICR, E1000_ICR_RXT)
        logger.info(
            f"[e1000] RX packet injected: {len(data)} bytes, "
            f"RDH={next_head}, ICR=0x{E1000_ICR_RXT:x}"
        )
        return True
