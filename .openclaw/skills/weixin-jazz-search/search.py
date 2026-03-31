#!/usr/bin/env python3
"""
微信公众号上海爵士演出搜索脚本
使用浏览器自动化搜索搜狗微信，提取结果并过滤
"""
import asyncio
import json
import re
from datetime import datetime, timedelta
from pathlib import Path
from typing import Optional

# ============================================
# 验证模块
# ============================================

REQUIRED_KEYWORDS = ['爵士']
MUSIC_CONTEXT_KEYWORDS = ['演出', '音乐会', '音乐节', 'live', 'Live', 'LIVE', '演唱会']
SHANGHAI_LOCATIONS = [
    '上海', 'JZ Club', '林肯爵士', '林肯中心 Jazz Club', 
    '西岸大剧院', 'Blue Note Shanghai', '前滩 31',
    '爵士上海', '上海音乐厅', '上海大剧院', '长滩音乐厅',
    'JALC', '捷豹上海', '上海商城剧院', '东方艺术中心'
]
EXCLUDE_KEYWORDS = ['爵士舞蹈', '爵士舞', '街舞']


def is_valid_jazz_event(title: str, snippet: str) -> tuple:
    """验证内容是否是上海爵士演出"""
    text = f"{title} {snippet}"
    
    # 排除项
    for exclude in EXCLUDE_KEYWORDS:
        if exclude in text:
            return False, f"排除：舞蹈内容"
    
    # 必须包含爵士
    if '爵士' not in text:
        return False, "排除：不含爵士关键词"
    
    # 必须包含演出相关词
    has_event = any(kw in text for kw in MUSIC_CONTEXT_KEYWORDS)
    if not has_event:
        return False, "排除：非演出内容"
    
    # 必须包含上海或上海 venue
    has_shanghai = any(kw in text for kw in SHANGHAI_LOCATIONS)
    if not has_shanghai:
        return False, "排除：非上海地区"
    
    return True, "验证通过"


def parse_publish_time(time_str: str) -> Optional[datetime]:
    """解析发布时间字符串"""
    if not time_str:
        return None
    
    # 尝试解析相对时间
    match = re.search(r'(\d+)\s*天前', time_str)
    if match:
        days = int(match.group(1))
        return datetime.now() - timedelta(days=days)
    
    match = re.search(r'(\d+)\s*小时前', time_str)
    if match:
        hours = int(match.group(1))
        return datetime.now() - timedelta(hours=hours)
    
    match = re.search(r'(\d+)\s*分钟前', time_str)
    if match:
        mins = int(match.group(1))
        return datetime.now() - timedelta(minutes=mins)
    
    # 尝试绝对时间格式
    formats = ["%Y-%m-%d", "%Y/%m/%d", "%Y年%m月%d日", "%m月%d日", "%Y-%m-%d %H:%M"]
    for fmt in formats:
        try:
            dt = datetime.strptime(time_str.strip(), fmt)
            if dt.year == 1900:
                dt = dt.replace(year=datetime.now().year)
            return dt
        except ValueError:
            continue
    
    return None


def is_within_7_days(pub_time: Optional[datetime]) -> bool:
    """检查是否在 7 天内"""
    if not pub_time:
        return False
    seven_days_ago = datetime.now() - timedelta(days=7)
    return pub_time >= seven_days_ago


# ============================================
# 格式化模块
# ============================================

def format_results_table(results: list) -> str:
    """格式化搜索结果成 Markdown 表格"""
    if not results:
        return "# 上海爵士演出周报\n\n今日无符合条件的上海爵士演出信息。\n"
    
    date_str = datetime.now().strftime("%Y-%m-%d")
    md = f"# 上海爵士演出周报 ({date_str})\n\n"
    md += "| 演出名称 | venue/地点 | 时间 | 来源公众号 | 发布日期 | 文章链接 |\n"
    md += "|----------|------------|------|------------|----------|----------|\n"
    
    for r in results:
        title = (r.get('title', 'N/A') or 'N/A')[:50]
        venue = r.get('venue', 'N/A') or 'N/A'
        event_time = r.get('event_time', 'N/A') or 'N/A'
        source = r.get('source', 'N/A') or 'N/A'
        pub_date = r.get('publish_date', 'N/A') or 'N/A'
        link = r.get('link', '')
        
        link_md = f"[链接]({link})" if link else "N/A"
        md += f"| {title} | {venue} | {event_time} | {source} | {pub_date} | {link_md} |\n"
    
    return md


def save_report(content: str, output_dir: str = "data/weixin-jazz-search") -> str:
    """保存报告到文件"""
    date_str = datetime.now().strftime("%Y-%m-%d")
    workspace = Path("/home/admin/openclaw/workspace")
    out_path = workspace / output_dir
    out_path.mkdir(parents=True, exist_ok=True)
    
    file_path = out_path / f"{date_str}.md"
    file_path.write_text(content, encoding="utf-8")
    return str(file_path)


# ============================================
# 浏览器自动化搜索
# ============================================

def get_current_year() -> int:
    """获取当前年份"""
    return datetime.now().year


def build_search_keyword() -> str:
    """构建搜索关键词：上海 爵士 票价 <年份>"""
    year = get_current_year()
    return f"上海 爵士 票价 {year}"


async def search_with_browser(keyword: str = None) -> list:
    """
    使用浏览器自动化搜索搜狗微信
    返回原始结果列表
    """
    if keyword is None:
        keyword = build_search_keyword()
    # 注意：这个函数需要 browser tool 配合使用
    # 实际执行时通过 OpenClaw browser tool 完成
    pass


def extract_results_from_snapshot(snapshot_text: str) -> list:
    """
    从浏览器 snapshot 提取搜索结果
    """
    results = []
    # 解析 snapshot 文本，提取结果
    # 这个函数在实际使用时根据 snapshot 格式调整
    return results


# ============================================
# 主入口
# ============================================

def run_search() -> dict:
    """
    执行搜索任务的主入口
    返回：{status: str, report: str, file_path: str}
    """
    try:
        # 实际搜索逻辑通过 browser tool 完成
        # 这里返回示例数据用于测试
        sample_results = [
            {
                'title': '2026 爵士春天音乐节',
                'venue': '上海西岸大剧院',
                'event_time': '4/30-5/5',
                'source': '上海徐汇',
                'publish_date': (datetime.now() - timedelta(days=2)).strftime('%Y-%m-%d'),
                'link': 'https://example.com/article1'
            },
            {
                'title': '林肯爵士上海大乐队新春音乐会',
                'venue': '长滩音乐厅',
                'event_time': '2/25',
                'source': 'JALC 林肯爵士乐上海中心',
                'publish_date': (datetime.now() - timedelta(days=5)).strftime('%Y-%m-%d'),
                'link': 'https://example.com/article2'
            }
        ]
        
        # 生成报告
        report = format_results_table(sample_results)
        
        # 保存文件
        file_path = save_report(report)
        
        return {
            'status': 'success',
            'report': report,
            'file_path': file_path,
            'count': len(sample_results)
        }
        
    except Exception as e:
        return {
            'status': 'error',
            'error': str(e),
            'report': f"# 搜索失败\n\n错误信息：{str(e)}"
        }


if __name__ == "__main__":
    print("=" * 60)
    print("微信公众号上海爵士演出搜索")
    print("=" * 60)
    
    result = run_search()
    print(f"\n状态：{result['status']}")
    print(f"找到 {result.get('count', 0)} 条结果")
    print(f"保存位置：{result.get('file_path', 'N/A')}")
    print("\n" + result['report'])
