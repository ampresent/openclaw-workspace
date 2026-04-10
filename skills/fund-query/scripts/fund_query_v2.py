#!/usr/bin/env python3
"""
Fund Query V2 - 支持实时估值 + 最新净值 fallback
解决 QDII 基金无法获取实时估值的问题。

用法:
    python3 fund_query_v2.py <fund_code>              # 查询（自动选择最佳数据源）
    python3 fund_query_v2.py <fund_code> estimate      # 仅查实时估值
    python3 fund_query_v2.py <fund_code> nav           # 仅查最新净值
    python3 fund_query_v2.py --portfolio <json_file>   # 批量查询持仓
"""

from __future__ import annotations

import sys
import json
import re
import urllib.request
import urllib.error
from datetime import datetime

VALID_CODE_PATTERN = re.compile(r'^\d{6}$')

HEADERS = {
    'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
    'Referer': 'http://fund.eastmoney.com/'
}


def fetch(url: str, timeout: int = 10) -> str:
    req = urllib.request.Request(url, headers=HEADERS)
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return resp.read().decode('utf-8', errors='ignore')


def parse_jsonp(jsonp_str: str) -> dict | None:
    match = re.search(r'\((.+)\)', jsonp_str, re.DOTALL)
    if match:
        try:
            return json.loads(match.group(1))
        except json.JSONDecodeError:
            return None
    return None


def get_estimate(fund_code: str) -> dict | None:
    """实时估值（fundgz API，仅支持国内基金交易时段）"""
    url = f"http://fundgz.1234567.com.cn/js/{fund_code}.js"
    try:
        raw = fetch(url)
        data = parse_jsonp(raw)
        if not data or not data.get('fundcode'):
            return None
        return {
            'source': 'estimate',
            'code': data.get('fundcode', fund_code),
            'name': data.get('name', ''),
            'date': data.get('gztime', ''),
            'nav': float(data.get('dwjz', 0)),
            'nav_date': data.get('jzrq', ''),
            'estimate_value': float(data.get('gsz', 0)),
            'change_pct': float(data.get('gszzl', 0)),
        }
    except Exception:
        return None


def get_nav(fund_code: str) -> dict | None:
    """最新净值（pingzhongdata API，所有基金可用）"""
    url = f"http://fund.eastmoney.com/pingzhongdata/{fund_code}.js"
    try:
        raw = fetch(url)
        name_match = re.search(r'var\s+fS_name\s*=\s*"([^"]*)"', raw)
        if not name_match:
            return None
        
        # Extract latest NAV from Data_netWorthTrend
        trend_match = re.search(r'var\s+Data_netWorthTrend\s*=\s*(\[.+?\]);', raw, re.DOTALL)
        if trend_match:
            trend = json.loads(trend_match.group(1))
            if trend and len(trend) >= 2:
                latest = trend[-1]
                prev = trend[-2]
                latest_nav = float(latest.get('y', 0))
                prev_nav = float(prev.get('y', 0))
                change_pct = ((latest_nav - prev_nav) / prev_nav * 100) if prev_nav else 0
                # Timestamp to date
                ts = latest.get('x', 0) / 1000
                nav_date = datetime.fromtimestamp(ts).strftime('%Y-%m-%d') if ts > 0 else ''
                return {
                    'source': 'nav',
                    'code': fund_code,
                    'name': name_match.group(1),
                    'date': nav_date,
                    'nav': latest_nav,
                    'nav_date': nav_date,
                    'estimate_value': latest_nav,
                    'change_pct': round(change_pct, 4),
                }
        return None
    except Exception:
        return None


def get_fund(fund_code: str, prefer: str = 'auto') -> dict | None:
    """获取基金数据，自动 fallback"""
    if prefer == 'estimate':
        return get_estimate(fund_code)
    elif prefer == 'nav':
        return get_nav(fund_code)
    else:
        # auto: try estimate first, fallback to nav
        result = get_estimate(fund_code)
        if result:
            return result
        return get_nav(fund_code)


