# 豆瓣音乐专辑导出 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 从豆瓣音乐页面导出用户标记"听过"的 3515 张专辑到 CSV 文件，包含完整专辑信息，具备反反爬虫策略。

**Architecture:** 使用 agent-browser 浏览器自动化工具，通过 snapshot 获取页面元素引用，分页遍历所有专辑，提取信息后写入 CSV。实现请求频率控制、失败检测和自动重试机制。

**Tech Stack:** agent-browser CLI, bash scripting, CSV 格式

**反爬虫策略:**
- 基础延迟：每页之间等待 3-5 秒
- 随机抖动：±1 秒随机变化
- 失败检测：检测豆瓣反爬虫页面特征
- 指数退避：失败后延迟翻倍，最大 60 秒
- 最大重试：5 次失败后通知用户

**数据字段:**
- 专辑名称
- 艺人/乐队
- 评分（用户评分）
- 专辑评分（豆瓣平均分）
- 发行年份
- 厂牌
- 听过的时间
- 评论（如有）
- 专辑 URL

---

## Task 1: 环境验证和页面访问

**Files:**
- Create: `scripts/douban-export/01-verify-access.sh`
- Output: `temp/douban-verify-screenshot.png`

- [ ] **Step 1: 创建验证脚本**

```bash
#!/bin/bash
# 验证豆瓣音乐页面访问
cd /home/admin/openclaw/workspace

# 打开豆瓣音乐页面
agent-browser open "https://music.douban.com/mine?status=collect"
sleep 3

# 截图验证
agent-browser screenshot --full
mv screenshot*.png temp/douban-verify-screenshot.png

# 获取页面标题
agent-browser get title
```

- [ ] **Step 2: 执行验证脚本**

Run: `bash scripts/douban-export/01-verify-access.sh`
Expected: 成功打开页面，标题包含"豆瓣音乐"

- [ ] **Step 3: 检查是否登录**

```bash
# 检查页面是否显示登录提示
agent-browser snapshot -i
# 检查是否有"登录"按钮元素
```

- [ ] **Step 4: 提交验证结果**
确认可以访问已登录的豆瓣音乐页面

## Task 2: 页面结构分析和元素定位

**Files:**
- Create: `scripts/douban-export/02-analyze-page.sh`
- Output: `temp/douban-page-structure.txt`

- [ ] **Step 1: 获取页面快照**

```bash
#!/bin/bash
cd /home/admin/openclaw/workspace

agent-browser open "https://music.douban.com/mine?status=collect"
sleep 3

# 获取交互式元素快照
agent-browser snapshot -i > temp/douban-page-structure.txt
cat temp/douban-page-structure.txt
```

- [ ] **Step 2: 分析专辑列表结构**

识别：
- 专辑条目容器元素
- 专辑名称元素
- 艺人元素
- 评分元素
- 分页导航元素

- [ ] **Step 3: 记录元素选择器**

在 `temp/douban-selectors.md` 中记录：
```markdown
# 豆瓣音乐页面元素选择器

- 专辑条目：@eXX (待确认)
- 专辑名称：@eXX (待确认)
- 艺人：@eXX (待确认)
- 用户评分：@eXX (待确认)
- 下一页：@eXX (待确认)
```

- [ ] **Step 4: 提交分析结果**

## Task 3: 单页数据提取脚本

**Files:**
- Create: `scripts/douban-export/03-extract-page.sh`
- Create: `temp/douban-albums-page1.csv`

- [ ] **Step 1: 创建提取脚本框架**

```bash
#!/bin/bash
# 提取单页专辑数据

CSV_FILE="temp/douban-albums-page1.csv"

# 写入 CSV 表头
echo "专辑名称，艺人，用户评分，专辑评分，发行年份，厂牌，听过时间，评论，URL" > "$CSV_FILE"

# 打开页面
agent-browser open "https://music.douban.com/mine?status=collect"
sleep 3

# 获取快照
agent-browser snapshot -i
```

- [ ] **Step 2: 实现单条目提取逻辑**

```bash
# 对每个专辑条目：
# 1. 获取专辑名称
album_name=$(agent-browser get text @eXX)

# 2. 获取艺人
artist=$(agent-browser get text @eXX)

# 3. 获取评分
rating=$(agent-browser get text @eXX)

# 4. 写入 CSV（处理逗号转义）
echo "\"$album_name\",\"$artist\",\"$rating\"" >> "$CSV_FILE"
```

- [ ] **Step 3: 测试单页提取**

Run: `bash scripts/douban-export/03-extract-page.sh`
Expected: 生成包含第一页数据的 CSV 文件

- [ ] **Step 4: 验证 CSV 格式**

