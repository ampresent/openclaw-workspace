# Mock Backend — 薄框架 + OpenClaw 决策

## 核心理念

**框架不知道任何设备信息。它只做拦截和注入。**

所有决策由 OpenClaw（AI）完成：查阅驱动源码、搜索手册、分析上下文、决定返回值。

## 架构

```
┌──────────────────────────────────────────────┐
│  mock 框架 (本进程)                           │
│                                              │
│  GDB → 断点命中 → 快照上下文 → HTTP API       │
│                        ↑                     │
│                        │ 等待响应             │
└────────────────────────┼─────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────┐
│  OpenClaw (AI agent)                         │
│                                              │
│  1. GET /event ← 收到断点事件                 │
│  2. grep 驱动源码 ← 查驱动怎么初始化这个寄存器 │
│  3. grep 手册 ← 查寄存器位定义                │
│  4. 分析上下文 ← 当前其他寄存器值             │
│  5. POST /respond → 注入返回值                │
└──────────────────────────────────────────────┘
```

## 文件结构

```
mock-backend/
├── src/
│   └── mock.py              # 唯一核心：GDB + HTTP API + 事件循环
├── references/              # 参考资料（按需查阅，不预加载）
│   ├── netdev.c             # Linux e1000e 驱动源码 (226K)
│   ├── defines.h            # 寄存器位定义 (36K)
│   ├── e1000.h              # 驱动数据结构 (19K)
│   ├── hw.h                 # 硬件抽象层 (19K)
│   └── regs.h               # 寄存器地址映射 (14K)
├── skills/
│   └── mock-backend/
│       └── SKILL.md         # OpenClaw 控制端 skill
├── tests/
│   └── test_api.py          # API 单元测试
├── README.md
├── STATUS.md
└── TODO.md
```

## HTTP API

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/state` | 当前状态和断点列表 |
| GET | `/event` | 阻塞等待断点命中事件 |
| POST | `/respond` | 注入值 `{"value":"0x...","register":"rax"}` |
| POST | `/continue` | 继续执行（不注入） |
| POST | `/breakpoint` | 设置断点 `{"location":"*0x..."}` |
| DELETE | `/breakpoint/{id}` | 删除断点 |
| GET | `/memory?addr=X&size=Y` | 读取目标内存 |
| POST | `/memory` | 写入目标内存 |
| GET | `/registers` | 读取寄存器 |
| POST | `/register` | 写入寄存器 |

## 用法

```bash
# 启动 mock
python src/mock.py /path/to/test_program -b "*0xFEBC0008" --port 19876

# OpenClaw 通过 API 交互
curl -s http://127.0.0.1:19876/event       # 等断点
curl -s http://127.0.0.1:19876/registers   # 看寄存器
grep -n "E1000_STATUS" references/regs.h   # 查文档
curl -s -X POST http://127.0.0.1:19876/respond -d '{"value":"0x83"}'
```

## 设计原则

1. **框架零知识** — 不知道 e1000、不知道寄存器含义
2. **按需查阅** — 只在需要时搜索源码/手册，不预加载
3. **AI 决策** — 所有 mock 响应由 AI 分析后决定
4. **可扩展** — 支持任何设备，只需提供参考文档

## 最近更新时间
- 2026-04-07 by Agent
