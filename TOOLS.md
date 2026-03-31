# TOOLS.md - Local Notes

Skills define _how_ tools work. This file is for _your_ specifics — the stuff that's unique to your setup.

## What Goes Here

Things like:

- Camera names and locations
- SSH hosts and aliases
- Preferred voices for TTS
- Speaker/room names
- Device nicknames
- Anything environment-specific

## Examples

```markdown
### Cameras

- living-room → Main area, 180° wide angle
- front-door → Entrance, motion-triggered

### SSH

- home-server → 192.168.1.100, user: admin

### TTS

- Preferred voice: "Nova" (warm, slightly British)
- Default speaker: Kitchen HomePod
```

## Why Separate?

Skills are shared. Your setup is yours. Keeping them apart means you can update skills without losing your notes, and share skills without leaking your infrastructure.

---

Add whatever helps you do your job. This is your cheat sheet.

---

## 📚 微信读书上传 (weread-upload)

**Skill 位置**：`~/.openclaw/skills/weread-upload/`

**命令**：
```bash
# 方法 1：Python 脚本（推荐）
python3 ~/.openclaw/skills/weread-upload/weread_upload.py <文件路径>

# 方法 2：Shell 脚本
~/.openclaw/skills/weread-upload/weread-upload <文件路径>
```

**支持格式**：pdf, txt, epub, doc, docx, mobi, azw3

**要求**：
- opencli browser 必须正在运行
- Chrome 浏览器必须已登录 weread.qq.com

**注意**：上传 PDF 到微信读书时，优先使用此 skill。

---

## 📖 Z-Library (z-library)

**Skill 位置**：`~/.openclaw/skills/z-library/`

**搜索命令**：
```bash
python3 ~/.openclaw/skills/z-library/weread_upload.py <搜索词>
```

**功能**：搜索和下载 Z-Library 电子书

**要求**：opencli browser 必须正在运行

**注意**：搜索电子书时，使用此 skill。

[PROJECT-GOVERNANCE:START]
Subagents may use session tool and spawn subagents when needed for project work.
Subagents may use any available tool needed to complete work.
Prefer sandboxed tool execution; warn if sandboxing is disabled.
Dangerous actions policy: Full autonomy (subagents can do anything it takes)
[PROJECT-GOVERNANCE:END]
