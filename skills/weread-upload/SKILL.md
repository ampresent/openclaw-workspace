---
name: weread-upload
description: 上传电子书到微信读书
---

# 微信读书上传

## 命令

```bash
# 方法 1：Python 脚本（推荐）
python3 ~/.openclaw/skills/weread-upload/weread_upload.py <文件路径>

# 方法 2：Shell 脚本
~/.openclaw/skills/weread-upload/weread-upload <文件路径>
```

## 支持格式

pdf, txt, epub, doc, docx, mobi, azw3

## 要求

- opencli browser 必须正在运行
- Chrome 浏览器必须已登录 weread.qq.com

## 使用场景

当用户要求上传书籍到微信读书时使用此 skill。上传 PDF 时优先使用此方法。
