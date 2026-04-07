"""
AI 响应引擎 — 根据上下文生成设备 mock 响应。

核心流程：
1. 接收拦截事件（什么地址被读/写了）
2. 查阅上下文（相关文档、寄存器定义、设备状态）
3. 调用 LLM 生成响应
4. 返回 mock 值 + 可选的状态更新

优化：
- 缓存：相同查询不重复调 LLM
- 状态追踪：设备寄存器不是无状态的，读某寄存器可能清中断
- 规则优先：已知的简单响应走规则，复杂情况才调 LLM
"""

import json
import logging
import hashlib
from typing import Optional, Dict, Any, List
from dataclasses import dataclass, field
from .context import ContextEngine
from .intercept import InterceptEvent, InterceptAction, AccessType

logger = logging.getLogger(__name__)


@dataclass
class DeviceState:
    """设备虚拟状态"""
    registers: Dict[int, int] = field(default_factory=dict)  # offset -> value
    interrupt_status: int = 0
    config_space: Dict[int, int] = field(default_factory=dict)  # PCI config
    custom: Dict[str, Any] = field(default_factory=dict)

    def get_reg(self, offset: int, default: int = 0) -> int:
        return self.registers.get(offset, default)

    def set_reg(self, offset: int, value: int):
        self.registers[offset] = value & 0xFFFFFFFF


