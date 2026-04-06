# 信息收集任务列表

> 最后更新：2026-04-06

## 使用说明

- `[x]` = 启用，`[ ]` = 禁用
- 每个任务有唯一的 `ID`，用于存放到 `collected/<ID>/` 目录
- 添加/修改任务后，下次「收集信息」自动生效

---

## 📰 资讯 & 新闻

### [ ] news-tech-daily — 科技日报
- **频率**：每日
- **来源**：Hacker News、机器之心、量子位
- **关键词**：AI、LLM、算力、芯片
- **输出**：`collected/news-tech-daily/YYYY-MM-DD.md`
- **格式**：标题 + 来源 + 一句话摘要 + 链接

### [ ] news-ai-weekly — AI 周报
- **频率**：每周
- **来源**：ArXiv、Twitter/X、知乎、公众号
- **关键词**：大模型、Agent、多模态、RLHF
- **输出**：`collected/news-ai-weekly/YYYY-WXX.md`
- **格式**：按主题分类 + 重要性标注

---

## 💰 金融 & 投资

### [ ] finance-fund-daily — 基金日报
- **频率**：每个交易日
- **来源**：fund-query skill
- **内容**：持仓基金当日估值/净值、涨跌幅
- **输出**：`collected/finance-fund-daily/YYYY-MM-DD.md`
- **格式**：表格（基金名、代码、估值、涨跌、持仓盈亏）

### [ ] finance-market-weekly — 市场周报
- **频率**：每周
- **来源**：财经新闻、市场数据
- **关键词**：美股、黄金、宏观政策
- **输出**：`collected/finance-market-weekly/YYYY-WXX.md`
- **格式**：市场概况 + 重点事件 + 影响分析

---

## 🎵 音乐 & 演出

### [x] music-jazz-shanghai — 上海爵士演出
- **频率**：每周
- **来源**：weixin-jazz-search skill
- **关键词**：上海、爵士、演出
- **输出**：`collected/music-jazz-shanghai/YYYY-MM-DD.md`
- **格式**：表格（演出名、场地、时间、票价、链接）

### [ ] music-new-releases — 新碟推荐
- **频率**：每月
- **来源**：Discogs、豆瓣、Bandcamp
- **风格偏好**：爵士、日本噪音/迷幻、暗黑工业/新古典
- **输出**：`collected/music-new-releases/YYYY-MM.md`

---

## ✈️ 旅游 & 机票

### [ ] travel-deals — 机票酒店优惠
- **频率**：按需
- **来源**：携程、去哪儿、飞猪
- **关注航线**：上海↔成都、上海↔各地
- **输出**：`collected/travel-deals/YYYY-MM-DD.md`
- **格式**：出发地、目的地、日期、价格、来源

---

## 🧠 AI 学习资料

### [ ] ai-papers-arxiv — ArXiv AI 论文
- **频率**：每周
- **来源**：ArXiv cs.CL、cs.AI、cs.LG
- **关键词**：LLM、Transformer、Agent、RLHF、Scaling Laws
- **筛选**：引用量 or 社区关注度
- **输出**：`collected/ai-papers-arxiv/YYYY-WXX.md`
- **格式**：标题 + 作者 + 一句话贡献 + 链接

### [ ] ai-courses-update — AI 课程更新
- **频率**：每月
- **来源**：Stanford CS 224n、CS 336、CMU 11-785 课程主页
- **内容**：新 lecture 视频、新 assignment、新 reading
- **输出**：`collected/ai-courses-update/YYYY-MM.md`

---

## 📋 项目待办

### [ ] project-status-weekly — 项目周报
- **频率**：每周
- **来源**：所有 `projects/*/STATUS.md`
- **内容**：各项目进展、阻塞、下一步
- **输出**：`collected/project-status-weekly/YYYY-WXX.md`
- **格式**：项目名 + 状态 + 进度 + 下一步

---

## 如何添加新任务

在对应分类下添加：

```markdown
### [ ] <task-id> — <任务名称>
- **频率**：<每日/每周/每月/按需>
- **来源**：<数据来源>
- **关键词**：<搜索关键词>
- **输出**：`collected/<task-id>/YYYY-MM-DD.md`
- **格式**：<输出格式说明>
```
