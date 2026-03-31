# 微信读书 PDF 上传工具

## ✅ 自动化上传已实现

**部署位置：** `/home/admin/openclaw/workspace/tools/weread_upload_auto.py`

---

## 🚀 快速开始

### 前置条件

1. **opencli browser 正在运行** (Chrome 实例已启动)
2. **已登录微信读书** (在浏览器中访问 https://weread.qq.com 并登录)

### 使用方法

```bash
# 上传 PDF 文件
python3 /home/admin/openclaw/workspace/tools/weread_upload_auto.py ~/documents/book.pdf

# 上传 TXT 文件
python3 /home/admin/openclaw/workspace/tools/weread_upload_auto.py ~/novels/story.txt

# 上传 EPUB 文件
python3 /home/admin/openclaw/workspace/tools/weread_upload_auto.py ~/books/guide.epub
```

---

## 📋 支持的文件格式

| 格式 | 扩展名 |
|------|--------|
| PDF | `.pdf` |
| 文本 | `.txt` |
| EPUB | `.epub` |
| Word | `.doc`, `.docx` |
| Kindle | `.mobi`, `.azw3` |

---

## 🔧 技术细节

### 工作原理

1. **连接现有 Chrome 实例** - 通过 CDP 端口 18800 连接到 opencli browser
2. **共享登录状态** - 使用相同的用户数据目录，共享 Cookie
3. **自动上传** - 找到上传区域并注入文件

### 关键配置

```python
# CDP 连接
browser = await p.chromium.connect_over_cdp('http://localhost:18800')

# 用户数据目录
~/.openclaw/browser/openclaw/user-data
```

---

## 📝 输出示例

```
============================================================
微信读书上传工具 (使用 opencli 浏览器实例)
============================================================
📄 文件：test.pdf (13264 bytes)

🔍 连接到 opencli Chrome 实例 (端口 18800)...
✅ 连接成功

🔍 获取当前标签页...
   找到 4 个标签页
   使用现有标签页：https://weread.qq.com/web/upload

🔍 导航到上传页面...
✅ 页面已加载
   标题：微信读书

🔍 检查登录状态...
   微信读书 Cookie 数量：10
✅ 登录状态正常

🔍 查找上传区域...
✅ 文本 找到：text=拖拽文件到此处

📤 上传文件...
✅ 文件已上传

⏳ 等待上传完成...

============================================================
✅ 操作完成
============================================================
```

---

## ⚠️ 故障排除

### 错误：连接失败

```
❌ 连接失败：Timeout
```

**解决方法：**
- 确保 opencli browser 正在运行
- 检查 Chrome 进程：`ps aux | grep chrome`

### 错误：Cookie 较少

```
⚠️  Cookie 较少，可能未登录
```

**解决方法：**
- 在浏览器中访问 https://weread.qq.com 并登录
- 重新运行上传命令

### 错误：未找到上传区域

```
❌ 未找到上传区域
```

**解决方法：**
- 确保上传页面已打开：`opencli browser navigate https://weread.qq.com/web/upload`
- 检查页面是否完全加载

---

## 📁 文件结构

```
/home/admin/openclaw/workspace/tools/
├── weread_upload_auto.py    # 自动化上传脚本 ✅
├── weread_upload_simple.py  # 简化版 (手动上传)
├── weread_upload_debug.py   # 调试版
└── weread-upload            # Shell 包装脚本
```

---

## 🔗 相关链接

- 微信读书网页版：https://weread.qq.com
- 微信读书上传页面：https://weread.qq.com/web/upload