```bash
head -5 temp/douban-albums-page1.csv
wc -l temp/douban-albums-page1.csv
```

Expected: 显示正确的 CSV 格式，行数符合预期（约 15-20 条/页）

## Task 4: 分页遍历逻辑

**Files:**
- Create: `scripts/douban-export/04-paginate.sh`
- Modify: `temp/douban-albums-page1.csv` → `temp/douban-albums-all.csv`

- [ ] **Step 1: 创建分页循环框架**

```bash
#!/bin/bash
# 分页遍历所有专辑

CSV_FILE="temp/douban-albums-all.csv"
PAGE=1
MAX_PAGES=200  # 3515 张 / 15 张每页 ≈ 235 页
BASE_DELAY=4   # 秒

# 写入表头
echo "专辑名称，艺人，用户评分，专辑评分，发行年份，厂牌，听过时间，评论，URL" > "$CSV_FILE"

# 打开初始页面
agent-browser open "https://music.douban.com/mine?status=collect"
sleep 3

while [ $PAGE -le $MAX_PAGES ]; do
    echo "处理第 $PAGE 页..."
    
    # 提取当前页数据
    # ... (调用提取逻辑)
    
    # 检查是否有下一页
    # 如果没有下一页，退出循环
    
    # 点击下一页
    # agent-browser click @eXX
    
    # 等待加载
    agent-browser wait --load networkidle
    sleep $BASE_DELAY
    
    PAGE=$((PAGE + 1))
done
```

- [ ] **Step 2: 实现下一页检测和点击**

```bash
# 获取快照检查下一页按钮
agent-browser snapshot -i

# 检查是否存在下一页元素
# 如果不存在，break 循环

# 点击下一页
agent-browser click @eXX_next
```

- [ ] **Step 3: 实现数据追加逻辑**

```bash
# 每页提取的数据追加到 CSV（不含表头）
extract_page_data() >> "$CSV_FILE"
```

- [ ] **Step 4: 测试分页（前 3 页）**

Run: `bash scripts/douban-export/04-paginate.sh` (限制 MAX_PAGES=3)
Expected: 成功遍历 3 页，CSV 包含约 45-60 条记录

## Task 5: 反爬虫频率控制

**Files:**
- Modify: `scripts/douban-export/04-paginate.sh`
- Create: `scripts/douban-export/config.sh`

- [ ] **Step 1: 创建配置文件**

```bash
#!/bin/bash
# config.sh - 豆瓣导出配置

# 基础延迟（秒）
BASE_DELAY=4

# 延迟抖动范围（±秒）
DELAY_JITTER=1

# 最大重试次数
MAX_RETRIES=5

# 初始重试延迟（秒）
RETRY_DELAY=5

# 最大重试延迟（秒）
MAX_RETRY_DELAY=60

# 失败检测关键词
FAIL_KEYWORDS="访问太频繁|验证|captcha|403"
```

- [ ] **Step 2: 实现随机延迟函数**

```bash
random_delay() {
    local base=$1
    local jitter=$2
    local delay=$((base + RANDOM % (jitter * 2 + 1) - jitter))
    echo "等待 $delay 秒..."
    sleep $delay
}
```

- [ ] **Step 3: 集成到分页脚本**

```bash
source scripts/douban-export/config.sh

# 每页处理完后
random_delay $BASE_DELAY $DELAY_JITTER
```

- [ ] **Step 4: 测试频率控制**

Run: `bash scripts/douban-export/04-paginate.sh` (限制 MAX_PAGES=5)
Expected: 每页之间有 3-5 秒延迟

## Task 6: 失败检测和重试机制

**Files:**
- Modify: `scripts/douban-export/04-paginate.sh`
- Create: `scripts/douban-export/05-error-handling.sh`

- [ ] **Step 1: 实现失败检测函数**

```bash
check_blocked() {
    # 获取当前页面内容
    local content=$(agent-browser get title)
    
    # 检查是否包含失败关键词
    if echo "$content" | grep -qE "访问太频繁 | 验证|captcha"; then
        return 0  # 被阻止
    fi
    return 1  # 正常
}
```

- [ ] **Step 2: 实现指数退避重试**

```bash
retry_with_backoff() {
    local attempt=1
    local delay=$RETRY_DELAY
    
    while [ $attempt -le $MAX_RETRIES ]; do
        echo "检测到访问限制，第 $attempt 次重试，等待 $delay 秒..."
        sleep $delay
        
        # 刷新页面
        agent-browser open "https://music.douban.com/mine?status=collect"
        sleep 3
        
        # 检查是否恢复
        if ! check_blocked; then
            echo "访问恢复"
            return 0
        fi
        
        # 指数退避
        delay=$((delay * 2))
        if [ $delay -gt $MAX_RETRY_DELAY ]; then
            delay=$MAX_RETRY_DELAY
        fi
        
        attempt=$((attempt + 1))
    done
    
    return 1  # 所有重试失败
}
```

