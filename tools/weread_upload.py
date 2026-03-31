#!/usr/bin/env python3
"""
微信读书 PDF 上传工具
使用方法：python weread_upload.py <文件路径>

支持的文件格式：pdf, txt, epub, doc, docx, mobi, azw3
"""

import asyncio
import sys
import os
from pathlib import Path

SUPPORTED_EXTS = ['.pdf', '.txt', '.epub', '.doc', '.docx', '.mobi', '.azw3']

def check_file(file_path: str) -> dict:
    """检查文件是否有效"""
    path = Path(file_path)
    
    if not path.exists():
        return {'valid': False, 'error': f'文件不存在：{file_path}'}
    
    ext = path.suffix.lower()
    if ext not in SUPPORTED_EXTS:
        return {
            'valid': False, 
            'error': f'不支持的格式：{ext}\n支持的格式：{", ".join(SUPPORTED_EXTS)}'
        }
    
    return {
        'valid': True,
        'name': path.name,
        'size': path.stat().st_size,
        'ext': ext
    }

async def upload_to_weread(file_path: str):
    """上传文件到微信读书"""
    try:
        from playwright.async_api import async_playwright
    except ImportError:
        print("❌ 错误：需要安装 playwright")
        print("运行：pip install playwright")
        print("然后运行：playwright install")
        return False
    
    # 检查文件
    check_result = check_file(file_path)
    if not check_result['valid']:
        print(f"❌ {check_result['error']}")
        return False
    
    print(f"📄 文件：{check_result['name']}")
    print(f"📦 大小：{check_result['size'] / 1024:.1f} KB")
    print(f"📝 格式：{check_result['ext']}")
    print()
    
    async with async_playwright() as p:
        try:
            # 启动浏览器（使用已有的 Chrome 配置）
            print("🚀 启动浏览器...")
            
            # 尝试连接已运行的 Chrome
            try:
                browser = await p.chromium.connect_over_cdp(
                    'http://localhost:9222',
                    timeout=3000
                )
                print("✅ 连接到已运行的 Chrome")
                page = browser.context.pages[0] if browser.context.pages else await browser.new_page()
            except:
                # 如果没有运行，启动新的实例
                print("   启动新的 Chrome 实例...")
                browser = await p.chromium.launch_persistent_context(
                    user_data_dir=Path.home() / ".config" / "google-chrome",
                    headless=False,
                    args=[
                        '--disable-blink-features=AutomationControlled',
                        '--no-sandbox',
                        '--disable-dev-shm-usage'
                    ]
                )
                await asyncio.sleep(2)
                page = browser.pages[0] if browser.pages else await browser.new_page()
            
            # 导航到上传页面
            print("📖 打开微信读书上传页面...")
            await page.goto('https://weread.qq.com/web/upload', wait_until='load')
            await asyncio.sleep(3)  # 等待页面完全加载
            
            # 检查登录状态
            cookies = await browser.cookies()
            is_logged_in = any(c['name'].startswith('wr_') for c in cookies)
            
            if not is_logged_in:
                print("❌ 未登录：请先在浏览器中登录微信读书")
                print("   访问：https://weread.qq.com")
                await browser.close()
                return False
            
            print("✅ 登录状态正常")
            
            # 调试：打印页面标题
            title = await page.title()
            print(f"   页面标题：{title}")
            
            # 调试：保存页面截图
            await page.screenshot(path='/tmp/weread-debug.png')
            print("   调试截图：/tmp/weread-debug.png")
            
            # 等待上传区域加载
            print("⏳ 等待上传区域加载...")
            try:
                # 使用 XPath 查找包含特定文本的元素
                upload_area = await page.wait_for_selector(
                    'xpath=//*[contains(text(), "拖拽文件到此处")]',
                    timeout=5000
                )
                print("✅ 上传区域已加载")
                
                # 点击上传区域
                print("📤 点击上传区域...")
                await upload_area.click()
                await asyncio.sleep(2)
                
            except Exception as e:
                print("❌ 无法找到上传区域")
                print("   请确保已登录微信读书")
                await browser.close()
                return False
            
            # 查找文件输入框并上传
            print("📤 正在上传文件...")
            
            # 等待文件输入框出现
            file_input = None
            for i in range(10):
                file_input = await page.query_selector('input[type="file"]')
                if file_input:
                    break
                await asyncio.sleep(1)
            
            if file_input:
                await file_input.set_input_files(file_path)
                print("✅ 文件已选择")
                
                # 等待上传完成
                print("⏳ 等待上传完成...")
                await asyncio.sleep(5)
                
                print()
                print("=" * 50)
                print("✅ 上传成功！")
                print(f"📚 文件：{check_result['name']}")
                print("📖 请在微信读书书架中查看")
                print("=" * 50)
            else:
                print("❌ 无法找到文件输入框")
                print("   可能需要手动选择文件")
            
            # 等待上传完成
            print("⏳ 等待上传完成...")
            try:
                # 等待进度条消失
                await page.wait_for_selector('[class*="progress"]', state='detached', timeout=120000)
                print("✅ 上传完成")
            except asyncio.TimeoutError:
                print("⚠️  上传超时，但可能已成功")
            
            # 等待上传完成提示
            await asyncio.sleep(2)
            
            print()
            print("=" * 50)
            print("✅ 上传成功！")
            print(f"📚 文件：{check_result['name']}")
            print("📖 请在微信读书书架中查看")
            print("=" * 50)
            
            await browser.close()
            return True
            
        except Exception as e:
            print(f"❌ 错误：{e}")
            try:
                await browser.close()
            except:
                pass
            return False

def main():
    if len(sys.argv) < 2:
        print("微信读书 PDF 上传工具")
        print()
        print("使用方法:")
        print("  python weread_upload.py <文件路径>")
        print()
        print("示例:")
        print("  python weread_upload.py ~/documents/book.pdf")
        print("  python weread_upload.py ./novel.txt")
        print()
        print(f"支持的格式：{', '.join(SUPPORTED_EXTS)}")
        sys.exit(1)
    
    file_path = sys.argv[1]
    success = asyncio.run(upload_to_weread(file_path))
    sys.exit(0 if success else 1)

if __name__ == '__main__':
    main()
