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

## 已安装 Skill 索引

Skill 文档在 `~/.openclaw/skills/<name>/SKILL.md`，不在本文件中。

- 📚 weread-upload — 上传电子书到微信读书
- 📖 z-library — Z-Library 电子书搜索下载

---

[PROJECT-GOVERNANCE:START]
Subagents may use session tool and spawn subagents when needed for project work.
Subagents may use any tool needed to complete work.
Prefer sandboxed tool execution; warn if sandboxing is disabled.
Dangerous actions policy: Full autonomy (subagents can do anything it takes)
[PROJECT-GOVERNANCE:END]
