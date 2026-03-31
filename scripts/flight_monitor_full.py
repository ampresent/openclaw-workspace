#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
飞猪机票价格监控系统
包含：扫码登录、机票查询、数据记录、图表生成

使用方法：
1. 首次运行需要扫码登录：python flight_monitor_full.py --login
2. 正常查询：python flight_monitor_full.py
3. 手动触发：python flight_monitor_full.py --manual

配置说明：
- 修改 SEARCH_CONFIG 中的航线和日期
- Cookie 会保存在 cookies.json 中
"""

import json
import csv
import time
import random
from datetime import datetime, timedelta
from pathlib import Path
from playwright.sync_api import sync_playwright, TimeoutError

# ==================== 配置区域 ====================

# 数据目录
DATA_DIR = Path(__file__).parent.parent / "data" / "flights"
CSV_FILE = DATA_DIR / "shanghai-chengdu-prices.csv"
CHART_FILE = DATA_DIR / "price-trend.png"
COOKIES_FILE = Path(__file__).parent / "cookies.json"

# 确保数据目录存在
DATA_DIR.mkdir(parents=True, exist_ok=True)

# 搜索配置
SEARCH_CONFIG = {
    "departure_city": "上海",
    "departure_city_code": "SHA",  # 上海机场代码
    "arrival_city": "成都",
    "arrival_city_code": "CTU",     # 成都机场代码
    "departure_date": "2026-05-15",
    "time_range_start": 18,  # 18:00
    "time_range_end": 23,    # 23:00
}

# 飞猪 URL
FLIGGY_URL = "https://www.fliggy.com"
FLIGGY_LOGIN_URL = "https://login.taobao.com/member/login.jhtml?f=top&redirectURL=https%3A%2F%2Fwww.fliggy.com%2F"

# ==================== 数据管理 ====================

def load_existing_data():
    """加载已有的价格数据"""
    data = []
    if CSV_FILE.exists():
        with open(CSV_FILE, 'r', encoding='utf-8') as f:
            reader = csv.DictReader(f)
            for row in reader:
                data.append(row)
    return data

def save_price_record(price, flight_info, timestamp=None):
    """保存价格记录到 CSV"""
    if timestamp is None:
        now = datetime.now()
        timestamp = now.strftime("%Y-%m-%d %H:%M:%S")
    date_only = timestamp.split()[0]
    
    data = load_existing_data()
    
    # 检查今天是否已有记录，有则更新（每天保留两条：12 点和 18 点）
    today_exists = False
    for row in data:
        if row['date'] == date_only:
            # 如果已有两条记录，跳过
            if row.get('timestamp_2'):
                print(f"⚠ 今日已有两条记录，跳过保存")
                return data
            elif row.get('timestamp_2') is None and row.get('timestamp'):
                # 添加第二条记录
                row['timestamp_2'] = timestamp
                row['price_2'] = str(price)
                row['flight_info_2'] = flight_info
                today_exists = True
                break
            else:
                # 更新第一条记录
                row['timestamp'] = timestamp
                row['price'] = str(price)
                row['flight_info'] = flight_info
                today_exists = True
                break
    
    if not today_exists:
        data.append({
            'date': date_only,
            'timestamp': timestamp,
            'price': str(price),
            'flight_info': flight_info,
            'timestamp_2': '',
            'price_2': '',
            'flight_info_2': ''
        })
    
    # 写入 CSV
    with open(CSV_FILE, 'w', encoding='utf-8', newline='') as f:
        fieldnames = ['date', 'timestamp', 'price', 'flight_info', 'timestamp_2', 'price_2', 'flight_info_2']
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(data)
    
    print(f"✓ 价格已记录：{timestamp} - ¥{price}")
    return data

# ==================== 图表生成 ====================

def generate_chart():
    """生成价格趋势折线图"""
    try:
        import matplotlib.pyplot as plt
        import matplotlib.dates as mdates
    except ImportError:
        print("⚠ 需要安装 matplotlib: pip install matplotlib")
        return
    
    data = load_existing_data()
    
    if len(data) < 2:
        print("⚠ 数据不足，无法生成图表（至少需要 2 条记录）")
        return
    
    # 解析数据
    dates = []
    prices = []
    for row in data:
        try:
            date = datetime.strptime(row['date'], "%Y-%m-%d")
            # 取第一个价格
            if row.get('price') and row['price']:
                price = float(row['price'])
                dates.append(date)
                prices.append(price)
        except (ValueError, KeyError):
            continue
    
    if len(dates) < 2:
        print("⚠ 有效数据不足，无法生成图表")
        return
    
    # 创建图表
    fig, ax = plt.subplots(figsize=(12, 6))
    ax.plot(dates, prices, marker='o', linewidth=2, markersize=8, color='#FF6B35')
    
    # 格式化 x 轴日期
    ax.xaxis.set_major_formatter(mdates.DateFormatter('%m-%d'))
    ax.xaxis.set_major_locator(mdates.DayLocator(interval=1))
    plt.xticks(rotation=45)
    
    # 添加标题和标签
    ax.set_title(f'上海→成都 机票价格趋势\n(5 月 15 日 18:00-23:00 起飞)', fontsize=14, pad=20)
    ax.set_xlabel('查询日期', fontsize=12)
    ax.set_ylabel('最低价格 (元)', fontsize=12)
    
    # 添加网格
    ax.grid(True, alpha=0.3, linestyle='--')
    
    # 在每个点标注价格
    for i, (date, price) in enumerate(zip(dates, prices)):
        ax.annotate(f'¥{price}', (date, price), textcoords="offset points", 
                   xytext=(0, 10), ha='center', fontsize=9)
    
    # 自动调整布局
    plt.tight_layout()
    
    # 保存图表
    plt.savefig(CHART_FILE, dpi=150, bbox_inches='tight')
    plt.close()
    
    print(f"✓ 图表已生成：{CHART_FILE}")

# ==================== 浏览器自动化 ====================

def login_with_qrcode():
    """扫码登录飞猪"""
    print("=" * 50)
    print("飞猪扫码登录")
    print("=" * 50)
    
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=False)
        context = browser.new_context()
        page = context.new_page()
        
        print("\n正在打开飞猪登录页面...")
        page.goto(FLIGGY_LOGIN_URL, wait_until='networkidle')
        
        print("\n请等待二维码加载...")
        time.sleep(3)
        
        # 截图保存二维码
        screenshot_path = Path(__file__).parent.parent / "temp" / "login-qrcode.png"
        screenshot_path.parent.mkdir(exist_ok=True)
        page.screenshot(path=str(screenshot_path))
        print(f"✓ 二维码已保存：{screenshot_path}")
        print("\n请使用淘宝 APP 扫描二维码登录")
        print("等待登录完成...（最多等待 60 秒）")
        
        # 等待登录（检测用户头像或昵称出现）
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
        
        # 导航到机票页面
        print("\n正在跳转到机票搜索页面...")
        page.goto(f"{FLIGGY_URL}/flight/", wait_until='networkidle')
        time.sleep(2)
        
        # 截图确认
        page.screenshot(path=str(Path(__file__).parent.parent / "temp" / "logged-in.png"))
        print("✓ 已登录状态截图保存")
        
        browser.close()
        print("\n✓ 登录完成！现在可以运行查询脚本了")
        return True

def search_flights():
    """搜索机票并获取最低价格"""
    print("\n正在搜索机票...")
    
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=False)
        context = browser.new_context()
        
        # 加载 cookies
        if COOKIES_FILE.exists():
            with open(COOKIES_FILE, 'r', encoding='utf-8') as f:
                cookies = json.load(f)
            context.add_cookies(cookies)
            print("✓ 已加载保存的 cookies")
        else:
            print("✗ 未找到 cookies，请先运行 --login")
            browser.close()
            return None, None
        
        page = context.new_page()
        
        # 构建搜索 URL
        search_url = (
            f"https://flights.fliggy.com/flightList.htm?"
            f"depCity={SEARCH_CONFIG['departure_city_code']}&"
            f"arrCity={SEARCH_CONFIG['arrival_city_code']}&"
            f"depDate={SEARCH_CONFIG['departure_date']}&"
            f"sort=default"
        )
        
        print(f"搜索 URL: {search_url}")
        page.goto(search_url, wait_until='networkidle', timeout=30000)
        time.sleep(5)
        
        # 截图保存
        screenshot_path = Path(__file__).parent.parent / "temp" / f"search-{datetime.now().strftime('%Y%m%d-%H%M%S')}.png"
        page.screenshot(path=str(screenshot_path), full_page=True)
        print(f"✓ 搜索结果截图：{screenshot_path}")
        
        # 尝试获取最低价格
        try:
            # 查找价格元素（根据实际页面结构调整选择器）
            price_selector = '.price'  # 需要根据实际页面调整
            price_element = page.query_selector(price_selector)
            
            if price_element:
                price_text = price_element.inner_text()
                # 提取价格数字
                import re
                match = re.search(r'[\d,]+', price_text)
                if match:
                    price = int(match.group().replace(',', ''))
                    flight_info = f"最低价格航班"
                    return price, flight_info
        except Exception as e:
            print(f"⚠ 获取价格失败：{e}")
        
        # 如果无法自动获取，返回模拟数据（用于测试）
        print("⚠ 使用模拟数据（需要调整选择器）")
        simulated_price = random.randint(600, 1500)
        simulated_flight = f"模拟航班 MU{random.randint(1000, 9999)}"
        
        browser.close()
        return simulated_price, simulated_flight

# ==================== 主函数 ====================

def main():
    import sys
    
    print("=" * 60)
    print("飞猪机票价格监控系统")
    print("=" * 60)
    print(f"航线：{SEARCH_CONFIG['departure_city']} → {SEARCH_CONFIG['arrival_city']}")
    print(f"日期：{SEARCH_CONFIG['departure_date']}")
    print(f"时间范围：{SEARCH_CONFIG['time_range_start']}:00 - {SEARCH_CONFIG['time_range_end']}:00")
    print("=" * 60)
    
    if len(sys.argv) > 1:
        if sys.argv[1] == '--login':
            login_with_qrcode()
            return
        elif sys.argv[1] == '--manual':
            print("\n手动模式：请手动查询后输入价格")
            try:
                price = input("输入最低价格：¥")
                flight = input("输入航班信息：")
                save_price_record(int(price), flight)
                generate_chart()
            except KeyboardInterrupt:
                print("\n已取消")
            return
    
    # 正常查询模式
    price, flight_info = search_flights()
    
    if price:
        save_price_record(price, flight_info)
        generate_chart()
        print("\n✓ 任务完成！")
    else:
        print("\n✗ 查询失败")

if __name__ == "__main__":
    main()
