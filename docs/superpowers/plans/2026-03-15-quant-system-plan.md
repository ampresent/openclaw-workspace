# A 股量化交易系统实现计划

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建一个入门级 A 股量化交易系统，包含数据获取、指标计算、信号生成、回测引擎和 Markdown 报告输出。

**Architecture:** 模块化设计，分为 data（数据）、indicators（指标）、strategy（策略）、backtest（回测）、reports（报告）五个模块，通过 main.py 统一入口。

**Tech Stack:** Python 3.8+, AkShare, pandas, numpy

---

## Chunk 1: 项目骨架与配置

### Task 1: 创建项目目录结构

**Files:**
- Create: `quant-system/` (根目录)
- Create: `quant-system/data/`
- Create: `quant-system/indicators/`
- Create: `quant-system/strategy/`
- Create: `quant-system/backtest/`
- Create: `quant-system/reports/`
- Create: `quant-system/data/cache/`

- [ ] **Step 1: 创建项目目录**

```bash
cd /home/admin/openclaw/workspace
mkdir -p quant-system/{data/cache,indicators,strategy,backtest,reports}
```

- [ ] **Step 2: 验证目录创建成功**

```bash
tree quant-system/
```

Expected: 显示完整的目录结构

- [ ] **Step 3: 创建各模块__init__.py**

```bash
touch quant-system/data/__init__.py
touch quant-system/indicators/__init__.py
touch quant-system/strategy/__init__.py
touch quant-system/backtest/__init__.py
touch quant-system/reports/__init__.py
```

- [ ] **Step 4: 提交**

```bash
git add quant-system/
git commit -m "feat: 创建项目目录结构"
```

---

### Task 2: 创建配置文件

**Files:**
- Create: `quant-system/config.py`

- [ ] **Step 1: 编写配置文件**

```python
# quant-system/config.py
"""
量化交易系统配置
"""
from pathlib import Path

# 项目根目录
PROJECT_ROOT = Path(__file__).parent

# 数据配置
DATA_CACHE_DIR = PROJECT_ROOT / "data" / "cache"
DEFAULT_START_DATE = "2020-01-01"
DEFAULT_END_DATE = "2024-12-31"

# 策略参数 - MA 均线
MA_SHORT = 5      # 短期均线
MA_LONG = 20      # 长期均线

# 策略参数 - MACD
MACD_FAST = 12
MACD_SLOW = 26
MACD_SIGNAL = 9

# 策略参数 - RSI
RSI_PERIOD = 14
RSI_OVERBOUGHT = 80    # 超买线
RSI_OVERSOLD = 70      # 超卖线（买入条件）

# 回测配置
INITIAL_CAPITAL = 100000  # 初始资金
COMMISSION_RATE = 0.0003  # 佣金率（万分之三）
SLIPPAGE = 0.001          # 滑点（0.1%）

# 报告配置
REPORTS_DIR = PROJECT_ROOT / "reports"
```

- [ ] **Step 2: 提交**

```bash
git add quant-system/config.py
git commit -m "feat: 添加配置文件"
```

---

### Task 3: 创建 requirements.txt

**Files:**
- Create: `quant-system/requirements.txt`

- [ ] **Step 1: 编写依赖列表**

```
akshare>=1.10.0
pandas>=1.5.0
numpy>=1.20.0
```

- [ ] **Step 2: 创建 README.md**

```markdown
# A 股入门级量化交易系统

一个基于技术指标组合策略的 A 股量化交易系统，支持回测和信号生成。

## 功能特点

- 📊 使用 AkShare 免费数据源
- 📈 MA+MACD+RSI 组合策略
- 🧪 自研轻量级回测引擎
- 📝 Markdown 格式报告输出

## 安装

```bash
pip install -r requirements.txt
```

## 使用

```bash
python main.py --symbol 000001 --start 2020-01-01 --end 2024-12-31
```

## 输出

- `reports/<symbol>_<start>_<end>.md` - 回测报告
- `reports/<symbol>_signals.csv` - 信号明细

## 策略说明

**买入信号**（同时满足）:
- 5 日线上穿 20 日线（金叉）
- MACD 柱状图由负转正
- RSI < 70（非超买区）

**卖出信号**（满足任一）:
- 5 日线下穿 20 日线（死叉）
- MACD 柱状图由正转负
- RSI > 80（严重超买）

## 风险说明

本系统仅供学习和研究使用，不构成投资建议。
```

- [ ] **Step 3: 提交**

```bash
git add quant-system/requirements.txt quant-system/README.md
git commit -m "docs: 添加依赖列表和使用说明"
```

---

## Chunk 2: 数据模块

### Task 4: 实现数据下载模块

**Files:**
- Create: `quant-system/data/downloader.py`
- Test: `quant-system/data/test_downloader.py`

- [ ] **Step 1: 编写数据下载测试**

