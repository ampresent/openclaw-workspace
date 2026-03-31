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

## Notes

### 项目索引

- 🎹 钢琴练习 → `projects/piano-practice/`
- 📋 日常事务 → `projects/daily-affairs/`
- 💡 灵感收集 → `projects/inspiration/`

项目状态、进度、待办全部在项目文件内，MEMORY.md 不重复记录。

### 📚 微信读书上传技能 (weread-upload)

**Skill 位置**：`~/.openclaw/skills/weread-upload/`

**上传命令**：
```bash
# 方法 1：Python 脚本（推荐）
python3 ~/.openclaw/skills/weread-upload/weread_upload.py <文件路径>

# 方法 2：Shell 脚本
~/.openclaw/skills/weread-upload/weread-upload <文件路径>
```

**支持格式**：PDF, TXT, EPUB, DOC, DOCX, MOBI, AZW3

**要求**：
- opencli browser 必须正在运行
- Chrome 必须已登录 weread.qq.com

**记忆点**：当用户要求上传书籍到微信读书时，使用 `weread-upload` skill（workspace/tools 下的工具已复制到 ~/.openclaw/skills/weread-upload/）。

### 📖 Z-Library 技能 (z-library)

**Skill 位置**：`~/.openclaw/skills/z-library/`

**搜索命令**：
```bash
python3 ~/.openclaw/skills/z-library/weread_upload.py <搜索词>
```

**功能**：
- 搜索 Z-Library 电子书
- 获取书籍标题、作者、下载链接
- 自动尝试多个 Z-Library 镜像站点

**要求**：
- opencli browser 必须正在运行

**可用镜像站点**：
- singlelogin.re / singlelogin.se / singlelogin.me (官方登录)
- z-library.se / z-library.sk (主镜像)
- 1lib.sk / 1lib.io / 1lib.domains
- b-ok.org / b-ok.cc / b-ok.asia
- zh.nyumiyu101.ru (国内镜像)

**记忆点**：当用户需要搜索或下载电子书时，使用 `z-library` skill。官网经常被封，使用 skill 中的镜像站点。

