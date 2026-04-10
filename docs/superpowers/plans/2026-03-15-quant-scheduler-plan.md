# A 股量化交易系统 - 周期运行实现计划

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为量化交易系统添加周期运行功能，实现每日自动获取小微盘股数据、生成信号、发送邮件通知。

**Architecture:** 新增 stock_pool（股票池筛选）、scheduler（定时任务）、notifier（邮件通知）三个模块，通过 cron 定时触发。

**Tech Stack:** Python 3.8+, AkShare, smtplib (邮件), cron (定时任务)

---

## Chunk 1: 项目目录与配置

### Task 1: 创建目录结构

**Files:**
- Create: `quant-system/scheduler/`
- Create: `quant-system/stock_pool/`
- Create: `quant-system/notifier/`
- Create: `quant-system/logs/`

- [ ] **Step 1: 创建目录**

```bash
cd /home/admin/openclaw/workspace/quant-system
mkdir -p scheduler stock_pool notifier logs
```

- [ ] **Step 2: 创建模块__init__.py**

```bash
touch scheduler/__init__.py
touch stock_pool/__init__.py
touch notifier/__init__.py
touch logs/.gitkeep
```

- [ ] **Step 3: 验证目录结构**

```bash
ls -la scheduler/ stock_pool/ notifier/ logs/
```

Expected: 显示四个目录内容

---

### Task 2: 更新配置文件

**Files:**
- Modify: `quant-system/config.py`

- [ ] **Step 1: 添加邮件和股票池配置到 config.py**

```python
# 邮件配置
SMTP_SERVER = "smtp.163.com"
SMTP_PORT = 465
SMTP_USER = ""  # 需要用户配置 163 邮箱
SMTP_PASS = ""  # 需要用户配置授权码
NOTIFY_EMAIL = "wuyihao13@gmail.com"

# 股票池配置
SMALL_CAP_TOP_N = 300  # 筛选前 N 只小微盘股
WATCHLIST_FILE = PROJECT_ROOT / "stock_pool" / "watchlist.txt"

# 日志配置
LOG_DIR = PROJECT_ROOT / "logs"
LOG_LEVEL = "INFO"
```

- [ ] **Step 2: 提交**

```bash
cd /home/admin/openclaw/workspace/quant-system
git add config.py 2>/dev/null || echo "Not a git repo"
```

---

## Chunk 2: 股票池筛选模块

### Task 3: 实现股票池筛选

**Files:**
- Create: `quant-system/stock_pool/screener.py`

- [ ] **Step 1: 编写筛选函数**

```python
# quant-system/stock_pool/screener.py
"""
股票池筛选模块 - 筛选小微盘股
"""
import akshare as ak
import pandas as pd
from pathlib import Path
from datetime import datetime


def get_all_stocks() -> pd.DataFrame:
    """
    获取全市场股票列表和市值数据
    
    Returns:
        DataFrame with columns: code, name, market_cap
    """
    try:
        # 获取 A 股实时行情
        df = ak.stock_zh_a_spot_em()
        
        # 选择需要的列
        df = df[["代码", "名称", "总市值", "流通市值"]]
        df = df.rename(columns={
            "代码": "code",
            "名称": "name",
            "总市值": "market_cap",
            "流通市值": "float_cap"
        })
        
        # 过滤掉空值
        df = df.dropna(subset=["market_cap"])
        
        # 转换市值为数值
        df["market_cap"] = pd.to_numeric(df["market_cap"], errors="coerce")
        
        return df
        
    except Exception as e:
        raise Exception(f"获取股票列表失败：{str(e)}")


def filter_small_cap(df: pd.DataFrame, top_n: int = 300) -> pd.DataFrame:
    """
    筛选市值最小的 N 只股票
    
    Args:
        df: 股票列表 DataFrame
        top_n: 筛选前 N 只
    
    Returns:
        筛选后的 DataFrame
    """
    # 按市值升序排序
    df_sorted = df.sort_values("market_cap", ascending=True)
    
    # 取前 N 只
    result = df_sorted.head(top_n).copy()
    
    return result


def save_watchlist(df: pd.DataFrame, filepath: Path) -> None:
    """保存股票列表到文件"""
    filepath.parent.mkdir(parents=True, exist_ok=True)
    df[["code", "name"]].to_csv(filepath, index=False, encoding="utf-8")


def load_watchlist(filepath: Path) -> pd.DataFrame:
    """从文件加载股票列表"""
    if filepath.exists():
        return pd.read_csv(filepath, encoding="utf-8")
    return None


def update_watchlist(top_n: int = 300, save_path: Path = None) -> pd.DataFrame:
    """
    更新股票池
    
    Args:
        top_n: 筛选前 N 只
        save_path: 保存路径
    
    Returns:
        筛选后的股票列表
    """
    # 获取全市场数据
    all_stocks = get_all_stocks()
    
    # 筛选小微盘
    small_cap = filter_small_cap(all_stocks, top_n)
    
    # 保存
    if save_path:
        save_watchlist(small_cap, save_path)
    
    return small_cap
```

