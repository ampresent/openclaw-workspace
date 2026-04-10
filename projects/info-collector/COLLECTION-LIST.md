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
- **关键词**：AI、LLM、算力、芯片

| # | 信息源 | 形式 | 采集方式 |
|---|--------|------|----------|
| 1 | Hacker News (news.ycombinator.com) | 网站 | web_fetch 抓取首页 + Show HN |
| 2 | 机器之心 (jiqizhixin.com) | 微信公众号/网站 | mimo_web_search 或 web_fetch |
| 3 | 量子位 (qbitai.com) | 微信公众号/网站 | mimo_web_search 或 web_fetch |

- **输出**：`collected/news-tech-daily/YYYY-MM-DD.md`
- **格式**：标题 + 来源 + 一句话摘要 + 链接

### [ ] news-ai-weekly — AI 周报
- **频率**：每周
- **关键词**：大模型、Agent、多模态、RLHF

| # | 信息源 | 形式 | 采集方式 |
|---|--------|------|----------|
| 1 | ArXiv cs.CL / cs.AI / cs.LG | 论文预印本 | web_fetch 抓取 recent listings |
| 2 | Twitter/X AI 话题 | 社交媒体 | mimo_web_search 关键词搜索 |
| 3 | 知乎 AI 话题 | 问答社区 | mimo_web_search 关键词搜索 |
| 4 | 微信公众号（量子位、机器之心等） | 公众号文章 | mimo_web_search 搜狗微信搜索 |

- **输出**：`collected/news-ai-weekly/YYYY-WXX.md`
- **格式**：按主题分类 + 重要性标注

---

## 💰 金融 & 投资

### [ ] finance-fund-daily — 基金日报
- **频率**：每个交易日

| # | 信息源 | 形式 | 采集方式 |
|---|--------|------|----------|
| 1 | fundgz.1234567.com.cn | 实时估值 API | fund-query skill（Python 脚本） |
| 2 | fund.eastmoney.com | 净值数据 | fund-query skill（QDII fallback） |
| 3 | 持仓文件 portfolio.json | 本地 JSON | 直接读取 |

- **输出**：`collected/finance-fund-daily/YYYY-MM-DD.md`
- **格式**：表格（基金名、代码、估值、涨跌、持仓盈亏）

### [ ] finance-market-weekly — 市场周报
- **频率**：每周

| # | 信息源 | 形式 | 采集方式 |
|---|--------|------|----------|
| 1 | 新浪财经 (finance.sina.com.cn) | 新闻网站 | mimo_web_search 搜索周报/市场综述 |
| 2 | 华尔街见闻 (wallstreetcn.com) | 财经媒体 | mimo_web_search 搜索宏观/市场 |
| 3 | 金十数据 (jin10.com) | 数据快讯 | mimo_web_search 搜索周度数据 |

- **关注领域**：美股、黄金、宏观政策
- **输出**：`collected/finance-market-weekly/YYYY-WXX.md`
- **格式**：市场概况 + 重点事件 + 影响分析

---

## 🎵 音乐 & 演出

### [x] music-jazz-shanghai — 上海爵士演出
- **频率**：每周
- **关键词**：上海、爵士、演出

| # | 信息源 | 形式 | 采集方式 |
|---|--------|------|----------|
| 1 | 搜狗微信搜索 (weixin.sogou.com) | 微信公众号聚合 | browser 工具自动化搜索 |
| 2 | Blue Note Shanghai 官网 | 场馆官网 | web_fetch 抓取演出日历 |
| 3 | JZ Club 官网 | 场馆官网 | web_fetch 抓取演出日历 |

- **已有 cron job**：`d2eba45f`，每天 20:00 执行
- **输出**：`collected/music-jazz-shanghai/YYYY-MM-DD.md`
- **格式**：表格（演出名、场地、时间、票价、链接）

### [ ] music-new-releases — 新碟推荐
- **频率**：每月

| # | 信息源 | 形式 | 采集方式 |
|---|--------|------|----------|
| 1 | Discogs | 音乐数据库 | agent-discogs skill 搜索 |
| 2 | 豆瓣音乐 | 评分/评论网站 | mimo_web_search 搜索新碟 |
| 3 | Bandcamp | 独立音乐平台 | mimo_web_search 搜索新发行 |

- **风格偏好**：爵士、日本噪音/迷幻、暗黑工业/新古典
- **输出**：`collected/music-new-releases/YYYY-MM.md`

---

## ✈️ 旅游 & 机票

### [ ] airline-passes — 航空公司套票/随心飞
- **频率**：每周
- **关键词**：套票、随心飞、畅飞、想飞就飞、优享飞、安逸飞、畅飞中国、多次卡、季卡、权益卡

