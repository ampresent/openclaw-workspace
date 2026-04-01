---
name: fund-query
description: 查询场外基金（公募基金）的实时估值、净值、基本信息及持仓日报。支持 QDII 基金自动 fallback。当用户询问基金净值、估值、涨跌、持仓盈亏时使用。
metadata:
  requires:
    bins: [python3]
---

# 场外基金查询

查询公募基金的实时估值、历史净值、基本信息及持仓日报。QDII 基金自动 fallback 到最新净值。

## 使用场景

- 查询单只基金估值/净值
- 批量查询持仓当日盈亏（`--portfolio`）
- QDII 基金查询（自动用最新净值替代实时估值）

## 使用方法

### 单只查询

```bash
python3 {{SKILL_DIR}}/scripts/fund_query_v2.py <基金代码> [estimate|nav]
```

- 不传 / `estimate`：实时估值优先，自动 fallback 到净值
- `nav`：仅查最新净值

### 持仓日报

```bash
python3 {{SKILL_DIR}}/scripts/fund_query_v2.py --portfolio <json文件>
```

JSON 格式：
```json
[
  {"code": "000217", "name": "华安黄金ETF联接C", "amount": 344359.66},
  {"code": "007722", "name": "天弘标普500(QDII-FOF)C", "amount": 102352.58}
]
```

持仓文件：`projects/investment/portfolio.json`

## 数据来源

- 📊 实时估值：fundgz.1234567.com.cn（国内基金交易时段 9:30-15:00）
- 📋 最新净值：fund.eastmoney.com pingzhongdata（所有基金可用，QDII fallback）

## 注意事项

1. 实时估值仅在交易时段更新
2. QDII 基金不支持实时估值，自动使用最近一个交易日的净值涨跌幅
3. 基金代码必须为 6 位数字
4. 旧版 `fund_query.py` 保留但不再推荐使用
