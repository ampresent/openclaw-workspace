# TOOLS.md - Local Notes

Skills 定义工具怎么用，这个文件记录你自己的环境细节。

---

## 已安装 Skills（23 个）

Skill 文档在 `skills/<name>/SKILL.md`。

### 简历 & 求职
- 📄 resume-assistant — 简历润色、定制、评分、检查清单、多格式导出

### 投资 & 金融
- 💰 fund-query — 场外基金查询（实时估值、净值、涨跌）

### 音乐 & 文化
- 🎵 music-recommender — 基于豆瓣收藏推荐专辑
- 🎸 agent-discogs — Discogs 音乐数据库查询
- 📀 douban-sync-skill — 豆瓣音乐同步导出
- 🎷 weixin-jazz-search — 微信公众号上海爵士演出搜索

### 开发工作流（superpowers）
- 🦸 using-superpowers — 超能力总入口
- 📝 writing-plans — 写执行计划
- ⚡ executing-plans — 执行计划
- 🤖 subagent-driven-development — 子代理驱动开发
- 🔀 dispatching-parallel-agents — 并行分派
- 🧪 test-driven-development — TDD
- 🐛 systematic-debugging — 系统化调试
- ✅ verification-before-completion — 完成前验证
- 🧠 brainstorming — 头脑风暴
- 🔍 requesting-code-review / receiving-code-review — 代码审查
- 🏁 finishing-a-development-branch — 分支收尾
- 🌳 using-git-worktrees — Git worktrees
- ✍️ writing-skills — 编写新 skill

### 工具类
- 📚 weread-upload — 上传电子书到微信读书
- 📖 z-library — Z-Library 电子书搜索下载

### 项目管理
- 📋 long-project-manager — 长期项目管理
- 📊 mermaid — Mermaid 图表生成

---

## 微信读书上传

```bash
python3 tools/weread_upload.py <文件路径>
# 或
tools/weread-upload <文件路径>
```
