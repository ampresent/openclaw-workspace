#!/usr/bin/env python3
"""
微信读书上传工具 - 使用 opencli 浏览器实例
"""

import asyncio
import sys
from pathlib import Path

SUPPORTED_EXTS = ['.pdf', '.txt', '.epub', '.doc', '.docx', '.mobi', '.azw3']

async def upload_to_weread(file_path: str):
    from playwright.async_api import async_playwright
    
    print("=" * 60)
    print("微信读书上传工具 (使用 opencli 浏览器实例)")
    print("=" * 60)
    
    path = Path(file_path)
    if not path.exists():
        print(f"❌ 文件不存在：{file_path}")
        return False
    
    ext = path.suffix.lower()
    if ext not in SUPPORTED_EXTS:
        print(f"❌ 不支持的格式：{ext}")
        return False
    
    print(f"📄 文件：{path.name} ({path.stat().st_size} bytes)")
    print()
    
    async with async_playwright() as p:
        print("🔍 连接到 opencli Chrome 实例 (端口 18800)...")
        
        try:
            browser = await p.chromium.connect_over_cdp(
                'http://localhost:18800',
                timeout=10000
            )
            print("✅ 连接成功")
        except Exception as e:
            print(f"❌ 连接失败：{e}")
            return False
        
        print()
        print("🔍 获取当前标签页...")
        context = browser.context if hasattr(browser, 'context') else browser.contexts[0]
        pages = list(context.pages)
        print(f"   找到 {len(pages)} 个标签页")
        
        # 使用第一个标签页或创建新的
        if pages:
            page = pages[0]
            print(f"   使用现有标签页：{page.url}")
        else:
            page = await context.new_page()
            print("   创建新标签页")
        
        print()
        print("🔍 导航到上传页面...")
        try:
            await page.goto('https://weread.qq.com/web/upload', wait_until='domcontentloaded', timeout=30000)
            await asyncio.sleep(2)
            print(f"✅ 页面已加载")
            print(f"   标题：{await page.title()}")
            print(f"   URL: {page.url}")
        except Exception as e:
            print(f"⚠️  导航超时：{e}")
            print("   使用当前页面继续...")
        
        print()
        print("🔍 检查登录状态...")
        try:
            cookies = await context.cookies()
            wr_cookies = [c for c in cookies if c['name'].startswith('wr_')]
            print(f"   微信读书 Cookie 数量：{len(wr_cookies)}")
            
            if len(wr_cookies) >= 3:
                print("✅ 登录状态正常")
            else:
                print("⚠️  Cookie 较少，可能未登录")
        except Exception as e:
            print(f"⚠️  无法获取 Cookie: {e}")
        
        print()
        print("🔍 查找上传区域...")
        
        # 使用多种选择器
        selectors = [
            ('文本', 'text=拖拽文件到此处'),
            ('文本', 'text=选择文件'),
            ('CSS', 'input[type="file"]'),
        ]
        
        found_element = None
        for selector_type, selector in selectors:
            try:
                element = await page.wait_for_selector(selector, timeout=5000)
                print(f"✅ {selector_type} 找到：{selector}")
                found_element = element
                break
            except:
                print(f"❌ {selector_type} 未找到：{selector}")
        
        if not found_element:
            print()
            print("❌ 未找到上传区域")
            print()
            print("请使用 browser 工具上传:")
            print(f"  1. 确保上传页面已打开")
            print(f"  2. opencli browser upload --paths {file_path}")
            await browser.close()
            return False
        
        print()
        print("📤 上传文件...")
        
        # 尝试直接上传
        file_input = await page.query_selector('input[type="file"]')
        
        if file_input:
            await file_input.set_input_files(file_path)
            print("✅ 文件已上传")
        elif found_element:
            await found_element.click()
            await asyncio.sleep(2)
            file_input = await page.query_selector('input[type="file"]')
            if file_input:
                await file_input.set_input_files(file_path)
                print("✅ 文件已上传")
            else:
                print("⚠️  需要手动选择文件")
        
        print()
        print("⏳ 等待上传完成...")
        await asyncio.sleep(5)
        
        print()
        print("=" * 60)
        print("✅ 操作完成")
        print("=" * 60)
        
        await browser.close()
        return True

def main():
    if len(sys.argv) < 2:
        print("使用方法：python weread_upload_auto.py <文件路径>")
        sys.exit(1)
    
    file_path = sys.argv[1]
    success = asyncio.run(upload_to_weread(file_path))
    sys.exit(0 if success else 1)

if __name__ == '__main__':
    main()
