# A 股入门级量化交易系统设计文档

**创建时间**: 2026-03-15  
**状态**: 已批准  
**版本**: 1.0

---

## 1. 系统概述

构建一个入门级 A 股量化交易系统，使用技术指标组合策略生成交易信号，支持历史回测，输出 Markdown 格式报告。系统不涉及自动下单，仅提供信号和回测分析功能。

### 1.1 设计目标

- **入门友好**: 代码透明、结构清晰、便于学习
- **免费数据**: 使用 AkShare 免费数据源
- **轻量回测**: 自研回测引擎，代码完全可控
- **实用输出**: Markdown 报告 + CSV 信号文件

---

## 2. 技术选型

| 模块 | 选择 | 说明 |
|------|------|------|
| 编程语言 | Python 3.8+ | 量化生态成熟 |
| 数据源 | AkShare | 开源免费、A 股数据全面 |
| 策略类型 | 技术指标组合 | MA + MACD + RSI |
| 数据频率 | 日线 | 适合上班族，信号稳定 |
| 回测引擎 | 自研轻量级 | 代码透明、易学习修改 |
| 输出格式 | Markdown + CSV | 可读性强、便于分享 |

---

## 3. 策略逻辑

### 3.1 技术指标

| 指标 | 参数 | 作用 |
|------|------|------|
| MA 均线 | 5 日、20 日 | 判断趋势方向 |
| MACD | (12, 26, 9) | 确认动能变化 |
| RSI | 14 日 | 过滤超买超卖 |

### 3.2 买入信号（同时满足）

1. 5 日线上穿 20 日线（金叉）
2. MACD 柱状图由负转正
3. RSI < 70（非超买区）

### 3.3 卖出信号（满足任一）

1. 5 日线下穿 20 日线（死叉）
2. MACD 柱状图由正转负
3. RSI > 80（严重超买）

---

## 4. 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                        main.py                              │
│                     （主入口）                               │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌──────────────┐    ┌──────────────────┐    ┌──────────────┐
│   data/      │    │    strategy/     │    │  backtest/   │
│  downloader  │───▶│    signal.py     │───▶│   engine.py  │
└──────────────┘    └──────────────────┘    └──────────────┘
        │                     │                     │
        │                     │                     ▼
        │                     │            ┌──────────────┐
        │                     │            │  reports/    │
        │                     │            │  markdown.py │
        │                     │            └──────────────┘
        ▼                     │
┌──────────────┐              │
│  indicators/ │──────────────┘
│  MA/MACD/RSI │
└──────────────┘
```

---

## 5. 项目结构

```
quant-system/
├── data/                   # 数据模块
│   ├── __init__.py
│   ├── downloader.py       # AkShare 数据下载
│   └── cache/              # 本地数据缓存
├── indicators/             # 技术指标计算
│   ├── __init__.py
│   ├── ma.py               # 均线指标
│   ├── macd.py             # MACD 指标
│   └── rsi.py              # RSI 指标
├── strategy/               # 策略模块
│   ├── __init__.py
│   └── signal.py           # 信号生成逻辑
├── backtest/               # 回测模块
│   ├── __init__.py
│   ├── engine.py           # 回测核心引擎
│   └── metrics.py          # 性能指标计算
├── reports/                # 报告模块
│   ├── __init__.py
│   └── markdown.py         # Markdown 报告生成
├── config.py               # 配置管理
├── main.py                 # 主入口
├── requirements.txt        # 依赖列表
└── README.md               # 使用说明
```

---

## 6. 核心模块设计

### 6.1 数据模块 (data/)

**职责**: 从 AkShare 获取 A 股日线数据，支持本地缓存

**主要函数**:
```python
def fetch_stock_data(symbol: str, start_date: str, end_date: str) -> pd.DataFrame
def cache_data(symbol: str, data: pd.DataFrame) -> None
def load_cached_data(symbol: str) -> Optional[pd.DataFrame]
```

**数据字段**:
- date: 日期
- open: 开盘价
- high: 最高价
- low: 最低价
- close: 收盘价
- volume: 成交量

### 6.2 指标模块 (indicators/)

**MA 均线**:
```python
def calculate_ma(close: pd.Series, period: int) -> pd.Series
```

**MACD**:
```python
def calculate_macd(close: pd.Series) -> Tuple[pd.Series, pd.Series, pd.Series]
# 返回：DIF, DEA, MACD 柱
```

**RSI**:
```python
def calculate_rsi(close: pd.Series, period: int = 14) -> pd.Series
```

### 6.3 策略模块 (strategy/)

**信号生成**:
```python
def generate_signals(df: pd.DataFrame) -> pd.DataFrame
# 返回包含 signal 列的 DataFrame: 1=买入，-1=卖出，0=持有
```

### 6.4 回测模块 (backtest/)

**回测引擎**:
```python
class BacktestEngine:
    def __init__(self, initial_capital: float = 100000)
    def run(self, data: pd.DataFrame, signals: pd.Series) -> BacktestResult