class AIResponseEngine:
    """
    AI 驱动的响应引擎。

    用法：
        engine = AIResponseEngine(context_engine, llm_client)
        action = engine.handle_event(event, device_state)
    """

    def __init__(
        self,
        context: ContextEngine,
        llm_client=None,   # OpenAI 兼容客户端
        model: str = "gpt-4o-mini",
        enable_cache: bool = True,
    ):
        self.context = context
        self.llm = llm_client
        self.model = model
        self._cache: Dict[str, InterceptAction] = {}
        self._enable_cache = enable_cache
        self._rules: List[Dict] = []  # 手动规则（优先于 LLM）

    def add_rule(
        self,
        address: int,
        access_type: AccessType,
        handler,
        address_mask: int = 0xFFFFFFFF,
        description: str = "",
    ):
        """
        添加手动规则（优先于 LLM 调用）。

        用于已知的简单响应，避免每次都调 LLM。
        address: 要匹配的地址
        address_mask: 匹配掩码，(event.address & mask) == (address & mask) 时命中
        """
        self._rules.append({
            "address": address,
            "mask": address_mask,
            "access_type": access_type,
            "handler": handler,
            "description": description,
        })
        # 添加规则后清除缓存
        self._cache.clear()

    def handle_event(
        self,
        event: InterceptEvent,
        state: DeviceState,
    ) -> InterceptAction:
        """
        处理拦截事件，生成 mock 响应。

        优先级：
        1. 缓存
        2. 手动规则
        3. LLM 生成
        """
        # 生成缓存键
        cache_key = self._make_cache_key(event, state)

        # 1. 检查缓存
        if self._enable_cache and cache_key in self._cache:
            logger.debug(f"缓存命中: 0x{event.address:x}")
            return self._cache[cache_key]

        # 2. 检查手动规则
        for rule in self._rules:
            if (event.address & rule["mask"]) == (rule["address"] & rule["mask"]):
                if event.access_type == rule["access_type"] or rule["access_type"] is None:
                    action = rule["handler"](event, state)
                    if action:
                        if self._enable_cache:
                            self._cache[cache_key] = action
                        return action

        # 3. 调用 LLM
        action = self._llm_generate(event, state)

        if self._enable_cache and action:
            self._cache[cache_key] = action

        return action

    def _make_cache_key(self, event: InterceptEvent, state: DeviceState) -> str:
        """生成缓存键"""
        key_data = f"{event.access_type.value}:{event.address:x}:{event.value}:{state.interrupt_status}"
        return hashlib.md5(key_data.encode()).hexdigest()

    def _llm_generate(
        self,
        event: InterceptEvent,
        state: DeviceState,
    ) -> InterceptAction:
        """调用 LLM 生成 mock 响应"""
        if not self.llm:
            logger.warning("LLM 客户端未配置，返回默认值 0")
            return InterceptAction(return_value=0, log_message="LLM 未配置，返回 0")

        # 1. 检索相关上下文
        context_text = self.context.dump_context_for_query(
            f"register at offset 0x{event.address:x} "
            f"access type {event.access_type.value} "
            f"function {event.instruction}"
        )

        # 2. 构造 prompt
        prompt = self._build_prompt(event, state, context_text)

        # 3. 调用 LLM
        try:
            response = self.llm.chat.completions.create(
                model=self.model,
                messages=[
                    {
                        "role": "system",
                        "content": (
                            "你是一个硬件设备模拟器。你的任务是根据设备文档和当前状态，"
                            "生成驱动程序期望读取到的寄存器值。\n\n"
                            "规则：\n"
                            "- 只返回 JSON，不要其他文字\n"
                            "- 值用十六进制字符串表示（如 \"0x1234\"）\n"
                            "- 如果不确定，返回一个合理的默认值\n"
                            "- 考虑设备状态机的连贯性"
                        ),
                    },
                    {"role": "user", "content": prompt},
                ],
                temperature=0.1,
                max_tokens=500,
            )

            reply = response.choices[0].message.content.strip()
            return self._parse_llm_reply(reply, event)

        except Exception as e:
            logger.error(f"LLM 调用失败: {e}")
            return InterceptAction(
                return_value=0,
                log_message=f"LLM 错误: {e}",
            )

    def _build_prompt(
        self,
        event: InterceptEvent,
        state: DeviceState,
        context_text: str,
    ) -> str:
        """构造 LLM prompt"""
        # 当前设备寄存器状态摘要
        reg_state = "\n".join(
            f"  offset 0x{offset:x}: 0x{value:08x}"
            for offset, value in sorted(state.registers.items())
        ) if state.registers else "  (no registers initialized)"

        return f"""## 任务
驱动程序正在访问设备寄存器。请生成它期望读取到的值。

## 访问信息
- 访问类型: {event.access_type.value}
- 地址偏移: 0x{event.address:x}
- 访问大小: {event.size} bytes
- 写入值: {f"0x{event.value:x}" if event.value is not None else "N/A (读操作)"}
- 调用函数: {event.instruction}
- 寄存器上下文: rax=0x{event.extra.get('rax', 0):x}, rdx=0x{event.extra.get('rdx', 0):x}

## 当前设备状态
### 寄存器
{reg_state}
### 中断状态: 0x{state.interrupt_status:x}

## 设备文档/知识
{context_text[:3000]}

## 返回格式
```json
{{
  "value": "0x...",
  "reason": "为什么返回这个值",
  "state_updates": {{
    "reg_offset": "new_value",
    ...
  }}
}}
```"""

    def _parse_llm_reply(self, reply: str, event: InterceptEvent) -> InterceptAction:
        """解析 LLM 响应"""
        # 提取 JSON
        json_match = re.search(r'\{[^{}]+\}', reply, re.DOTALL)
        if not json_match:
            logger.warning(f"无法从 LLM 响应中提取 JSON: {reply[:200]}")
            return InterceptAction(return_value=0, log_message="LLM 响应格式错误")

        try:
            data = json.loads(json_match.group())
        except json.JSONDecodeError:
            return InterceptAction(return_value=0, log_message="LLM 响应 JSON 解析失败")

        # 解析返回值
        val_str = data.get("value", "0x0")
        if isinstance(val_str, str):
            val = int(val_str, 16) if val_str.startswith("0x") else int(val_str)
        else:
            val = int(val_str)

        reason = data.get("reason", "")

        action = InterceptAction(
            return_value=val,
            log_message=f"LLM mock: 0x{event.address:x} -> 0x{val:x} ({reason})",
        )

        return action
