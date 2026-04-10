---
name: spaced-repetition
description: >
  Spaced repetition / memory curve learning tracker. When the user says "需要复习",
  "复习一下", "我要学", "add to review list", "study", or any learning review request,
  use this skill. It manages a learning list with Ebbinghaus-style review intervals,
  links related project files, and tracks review history.
---

# Spaced Repetition Learning Tracker

基于艾宾浩斯遗忘曲线的学习复习管理系统。

## 核心文件

- **学习列表**: `memory/learning-list.json` — 所有待复习条目
- **复习日志**: `memory/learning-log.md` — 复习记录

## 触发条件

用户说了以下内容时触发本 skill：
- "需要复习"、"复习一下"、"帮我复习"
- "我要学"、"学习这个"、"add to review"
- 提到某个知识/概念需要巩固

## 复习间隔（艾宾浩斯曲线）

| 复习次数 | 间隔 | 说明 |
|---------|------|------|
| 第1次 | 1天后 | 首次复习 |
| 第2次 | 2天后 | 第二次 |
| 第3次 | 4天后 | 第三次 |
| 第4次 | 7天后 | 第四次 |
| 第5次 | 15天后 | 第五次 |
| 第6次 | 30天后 | 第六次 |
| 第7次 | 60天后 | 长期记忆 |
| 第8次 | 90天后 | 巩固 |

## 操作流程

### 1. 添加学习条目

当用户说需要复习某知识时：

1. 读取 `memory/learning-list.json`（不存在则创建）
2. 创建条目：
```json
{
  "id": "唯一ID（用日期+简写，如 2026-04-06-piano-jazz-chords）",
  "topic": "复习主题名称",
  "description": "简要描述",
  "relatedLinks": ["关联文件或项目的路径"],
  "addedAt": "ISO 时间戳",
  "reviewCount": 0,
  "nextReview": "首次复习日期（明天）",
  "history": [],
  "status": "active"
}
```
3. 保存文件
4. 确认给用户：条目已添加 + 首次复习日期

**关联链接收集**（主动搜索）：
- 搜索 workspace 中相关文件（grep/topic 关键词）
- 搜索 `projects/` 目录下相关项目
- 用户当前打开/讨论的文件
- 询问用户是否有其他关联资源

### 2. 检查今日待复习

当用户问"今天要复习什么"或定时检查时：

1. 读取 `memory/learning-list.json`
2. 筛选 `nextReview <= 今天` 且 `status == "active"` 的条目
3. 列出待复习条目 + 关联链接
4. 提醒用户复习

### 3. 完成复习

用户确认复习完一个条目后：

1. 找到对应条目，`reviewCount += 1`
2. 根据新的 reviewCount 查表计算下次复习日期
3. 将本次记录追加到 `history` 数组：
```json
{
  "reviewedAt": "ISO 时间戳",
  "count": 当前次数,
  "nextInterval": "下次间隔天数",
  "userNote": "用户备注（如有）"
}
```
4. 如果 reviewCount >= 8，将 status 改为 `"mastered"`
5. 保存文件

### 4. 查看学习列表

当用户想看所有学习条目时：
- 展示所有 active 条目 + 复习进度
- 展示已 mastered 的条目（可折叠）

### 5. Cron 提醒

可选：设置每日 cron job 检查今日待复习项并提醒用户。

```json
{
  "name": "daily-review-reminder",
  "schedule": { "kind": "cron", "expr": "0 9 * * *", "tz": "Asia/Shanghai" },
  "payload": { "kind": "agentTurn", "message": "检查 memory/learning-list.json 中今天需要复习的条目，如果有待复习项，列出并提醒用户。" },
  "sessionTarget": "isolated",
  "delivery": { "mode": "announce" }
}
```

## 格式约束

- **Discord/WhatsApp**：用列表格式，不用 markdown 表格
- 每个条目显示：主题 + 复习进度 (2/8) + 下次复习日期 + 关联链接数
- mastered 的条目加 ✅ 标记
