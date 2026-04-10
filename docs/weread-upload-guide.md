# 微信读书 PDF 上传工具

## 快速开始

```bash
python3 skills/weread-upload/weread_upload.py ~/documents/book.pdf
```

## 支持格式

pdf, txt, epub, doc, docx, mobi, azw3

## 前置条件

1. Chrome 浏览器已登录 https://weread.qq.com
2. 安装依赖：`pip install playwright`

## 工作原理

1. 尝试通过 CDP 端口 18800 连接到已有 Chrome 实例
2. 若无运行中的 Chrome，启动新的持久化上下文
3. 导航到上传页面，注入文件
4. 等待上传完成

## 文件位置

```
skills/weread-upload/
├── SKILL.md              # 技能文档
├── weread_upload.py      # 主脚本（Python）
└── weread-upload         # Shell 包装器
```

## 故障排除

- **连接失败**：确保 Chrome 正在运行且已登录微信读书
- **Cookie 较少**：在浏览器中访问 weread.qq.com 并重新登录
- **未找到上传区域**：确认页面已完全加载