```python
# quant-system/data/test_downloader.py
import pandas as pd
from downloader import fetch_stock_data, validate_data

def test_fetch_stock_data():
    """测试获取股票数据"""
    df = fetch_stock_data("000001", "2024-01-01", "2024-01-31")
    
    assert isinstance(df, pd.DataFrame)
    assert len(df) > 0
    assert all(col in df.columns for col in 
               ["date", "open", "high", "low", "close", "volume"])
    assert df["close"].notna().all()

def test_validate_data():
    """测试数据验证"""
    df = pd.DataFrame({
        "close": [10.0, 10.5, 11.0, None, 10.8]
    })
    assert validate_data(df) == False
    
    df_valid = pd.DataFrame({
        "close": [10.0, 10.5, 11.0, 10.8, 10.9]
    })
    assert validate_data(df_valid) == True
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cd quant-system
python -m pytest data/test_downloader.py -v
```
Expected: FAIL (模块不存在)

- [ ] **Step 3: 实现数据下载模块**

```python
# quant-system/data/downloader.py
"""
数据下载模块 - 从 AkShare 获取 A 股日线数据
"""
import akshare as ak
import pandas as pd
from pathlib import Path
from datetime import datetime


def fetch_stock_data(symbol: str, start_date: str, end_date: str) -> pd.DataFrame:
    """
    获取股票日线数据
    
    Args:
        symbol: 股票代码，如 "000001"
        start_date: 开始日期，格式 "YYYY-MM-DD"
        end_date: 结束日期，格式 "YYYY-MM-DD"
    
    Returns:
        DataFrame with columns: date, open, high, low, close, volume
    """
    try:
        # 使用 akshare 获取日线数据
        df = ak.stock_zh_a_hist(
            symbol=symbol,
            period="daily",
            start_date=start_date.replace("-", ""),
            end_date=end_date.replace("-", ""),
            adjust="qfq"  # 前复权
        )
        
        # 重命名列
        df = df.rename(columns={
            "日期": "date",
            "开盘": "open",
            "最高": "high",
            "最低": "low",
            "收盘": "close",
            "成交量": "volume"
        })
        
        # 选择需要的列
        df = df[["date", "open", "high", "low", "close", "volume"]]
        
        # 转换日期格式
        df["date"] = pd.to_datetime(df["date"])
        
        # 数据类型转换
        for col in ["open", "high", "low", "close", "volume"]:
            df[col] = df[col].astype(float)
        
        return df
        
    except Exception as e:
        raise Exception(f"获取数据失败：{str(e)}")


def validate_data(df: pd.DataFrame) -> bool:
    """
    验证数据质量
    
    Args:
        df: 数据 DataFrame
    
    Returns:
        bool: 数据是否有效
    """
    if df.empty:
        return False
    
    required_cols = ["date", "open", "high", "low", "close", "volume"]
    if not all(col in df.columns for col in required_cols):
        return False
    
    # 检查是否有空值
    if df["close"].isna().any():
        return False
    
    # 检查价格是否为正
    if (df["close"] <= 0).any():
        return False
    
    return True


def save_cache(symbol: str, data: pd.DataFrame, cache_dir: Path) -> None:
    """保存数据到缓存"""
    cache_dir.mkdir(parents=True, exist_ok=True)
    cache_file = cache_dir / f"{symbol}.csv"
    data.to_csv(cache_file, index=False)


def load_cache(symbol: str, cache_dir: Path) -> pd.DataFrame:
    """从缓存加载数据"""
    cache_file = cache_dir / f"{symbol}.csv"
    if cache_file.exists():
        df = pd.read_csv(cache_file)
        df["date"] = pd.to_datetime(df["date"])
        return df
    return None
```

- [ ] **Step 4: 运行测试确认通过**

```bash
cd quant-system
python -m pytest data/test_downloader.py -v
```
Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add data/downloader.py data/test_downloader.py
git commit -m "feat: 实现数据下载模块"
```

---

## Chunk 3: 技术指标模块

### Task 5: 实现 MA 均线指标

**Files:**
- Create: `quant-system/indicators/ma.py`
- Test: `quant-system/indicators/test_ma.py`

- [ ] **Step 1: 编写 MA 测试**

```python
# quant-system/indicators/test_ma.py
import pandas as pd
import numpy as np
from ma import calculate_ma