- [ ] **Step 2: 更新 stock_pool/__init__.py**

```python
# quant-system/stock_pool/__init__.py
from .screener import (
    get_all_stocks,
    filter_small_cap,
    save_watchlist,
    load_watchlist,
    update_watchlist,
)

__all__ = [
    "get_all_stocks",
    "filter_small_cap",
    "save_watchlist",
    "load_watchlist",
    "update_watchlist",
]
```

---

## Chunk 3: 邮件通知模块

### Task 4: 实现邮件通知

**Files:**
- Create: `quant-system/notifier/email_notifier.py`

- [ ] **Step 1: 编写邮件发送函数**

```python
# quant-system/notifier/email_notifier.py
"""
邮件通知模块
"""
import smtplib
from email.mime.text import MIMEText
from email.mime.multipart import MIMEMultipart
from email.header import Header
from datetime import datetime
from typing import List, Dict


def format_signal_html(signals: List[Dict], date: str = None) -> str:
    """
    生成 HTML 格式的邮件内容
    
    Args:
        signals: 信号列表，每个包含 type, code, name, price, strength
        date: 日期
    
    Returns:
        HTML 字符串
    """
    if date is None:
        date = datetime.now().strftime("%Y-%m-%d")
    
    # 分离买入和卖出信号
    buy_signals = [s for s in signals if s["type"] == "buy"]
    sell_signals = [s for s in signals if s["type"] == "sell"]
    
    html = f"""
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; }}
        h2 {{ color: #333; }}
        h3 {{ color: #666; border-bottom: 1px solid #ddd; padding-bottom: 5px; }}
        table {{ border-collapse: collapse; width: 100%; margin: 10px 0; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #f2f2f2; }}
        .buy {{ color: #d9534f; }}
        .sell {{ color: #5cb85c; }}
        hr {{ border: none; border-top: 1px solid #ddd; margin: 20px 0; }}
        .footer {{ color: #999; font-size: 12px; }}
    </style>
</head>
<body>
    <h2>📊 交易信号通知</h2>
    <p>日期：{date}</p>
"""
    
    # 买入信号
    if buy_signals:
        html += f"""
    <h3 class="buy">🟢 买入信号 ({len(buy_signals)})</h3>
    <table>
        <tr><th>代码</th><th>名称</th><th>价格</th><th>信号强度</th></tr>
"""
        for s in buy_signals:
            html += f"""        <tr><td>{s['code']}</td><td>{s['name']}</td><td>{s['price']:.2f}</td><td>{s['strength']}</td></tr>
"""
        html += """    </table>
"""
    
    # 卖出信号
    if sell_signals:
        html += f"""
    <h3 class="sell">🔴 卖出信号 ({len(sell_signals)})</h3>
    <table>
        <tr><th>代码</th><th>名称</th><th>价格</th><th>信号强度</th></tr>
"""
        for s in sell_signals:
            html += f"""        <tr><td>{s['code']}</td><td>{s['name']}</td><td>{s['price']:.2f}</td><td>{s['strength']}</td></tr>
"""
        html += """    </table>
"""
    
    if not buy_signals and not sell_signals:
        html += """
    <p>今日无交易信号。</p>
"""
    
    html += f"""
    <hr>
    <p class="footer">
        策略：MA+MACD+RSI 组合<br>
        本邮件由 A 股量化交易系统自动生成
    </p>
</body>
</html>
"""
    
    return html


def send_signal_email(signals: List[Dict],
                      recipient: str,
                      smtp_server: str,
                      smtp_port: int,
                      smtp_user: str,
                      smtp_pass: str,
                      subject_prefix: str = "[量化信号]") -> bool:
    """
    发送信号邮件
    
    Args:
        signals: 信号列表
        recipient: 收件人
        smtp_server: SMTP 服务器
        smtp_port: SMTP 端口
        smtp_user: SMTP 用户名
        smtp_pass: SMTP 密码/授权码
        subject_prefix: 邮件主题前缀
    
    Returns:
        bool: 是否发送成功
    """
    if not signals:
        return True  # 无信号，不需要发送
    
    date = datetime.now().strftime("%Y-%m-%d")
    
    # 统计信号
    buy_count = sum(1 for s in signals if s["type"] == "buy")
    sell_count = sum(1 for s in signals if s["type"] == "sell")
    
    # 邮件主题
    subject = f"{subject_prefix} 买入：{buy_count}只 | 卖出：{sell_count}只 - {date}"
    
    # 邮件内容
    html_content = format_signal_html(signals, date)
    
    # 创建邮件
    msg = MIMEMultipart("alternative")
    msg["From"] = smtp_user
    msg["To"] = recipient
    msg["Subject"] = Header(subject, "utf-8")
    
    # 添加 HTML 内容
    part = MIMEText(html_content, "html", "utf-8")
    msg.attach(part)
    
    try:
        # 发送邮件
        server = smtplib.SMTP_SSL(smtp_server, smtp_port)
        server.login(smtp_user, smtp_pass)
        server.sendmail(smtp_user, [recipient], msg.as_string())
        server.quit()
        
        return True
        
    except Exception as e:
        print(f"邮件发送失败：{str(e)}")
        return False
```

