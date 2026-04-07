---
name: mock-backend
description: 控制 mock backend 框架，拦截设备访问，查阅参考文档/源码，AI 决策 mock 响应。用于在不连接真实硬件的情况下测试驱动程序。
---

# Mock Backend — OpenClaw 控制端

## 工作流程

```
1. 启动 mock 框架: python src/mock.py <target> -b <breakpoint>
2. 框架暴露 HTTP API (默认 http://127.0.0.1:19876)
3. 通过 curl / fetch 与 API 交互:
   - GET  /event         → 等待断点命中，返回事件详情
   - POST /respond       → 注入值并继续执行
   - GET  /registers     → 读取寄存器
   - GET  /memory        → 读取内存
   - POST /breakpoint    → 设置断点
```

## 断点事件处理流程

收到断点事件后，按以下步骤决策：

1. **分析事件**：看地址、寄存器、指令、调用栈
2. **查阅源码**：在 `references/` 目录 grep 驱动源码
   - `grep -n "目标地址偏移" references/*.h` — 找寄存器定义
   - `grep -n "函数名" references/netdev.c` — 看驱动怎么用这个寄存器
3. **查阅手册**：在 `references/` 目录搜索
4. **决策返回值**：根据文档和驱动逻辑决定返回什么
5. **注入响应**：POST /respond

## 查阅资料的方法

不需要提前加载所有文档。按需搜索：

```bash
# 找寄存器定义
grep -n "E1000_STATUS" references/regs.h references/defines.h

# 看驱动怎么初始化这个寄存器
grep -n "E1000_STATUS" references/netdev.c

# 找寄存器位定义
grep -n "STATUS_.*=" references/defines.h

# 看驱动读写流程
grep -n "er32(STATUS)" references/netdev.c
```

## 响应决策原则

1. **优先查源码**：驱动的初始化代码告诉你它期望什么值
2. **其次查手册**：寄存器文档告诉你合法值的范围
3. **参考现有状态**：其他寄存器的值可能影响当前寄存器
4. **保守默认**：不确定时返回 0 或最安全的值

## 与 mock 框架的通信示例

```bash
# 等待断点
curl -s http://127.0.0.1:19876/event

# 查看寄存器
curl -s http://127.0.0.1:19876/registers

# 读内存
curl -s "http://127.0.0.1:19876/memory?addr=0xFEBC0008&size=4"

# 注入响应值到 rax 并继续
curl -s -X POST http://127.0.0.1:19876/respond \
  -H "Content-Type: application/json" \
  -d '{"value": "0x83", "register": "rax"}'

# 继续执行（不注入）
curl -s -X POST http://127.0.0.1:19876/continue
```
