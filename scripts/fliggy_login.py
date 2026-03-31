#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
飞猪扫码登录脚本（简化版）
运行后会打开浏览器显示二维码，用淘宝 APP 扫描即可
"""

import json
import time
from pathlib import Path
from playwright.sync_api import sync_playwright, TimeoutError

FLIGGY_LOGIN_URL = "https://login.taobao.com/member/login.jhtml?f=top&redirectURL=https%3A%2F%2Fwww.fliggy.com%2F"
COOKIES_FILE = Path(__file__).parent / "cookies.json"

def main():
    print("=" * 50)
    print("飞猪扫码登录")
    print("=" * 50)
    print("\n即将打开浏览器显示二维码...")
    print("请使用【淘宝 APP】扫描二维码完成登录\n")
    
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=False)
        context = browser.new_context()
        page = context.new_page()
        
        print("正在打开登录页面...")
        page.goto(FLIGGY_LOGIN_URL, wait_until='networkidle')
        
        print("\n等待二维码加载...")
        time.sleep(3)
        
        # 截图保存二维码
        screenshot_path = Path(__file__).parent.parent / "temp" / "fliggy-login-qrcode.png"
        screenshot_path.parent.mkdir(exist_ok=True)
        page.screenshot(path=str(screenshot_path))
        print(f"\n✓ 二维码截图已保存：{screenshot_path}")
        print("\n如果浏览器窗口未自动弹出，请查看上方截图")
        print("\n等待您扫码登录...（最多等待 60 秒）")
        
        # 等待登录
        try:
            page.wait_for_selector('a[href*="buyertrade.taobao.com"]', timeout=60000)
            print("\n✓ 登录成功！")
        except TimeoutError:
            print("\n✗ 登录超时，请重新运行脚本")
            browser.close()
            return False
        
        # 保存 cookies
        cookies = context.cookies()
        with open(COOKIES_FILE, 'w', encoding='utf-8') as f:
            json.dump(cookies, f, ensure_ascii=False, indent=2)
        print(f"✓ Cookies 已保存：{COOKIES_FILE}")
        
        # 验证登录状态
        page.goto("https://www.fliggy.com/", wait_until='networkidle')
        time.sleep(2)
        
        page.screenshot(path=str(Path(__file__).parent.parent / "temp" / "fliggy-logged-in.png"))
        print("✓ 登录状态截图已保存")
        
        browser.close()
        
        print("\n" + "=" * 50)
        print("✓ 登录完成！")
        print("=" * 50)
        print("\n现在可以运行查询脚本：")
        print("  python scripts/flight_monitor_full.py")
        print("\n或者手动查询模式：")
        print("  python scripts/flight_monitor_full.py --manual")
        return True

if __name__ == "__main__":
    main()
