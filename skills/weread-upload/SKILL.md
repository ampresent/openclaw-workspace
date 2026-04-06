---
name: weread-upload
description: 上传电子书到微信读书
---

# 微信读书上传

## 命令

```bash
python3 skills/weread-upload/weread_upload.py <文件路径>
```

## 支持格式

pdf, txt, epub, doc, docx, mobi, azw3

## 登录流程

首次使用（或 cookies 过期）时：
1. 脚本自动打开微信读书登录页
2. 截取二维码保存到 `/tmp/weread-qr.png`
3. 用户用手机微信扫描二维码
4. 登录成功后 cookies 自动保存到 `skills/weread-upload/weread_cookies.json`
5. 后续使用自动复用 cookies，无需重复扫码

**获取二维码的方式：**
- 读取 `/tmp/weread-qr.png` 展示给用户
- 或复制到 workspace 后通过 read 工具展示

## 使用场景

当用户要求上传书籍到微信读书时使用此 skill。