def format_fund(data: dict) -> str:
    """格式化输出"""
    if not data:
        return "❌ 未找到基金数据"
    
    arrow = "↑" if data['change_pct'] >= 0 else "↓"
    source_tag = "📊 估值" if data['source'] == 'estimate' else "📋 净值"
    
    return f"""**{data['name']}**（{data['code']}）
{source_tag}：{data['estimate_value']:.4f} 元 | 涨跌：{data['change_pct']:+.2f}% {arrow}
净值：{data['nav']:.4f} 元（{data['nav_date']}）"""


def calc_daily_pnl(data: dict, amount: float) -> dict:
    """计算单只基金的当日盈亏"""
    pnl = amount * (data['change_pct'] / 100)
    return {
        'code': data['code'],
        'name': data['name'],
        'amount': amount,
        'change_pct': data['change_pct'],
        'daily_pnl': round(pnl, 2),
        'source': data['source'],
    }


def portfolio_analysis(portfolio_file: str) -> str:
    """批量持仓分析"""
    with open(portfolio_file) as f:
        portfolio = json.load(f)
    
    results = []
    total_amount = 0
    total_pnl = 0
    errors = []
    
    for item in portfolio:
        code = item['code']
        amount = item['amount']
        name = item.get('name', code)
        total_amount += amount
        
        data = get_fund(code)
        if data:
            pnl_info = calc_daily_pnl(data, amount)
            results.append(pnl_info)
            total_pnl += pnl_info['daily_pnl']
        else:
            errors.append(f"  ❌ {name} ({code}): 查询失败")
            results.append({
                'code': code, 'name': name, 'amount': amount,
                'change_pct': 0, 'daily_pnl': 0, 'source': 'error'
            })
    
    # Sort by daily P&L
    results.sort(key=lambda x: x['daily_pnl'], reverse=True)
    
    # Build output
    lines = [f"# 持仓日报 ({datetime.now().strftime('%Y-%m-%d %H:%M')})\n"]
    lines.append(f"**总资产**: {total_amount:,.2f} 元 | **今日盈亏**: {total_pnl:+,.2f} 元 ({total_pnl/total_amount*100:+.2f}%)\n")
    
    lines.append("## 盈利\n")
    for r in results:
        if r['daily_pnl'] > 0:
            src = "📊" if r['source'] == 'estimate' else "📋"
            lines.append(f"  {src} {r['name']}: {r['daily_pnl']:+,.2f} ({r['change_pct']:+.2f}%) — {r['amount']:,.0f} 元")
    
    lines.append("\n## 亏损\n")
    for r in results:
        if r['daily_pnl'] < 0:
            src = "📊" if r['source'] == 'estimate' else "📋"
            lines.append(f"  {src} {r['name']}: {r['daily_pnl']:+,.2f} ({r['change_pct']:+.2f}%) — {r['amount']:,.0f} 元")
    
    lines.append("\n## 持平\n")
    for r in results:
        if r['daily_pnl'] == 0:
            src = "📊" if r['source'] == 'estimate' else "📋"
            lines.append(f"  {src} {r['name']}: {r['daily_pnl']:+,.2f} ({r['change_pct']:+.2f}%) — {r['amount']:,.0f} 元")
    
    if errors:
        lines.append("\n## 查询失败\n")
        lines.extend(errors)
    
    lines.append(f"\n---\n📊 = 实时估值 | 📋 = 最新净值（QDII/非交易时段）")
    
    return '\n'.join(lines)


def main():
    if len(sys.argv) < 2:
        print("用法:")
        print("  python3 fund_query_v2.py <fund_code> [estimate|nav]")
        print("  python3 fund_query_v2.py --portfolio <json_file>")
        sys.exit(1)
    
    if sys.argv[1] == '--portfolio':
        if len(sys.argv) < 3:
            print("需要指定 JSON 文件路径")
            sys.exit(1)
        print(portfolio_analysis(sys.argv[2]))
        return
    
    fund_code = sys.argv[1].strip()
    if not VALID_CODE_PATTERN.match(fund_code):
        print(f"❌ 非法基金代码 '{fund_code}'，必须是6位数字")
        sys.exit(1)
    
    prefer = sys.argv[2].strip().lower() if len(sys.argv) > 2 else 'auto'
    data = get_fund(fund_code, prefer)
    print(format_fund(data))


if __name__ == '__main__':
    main()