- [ ] **Step 2: 更新 notifier/__init__.py**

```python
# quant-system/notifier/__init__.py
from .email_notifier import format_signal_html, send_signal_email

__all__ = [
    "format_signal_html",
    "send_signal_email",
]
```

---

## Chunk 4: 定时任务模块

### Task 5: 实现定时任务入口

**Files:**
- Create: `quant-system/scheduler/run_daily.py`

- [ ] **Step 1: 编写每日任务脚本**

```python
#!/usr/bin/env python3
"""
每日定时任务入口

使用方法:
    python3 run_daily.py
"""
import sys
import logging
from pathlib import Path
from datetime import datetime, timedelta

# 添加项目根目录到路径
sys.path.insert(0, str(Path(__file__).parent.parent))

from config import (
    SMALL_CAP_TOP_N,
    WATCHLIST_FILE,
    LOG_DIR,
    LOG_LEVEL,
    SMTP_SERVER,
    SMTP_PORT,
    SMTP_USER,
    SMTP_PASS,
    NOTIFY_EMAIL,
)
from stock_pool.screener import update_watchlist, load_watchlist
from data.downloader import fetch_stock_data, validate_data
from strategy.signal import generate_signals
from notifier.email_notifier import send_signal_email


# 配置日志
LOG_DIR.mkdir(parents=True, exist_ok=True)
log_file = LOG_DIR / f"daily_{datetime.now().strftime('%Y%m%d')}.log"

logging.basicConfig(
    level=getattr(logging, LOG_LEVEL),
    format="%(asctime)s %(levelname)s - %(message)s",
    handlers=[
        logging.FileHandler(log_file, encoding="utf-8"),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger(__name__)


def is_trading_day() -> bool:
    """
    判断是否为交易日
    
    Returns:
        bool: 是否为交易日
    """
    now = datetime.now()
    
    # 周末检查
    if now.weekday() >= 5:  # 5=周六，6=周日
        return False
    
    # TODO: 节假日检查（需要接入节假日 API）
    
    return True


def run_daily_task() -> dict:
    """
    执行每日任务
    
    Returns:
        任务执行结果
    """
    result = {
        "success": False,
        "date": datetime.now().strftime("%Y-%m-%d"),
        "stocks_processed": 0,
        "buy_signals": 0,
        "sell_signals": 0,
        "email_sent": False,
        "error": None
    }
    
    try:
        # 1. 检查交易日
        logger.info("开始执行每日任务")
        
        if not is_trading_day():
            logger.info("今日非交易日，跳过")
            result["success"] = True
            return result
        
        logger.info("检查交易日：是")
        
        # 2. 更新股票池
        logger.info("更新股票池...")
        watchlist = update_watchlist(SMALL_CAP_TOP_N, WATCHLIST_FILE)
        logger.info(f"筛选小微盘股：{len(watchlist)}只")
        result["stocks_processed"] = len(watchlist)
        
        # 3. 获取数据并生成信号
        signals = []
        
        for _, row in watchlist.iterrows():
            code = row["code"]
            name = row["name"]
            
            try:
                # 获取最近 60 天数据（用于计算指标）
                end_date = datetime.now()
                start_date = end_date - timedelta(days=90)
                
                df = fetch_stock_data(
                    code,
                    start_date.strftime("%Y-%m-%d"),
                    end_date.strftime("%Y-%m-%d")
                )
                
                if not validate_data(df):
                    continue
                
                # 生成信号
                df = generate_signals(df)
                
                # 检查最新信号
                latest_signal = df.iloc[-1]["signal"]
                
                if latest_signal == 1:  # 买入
                    signals.append({
                        "type": "buy",
                        "code": code,
                        "name": name,
                        "price": df.iloc[-1]["close"],
                        "strength": "强 (3/3)"  # 简化处理
                    })
                    result["buy_signals"] += 1
                    
                elif latest_signal == -1:  # 卖出
                    signals.append({
                        "type": "sell",
                        "code": code,
                        "name": name,
                        "price": df.iloc[-1]["close"],
                        "strength": "中 (2/3)"  # 简化处理
                    })
                    result["sell_signals"] += 1
                    
            except Exception as e:
                logger.debug(f"{code} 处理失败：{str(e)}")
                continue
        
        logger.info(f"生成信号：买入{result['buy_signals']}，卖出{result['sell_signals']}")
        
        # 4. 发送邮件
        if signals and SMTP_USER and SMTP_PASS:
            logger.info("发送邮件通知...")
            success = send_signal_email(
                signals=signals,
                recipient=NOTIFY_EMAIL,
                smtp_server=SMTP_SERVER,
                smtp_port=SMTP_PORT,
                smtp_user=SMTP_USER,
                smtp_pass=SMTP_PASS
            )
            result["email_sent"] = success
            logger.info(f"邮件发送：{'成功' if success else '失败'}")
        elif signals:
            logger.warning("邮件配置为空，跳过发送")
        
        result["success"] = True
        logger.info("任务完成")
        
    except Exception as e:
        result["error"] = str(e)
        logger.error(f"任务执行失败：{str(e)}")
    
    return result


def main():
    """主函数"""
    result = run_daily_task()
    
    if result["success"]:
        print(f"\n✅ 任务完成 - {result['date']}")
        print(f"   处理股票：{result['stocks_processed']}只")
        print(f"   买入信号：{result['buy_signals']}")
        print(f"   卖出信号：{result['sell_signals']}")
        print(f"   邮件发送：{'成功' if result['email_sent'] else '跳过'}")
    else:
        print(f"\n❌ 任务失败：{result['error']}")
        sys.exit(1)


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: 创建 crontab 配置示例**

```bash
# quant-system/scheduler/crontab.txt
# A 股量化交易系统 - Cron 配置示例
# 
# 使用方法:
# 1. 编辑 crontab: crontab -e
# 2. 添加以下内容（修改路径）:

