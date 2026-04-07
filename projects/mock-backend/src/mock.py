"""
Mock Backend — 主入口。

用法：
    # 基础用法：启动 mock，拦截 e1000 MMIO 访问
    python -m src.mock --device e1000 --target ./my_kernel_module

    # 指定 LLM
    python -m src.mock --device e1000 --llm http://localhost:8000/v1

    # 自定义地址范围
    python -m src.mock --device e1000 --mmio-base 0xFEBC0000 --mmio-size 128K
"""

import argparse
import logging
import signal
import sys
import time
from pathlib import Path

from .gdb_backend import GDBMIBackend
from .intercept import InterceptLayer, InterceptAction
from .context import ContextEngine
from .response import AIResponseEngine
from .devices.nic import E1000Device, E1000_STATUS
from .devices.base import DeviceRegistry

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [%(levelname)s] %(name)s: %(message)s',
    datefmt='%H:%M:%S',
)
logger = logging.getLogger("mock-backend")


def create_device(device_type: str, **kwargs):
    """工厂函数：创建 mock 设备"""
    if device_type == "e1000":
        mac = kwargs.get('mac', b'\x52\x54\x00\x12\x34\x56')
        if isinstance(mac, str):
            mac = bytes.fromhex(mac.replace(':', ''))
        return E1000Device(mac_address=mac)
    else:
        raise ValueError(f"未知设备类型: {device_type}")


def run_mock(
    device_type: str,
    target_program: str,
    target_args: list = None,
    mmio_base: int = 0xFEBC0000,
    llm_url: str = None,
    llm_model: str = "gpt-4o-mini",
    reference_files: list = None,
    mac_address: str = None,
    verbose: bool = False,
):
    """
    运行 mock 后端。

    流程：
    1. 创建 mock 设备
    2. 加载参考文档
    3. 启动 GDB，加载目标程序
    4. 在 MMIO 地址设置断点
    5. 事件循环：断点命中 → mock 响应 → 继续
    """
    if verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    # 1. 创建设备
    kwargs = {}
    if mac_address:
        kwargs['mac'] = mac_address
    device = create_device(device_type, **kwargs)
    device.init_state()
    logger.info(f"设备创建: {device.name}")

    # 2. 加载参考文档
    if reference_files:
        for f in reference_files:
            device.load_reference(f)
            logger.info(f"加载参考文档: {f}")

    # 3. 设置 AI 引擎
    ai_engine = None
    if llm_url:
        try:
            import openai
            client = openai.OpenAI(
                base_url=llm_url,
                api_key="not-needed",
            )
            ai_engine = AIResponseEngine(
                context=device.context,
                llm_client=client,
                model=llm_model,
            )
            logger.info(f"AI 引擎: {llm_url} / {llm_model}")
        except ImportError:
            logger.warning("openai 库未安装，AI 引擎不可用")

    device.set_ai_engine(ai_engine)

    # 4. 启动 GDB
    gdb = GDBMIBackend()
    try:
        gdb.start(target_program, target_args or [])
    except Exception as e:
        logger.error(f"GDB 启动失败: {e}")
        return 1

    # 5. 拦截层
    intercept = InterceptLayer(gdb)

    # 在 MMIO 区域设置断点
    regions = device.get_mmio_regions()
    for region in regions:
        base = mmio_base + region.base_offset
        # 每 4 字节一个断点（实际中选择关键寄存器）
        # 先只设置关键寄存器的断点
        key_registers = [
            0x00000,  # CTRL
            0x00008,  # STATUS
            0x00010,  # EECD
            0x000C0,  # ICR
            0x000C8,  # ICS
            0x000D0,  # IMS
            0x000D8,  # IMC
            0x00100,  # RCTL
            0x02800,  # RDBAL
            0x02804,  # RDBAH
            0x02808,  # RDLEN
            0x02810,  # RDH
            0x02818,  # RDT
            0x00400,  # TCTL
            0x03800,  # TDBAL
            0x03804,  # TDBAH
            0x03808,  # TDLEN
            0x03810,  # TDH
            0x03818,  # TDT
            0x05400,  # RAL
            0x05404,  # RAH
        ]
        for reg_offset in key_registers:
            addr = base + reg_offset
            try:
                intercept.register_address(
                    name=f"e1000+0x{reg_offset:05x}",
                    address=addr,
                    size=4,
                    handler=device.handle_access,
                )
            except Exception as e:
                logger.warning(f"设置断点失败 @ 0x{addr:x}: {e}")

    logger.info(f"已设置 {len(key_registers)} 个断点, 启动目标程序...")

    # 6. 主事件循环
    intercept.add_global_handler(lambda event: None)  # 全局日志

    running = True
    def signal_handler(sig, frame):
        nonlocal running
        running = False
        logger.info("收到中断信号，退出...")

    signal.signal(signal.SIGINT, signal_handler)

    try:
        gdb.continue_execution()
        while running:
            hit = gdb.wait_for_stop(timeout=1.0)
            if hit:
                action = intercept.handle_breakpoint_hit(hit)
                intercept.execute_action(action)

                # 继续执行
                if action.should_continue:
                    gdb.continue_execution()
            else:
                # 超时，检查 GDB 是否还活着
                if gdb._proc and gdb._proc.poll() is not None:
                    logger.info("目标程序已退出")
                    break
    except Exception as e:
        logger.error(f"事件循环异常: {e}")
    finally:
        # 输出统计
        logger.info("=== Mock 统计 ===")
        logger.info(f"总拦截事件: {len(intercept.event_log)}")
        gdb.stop()

    return 0


def main():
    parser = argparse.ArgumentParser(
        description="Mock Backend — AI 驱动的设备/后端模拟框架",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--device", required=True, help="设备类型 (e1000)")
    parser.add_argument("--target", required=True, help="目标程序路径")
    parser.add_argument("--args", nargs="*", default=[], help="目标程序参数")
    parser.add_argument("--mmio-base", default="0xFEBC0000", help="MMIO 基地址")
    parser.add_argument("--llm", help="LLM API URL (OpenAI 兼容)")
    parser.add_argument("--llm-model", default="gpt-4o-mini", help="LLM 模型名")
    parser.add_argument("--ref", nargs="*", help="参考文档路径")
    parser.add_argument("--mac", help="MAC 地址 (e1000)")
    parser.add_argument("-v", "--verbose", action="store_true", help="详细输出")

    args = parser.parse_args()

    mmio_base = int(args.mmio_base, 16) if args.mmio_base.startswith("0x") else int(args.mmio_base)

    return run_mock(
        device_type=args.device,
        target_program=args.target,
        target_args=args.args,
        mmio_base=mmio_base,
        llm_url=args.llm,
        llm_model=args.llm_model,
        reference_files=args.ref,
        mac_address=args.mac,
        verbose=args.verbose,
    )


if __name__ == "__main__":
    sys.exit(main())
