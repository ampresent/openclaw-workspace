# 信息收集管理器

> 创建时间：2026-04-06

## 使用方式

当用户说「收集信息」时，按以下流程执行：

1. 读取 `COLLECTION-LIST.md`，获取所有启用的收集任务
2. 对每个启用的任务，执行信息采集
3. 结果存放到 `collected/<任务ID>/YYYY-MM-DD.md`
4. 更新 `STATUS.md` 记录本次收集情况

## 目录结构

```
info-collector/
├── README.md            ← 本文件（使用说明）
├── COLLECTION-LIST.md   ← 收集任务列表（核心配置）
├── STATUS.md            ← 收集状态跟踪
├── collected/           ← 收集结果存放
│   ├── <任务ID>/
│   │   ├── YYYY-MM-DD.md
│   │   └── ...
│   └── ...
└── templates/           ← 输出模板
    └── default.md
```

## 收集任务 ID 规范

- 使用小写英文 + 短横线
- 格式：`<领域>-<具体描述>`
- 例：`finance-fund-daily`、`ai-papers-weekly`、`travel-deals`

## 收集触发方式

- **手动触发**：用户说「收集信息」→ 执行所有 ENABLED 任务
- **指定任务**：用户说「收集 XXX」→ 只执行匹配的任务
- **定时触发**：可通过 cron 设置定时收集（需单独配置）
