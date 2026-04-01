---
name: weread-upload
description: 上传电子书到微信读书
---

# 微信读书上传

## 命令

```bash
# 方法 1：Python 脚本（推荐）
python3 skills/weread-upload/weread_upload.py <文件路径>

# 方法 2：Shell 包装器
skills/weread-upload/weread-upload <文件路径>
```

## 支持格式

pdf, txt, epub, doc, docx, mobi, azw3

## 前置条件

1. Chrome 浏览器已登录 https://weread.qq.com
2. 安装依赖：`pip install playwright`

## 使用场景

当用户要求上传书籍到微信读书时使用此 skill。
