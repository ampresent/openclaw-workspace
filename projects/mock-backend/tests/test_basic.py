#!/usr/bin/env python3
"""
测试脚本 — 验证 mock 后端各模块。

不需要实际 GDB，使用 mock 的单元测试。
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from src.context import ContextEngine
from src.intercept import InterceptEvent, AccessType, InterceptAction
from src.response import AIResponseEngine, DeviceState
from src.devices.nic import E1000Device, E1000_STATUS, E1000_RAL, E1000_RAH, E1000_ICR


def test_context_engine():
    """测试上下文引擎"""
    print("=" * 60)
    print("TEST: ContextEngine")
    print("=" * 60)

    ctx = ContextEngine()

    # 加载内联文本
    ctx.load_text("e1000_manual", """
    E1000 STATUS Register (0x00008):
    Bit 0: FD - Full Duplex
    Bit 1: LU - Link Up
    Bit 4: SPEED_100 - Speed 100Mbps
    Bit 8: SPEED_1000 - Speed 1000Mbps

    E1000 CTRL Register (0x00000):
    Bit 0: FD - Full Duplex
    Bit 5: ASDE - Auto-Speed Detection Enable
    Bit 6: SLU - Set Link Up
    """, doc_type="manual")

    # 搜索
    results = ctx.search("STATUS register Link Up")
    assert len(results) > 0, "搜索不应返回空"
    print(f"  ✓ 搜索 'STATUS register Link Up' -> {len(results)} 结果")
    for r in results:
        print(f"    [{r.document}] score={r.score:.1f}: {r.chunk[:80]}...")

    # dump context
    context_text = ctx.dump_context_for_query("e1000 STATUS register")
    assert "LU" in context_text, "上下文应包含 LU"
    print(f"  ✓ dump_context 包含相关定义 ({len(context_text)} chars)")

    print()


def test_e1000_device():
    """测试 e1000 设备"""
    print("=" * 60)
    print("TEST: E1000Device")
    print("=" * 60)

    device = E1000Device(mac_address=b'\x00\x11\x22\x33\x44\x55')
    device.init_state()

    # 检查 PCI 配置
    pci = device.get_pci_config()
    assert pci.vendor_id == 0x8086, f"Vendor ID 应为 0x8086, 实际 0x{pci.vendor_id:x}"
    assert pci.device_id == 0x100E, f"Device ID 应为 0x100E, 实际 0x{pci.device_id:x}"
    print(f"  ✓ PCI: vendor=0x{pci.vendor_id:04x}, device=0x{pci.device_id:04x}")

    # 检查 STATUS 寄存器
    event = InterceptEvent(
        access_type=AccessType.MMIO_READ,
        address=E1000_STATUS,
        size=4,
    )
    action = device.handle_access(event)
    assert action.return_value is not None, "应返回 STATUS 值"
    link_up = bool(action.return_value & 0x0002)
    full_duplex = bool(action.return_value & 0x0001)
    print(f"  ✓ STATUS: 0x{action.return_value:08x} (LinkUp={link_up}, FD={full_duplex})")

    # 检查 MAC 地址
    event_ral = InterceptEvent(
        access_type=AccessType.MMIO_READ,
        address=E1000_RAL,
        size=4,
    )
    action_ral = device.handle_access(event_ral)
    print(f"  ✓ RAL (MAC low): 0x{action_ral.return_value:08x}")

    event_rah = InterceptEvent(
        access_type=AccessType.MMIO_READ,
        address=E1000_RAH,
        size=4,
    )
    action_rah = device.handle_access(event_rah)
    print(f"  ✓ RAH (MAC high): 0x{action_rah.return_value:08x}")
    assert action_rah.return_value & 0x80000000, "AV bit 应该置位"

    # 检查 ICR 读后清零
    device.state.set_reg(E1000_ICR, 0x04)  # 设置 LSC 中断
    event_icr = InterceptEvent(
        access_type=AccessType.MMIO_READ,
        address=E1000_ICR,
        size=4,
    )
    action_icr1 = device.handle_access(event_icr)
    assert action_icr1.return_value == 0x04, "第一次读 ICR 应返回 0x04"
    action_icr2 = device.handle_access(event_icr)
    assert action_icr2.return_value == 0, "第二次读 ICR 应返回 0 (读后清零)"
    print(f"  ✓ ICR 读后清零: 第一次=0x{action_icr1.return_value:x}, 第二次=0x{action_icr2.return_value:x}")

    # 注入接收包
    success = device.inject_rx_packet(b'\xff' * 64)
    assert success, "RX 注入应成功"
    print(f"  ✓ RX 包注入成功, RDH={device.state.get_reg(0x02810)}")

    print()


def test_ai_response_engine():
    """测试 AI 响应引擎（无 LLM 模式）"""
    print("=" * 60)
    print("TEST: AIResponseEngine (no LLM)")
    print("=" * 60)

    ctx = ContextEngine()
    ctx.load_text("test", "#define E1000_STATUS_LU 0x0002\n#define E1000_STATUS_FD 0x0001")

    engine = AIResponseEngine(context=ctx, llm_client=None)

    # 无 LLM 时应返回 0
    state = DeviceState()
    event = InterceptEvent(
        access_type=AccessType.MMIO_READ,
        address=0x00008,
        size=4,
    )
    action = engine.handle_event(event, state)
    assert action.return_value == 0, "无 LLM 应返回 0"
    print(f"  ✓ 无 LLM 模式: 返回 0")

    # 添加手动规则
    def rule_status(event, state):
        return InterceptAction(return_value=0x0003, log_message="LinkUp+FD")

    engine.add_rule(
        address=0x00008,
        address_mask=0xFFFFFFF8,
        access_type=AccessType.MMIO_READ,
        handler=rule_status,
    )
    action2 = engine.handle_event(event, state)
    assert action2.return_value == 0x0003, "规则应返回 0x0003"
    print(f"  ✓ 手动规则: 返回 0x{action2.return_value:04x}")

    # 缓存测试
    action3 = engine.handle_event(event, state)
    assert action3.return_value == 0x0003, "缓存应返回相同值"
    print(f"  ✓ 缓存命中")

    print()


def test_device_state():
    """测试设备状态"""
    print("=" * 60)
    print("TEST: DeviceState")
    print("=" * 60)

    state = DeviceState()
    state.set_reg(0x00, 0x12345678)
    assert state.get_reg(0x00) == 0x12345678
    print(f"  ✓ 寄存器读写: 0x{state.get_reg(0x00):08x}")

    state.set_reg(0x04, 0xFFFFFFFF)
    assert state.get_reg(0x04) == 0xFFFFFFFF
    print(f"  ✓ 32位值: 0x{state.get_reg(0x04):08x}")

    assert state.get_reg(0x999, 0xDEAD) == 0xDEAD
    print(f"  ✓ 未定义寄存器返回默认值")

    print()


def main():
    print("\n🧪 Mock Backend 测试套件\n")

    tests = [
        test_device_state,
        test_context_engine,
        test_e1000_device,
        test_ai_response_engine,
    ]

    passed = 0
    failed = 0
    for test in tests:
        try:
            test()
            passed += 1
        except AssertionError as e:
            print(f"  ✗ FAIL: {e}")
            failed += 1
        except Exception as e:
            print(f"  ✗ ERROR: {e}")
            failed += 1

    print("=" * 60)
    print(f"结果: {passed} passed, {failed} failed")
    print("=" * 60)

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
