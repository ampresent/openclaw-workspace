# runb-tui

Lazygit 风格的 TUI 管理工具，用于管理 [runb](../runb/) 容器运行时。

## 截图预览

```
⬢ runb — Lightweight OCI Container Runtime

 ▸ Containers    Layers    Overlay    System
 ──────────────────────────────────────────────────────────────────────────────
 Containers (2)
 ▸ nginx          ● running
   redis          ○ created                                          Select a container
                                                              State: running
 j/k Navigate   s Start   k Stop   d Delete   u Upgrade   r Refresh   PID: 1234
                                                                       Bundle: /opt/bundle-nginx
 ──────────────────────────────────────────────────────────────────────────────
 Tab Next   1-4 Tab   Ctrl+Q Quit                               runb-tui v0.1.0
```

## 快速开始

```bash
cd projects/runb-tui
npm install
npm run build
node dist/index.js
```

或开发模式：

```bash
npm run dev
```

## 快捷键

### 全局
| 键 | 功能 |
|---|---|
| `Tab` | 下一个模块 |
| `1-4` | 直接跳转模块 |
| `Ctrl+Q` | 退出 |

### Containers 视图
| 键 | 功能 |
|---|---|
| `j/k` | 上下导航 |
| `s` | 启动容器 |
| `k` | 停止容器 |
| `d` | 删除容器 |
| `u` | 热升级 |
| `r` | 刷新 |

### Layers 视图
| 键 | 功能 |
|---|---|
| `c` | 切换容器 |
| `i` | 初始化 layer |
| `m` | 提交 layer |
| `b` | 运行基准测试 |

### Overlay 视图
| 键 | 功能 |
|---|---|
| `c` | 切换容器 |
| `p` | 挂载 overlay |
| `t` | 卸载 overlay |
| `v` | 验证 overlay |

## 架构

```
src/
├── app.tsx              # 入口
├── App.tsx              # 主组件 + Tab 路由
├── components/
│   ├── Shared.tsx       # 通用组件 (TabBar, SplitPane, ListItem...)
│   ├── ContainersView.tsx
│   ├── LayersView.tsx
│   ├── OverlayView.tsx
│   └── SystemView.tsx
├── hooks/
│   └── useSelection.ts  # 列表选择 + Tab 状态管理
└── utils/
    └── runb.ts          # runb 数据读取 + 命令执行
```

## 依赖

- `ink` — React for CLI
- `react` — UI 组件
- `tsx` — 开发用 TS 运行时

## License

MIT