- [ ] **Step 3: 集成到主循环**

```bash
# 每页处理前检查
if check_blocked; then
    if ! retry_with_backoff; then
        echo "ERROR: 所有重试失败，通知用户"
        # 保存当前进度
        # 发送通知
        exit 1
    fi
fi
```

- [ ] **Step 4: 实现进度保存**

```bash
# 保存当前页码和已导出数量
echo "$PAGE" > temp/douban-export-progress.txt
```

- [ ] **Step 5: 测试重试逻辑**

手动触发失败场景（可选），验证重试机制

## Task 7: 用户通知机制

**Files:**
- Modify: `scripts/douban-export/04-paginate.sh`
- Create: `scripts/douban-export/notify-user.sh`

- [ ] **Step 1: 创建通知函数**

```bash
notify_failure() {
    local message="豆瓣导出失败：经过 $MAX_RETRIES 次重试仍无法访问，可能触发了反爬虫限制。当前进度：第 $PAGE 页，已导出 $COUNT 张专辑。"
    echo "$message"
    # 可以通过 message tool 或其他渠道通知
}
```

- [ ] **Step 2: 实现完成通知**

```bash
notify_success() {
    local count=$(wc -l < "$CSV_FILE")
    count=$((count - 1))  # 减去表头
    echo "导出完成：共 $count 张专辑，保存到 $CSV_FILE"
}
```

- [ ] **Step 3: 集成到脚本退出点**

```bash
trap notify_failure EXIT
# 成功时
notify_success
trap - EXIT
```

## Task 8: 完整测试（小规模）

**Files:**
- Create: `scripts/douban-export/test-run.sh`

- [ ] **Step 1: 创建测试脚本**

```bash
#!/bin/bash
# 测试运行 - 仅导出前 10 页

export MAX_PAGES=10
bash scripts/douban-export/04-paginate.sh
```

- [ ] **Step 2: 执行测试**

Run: `bash scripts/douban-export/test-run.sh`
Expected: 成功导出约 150-200 张专辑

- [ ] **Step 3: 验证数据质量**

```bash
# 检查行数
wc -l temp/douban-albums-all.csv

# 检查数据格式
head -10 temp/douban-albums-all.csv

# 检查是否有空值
grep -c '""' temp/douban-albums-all.csv
```

- [ ] **Step 4: 提交测试结果**

## Task 9: 全量执行

**Files:**
- Create: `scripts/douban-export/full-export.sh`

- [ ] **Step 1: 创建全量执行脚本**

```bash
#!/bin/bash
# 全量导出 - 3515 张专辑

export MAX_PAGES=250  # 留有余量
bash scripts/douban-export/04-paginate.sh
```

- [ ] **Step 2: 执行全量导出**

Run: `bash scripts/douban-export/full-export.sh`
Expected: 后台运行，预计耗时约 20-30 分钟

- [ ] **Step 3: 监控进度**

```bash
# 查看进度
cat temp/douban-export-progress.txt
wc -l temp/douban-albums-all.csv
```

- [ ] **Step 4: 验证最终结果**

Expected: CSV 包含约 3515 条记录

## Task 10: 数据清理和归档

**Files:**
- Output: `data/douban-albums-2026-03-23.csv`

- [ ] **Step 1: 移动最终文件**

```bash
mv temp/douban-albums-all.csv data/douban-albums-2026-03-23.csv
```

- [ ] **Step 2: 创建数据说明**

```markdown
# 豆瓣音乐专辑导出

导出时间：2026-03-23
总数：XXX 张专辑
字段：专辑名称，艺人，用户评分，专辑评分，发行年份，厂牌，听过时间，评论，URL
```

- [ ] **Step 3: 清理临时文件**

```bash
rm temp/douban-*.txt temp/douban-*.png
```

- [ ] **Step 4: 提交最终结果**

---

## 风险缓解

| 风险 | 缓解措施 |
|------|----------|
| 豆瓣反爬虫限制 | 低频请求 + 随机抖动 + 指数退避 |
| 页面结构变化 | 元素选择器可能失效，需要重新分析 |
| 网络中断 | 进度保存，可从断点恢复 |
| 数据不完整 | 部分字段可能为空，CSV 保留空值 |

## 预计耗时

- Task 1-3: 10 分钟（环境验证和单页提取）
- Task 4-6: 15 分钟（分页和错误处理）
- Task 7-8: 10 分钟（测试）
- Task 9: 20-30 分钟（全量导出，后台运行）
- Task 10: 5 分钟（清理归档）

**总计：约 60-70 分钟**
