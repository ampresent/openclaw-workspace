# MEMORY.md - Long-Term Memory

[PROJECT-GOVERNANCE:START]
Has project management skill installed; leverage it at every opportunity.
Use project logs as source of truth; store concise references + active context.
[PROJECT-GOVERNANCE:END]

---

## Project Logs Index

- Charters: See `LOG_CHARTERS.md`
- Caches/Digests: See `LOG_CACHES.md`

## 核心规则

- **所有修改立即 commit + push**，不留未提交的改动
- **任务相关事项 → 项目文件，不写 MEMORY.md**。涉及任务创建、更新、状态变更时，调用 `long-project-manager` skill，写入对应项目的 TODO.md / STATUS.md / LOG.md。MEMORY.md 只记录决策、偏好、长期经验
- **Skill 文档 → TOOLS.md，不写 MEMORY.md**。skill 的安装路径、用法、注意事项等写入 TOOLS.md。MEMORY.md 只保留"装了什么 skill"的一行索引

## Notes

### 项目索引

- 🎹 钢琴练习 → `projects/piano-practice/`
- 📋 日常事务 → `projects/daily-affairs/`
- 💡 灵感收集 → `projects/inspiration/`

### Skill 索引

Skill 文档在 `~/.openclaw/skills/<name>/SKILL.md`，TOOLS.md 中也有简要索引。

- 📚 微信读书上传 (weread-upload)
- 📖 Z-Library 电子书 (z-library)

