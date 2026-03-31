#!/usr/bin/env python3
"""
微信读书上传工具 - 简化版
使用方法：python weread_upload_simple.py <文件路径>

支持的文件格式：pdf, txt, epub, doc, docx, mobi, azw3
"""

import sys
import os
from pathlib import Path

SUPPORTED_EXTS = ['.pdf', '.txt', '.epub', '.doc', '.docx', '.mobi', '.azw3']

def main():
    if len(sys.argv) < 2:
        print("微信读书上传工具（简化版）")
        print()
        print("使用方法:")
        print("  python weread_upload_simple.py <文件路径>")
        print()
        print("示例:")
        print("  python weread_upload_simple.py ~/documents/book.pdf")
        print()
        print(f"支持的格式：{', '.join(SUPPORTED_EXTS)}")
        sys.exit(1)
    
    file_path = sys.argv[1]
    path = Path(file_path)
    
    # 检查文件
    if not path.exists():
        print(f"❌ 文件不存在：{file_path}")
        sys.exit(1)
    
    ext = path.suffix.lower()
    if ext not in SUPPORTED_EXTS:
        print(f"❌ 不支持的格式：{ext}")
        print(f"支持的格式：{', '.join(SUPPORTED_EXTS)}")
        sys.exit(1)
    
    print(f"📄 文件：{path.name}")
    print(f"📦 大小：{path.stat().st_size / 1024:.1f} KB")
    print()
    print("请使用以下命令打开微信读书上传页面：")
    print()
    print(f"  opencli browser navigate https://weread.qq.com/web/upload")
    print()
    print("然后：")
    print("  1. 确保已登录微信读书")
    print("  2. 拖拽文件到上传区域，或点击选择文件")
    print("  3. 等待上传完成")
    print()
    print(f"文件路径：{path.absolute()}")

if __name__ == '__main__':
    main()