def test_calculate_ma_basic():
    """测试基本 MA 计算"""
    close = pd.Series([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
    ma5 = calculate_ma(close, 5)
    
    assert len(ma5) == 10
    # 前 4 个应该是 NaN（不足 5 个数据）
    assert pd.isna(ma5[:4]).all()
    # 第 5 个是 (1+2+3+4+5)/5 = 3.0
    assert ma5.iloc[4] == 3.0
    # 最后一个 (6+7+8+9+10)/5 = 8.0
    assert ma5.iloc[9] == 8.0

def test_calculate_ma_known_values():
    """测试已知值"""
    close = pd.Series([10.0, 10.5, 11.0, 10.8, 10.9])
    ma5 = calculate_ma(close, 5)
    
    expected = (10.0 + 10.5 + 11.0 + 10.8 + 10.9) / 5
    assert abs(ma5.iloc[4] - expected) < 0.001
```

- [ ] **Step 2: 实现 MA 指标**

```python
# quant-system/indicators/ma.py
"""
MA 均线指标计算
"""
import pandas as pd


def calculate_ma(close: pd.Series, period: int) -> pd.Series:
    """
    计算移动平均线
    
    Args:
        close: 收盘价序列
        period: 均线周期
    
    Returns:
        MA 值序列
    """
    return close.rolling(window=period).mean()


def calculate_ma_cross(ma_short: pd.Series, ma_long: pd.Series) -> pd.Series:
    """
    检测均线交叉
    
    Returns:
        1: 金叉（短线上穿长线）
        -1: 死叉（短线下穿长线）
        0: 无交叉
    """
    cross = pd.Series(0, index=ma_short.index)
    
    # 金叉：短期均线从下向上穿越长期均线
    golden_cross = (ma_short > ma_long) & (ma_short.shift(1) <= ma_long.shift(1))
    cross[golden_cross] = 1
    
    # 死叉：短期均线从上向下穿越长期均线
    death_cross = (ma_short < ma_long) & (ma_short.shift(1) >= ma_long.shift(1))
    cross[death_cross] = -1
    
    return cross
```

- [ ] **Step 3: 运行测试**

```bash
cd quant-system
python -m pytest indicators/test_ma.py -v
```
Expected: PASS

- [ ] **Step 4: 提交**

```bash
git add indicators/ma.py indicators/test_ma.py
git commit -m "feat: 实现 MA 均线指标"
```

---

### Task 6: 实现 MACD 指标

**Files:**
- Create: `quant-system/indicators/macd.py`
- Test: `quant-system/indicators/test_macd.py`

- [ ] **Step 1: 编写 MACD 测试**

```python
# quant-system/indicators/test_macd.py
import pandas as pd
from macd import calculate_macd

def test_calculate_macd_structure():
    """测试 MACD 输出结构"""
    close = pd.Series(range(1, 101))  # 100 个数据点
    dif, dea, macd_hist = calculate_macd(close)
    
    assert len(dif) == 100
    assert len(dea) == 100
    assert len(macd_hist) == 100
    assert all(macd_hist == dif - dea)

def test_calculate_macd_sign_change():
    """测试 MACD 柱状图正负变化"""
    # 创建一个先上涨后下跌的序列
    close = pd.Series([10] * 30 + [15] * 30 + [10] * 40)
    dif, dea, macd_hist = calculate_macd(close)
    
    # MACD 柱状图应该有正有负
    assert (macd_hist > 0).any()
    assert (macd_hist < 0).any()
```

- [ ] **Step 2: 实现 MACD 指标**

```python
# quant-system/indicators/macd.py
"""
MACD 指标计算
"""
import pandas as pd


def calculate_ema(series: pd.Series, span: int) -> pd.Series:
    """计算指数移动平均"""
    return series.ewm(span=span, adjust=False).mean()


def calculate_macd(close: pd.Series, 
                   fast: int = 12, 
                   slow: int = 26, 
                   signal: int = 9) -> tuple:
    """
    计算 MACD 指标
    
    Args:
        close: 收盘价序列
        fast: 快速 EMA 周期
        slow: 慢速 EMA 周期
        signal: 信号线周期
    
    Returns:
        (DIF, DEA, MACD 柱状图)
    """
    # 计算快速和慢速 EMA
    ema_fast = calculate_ema(close, fast)
    ema_slow = calculate_ema(close, slow)
    
    # DIF = 快速 EMA - 慢速 EMA
    dif = ema_fast - ema_slow
    
    # DEA = DIF 的 EMA
    dea = calculate_ema(dif, signal)
    
    # MACD 柱状图 = DIF - DEA
    macd_hist = dif - dea
    
    return dif, dea, macd_hist


def detect_macd_turn(macd_hist: pd.Series) -> pd.Series:
    """
    检测 MACD 柱状图转向
    
    Returns:
        1: 由负转正
        -1: 由正转负
        0: 无转向
    """
    turn = pd.Series(0, index=macd_hist.index)
    
    # 由负转正
    turn_positive = (macd_hist > 0) & (macd_hist.shift(1) <= 0)
    turn[turn_positive] = 1
    
    # 由正转负
    turn_negative = (macd_hist < 0) & (macd_hist.shift(1) >= 0)
    turn[turn_negative] = -1
    
    return turn
```

- [ ] **Step 3: 运行测试**

```bash
cd quant-system
python -m pytest indicators/test_macd.py -v
```
Expected: PASS

- [ ] **Step 4: 提交**

```bash
git add indicators/macd.py indicators/test_macd.py
git commit -m "feat: 实现 MACD 指标"
```

---

### Task 7: 实现 RSI 指标

**Files:**
- Create: `quant-system/indicators/rsi.py`
- Test: `quant-system/indicators/test_rsi.py`

- [ ] **Step 1: 编写 RSI 测试**

```python
# quant-system/indicators/test_rsi.py
import pandas as pd
from rsi import calculate_rsi

def test_calculate_rsi_range():
    """测试 RSI 值范围"""
    close = pd.Series(range(1, 101))
    rsi = calculate_rsi(close, 14)
    
    # RSI 应该在 0-100 之间
    assert (rsi >= 0).all()
    assert (rsi <= 100).all()

def test_calculate_rsi_known_values():
    """测试已知 RSI 值"""
    # 连续上涨，RSI 应该较高
    close_up = pd.Series([10 + i * 0.5 for i in range(30)])
    rsi_up = calculate_rsi(close_up, 14)
    
    # 最后几个 RSI 应该大于 50（上涨趋势）
    assert rsi_up.iloc[-1] > 50
    
    # 连续下跌，RSI 应该较低
    close_down = pd.Series([20 - i * 0.5 for i in range(30)])
    rsi_down = calculate_rsi(close_down, 14)
    
    # 最后几个 RSI 应该小于 50（下跌趋势）
    assert rsi_down.iloc[-1] < 50
```

- [ ] **Step 2: 实现 RSI 指标**

```python
# quant-system/indicators/rsi.py
"""
RSI 相对强弱指标计算
"""
import pandas as pd


def calculate_rsi(close: pd.Series, period: int = 14) -> pd.Series:
    """
    计算 RSI 指标
    
    Args:
        close: 收盘价序列
        period: RSI 周期
    
    Returns:
        RSI 值序列 (0-100)
    """
    # 计算价格变化
    delta = close.diff()
    
    # 分离上涨和下跌
    gain = delta.where(delta > 0, 0)
    loss = (-delta).where(delta < 0, 0)
    
    # 计算平均涨幅和跌幅
    avg_gain = gain.rolling(window=period).mean()
    avg_loss = loss.rolling(window=period).mean()
    
    # 计算 RS 和 RSI
    rs = avg_gain / avg_loss
    rsi = 100 - (100 / (1 + rs))
    
    return rsi


def check_rsi_condition(rsi: pd.Series, 
                        threshold: float, 
                        above: bool = True) -> pd.Series:
    """
    检查 RSI 条件
    
    Args:
        rsi: RSI 序列
        threshold: 阈值
        above: True=检查是否大于阈值，False=检查是否小于阈值
    
    Returns:
        布尔序列
    """
    if above:
        return rsi > threshold
    else:
        return rsi < threshold
```

- [ ] **Step 3: 运行测试**

```bash
cd quant-system
python -m pytest indicators/test_rsi.py -v
```
Expected: PASS

- [ ] **Step 4: 提交**

```bash
git add indicators/rsi.py indicators/test_rsi.py
git commit -m "feat: 实现 RSI 指标"
```

---

### Task 8: 更新 indicators 模块导出

**Files:**
- Modify: `quant-system/indicators/__init__.py`

- [ ] **Step 1: 更新__init__.py**

```python
# quant-system/indicators/__init__.py
"""
技术指标模块
"""
from .ma import calculate_ma, calculate_ma_cross
from .macd import calculate_macd, detect_macd_turn
from .rsi import calculate_rsi, check_rsi_condition

__all__ = [
    "calculate_ma",
    "calculate_ma_cross",
    "calculate_macd",
    "detect_macd_turn",
    "calculate_rsi",
    "check_rsi_condition",
]
```

- [ ] **Step 2: 提交**

```bash
git add indicators/__init__.py
git commit -m "feat: 更新指标模块导出"
```

---

## Chunk 4: 策略模块

### Task 9: 实现信号生成策略

**Files:**
- Create: `quant-system/strategy/signal.py`
- Test: `quant-system/strategy/test_signal.py`

- [ ] **Step 1: 编写信号生成测试**

```python
# quant-system/strategy/test_signal.py
import pandas as pd
from signal import generate_signals

def test_generate_signals_structure():
    """测试信号生成输出结构"""
    df = pd.DataFrame({
        "date": pd.date_range("2024-01-01", periods=100),
        "close": range(100, 200),
        "ma_short": range(98, 198),
        "ma_long": range(95, 195),
        "macd_hist": [i - 50 for i in range(100)],
        "rsi": [50 + i % 30 for i in range(100)]
    })
    
    result = generate_signals(df)
    
    assert "signal" in result.columns
    assert len(result) == 100
    # 信号应该是 -1, 0, 1
    assert result["signal"].isin([-1, 0, 1]).all()

def test_generate_signals_buy():
    """测试买入信号生成"""
    # 构造满足买入条件的数据
    df = pd.DataFrame({
        "date": pd.date_range("2024-01-01", periods=50),
        "close": range(100, 150),
        "ma_cross": [0] * 25 + [1] + [0] * 24,  # 金叉
        "macd_turn": [0] * 25 + [1] + [0] * 24,  # MACD 转正
        "rsi": [50] * 50  # RSI 适中
    })
    
    result = generate_signals(df)
    # 金叉位置应该有买入信号
    assert result.loc[25, "signal"] == 1
```

- [ ] **Step 2: 实现信号生成策略**

```python
# quant-system/strategy/signal.py
"""
交易信号生成策略
"""
import pandas as pd
import numpy as np

from indicators import (
    calculate_ma,
    calculate_ma_cross,
    calculate_macd,
    detect_macd_turn,
    calculate_rsi,
)
from config import (
    MA_SHORT,
    MA_LONG,
    RSI_PERIOD,
    RSI_OVERBOUGHT,
    RSI_OVERSOLD,
)


def compute_indicators(df: pd.DataFrame) -> pd.DataFrame:
    """
    计算所有技术指标
    
    Args:
        df: 包含 close 列的 DataFrame
    
    Returns:
        添加了指标列的 DataFrame
    """
    df = df.copy()
    
    # MA 均线
    df["ma_short"] = calculate_ma(df["close"], MA_SHORT)
    df["ma_long"] = calculate_ma(df["close"], MA_LONG)
    df["ma_cross"] = calculate_ma_cross(df["ma_short"], df["ma_long"])
    
    # MACD
    dif, dea, macd_hist = calculate_macd(df["close"])
    df["macd_hist"] = macd_hist
    df["macd_turn"] = detect_macd_turn(macd_hist)
    
    # RSI
    df["rsi"] = calculate_rsi(df["close"], RSI_PERIOD)
    
    return df


def generate_signals(df: pd.DataFrame) -> pd.DataFrame:
    """
    生成交易信号
    
    买入信号（同时满足）:
    - 5 日线上穿 20 日线（金叉）
    - MACD 柱状图由负转正
    - RSI < 70（非超买区）
    
    卖出信号（满足任一）:
    - 5 日线下穿 20 日线（死叉）
    - MACD 柱状图由正转负
    - RSI > 80（严重超买）
    
    Args:
        df: 包含价格数据的 DataFrame
    
    Returns:
        添加了 signal 列的 DataFrame
    """
    df = compute_indicators(df)
    
    # 初始化信号为 0（持有）
    df["signal"] = 0
    
    # 买入条件
    buy_condition = (
        (df["ma_cross"] == 1) &      # 金叉
        (df["macd_turn"] == 1) &     # MACD 转正
        (df["rsi"] < RSI_OVERSOLD)   # 非超买
    )
    
    # 卖出条件
    sell_condition = (
        (df["ma_cross"] == -1) |     # 死叉
        (df["macd_turn"] == -1) |    # MACD 转负
        (df["rsi"] > RSI_OVERBOUGHT) # 严重超买
    )
    
    # 设置信号
    df.loc[buy_condition, "signal"] = 1
    df.loc[sell_condition & (df["signal"] == 0), "signal"] = -1
    
    return df
```

- [ ] **Step 3: 运行测试**

```bash
cd quant-system
python -m pytest strategy/test_signal.py -v
```
Expected: PASS

- [ ] **Step 4: 提交**

```bash
git add strategy/signal.py strategy/test_signal.py
git commit -m "feat: 实现信号生成策略"
```

---

## Chunk 5: 回测模块

### Task 10: 实现回测引擎

**Files:**
- Create: `quant-system/backtest/engine.py`
- Test: `quant-system/backtest/test_engine.py`

- [ ] **Step 1: 编写回测测试**

```python
# quant-system/backtest/test_engine.py
import pandas as pd
from engine import BacktestEngine, BacktestResult

def test_backtest_basic():
    """测试基本回测功能"""
    # 构造测试数据
    dates = pd.date_range("2024-01-01", periods=100)
    df = pd.DataFrame({
        "date": dates,
        "close": 100 + (range(100)),
        "signal": [0] * 30 + [1] + [0] * 40 + [-1] + [0] * 27
    })
    
    engine = BacktestEngine(initial_capital=100000)
    result = engine.run(df)
    
    assert isinstance(result, BacktestResult)
    assert result.initial_capital == 100000
    assert result.final_capital > 0

def test_backtest_metrics():
    """测试回测指标计算"""
    engine = BacktestEngine(initial_capital=100000)
    
    # 构造一个简单盈利场景
    df = pd.DataFrame({
        "date": pd.date_range("2024-01-01", periods=50),
        "close": [100 + i for i in range(50)],
        "signal": [1] + [0] * 48 + [-1]
    })
    
    result = engine.run(df)
    
    assert result.total_return > 0
    assert result.trade_count >= 1
```

- [ ] **Step 2: 实现回测引擎**

```python
# quant-system/backtest/engine.py
"""
回测引擎
"""
import pandas as pd
import numpy as np
from dataclasses import dataclass
from typing import List, Dict

from config import INITIAL_CAPITAL, COMMISSION_RATE, SLIPPAGE


@dataclass
class BacktestResult:
    """回测结果"""
    initial_capital: float
    final_capital: float
    total_return: float  # 总收益率
    annual_return: float  # 年化收益率
    max_drawdown: float  # 最大回撤
    sharpe_ratio: float  # 夏普比率
    win_rate: float  # 胜率
    profit_loss_ratio: float  # 盈亏比
    trade_count: int  # 交易次数
    trades: List[Dict]  # 交易明细
    equity_curve: pd.Series  # 资金曲线


class BacktestEngine:
    """回测引擎"""
    
    def __init__(self, initial_capital: float = INITIAL_CAPITAL):
        self.initial_capital = initial_capital
        self.commission_rate = COMMISSION_RATE
        self.slippage = SLIPPAGE
    
    def run(self, df: pd.DataFrame) -> BacktestResult:
        """
        运行回测
        
        Args:
            df: 包含 close 和 signal 列的 DataFrame
        
        Returns:
            BacktestResult
        """
        df = df.copy()
        df = df.dropna(subset=["close", "signal"])
        
        if len(df) == 0:
            return self._empty_result()
        
        # 初始化
        capital = self.initial_capital
        position = 0  # 持仓数量
        trades = []
        entry_price = 0
        entry_date = None
        
        equity_curve = []
        
        # 遍历每一天
        for idx, row in df.iterrows():
            date = row["date"]
            price = row["close"]
            signal = row["signal"]
            
            # 买入信号
            if signal == 1 and position == 0:
                # 考虑滑点
                buy_price = price * (1 + self.slippage)
                # 计算可买数量
                shares = int(capital / (buy_price * (1 + self.commission_rate)))
                
                if shares > 0:
                    cost = shares * buy_price * (1 + self.commission_rate)
                    capital -= cost
                    position = shares
                    entry_price = buy_price
                    entry_date = date
            
            # 卖出信号
            elif signal == -1 and position > 0:
                # 考虑滑点
                sell_price = price * (1 - self.slippage)
                # 计算卖出金额
                proceeds = position * sell_price * (1 - self.commission_rate)
                
                # 记录交易
                profit = (sell_price - entry_price) * position
                profit_rate = profit / (entry_price * position)
                
                trades.append({
                    "entry_date": entry_date,
                    "exit_date": date,
                    "entry_price": entry_price,
                    "exit_price": sell_price,
                    "shares": position,
                    "profit": profit,
                    "profit_rate": profit_rate
                })
                
                capital += proceeds
                position = 0
            
            # 记录资金曲线
            total_value = capital + position * price
            equity_curve.append(total_value)
        
        # 如果还有持仓，按最后价格计算
        if position > 0:
            final_price = df.iloc[-1]["close"]
            capital += position * final_price
            position = 0
        
        # 计算结果
        return self._calculate_metrics(
            trades=trades,
            equity_curve=pd.Series(equity_curve, index=df.index),
            dates=df["date"]
        )
    
    def _calculate_metrics(self, trades: List[Dict], 
                          equity_curve: pd.Series,
                          dates: pd.Series) -> BacktestResult:
        """计算回测指标"""
        initial = self.initial_capital
        final = equity_curve.iloc[-1] if len(equity_curve) > 0 else initial
        
        # 总收益率
        total_return = (final - initial) / initial
        
        # 年化收益率
        if len(dates) > 1:
            days = (dates.iloc[-1] - dates.iloc[0]).days
            years = max(days / 365, 0.1)
            annual_return = (1 + total_return) ** (1 / years) - 1
        else:
            annual_return = 0
        
        # 最大回撤
        peak = equity_curve.expanding().max()
        drawdown = (equity_curve - peak) / peak
        max_drawdown = abs(drawdown.min()) if len(drawdown) > 0 else 0
        
        # 夏普比率（简化版，假设无风险利率为 0）
        if len(equity_curve) > 1:
            returns = equity_curve.pct_change().dropna()
            if returns.std() > 0:
                sharpe_ratio = returns.mean() / returns.std() * np.sqrt(252)
            else:
                sharpe_ratio = 0
        else:
            sharpe_ratio = 0
        
        # 胜率和盈亏比
        if trades:
            winning_trades = [t for t in trades if t["profit"] > 0]
            losing_trades = [t for t in trades if t["profit"] <= 0]
            
            win_rate = len(winning_trades) / len(trades)
            
            if losing_trades:
                avg_win = sum(t["profit"] for t in winning_trades) / len(winning_trades) if winning_trades else 0
                avg_loss = abs(sum(t["profit"] for t in losing_trades) / len(losing_trades))
                profit_loss_ratio = avg_win / avg_loss if avg_loss > 0 else float('inf')
            else:
                win_rate = 1.0
                profit_loss_ratio = float('inf')
        else:
            win_rate = 0
            profit_loss_ratio = 0
        
        return BacktestResult(
            initial_capital=initial,
            final_capital=final,
            total_return=total_return,
            annual_return=annual_return,
            max_drawdown=max_drawdown,
            sharpe_ratio=sharpe_ratio,
            win_rate=win_rate,
            profit_loss_ratio=profit_loss_ratio,
            trade_count=len(trades),
            trades=trades,
            equity_curve=equity_curve
        )
    
    def _empty_result(self) -> BacktestResult:
        """返回空结果"""
        return BacktestResult(
            initial_capital=self.initial_capital,
            final_capital=self.initial_capital,
            total_return=0,
            annual_return=0,
            max_drawdown=0,
            sharpe_ratio=0,
            win_rate=0,
            profit_loss_ratio=0,
            trade_count=0,
            trades=[],
            equity_curve=pd.Series([self.initial_capital])
        )
```

- [ ] **Step 3: 运行测试**

```bash
cd quant-system
python -m pytest backtest/test_engine.py -v
```
Expected: PASS

- [ ] **Step 4: 提交**

```bash
git add backtest/engine.py backtest/test_engine.py
git commit -m "feat: 实现回测引擎"
```

---

### Task 11: 实现回测指标模块

**Files:**
- Create: `quant-system/backtest/metrics.py`

- [ ] **Step 1: 实现指标计算辅助函数**

```python
# quant-system/backtest/metrics.py
"""
回测指标计算辅助函数
"""
import pandas as pd
import numpy as np


def format_percentage(value: float, decimals: int = 2) -> str:
    """格式化为百分比字符串"""
    return f"{value * 100:.{decimals}f}%"


def format_number(value: float, decimals: int = 2) -> str:
    """格式化为数字字符串"""
    return f"{value:.{decimals}f}"


def calculate_monthly_returns(equity_curve: pd.Series) -> pd.Series:
    """计算月度收益率"""
    # 重采样到月末
    monthly = equity_curve.resample("M").last()
    return monthly.pct_change()


def calculate_max_consecutive_losses(trades: list) -> int:
    """计算最大连续亏损次数"""
    max_consecutive = 0
    current_consecutive = 0
    
    for trade in trades:
        if trade["profit"] <= 0:
            current_consecutive += 1
            max_consecutive = max(max_consecutive, current_consecutive)
        else:
            current_consecutive = 0
    
    return max_consecutive
```

- [ ] **Step 2: 更新 backtest 模块导出**

```python
# quant-system/backtest/__init__.py
from .engine import BacktestEngine, BacktestResult
from .metrics import format_percentage, format_number

__all__ = [
    "BacktestEngine",
    "BacktestResult",
    "format_percentage",
    "format_number",
]
```

- [ ] **Step 3: 提交**

```bash
git add backtest/metrics.py backtest/__init__.py
git commit -m "feat: 实现回测指标辅助函数"
```

---

## Chunk 6: 报告模块

### Task 12: 实现 Markdown 报告生成

**Files:**
- Create: `quant-system/reports/markdown.py`
- Test: `quant-system/reports/test_markdown.py`

- [ ] **Step 1: 编写报告生成测试**

```python
# quant-system/reports/test_markdown.py
from markdown import generate_report
from backtest import BacktestResult
import pandas as pd

def test_generate_report_structure():
    """测试报告生成结构"""
    result = BacktestResult(
        initial_capital=100000,
        final_capital=120000,
        total_return=0.2,
        annual_return=0.15,
        max_drawdown=0.1,
        sharpe_ratio=1.5,
        win_rate=0.6,
        profit_loss_ratio=2.0,
        trade_count=10,
        trades=[],
        equity_curve=pd.Series([100000, 110000, 120000])
    )
    
    report = generate_report(result, "000001")
    
    assert isinstance(report, str)
    assert "000001" in report
    assert "总收益率" in report or "Total Return" in report
    assert "20.00%" in report
```

- [ ] **Step 2: 实现 Markdown 报告生成**

```python
# quant-system/reports/markdown.py
"""
Markdown 报告生成
"""
from datetime import datetime
from backtest import BacktestResult, format_percentage, format_number


def generate_report(result: BacktestResult, 
                    symbol: str,
                    start_date: str = None,
                    end_date: str = None) -> str:
    """
    生成 Markdown 格式回测报告
    
    Args:
        result: 回测结果
        symbol: 股票代码
        start_date: 开始日期
        end_date: 结束日期
    
    Returns:
        Markdown 字符串
    """
    now = datetime.now().strftime("%Y-%m-%d %H:%M")
    
    report = f"""# A 股量化交易回测报告

**股票代码**: {symbol}  
**生成时间**: {now}  
**回测区间**: {start_date or "N/A"} 至 {end_date or "N/A"}

---

## 📊 核心指标

| 指标 | 数值 |
|------|------|
| 初始资金 | ¥{result.initial_capital:,.2f} |
| 最终资金 | ¥{result.final_capital:,.2f} |
| 总收益率 | {format_percentage(result.total_return)} |
| 年化收益率 | {format_percentage(result.annual_return)} |
| 最大回撤 | {format_percentage(result.max_drawdown)} |
| 夏普比率 | {format_number(result.sharpe_ratio)} |
| 交易次数 | {result.trade_count} |

---

## 📈 交易表现

| 指标 | 数值 |
|------|------|
| 胜率 | {format_percentage(result.win_rate)} |
| 盈亏比 | {format_number(result.profit_loss_ratio)} |

---

## 💰 交易明细

"""
    
    if result.trades:
        report += "| 序号 | 入场日期 | 出场日期 | 入场价 | 出场价 | 股数 | 盈亏 | 收益率 |\n"
        report += "|------|----------|----------|--------|--------|------|------|--------|\n"
        
        for i, trade in enumerate(result.trades, 1):
            report += f"| {i} | {trade['entry_date'].strftime('%Y-%m-%d') if hasattr(trade['entry_date'], 'strftime') else trade['entry_date']} | {trade['exit_date'].strftime('%Y-%m-%d') if hasattr(trade['exit_date'], 'strftime') else trade['exit_date']} | ¥{trade['entry_price']:.2f} | ¥{trade['exit_price']:.2f} | {trade['shares']} | ¥{trade['profit']:.2f} | {format_percentage(trade['profit_rate'])} |\n"
    else:
        report += "*无交易记录*\n"
    
    report += f"""
---

## ⚠️ 风险说明

1. 历史回测结果不代表未来收益
2. 回测未考虑极端市场情况
3. 实盘交易可能存在滑点和流动性问题
4. 本系统仅供学习研究，不构成投资建议

---

*报告由 A 股量化交易系统自动生成*
"""
    
    return report


def save_report(report: str, filepath: str) -> None:
    """保存报告到文件"""
    with open(filepath, "w", encoding="utf-8") as f:
        f.write(report)
```

- [ ] **Step 3: 更新 reports 模块导出**

```python
# quant-system/reports/__init__.py
from .markdown import generate_report, save_report

__all__ = [
    "generate_report",
    "save_report",
]
```

- [ ] **Step 4: 提交**

```bash
git add reports/markdown.py reports/test_markdown.py reports/__init__.py
git commit -m "feat: 实现 Markdown 报告生成"
```

---

## Chunk 7: 主入口与集成

### Task 13: 实现主程序入口

**Files:**
- Create: `quant-system/main.py`

- [ ] **Step 1: 实现主程序**

```python
#!/usr/bin/env python3
"""
A 股量化交易系统 - 主入口

使用方法:
    python main.py --symbol 000001 --start 2020-01-01 --end 2024-12-31
"""
import argparse
from pathlib import Path
from datetime import datetime

from data.downloader import fetch_stock_data, validate_data
from strategy.signal import generate_signals
from backtest.engine import BacktestEngine
from reports.markdown import generate_report, save_report
from config import REPORTS_DIR, DEFAULT_START_DATE, DEFAULT_END_DATE


def main():
    parser = argparse.ArgumentParser(description="A 股量化交易系统")
    parser.add_argument("--symbol", type=str, required=True, help="股票代码")
    parser.add_argument("--start", type=str, default=DEFAULT_START_DATE, help="开始日期")
    parser.add_argument("--end", type=str, default=DEFAULT_END_DATE, help="结束日期")
    parser.add_argument("--capital", type=float, default=100000, help="初始资金")
    parser.add_argument("--output", type=str, default=None, help="输出目录")
    
    args = parser.parse_args()
    
    output_dir = Path(args.output) if args.output else REPORTS_DIR
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print(f"🚀 开始回测 {args.symbol}")
    print(f"📅 区间：{args.start} 至 {args.end}")
    print(f"💰 初始资金：¥{args.capital:,.2f}")
    print("-" * 50)
    
    # 1. 获取数据
    print("📊 获取数据...")
    df = fetch_stock_data(args.symbol, args.start, args.end)
    
    if not validate_data(df):
        print("❌ 数据验证失败")
        return
    
    print(f"✅ 获取到 {len(df)} 条数据")
    
    # 2. 生成信号
    print("📈 生成信号...")
    df = generate_signals(df)
    
    buy_count = (df["signal"] == 1).sum()
    sell_count = (df["signal"] == -1).sum()
    print(f"✅ 买入信号：{buy_count}, 卖出信号：{sell_count}")
    
    # 3. 运行回测
    print("🧪 运行回测...")
    engine = BacktestEngine(initial_capital=args.capital)
    result = engine.run(df)
    
    # 4. 输出结果
    print("-" * 50)
    print("📊 回测结果")
    print(f"  总收益率：{result.total_return * 100:.2f}%")
    print(f"  年化收益率：{result.annual_return * 100:.2f}%")
    print(f"  最大回撤：{result.max_drawdown * 100:.2f}%")
    print(f"  夏普比率：{result.sharpe_ratio:.2f}")
    print(f"  胜率：{result.win_rate * 100:.2f}%")
    print(f"  交易次数：{result.trade_count}")
    print("-" * 50)
    
    # 5. 生成报告
    print("📝 生成报告...")
    report = generate_report(
        result, 
        args.symbol, 
        args.start, 
        args.end
    )
    
    # 保存报告
    date_str = datetime.now().strftime("%Y%m%d_%H%M%S")
    report_file = output_dir / f"{args.symbol}_{args.start}_{args.end}.md"
    save_report(report, report_file)
    print(f"✅ 报告已保存：{report_file}")
    
    # 保存信号明细
    signals_file = output_dir / f"{args.symbol}_signals.csv"
    df.to_csv(signals_file, index=False)
    print(f"✅ 信号已保存：{signals_file}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: 提交**

```bash
git add main.py
git commit -m "feat: 实现主程序入口"
```

---

### Task 14: 完整系统测试

**Files:**
- 无新建文件

- [ ] **Step 1: 安装依赖**

```bash
cd quant-system
pip install -r requirements.txt
```

- [ ] **Step 2: 运行完整回测测试**

```bash
cd quant-system
python main.py --symbol 000001 --start 2023-01-01 --end 2023-12-31
```

Expected: 成功运行并生成报告

- [ ] **Step 3: 检查输出文件**

```bash
ls -la reports/
cat reports/000001_2023-01-01_2023-12-31.md
```

- [ ] **Step 4: 运行所有单元测试**

```bash
cd quant-system
python -m pytest . -v --tb=short
```

Expected: 所有测试通过

- [ ] **Step 5: 提交**

```bash
git add .
git commit -m "test: 完整系统测试通过"
```

---

## 完成检查清单

- [ ] 项目目录结构创建完成
- [ ] 配置文件编写完成
- [ ] 数据下载模块测试通过
- [ ] MA/MACD/RSI指标模块测试通过
- [ ] 信号生成策略测试通过
- [ ] 回测引擎测试通过
- [ ] Markdown 报告生成测试通过
- [ ] 主程序可以正常运行
- [ ] 所有单元测试通过
- [ ] 生成示例回测报告

---

## 后续扩展建议

1. **可视化**: 添加 matplotlib/pyecharts 绘制资金曲线
2. **多股票支持**: 支持股票池批量回测
3. **参数优化**: 添加网格搜索功能
4. **更多策略**: 实现其他常见策略
5. **Web 界面**: 使用 Streamlit 创建简易前端
