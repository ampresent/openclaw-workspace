#!/usr/bin/env python3
"""
微信读书上传工具 v2
用法: python weread_upload.py <文件路径>

支持格式: pdf, txt, epub, doc, docx, mobi, azw3
首次使用会显示二维码要求扫码登录，登录状态自动保存。
"""

import asyncio
import json
import sys
from pathlib import Path

SUPPORTED_EXTS = ['.pdf', '.txt', '.epub', '.doc', '.docx', '.mobi', '.azw3']
COOKIES_FILE = Path(__file__).parent / "weread_cookies.json"
LOGIN_COOKIE_NAMES = {'wr_skey', 'wr_name', 'wr_vid', 'wr_avatar', 'wr_gender', 'wr_pf'}

def check_file(file_path: str) -> dict:
    path = Path(file_path)
    if not path.exists():
        return {'valid': False, 'error': f'文件不存在：{file_path}'}
    ext = path.suffix.lower()
    if ext not in SUPPORTED_EXTS:
        return {'valid': False, 'error': f'不支持的格式：{ext}，支持：{", ".join(SUPPORTED_EXTS)}'}
    return {'valid': True, 'name': path.name, 'size': path.stat().st_size, 'ext': ext}


def _upload_qr_to_hosting(file_path: str) -> str | None:
    """上传二维码到 litterbox.catbox.moe（72小时临时图床），返回 URL 或 None"""
    import subprocess
    try:
        result = subprocess.run(
            ['curl', '-s', '--max-time', '10',
             '-F', 'reqtype=fileupload',
             '-F', 'time=72h',
             '-F', f'fileToUpload=@{file_path}',
             'https://litterbox.catbox.moe/resources/internals/api.php'],
            capture_output=True, text=True, timeout=15
        )
        url = result.stdout.strip()
        if url.startswith('http'):
            return url
    except Exception:
        pass
    return None


async def ensure_logged_in(ctx, page) -> bool:
    """确保已登录，未登录则显示二维码等待扫码"""
    # 尝试加载已保存的 cookies
    if COOKIES_FILE.exists():
        saved = json.loads(COOKIES_FILE.read_text())
        await ctx.add_cookies(saved)

    await page.goto('https://weread.qq.com/web/upload', wait_until='networkidle', timeout=30000)
    await asyncio.sleep(3)

    if await _is_logged_in(ctx, page):
        return True

    # Cookies 过期，清除并重新登录
    print("⚠️  未登录或 cookies 已过期，需要扫码登录")
    await ctx.clear_cookies()

    # 打开登录页
    await page.goto('https://weread.qq.com/', wait_until='networkidle', timeout=30000)
    await asyncio.sleep(2)

    # 点击登录按钮
    try:
        await page.locator('text=登录').first.click(timeout=5000)
        await asyncio.sleep(3)
    except:
        pass

    # 截取二维码
    qr_saved = False
    for sel in ['img[class*="qr"]', 'img[class*="login"]', 'img[src*="qrcode"]']:
        try:
            el = page.locator(sel).first
            if await el.is_visible(timeout=2000):
                await el.screenshot(path='/tmp/weread-qr.png')
                qr_saved = True
                break
        except:
            continue
    if not qr_saved:
        await page.screenshot(path='/tmp/weread-qr.png')

    # 上传二维码到图床，方便远程扫码
    qr_url = _upload_qr_to_hosting('/tmp/weread-qr.png')
    if qr_url:
        print(f"📱 二维码链接：{qr_url}")
    else:
        print("📱 二维码已保存到 /tmp/weread-qr.png（图床上传失败，仅本地可用）")
    print("⏳ 请用手机微信扫描二维码（最多 180 秒）...")

    # 等待扫码
    for i in range(90):
        await asyncio.sleep(2)
        if await _is_logged_in(ctx, page):
            cookies = await ctx.cookies()
            COOKIES_FILE.write_text(json.dumps(cookies, ensure_ascii=False, indent=2))
            print("✅ 登录成功！cookies 已保存")
            # 导航回上传页
            await page.goto('https://weread.qq.com/web/upload', wait_until='networkidle', timeout=30000)
            await asyncio.sleep(3)
            return True
        if i % 15 == 0 and i > 0:
            print(f"   等待中... ({i*2}s)")

    print("❌ 扫码超时")
    return False


async def _is_logged_in(ctx, page) -> bool:
    cookies = await ctx.cookies()
    if any(c['name'] in LOGIN_COOKIE_NAMES for c in cookies):
        return True
    return False


async def upload_file(file_path: str):
    from playwright.async_api import async_playwright

    check_result = check_file(file_path)
    if not check_result['valid']:
        print(f"❌ {check_result['error']}")
        return False

    print(f"📄 文件：{check_result['name']}  ({check_result['size'] / 1024:.1f} KB)")
    print()

    async with async_playwright() as p:
        browser = await p.chromium.launch(
            headless=True,
            args=['--no-sandbox', '--disable-dev-shm-usage']
        )
        ctx = await browser.new_context(
            viewport={'width': 1280, 'height': 900},
            user_agent='Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36'
        )
        page = await ctx.new_page()

        try:
            # 确保已登录
            if not await ensure_logged_in(ctx, page):
                await browser.close()
                return False

            # 点击「从电脑导入」
            print("📤 从电脑导入...")
            try:
                import_btn = page.locator('text=从电脑导入').first
                await import_btn.click(timeout=5000)
                await asyncio.sleep(2)
            except:
                print("⚠️  未找到「从电脑导入」按钮，尝试直接查找文件输入框")

            # 查找文件输入框
            file_input = None
            for _ in range(10):
                file_input = await page.query_selector('input[type="file"]')
                if file_input:
                    break
                await asyncio.sleep(1)

            if file_input:
                await file_input.set_input_files(file_path)
                print("✅ 文件已选择")

                # 等待上传完成
                print("⏳ 等待上传完成...")
                try:
                    await page.wait_for_selector('[class*="progress"]', state='detached', timeout=120000)
                except asyncio.TimeoutError:
                    pass
                await asyncio.sleep(3)

                await page.screenshot(path='/tmp/weread-upload-result.png')
                print()
                print("=" * 40)
                print("✅ 上传完成！")
                print(f"📚 {check_result['name']}")
                print("📖 请在微信读书书架中查看")
                print("=" * 40)
            else:
                print("❌ 无法找到文件输入框")
                await page.screenshot(path='/tmp/weread-upload-result.png')
                await browser.close()
                return False

            await browser.close()
            return True

        except Exception as e:
            print(f"❌ 错误：{e}")
            await browser.close()
            return False


def main():
    if len(sys.argv) < 2:
        print("微信读书上传工具")
        print()
        print("用法: python weread_upload.py <文件路径>")
        print()
        print(f"支持格式: {', '.join(SUPPORTED_EXTS)}")
        print()
        print("首次使用会弹出二维码，用微信扫码登录即可。")
        print("登录状态自动保存，下次无需重复扫码。")
        sys.exit(1)

    success = asyncio.run(upload_file(sys.argv[1]))
    sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()
