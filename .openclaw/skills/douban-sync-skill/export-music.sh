#!/bin/bash
# 豆瓣音乐专辑导出 - 使用 agent-browser
# 会自动打开浏览器，请登录豆瓣后按 Enter 继续

set -e
cd /home/admin/openclaw/workspace

AGENT_BROWSER="/home/admin/.nvm/versions/node/v24.14.0/lib/node_modules/agent-browser/bin/agent-browser-linux-x64"
export AGENT_BROWSER_EXECUTABLE_PATH="/usr/bin/google-chrome"

USER_ID="${DOUBAN_USER:-4428030}"
OUTPUT_FILE="${OUTPUT_FILE:-temp/douban-music-collect.csv}"
BASE_URL="https://music.douban.com/people/${USER_ID}/collect?sort=time&mode=list"

echo "=== 豆瓣音乐专辑导出 ==="
echo "用户 ID: $USER_ID"
echo "输出文件：$OUTPUT_FILE"
echo ""

# 创建输出目录
mkdir -p $(dirname $OUTPUT_FILE)

# 写入 CSV 表头
echo "专辑名称，艺人，用户评分，听过时间，评论，URL" > "$OUTPUT_FILE"

# 打开页面
echo "打开豆瓣音乐页面..."
echo "如果浏览器未登录，请在打开的窗口中登录豆瓣"
echo ""

$AGENT_BROWSER open "$BASE_URL"

echo ""
echo "等待 30 秒让用户登录（如果需要）..."
for i in $(seq 1 6); do
    echo "  $((i*5))秒..."
    sleep 5
done

# 检查登录状态
TITLE=$($AGENT_BROWSER get title)
echo "页面标题：$TITLE"

if echo "$TITLE" | grep -qi "登录"; then
    echo ""
    echo "⚠️  未检测到登录状态"
    echo "请在浏览器窗口中登录豆瓣，然后按 Enter 继续..."
    read
fi

echo ""
echo "开始导出..."

COUNT=0
PAGE=1
MAX_PAGES=250

while [ $PAGE -le $MAX_PAGES ]; do
    echo ""
    echo "=== 第 $PAGE 页 ==="
    
    # 获取页面快照
    echo "获取页面快照..."
    $AGENT_BROWSER snapshot -i > temp/douban-snapshot.txt 2>&1 || true
    
    # 使用 JavaScript 提取数据
    echo "提取专辑数据..."
    
    EXTRACT_JS='() => {
        const items = document.querySelectorAll(".list-view .item, .article-list .item");
        const results = [];
        for (const item of items) {
            const titleElem = item.querySelector(".title a");
            const artistElem = item.querySelector(".artist, .meta a");
            const ratingElem = item.querySelector(".rating-star, .stars");
            const dateElem = item.querySelector(".date, .time");
            const commentElem = item.querySelector(".comment, .review");
            
            results.push({
                title: titleElem ? titleElem.textContent.trim() : "",
                url: titleElem ? titleElem.href : "",
                artist: artistElem ? artistElem.textContent.trim() : "",
                rating: ratingElem ? (ratingElem.className.match(/rating(\d+)/) || ["", ""])[1] : "",
                date: dateElem ? (dateElem.textContent.match(/\d{4}-\d{2}-\d{2}/) || [""])[0] : "",
                comment: commentElem ? commentElem.textContent.trim() : ""
            });
        }
        return results;
    }'
    
    # 执行提取
    RESULT=$($AGENT_BROWSER eval "$EXTRACT_JS" 2>/dev/null || echo "[]")
    echo "提取结果：$RESULT"
    
    # 解析 JSON 并写入 CSV（简化处理）
    # 由于 agent-browser eval 返回格式不确定，我们使用另一种方法
    
    # 获取专辑数量
    ITEM_COUNT=$($AGENT_BROWSER eval "() => document.querySelectorAll('.list-view .item, .article-list .item').length" 2>/dev/null || echo "0")
    echo "找到 $ITEM_COUNT 张专辑"
    
    if [ "$ITEM_COUNT" = "0" ] || [ "$ITEM_COUNT" = "[]" ]; then
        echo "未找到专辑，可能已到最后页"
        break
    fi
    
    # 逐个提取专辑信息
    for i in $(seq 0 $((ITEM_COUNT - 1))); do
        ITEM_JS="() => {
            const items = document.querySelectorAll('.list-view .item, .article-list .item');
            const item = items[$i];
            if (!item) return null;
            const titleElem = item.querySelector('.title a');
            const artistElem = item.querySelector('.artist, .meta a');
            const ratingElem = item.querySelector('.rating-star');
            const dateElem = item.querySelector('.date');
            const commentElem = item.querySelector('.comment');
            return {
                title: titleElem?.textContent?.trim() || '',
                url: titleElem?.href || '',
                artist: artistElem?.textContent?.trim() || '',
                rating: ratingElem?.className?.match(/rating(\d+)/)?.[1] || '',
                date: dateElem?.textContent?.match(/\d{4}-\d{2}-\d{2}/)?.[0] || '',
                comment: commentElem?.textContent?.trim() || ''
            };
        }"
        
        ITEM_DATA=$($AGENT_BROWSER eval "$ITEM_JS" 2>/dev/null || echo "{}")
        
        # 简单解析（实际应该用 jq）
        TITLE=$(echo "$ITEM_DATA" | grep -o '"title":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "")
        URL=$(echo "$ITEM_DATA" | grep -o '"url":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "")
        ARTIST=$(echo "$ITEM_DATA" | grep -o '"artist":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "")
        RATING=$(echo "$ITEM_DATA" | grep -o '"rating":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "")
        DATE=$(echo "$ITEM_DATA" | grep -o '"date":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "")
        COMMENT=$(echo "$ITEM_DATA" | grep -o '"comment":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "")
        
        # 转换为星级
        STARS=""
        if [ -n "$RATING" ] && [ "$RATING" != "" ]; then
            for j in $(seq 1 $RATING 2>/dev/null || echo ""); do
                STARS="${STARS}★"
            done
        fi
        
        # 写入 CSV
        echo "\"$TITLE\",\"$ARTIST\",\"$STARS\",\"$DATE\",\"$COMMENT\",\"$URL\"" >> "$OUTPUT_FILE"
        COUNT=$((COUNT + 1))
    done
    
    echo "已导出 $COUNT 张专辑"
    
    # 检查下一页
    echo "检查下一页..."
    HAS_NEXT=$($AGENT_BROWSER eval "() => { const next = document.querySelector('.paginator .next a, .next a'); return next && next.style.display !== 'none' ? 'yes' : 'no'; }" 2>/dev/null || echo "no")
    
    if [ "$HAS_NEXT" != "yes" ]; then
        echo "没有更多页面"
        break
    fi
    
    # 点击下一页
    echo "点击下一页..."
    $AGENT_BROWSER click ".paginator .next a, .next a" 2>/dev/null || true
    
    # 等待加载
    echo "等待页面加载..."
    sleep 4
    
    PAGE=$((PAGE + 1))
done

echo ""
echo "=== 导出完成 ==="
echo "共导出 $COUNT 张专辑"
echo "文件：$OUTPUT_FILE"

# 显示前几行
echo ""
echo "文件预览:"
head -10 "$OUTPUT_FILE"
