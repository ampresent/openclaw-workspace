# 飞猪机票价格监控 - Cron 定时任务配置

## 任务说明
每天中午 12 点和下午 6 点，自动查询上海到成都 5 月 15 日 18-23 点起飞的最便宜机票价格。

## Cron 表达式
```
0 12 * * * /usr/bin/python3 /home/admin/openclaw/workspace/scripts/flight_monitor_full.py
0 18 * * * /usr/bin/python3 /home/admin/openclaw/workspace/scripts/flight_monitor_full.py
```

## 安装方法

### 方法 1: 使用 crontab
```bash
# 编辑 crontab
crontab -e

# 添加以下两行
0 12 * * * cd /home/admin/openclaw/workspace && /usr/bin/python3 scripts/flight_monitor_full.py >> logs/flight-monitor.log 2>&1
0 18 * * * cd /home/admin/openclaw/workspace && /usr/bin/python3 scripts/flight_monitor_full.py >> logs/flight-monitor.log 2>&1
```

### 方法 2: 使用 OpenClaw cron 工具
通过 OpenClaw 的 cron 功能设置定时任务（推荐）

## 依赖安装
```bash
pip install playwright matplotlib
playwright install chromium
```

## 首次运行
```bash
# 1. 扫码登录（保存 cookies）
python scripts/flight_monitor_full.py --login

# 2. 测试查询
python scripts/flight_monitor_full.py

# 3. 查看生成的数据
cat data/flights/shanghai-chengdu-prices.csv

# 4. 查看生成的图表
open data/flights/price-trend.png  # macOS
xdg-open data/flights/price-trend.png  # Linux
```

## 输出文件
- `data/flights/shanghai-chengdu-prices.csv` - 价格记录
- `data/flights/price-trend.png` - 价格趋势折线图
- `temp/login-qrcode.png` - 登录二维码（首次登录时生成）
- `scripts/cookies.json` - 登录状态（自动保存）

## 手动查询
如果需要手动查询并记录：
```bash
python scripts/flight_monitor_full.py --manual
```
