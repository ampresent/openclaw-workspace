# SolarFlare (sfc) — 第二个用例

## 为什么选 SolarFlare？

SolarFlare 和 e1000 是两种**完全不同的设备交互模式**，正好验证框架的通用性：

| | e1000 | SolarFlare (sfc) |
|---|---|---|
| 交互方式 | 直接 MMIO 读写寄存器 | MCDI 邮箱命令/响应协议 |
| 驱动写什么 | `readl(reg_offset)` | 写命令到共享内存 → 敲 doorbell |
| 驱动读什么 | `readl(reg_offset)` | 等中断/轮询响应内存 |
| 复杂度 | 寄存器值 = 设备状态 | 命令响应 = 协议状态机 |

**关键差异**：SolarFlare 的 MCDI 是一个完整的 RPC 协议，驱动发命令（如 "获取 MAC 地址"、"设置滤波器"），设备返回响应。这比简单寄存器读写复杂得多——但对框架来说，**无区别**，因为框架只看地址和值，具体含义由 OpenClaw 决策。

## SolarFlare MCDI 协议概要

```
驱动                          NIC
  │                            │
  │  1. 写命令到 MCDI 请求区    │
  │  2. 敲 doorbell (MMIO 写)   │
  │                            │
  │          ......             │
  │                            │
  │  3. NIC 写响应到 MCDI 响应区│
  │  4. NIC 发中断              │
  │                            │
  │  5. 驱动读响应              │
  │                            │
```

### 关键概念

- **MCDI 请求/响应区**：BAR2 中的共享内存区域
- **Doorbell**：BAR0 中的一个 MMIO 地址，写入触发 NIC 处理命令
- **MCDI Header**：包含命令码、长度、请求 ID
- **MC_CMD_xxx**：具体命令（如 MC_CMD_GET_MAC_ADDRESSES）

### 关键 MCDI 命令

| 命令码 | 名称 | 驱动何时调用 |
|--------|------|-------------|
| 0x0101 | MC_CMD_DRV_ATTACH | probe 时附加驱动 |
| 0x0108 | MC_CMD_GET_MAC_ADDRESSES | 获取 MAC 地址 |
| 0x0201 | MC_CMD_GET_BOARD_CFG | 获取板卡配置 |
| 0x0301 | MC_CMD_INIT_RXQ | 初始化接收队列 |
| 0x0302 | MC_CMD_INIT_TXQ | 初始化发送队列 |
| 0x0110 | MC_CMD_GET_WORKAROUNDS | 获取固件 workaround 列表 |

## 参考资料

已下载到 `references/` 目录：

| 文件 | 内容 | 大小 |
|------|------|------|
| `sfc_net_driver.h` | 主数据结构、efx_nic | 68K |
| `sfc_efx.c` | probe/初始化流程 | 37K |
| `sfc_mcdi.c` | MCDI 协议实现 | 69K |
| `sfc_mcdi.h` | MCDI 状态机、命令宏 | 19K |
| `sfc_nic.c` | NIC 抽象层 | 11K |
| `sfc_nic.h` | NIC 类型定义 | 7K |
| `sfc_tx.c` | 发送路径 | 17K |
| `sfc_rx.c` | 接收路径 | 12K |
| `sfc_io.h` | IO 操作宏 | 7K |
| `sfc_ethtool.c` | ethtool 接口 | 9K |

## OpenClaw 决策流程示例

当驱动执行 `efx_mcdi_rpc(efx, MC_CMD_GET_MAC_ADDRESSES, ...)` 时：

1. **框架拦截**：检测到对 MCDI doorbell 地址的 MMIO 写
2. **框架上报**：`POST /event` — 地址、当前寄存器、写入的命令数据
3. **OpenClaw 决策**：
   ```bash
   # 查这个命令的格式
   grep -n "MC_CMD_GET_MAC_ADDRESSES" references/sfc_mcdi.h

   # 查驱动怎么解析响应
   grep -n "GET_MAC_ADDRESSES" references/sfc_net_driver.h

   # 决定：构造一个包含 MAC 地址的 MCDI 响应
   ```
4. **OpenClaw 注入**：将响应数据写入 MCDI 响应区，触发中断
5. **框架执行**：`POST /memory` + `POST /register`

## 与 e1000 对比的框架行为差异

### e1000（简单）
```
断点命中 @ E1000_STATUS
→ OpenClaw: grep "E1000_STATUS" references/regs.h
→ 返回 0x83 (LinkUp+FD+1000Mbps)
→ POST /respond {"value":"0x83"}
```

### SolarFlare（复杂）
```
断点命中 @ MCDI doorbell
→ OpenClaw: 读 MCDI 请求区 (GET /memory)
→ 解析 MCDI header (命令码=0x0108 GET_MAC_ADDRESSES)
→ grep "MC_CMD_GET_MAC_ADDRESSES" references/sfc_mcdi.h
→ grep "GET_MAC_ADDRESSES" references/sfc_net_driver.h
→ 构造响应数据
→ POST /memory (写入响应区)
→ POST /respond (模拟中断状态)
```

**框架完全不知道 MCDI 协议的存在**。它只看到 "有人写了一个 MMIO 地址"，然后等 OpenClaw 告诉它该怎么做。

## 最近更新时间
- 2026-04-07 by Agent
