---
name: z-library
description: 搜索和下载 Z-Library 电子书
---

# Z-Library 电子书搜索

## 搜索命令

```bash
python3 ~/.openclaw/skills/z-library/weread_upload.py <搜索词>
```

## 功能

- 搜索 Z-Library 电子书
- 获取书籍标题、作者、下载链接
- 自动尝试多个 Z-Library 镜像站点

## 要求

- opencli browser 必须正在运行

## 可用镜像站点

- singlelogin.re / singlelogin.se / singlelogin.me (官方登录)
- z-library.se / z-library.sk (主镜像)
- 1lib.sk / 1lib.io / 1lib.domains
- b-ok.org / b-ok.cc / b-ok.asia
- zh.nyumiyu101.ru (国内镜像)

## 注意

官网经常被封，优先使用镜像站点。
