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
1. 脚本自动打开微信读书登录页并截取二维码
2. **自动上传到 litterbox.catbox.moe（72小时临时图床）**，返回在线链接
3. 用户打开链接，用手机微信扫描二维码（远程扫码，无需本地访问文件）
4. 登录成功后 cookies 自动保存到 `skills/weread-upload/weread_cookies.json`
5. 后续使用自动复用 cookies，无需重复扫码

**手动重新登录：**
- 删除 `skills/weread-upload/weread_cookies.json` 即可触发重新扫码
- 或用浏览器打开微信读书，登录后通过 CDP 导出 cookie 写入 `weread_cookies.json`

## 使用场景

当用户要求上传书籍到微信读书时使用此 skill。