# 每日 20:00 运行（包括周末，脚本内部会判断交易日）
0 20 * * * cd /home/admin/openclaw/workspace/quant-system && python3 scheduler/run_daily.py >> logs/cron.log 2>&1
```

- [ ] **Step 3: 更新 scheduler/__init__.py**

```python
# quant-system/scheduler/__init__.py
from .run_daily import run_daily_task, is_trading_day

__all__ = [
    "run_daily_task",
    "is_trading_day",
]
```

---

## Chunk 5: 测试与集成

### Task 6: 测试股票池筛选

**Files:**
- 无新建文件

- [ ] **Step 1: 测试股票池筛选功能**

```bash
cd /home/admin/openclaw/workspace/quant-system
python3 -c "
from stock_pool.screener import get_all_stocks, filter_small_cap

# 获取股票列表
df = get_all_stocks()
print(f'全市场股票：{len(df)}只')

# 筛选前 10 只最小市值
small = filter_small_cap(df, 10)
print('\\n市值最小的 10 只股票:')
print(small[['code', 'name', 'market_cap']].to_string())
"
```

Expected: 显示股票列表

---

### Task 7: 测试邮件通知

**Files:**
- 无新建文件

- [ ] **Step 1: 测试邮件发送（需要配置 SMTP）**

```bash
cd /home/admin/openclaw/workspace/quant-system
python3 -c "
from notifier.email_notifier import send_signal_email

# 测试信号
signals = [
    {'type': 'buy', 'code': '300123', 'name': '测试股 A', 'price': 12.34, 'strength': '强 (3/3)'},
    {'type': 'sell', 'code': '000456', 'name': '测试股 B', 'price': 23.45, 'strength': '中 (2/3)'},
]

