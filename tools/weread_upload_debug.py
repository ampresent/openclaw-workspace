#!/usr/bin/env python3
"""
微信读书上传工具 - 调试版
"""

import asyncio
import sys
import json
from pathlib import Path

SUPPORTED_EXTS = ['.pdf', '.txt', '.epub', '.doc', '.docx', '.mobi', '.azw3']

async def debug_weread_upload(file_path: str):
    from playwright.async_api import async_playwright
    
    print("=" * 60)
    print("微信读书上传调试工具")
    print("=" * 60)
    
    # 检查文件
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
        print("🔍 步骤 1: 启动浏览器...")
        
        # 尝试连接已运行的 Chrome (使用 opencli browser 相同的实例)
        browser = None
        try:
            browser = await p.chromium.connect_over_cdp(
                'http://localhost:9222',
                timeout=5000
            )
            print("✅ 连接到已运行的 Chrome (CDP 9222)")
        except Exception as e:
            print(f"⚠️  无法连接 CDP: {e}")
            print("   启动新的 Chrome 实例...")
            browser = await p.chromium.launch_persistent_context(
                user_data_dir=Path.home() / ".config" / "google-chrome",
                headless=False,
                args=[
                    '--disable-blink-features=AutomationControlled',
                    '--no-sandbox',
                    '--disable-dev-shm-usage',
                    '--remote-debugging-port=9222'
                ]
            )
            print("✅ 新 Chrome 实例已启动")
        
        print()
        print("🔍 步骤 2: 导航到上传页面...")
        page = browser.context.pages[0] if hasattr(browser, 'context') else browser.pages[0]
        
        await page.goto('https://weread.qq.com/web/upload', wait_until='load')
        await asyncio.sleep(5)  # 等待页面完全渲染
        
        print(f"✅ 页面已加载")
        print(f"   标题：{await page.title()}")
        print(f"   URL: {page.url}")
        print()
        
        print("🔍 步骤 3: 检查登录状态...")
        cookies = await browser.cookies()
        wr_cookies = [c for c in cookies if c['name'].startswith('wr_')]
        print(f"   微信读书 Cookie 数量：{len(wr_cookies)}")
        if wr_cookies:
            print(f"   Cookie 名称：{[c['name'] for c in wr_cookies[:5]]}")
        print()
        
        print("🔍 步骤 4: 查找上传区域...")
        
        # 尝试多种选择器
        selectors = [
            ('文本选择器', 'text=拖拽文件到此处'),
            ('文本选择器', 'text=选择文件'),
            ('XPath', 'xpath=//*[contains(text(), "拖拽")]'),
            ('CSS', '[class*="upload"]'),
            ('CSS', 'input[type="file"]'),
        ]
        
        found_selector = None
        for selector_type, selector in selectors:
            try:
                element = await page.wait_for_selector(selector, timeout=3000)
                print(f"✅ {selector_type} 找到：{selector}")
                found_selector = (selector_type, selector, element)
                break
            except Exception as e:
                print(f"❌ {selector_type} 未找到：{selector}")
        
        if not found_selector:
            print()
            print("🔍 步骤 5: 获取页面 HTML 片段...")
            
            # 获取 body 内容的前 5000 字符
            html = await page.content()
            print(f"   页面总长度：{len(html)} 字符")
            
            # 搜索关键词
            keywords = ['拖拽', '上传', '选择文件', 'input type="file"']
            for kw in keywords:
                count = html.count(kw)
                print(f"   '{kw}' 出现次数：{count}")
            
            # 保存调试 HTML
            debug_path = '/tmp/weread-debug.html'
            with open(debug_path, 'w', encoding='utf-8') as f:
                f.write(html)
            print(f"\n   调试 HTML 已保存到：{debug_path}")
            
            # 保存截图
            screenshot_path = '/tmp/weread-debug-page.png'
            await page.screenshot(path=screenshot_path)
            print(f"   页面截图已保存到：{screenshot_path}")
            
            print()
            print("❌ 无法找到上传区域")
            print("   可能原因:")
            print("   1. 页面未完全加载")
            print("   2. 需要登录 (请检查 Cookie)")
            print("   3. 页面结构已变更")
            print("   4. 反自动化检测")
            
            await browser.close()
            return False
        
        print()
        print("🔍 步骤 6: 上传文件...")
        selector_type, selector, element = found_selector
        
        # 获取元素信息
        tag = await element.evaluate('el => el.tagName')
        classes = await element.evaluate('el => el.className')
        text = await element.evaluate('el => el.textContent[:100]')
        
        print(f"   元素标签：{tag}")
        print(f"   元素类名：{classes[:100]}...")
        print(f"   元素文本：{text}")
        
        # 查找文件输入框
        file_input = await page.query_selector('input[type="file"]')
        
        if file_input:
            print("✅ 找到文件输入框")
            await file_input.set_input_files(file_path)
            print("✅ 文件已选择")
        else:
            print("⚠️  未找到文件输入框，点击上传区域...")
            await element.click()
            await asyncio.sleep(2)
            
            # 再次查找
            file_input = await page.query_selector('input[type="file"]')
            if file_input:
                print("✅ 找到文件输入框")
                await file_input.set_input_files(file_path)
                print("✅ 文件已选择")
            else:
                print("❌ 仍然找不到文件输入框")
        
        # 等待上传
        print()
        print("🔍 步骤 7: 等待上传完成...")
        await asyncio.sleep(5)
        
        # 检查是否有成功提示
        success_keywords = ['成功', '完成', '书架']
        for kw in success_keywords:
            element = await page.query_selector(f'text={kw}')
            if element:
                print(f"✅ 检测到成功提示：'{kw}'")
        
        print()
        print("=" * 60)
        print("✅ 调试完成")
        print("=" * 60)
        
        await browser.close()
        return True

def main():
    if len(sys.argv) < 2:
        print("使用方法：python weread_upload_debug.py <文件路径>")
        sys.exit(1)
    
    file_path = sys.argv[1]
    success = asyncio.run(debug_weread_upload(file_path))
    sys.exit(0 if success else 1)

if __name__ == '__main__':
    main()
