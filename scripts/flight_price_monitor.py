#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
飞猪机票价格监控脚本
查询上海到成都指定日期和时间的最便宜机票价格
"""

import requests
import csv
import json
from datetime import datetime
from pathlib import Path
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

# 配置
DATA_DIR = Path(__file__).parent.parent / "data" / "flights"
CSV_FILE = DATA_DIR / "shanghai-chengdu-prices.csv"
CHART_FILE = DATA_DIR / "price-trend.png"

# 确保数据目录存在
DATA_DIR.mkdir(parents=True, exist_ok=True)

# 查询参数
SEARCH_CONFIG = {
    "departure_city": "上海",
    "arrival_city": "成都",
    "departure_date": "2026-05-15",
    "time_range_start": 18,  # 18:00
    "time_range_end": 23,    # 23:00
}

def load_existing_data():
    """加载已有的价格数据"""
    data = []
    if CSV_FILE.exists():
        with open(CSV_FILE, 'r', encoding='utf-8') as f:
            reader = csv.DictReader(f)
            for row in reader:
                data.append(row)
    return data

def save_price_record(price, flight_info):
    """保存价格记录到 CSV"""
    now = datetime.now()
    timestamp = now.strftime("%Y-%m-%d %H:%M:%S")
    date_only = now.strftime("%Y-%m-%d")
    
    data = load_existing_data()
    
    # 检查今天是否已有记录，有则更新
    today_exists = False
    for row in data:
        if row['date'] == date_only:
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
            'flight_info': flight_info
        })
    
    # 写入 CSV
    with open(CSV_FILE, 'w', encoding='utf-8', newline='') as f:
        fieldnames = ['date', 'timestamp', 'price', 'flight_info']
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(data)
    
    print(f"✓ 价格已记录：{timestamp} - ¥{price}")
    return data

def generate_chart(data):
    """生成价格趋势折线图"""
    if len(data) < 2:
        print("⚠ 数据不足，无法生成图表（至少需要 2 条记录）")
        return
    
    # 解析数据
    dates = []
    prices = []
    for row in data:
        try:
            date = datetime.strptime(row['date'], "%Y-%m-%d")
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

def query_flight_price():
    """
    查询机票价格
    
    注意：由于飞猪网站有反爬虫机制，实际使用时需要：
    1. 使用 Selenium/Playwright 模拟浏览器
    2. 或者使用飞猪官方 API（如果有）
    3. 或者手动查询后更新数据
    
    这里提供一个模拟函数用于测试
    """
    # TODO: 实现真实的机票查询逻辑
    # 目前返回一个模拟价格用于测试
    import random
    simulated_price = random.randint(600, 1500)
    simulated_flight = f"模拟航班 MU{random.randint(1000, 9999)}"
    
    return simulated_price, simulated_flight

def main():
    """主函数"""
    print("=" * 50)
    print("飞猪机票价格监控")
    print(f"航线：{SEARCH_CONFIG['departure_city']} → {SEARCH_CONFIG['arrival_city']}")
    print(f"日期：{SEARCH_CONFIG['departure_date']}")
    print(f"时间：{SEARCH_CONFIG['time_range_start']}:00 - {SEARCH_CONFIG['time_range_end']}:00")
    print("=" * 50)
    
    # 查询价格
    print("\n正在查询机票价格...")
    try:
        price, flight_info = query_flight_price()
        print(f"✓ 找到最低价格：¥{price}")
        print(f"  航班信息：{flight_info}")
        
        # 保存记录
        data = save_price_record(price, flight_info)
        
        # 生成图表
        generate_chart(data)
        
        print("\n✓ 任务完成！")
        
    except Exception as e:
        print(f"✗ 查询失败：{e}")
        raise

if __name__ == "__main__":
    main()