```

**性能指标**:
- 总收益率 (%)
- 年化收益率 (%)
- 最大回撤 (%)
- 夏普比率
- 胜率 (%)
- 盈亏比
- 交易次数

### 6.5 报告模块 (reports/)

**Markdown 报告生成**:
```python
def generate_report(result: BacktestResult, symbol: str) -> str
def save_report(report: str, filepath: str) -> None
```

---

## 7. 数据流

```
用户输入 (股票代码、时间范围)
         │
         ▼
┌─────────────────┐
│  data.downloader│ ───▶ AkShare API
└─────────────────┘
         │
         ▼
    原始行情数据
         │
         ▼
┌─────────────────┐
│  indicators.*   │ ───▶ 计算 MA/MACD/RSI
└─────────────────┘
         │
         ▼
    带指标数据
         │
         ▼
┌─────────────────┐
│ strategy.signal │ ───▶ 生成买卖信号
└─────────────────┘
         │
         ▼
    信号序列
         │
         ▼
┌─────────────────┐
│ backtest.engine │ ───▶ 模拟交易
└─────────────────┘
         │
         ▼
    回测结果
         │
         ▼
┌─────────────────┐
│ reports.markdown│ ───▶ 输出报告
└─────────────────┘
```

---

## 8. 配置管理

**config.py**:
```python
# 数据配置
DATA_CACHE_DIR = "data/cache"
DEFAULT_START_DATE = "2020-01-01"

# 策略参数
MA_SHORT = 5
MA_LONG = 20
RSI_PERIOD = 14
RSI_OVERBOUGHT = 80
RSI_OVERSOLD = 70

# 回测配置
INITIAL_CAPITAL = 100000
COMMISSION_RATE = 0.0003  # 万分之三
```

---

## 9. 依赖列表

**requirements.txt**:
```
akshare>=1.10.0
pandas>=1.5.0
numpy>=1.20.0
```

---

## 10. 使用说明

### 10.1 安装依赖
```bash
pip install -r requirements.txt
```

### 10.2 运行回测
```bash
python main.py --symbol 000001 --start 2020-01-01 --end 2024-12-31
```

### 10.3 输出示例
```
reports/
├── 000001_20200101_20241231.md    # 回测报告
└── 000001_signals.csv             # 信号明细
```

---

## 11. 扩展方向

1. **多策略支持**: 添加其他策略（均值回归、动量突破等）
2. **多股票回测**: 支持股票池回测
3. **参数优化**: 网格搜索最优参数
4. **可视化**: 添加 matplotlib/pyecharts 图表
5. **Web 界面**: Streamlit 简易前端

---

## 12. 风险说明

- 本系统仅供学习和研究使用
- 历史回测结果不代表未来收益
- 不构成任何投资建议
- 实盘交易需考虑滑点、流动性等因素