# 需要配置 SMTP 信息
success = send_signal_email(
    signals=signals,
    recipient='wuyihao13@gmail.com',
    smtp_server='smtp.163.com',
    smtp_port=465,
    smtp_user='YOUR_163_EMAIL',  # 替换为你的 163 邮箱
    smtp_pass='YOUR_AUTH_CODE'   # 替换为授权码
)

print(f'邮件发送：{\"成功\" if success else \"失败\"}')
"
```

---

### Task 8: 完整系统测试

**Files:**
- 无新建文件

- [ ] **Step 1: 运行每日任务（手动测试）**

```bash
cd /home/admin/openclaw/workspace/quant-system
python3 scheduler/run_daily.py
```

Expected: 显示任务执行日志

- [ ] **Step 2: 检查日志文件**

```bash
ls -la logs/
cat logs/daily_*.log | tail -20
```

- [ ] **Step 3: 检查生成的文件**

```bash
cat stock_pool/watchlist.txt | head -10
```

---

## Chunk 6: 文档与配置

### Task 9: 更新 README

**Files:**
- Modify: `quant-system/README.md`

- [ ] **Step 1: 添加定时任务使用说明**

在 README.md 中添加：

```markdown
## 定时任务

### 配置邮件

编辑 `config.py`，填写 163 邮箱信息：

```python
SMTP_USER = "your_email@163.com"
SMTP_PASS = "your_authorization_code"  # 在 163 邮箱设置中获取授权码
```

### 设置 Cron

```bash
# 编辑 crontab
crontab -e

# 添加配置（每日 20:00 运行）
0 20 * * * cd /home/admin/openclaw/workspace/quant-system && python3 scheduler/run_daily.py >> logs/cron.log 2>&1
```

### 手动运行

```bash
python3 scheduler/run_daily.py
```

### 查看日志

```bash
tail -f logs/daily_$(date +%Y%m%d).log
```
```

---

### Task 10: 创建配置说明文档

**Files:**
- Create: `quant-system/SETUP.md`

- [ ] **Step 1: 创建配置说明文档**

```markdown
# 量化交易系统 - 配置说明

## 邮件通知配置

### 1. 获取 163 邮箱授权码

1. 登录 163 邮箱 (mail.163.com)
2. 点击"设置" → "POP3/SMTP/IMAP"
3. 开启"SMTP 服务"
4. 点击"授权码管理"，获取授权码

### 2. 配置 config.py

编辑 `quant-system/config.py`:

```python
SMTP_USER = "your_email@163.com"
SMTP_PASS = "your_authorization_code"  # 使用授权码，不是登录密码
NOTIFY_EMAIL = "wuyihao13@gmail.com"   # 收件人邮箱
```

### 3. 测试邮件发送

```bash
cd quant-system
python3 -c "
from notifier.email_notifier import send_signal_email
send_signal_email(
    signals=[{'type': 'buy', 'code': 'test', 'name': '测试', 'price': 10.0, 'strength': '测试'}],
    recipient='wuyihao13@gmail.com',
    smtp_server='smtp.163.com',
    smtp_port=465,
    smtp_user='YOUR_EMAIL',
    smtp_pass='YOUR_CODE'
)
"
```

## Cron 配置

### 1. 编辑 Crontab

```bash
crontab -e
```

### 2. 添加配置

```bash
# 每日 20:00 运行
0 20 * * * cd /home/admin/openclaw/workspace/quant-system && python3 scheduler/run_daily.py >> logs/cron.log 2>&1
```

### 3. 验证配置

```bash
crontab -l
```

## 常见问题

### 邮件发送失败

1. 检查 SMTP_USER 和 SMTP_PASS 是否正确
2. 确认使用的是授权码，不是登录密码
3. 检查 163 邮箱 SMTP 服务是否开启

### 数据获取失败

1. 检查网络连接
2. AkShare 接口可能临时不可用，稍后重试
3. 查看日志文件了解详情
```

---

## 完成检查清单

- [ ] 目录结构创建完成
- [ ] 配置文件更新完成
- [ ] 股票池筛选模块测试通过
- [ ] 邮件通知模块测试通过
- [ ] 定时任务脚本测试通过
- [ ] README 更新完成
- [ ] 配置说明文档创建完成
- [ ] 完整系统测试通过

---

## 用户待办事项

系统部署后，用户需要：

1. **配置 163 邮箱**: 获取授权码并填写到 `config.py`
2. **设置 cron**: 添加定时任务配置
3. **首次运行测试**: 手动运行 `python3 scheduler/run_daily.py` 验证功能