| # | 信息源 | 形式 | 采集方式 |
|---|--------|------|----------|
| 1 | 四川航空 (scal.com.cn) 官网/APP/小程序 | 航司官网 | mimo_web_search 搜索多组关键词 |
| 2 | 春秋航空 (china-sss.com) 官网/APP | 航司官网 | mimo_web_search 搜索「春秋航空 想飞就飞」 |
| 3 | 中国国航 (airchina.com.cn) 官网 | 航司官网 | mimo_web_search 搜索「国航 颐飞畅游卡 随心飞」 |
| 4 | 南方航空 (csair.com) 官网 | 航司官网 | mimo_web_search 搜索「南航 畅游中国」 |
| 5 | 华夏航空 官网 | 航司官网 | mimo_web_search 搜索「华夏航空 畅飞华夏」 |
| 6 | 深圳航空 官网 | 航司官网 | mimo_web_search 搜索「深圳航空 青年卡」 |
| 7 | 什么值得买 (smzdm.com) | 优惠聚合社区 | mimo_web_search 搜索「航司名 + 随心飞」 |
| 8 | 淘宝百科 (bk.taobao.com) | 产品百科 | mimo_web_search 搜索「航司名 + 套票」 |
| 9 | 微博/小红书 | 社交媒体 | mimo_web_search 搜索「航司名 + 想飞就飞 2026」 |
| 10 | 美团/飞猪旅行 | OTA 平台 | mimo_web_search 搜索「航司名 + 权益卡」 |

- **关注航司**：四川航空、春秋航空、国航、南航、华夏航空、深圳航空、吉祥航空等
- **关注产品类型**：想飞就飞、优享飞、随心飞、畅飞中国、安逸飞、畅飞华夏、多次卡、季卡、权益卡
- **关注要素**：价格、有效期、适用航线、退改规则、是否可转让、每单附加费用
- **⚠️ 搜索技巧**：每个航司至少用 3 组不同关键词搜索（产品名+历史名+通用名），避免遗漏
- **⚠️ 历史追踪**：关注航司换季节点（3月底/10月底/11月），此时通常推新品
- **输出**：`collected/airline-passes/YYYY-MM-DD.md`
- **格式**：航司、产品名、价格、有效期、航线范围、限制条件、购买链接

---

## 🧠 AI 学习资料

### [ ] ai-papers-arxiv — ArXiv AI 论文
- **频率**：每周

| # | 信息源 | 形式 | 采集方式 |
|---|--------|------|----------|
| 1 | ArXiv cs.CL (arxiv.org/list/cs.CL) | 论文预印本列表 | web_fetch 抓取 recent listings |
| 2 | ArXiv cs.AI (arxiv.org/list/cs.AI) | 论文预印本列表 | web_fetch 抓取 recent listings |
| 3 | ArXiv cs.LG (arxiv.org/list/cs.LG) | 论文预印本列表 | web_fetch 抓取 recent listings |
| 4 | Papers With Code (paperswithcode.com) | 论文+代码聚合 | mimo_web_search 搜索 trending |

- **筛选**：引用量 or 社区关注度
- **输出**：`collected/ai-papers-arxiv/YYYY-WXX.md`
- **格式**：标题 + 作者 + 一句话贡献 + 链接

### [ ] ai-courses-update — AI 课程更新
- **频率**：每月

| # | 信息源 | 形式 | 采集方式 |
|---|--------|------|----------|
| 1 | CS 224n (web.stanford.edu/class/cs224n/) | 课程主页 | web_fetch 检查新 lecture/assignment |
| 2 | CS 336 (stanford-cs336.github.io) | 课程主页 | web_fetch 检查新内容 |
| 3 | CMU 11-785 (deeplearning.cs.cmu.edu) | 课程主页 | web_fetch 检查新内容 |

- **输出**：`collected/ai-courses-update/YYYY-MM.md`

---

## 📋 项目待办

### [ ] project-status-weekly — 项目周报
- **频率**：每周

| # | 信息源 | 形式 | 采集方式 |
|---|--------|------|----------|
| 1 | projects/*/STATUS.md | 本地文件 | 直接读取所有 STATUS.md |
| 2 | projects/*/TODO.md | 本地文件 | 统计待办完成情况 |
| 3 | projects/*/LOG.md | 本地文件 | 汇总最近进展 |

- **输出**：`collected/project-status-weekly/YYYY-WXX.md`
- **格式**：项目名 + 状态 + 进度 + 下一步

---

## 信息源形式汇总

| 形式 | 说明 | 常用采集工具 |
|------|------|-------------|
| 🌐 网站 | 公开网页 | `web_fetch`、`mimo_web_search` |
| 🔌 API | 结构化数据接口 | Python 脚本、`web_fetch` |
| 📱 微信公众号 | 搜狗微信搜索聚合 | `browser` 自动化、`mimo_web_search` |
| 💬 社交媒体 | Twitter/X、知乎 | `mimo_web_search` |
| 📄 论文预印本 | ArXiv | `web_fetch` 抓取列表 |
| 📂 本地文件 | JSON、Markdown | `read` 直接读取 |
| 🛠️ Skill | 已封装的工具 | 直接调用对应 skill |

---

## 如何添加新任务

在对应分类下添加：

```markdown
### [ ] <task-id> — <任务名称>
- **频率**：<每日/每周/每月/按需>
- **关键词**：<搜索关键词>

| # | 信息源 | 形式 | 采集方式 |
|---|--------|------|----------|
| 1 | <来源名称> | <形式> | <工具/方法> |

- **输出**：`collected/<task-id>/YYYY-MM-DD.md`
- **格式**：<输出格式说明>
```
